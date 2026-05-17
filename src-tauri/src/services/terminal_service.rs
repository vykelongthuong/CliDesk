use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

use chrono::Utc;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::models::{AppError, ShellConfig, TerminalExitEvent, TerminalSession, TerminalStatus};

/// Holds a running PTY session: the master end for resize, and a writer for sending input.
struct PtySession {
    master_pty: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

pub struct TerminalService {
    sessions: Arc<Mutex<HashMap<String, TerminalSession>>>,
    pty_sessions: Arc<Mutex<HashMap<String, PtySession>>>,
}

impl TerminalService {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            pty_sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_default_shell() -> Vec<ShellConfig> {
        let mut shells = Vec::new();

        #[cfg(target_os = "windows")]
        {
            shells.push(ShellConfig {
                id: "powershell".to_string(),
                label: "PowerShell".to_string(),
                executable: "powershell.exe".to_string(),
                args: vec!["-NoLogo".to_string()],
            });
            shells.push(ShellConfig {
                id: "pwsh".to_string(),
                label: "PowerShell Core".to_string(),
                executable: "pwsh.exe".to_string(),
                args: vec!["-NoLogo".to_string()],
            });
            shells.push(ShellConfig {
                id: "cmd".to_string(),
                label: "Command Prompt".to_string(),
                executable: "cmd.exe".to_string(),
                args: vec![],
            });
        }

        #[cfg(not(target_os = "windows"))]
        {
            let shell_env = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
            shells.push(ShellConfig {
                id: "default".to_string(),
                label: format!("Default ({})", shell_env),
                executable: shell_env,
                args: vec![],
            });
            shells.push(ShellConfig {
                id: "bash".to_string(),
                label: "Bash".to_string(),
                executable: "/bin/bash".to_string(),
                args: vec![],
            });
            shells.push(ShellConfig {
                id: "sh".to_string(),
                label: "SH".to_string(),
                executable: "/bin/sh".to_string(),
                args: vec![],
            });
        }

        shells
    }

    pub fn detect_shells() -> Vec<ShellConfig> {
        Self::get_default_shell()
    }

    /// Spawn a real shell process inside a PTY, start an I/O reader thread,
    /// and return the TerminalSession to the frontend.
    pub fn spawn_terminal(
        &self,
        project_id: &str,
        cwd: &str,
        shell_id: &str,
        cols: u16,
        rows: u16,
        app_handle: AppHandle,
    ) -> Result<TerminalSession, AppError> {
        let shells = Self::get_default_shell();
        let shell = shells
            .iter()
            .find(|s| s.id == shell_id)
            .unwrap_or(&shells[0]);

        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        // Log which shell we're spawning
        let shell_path = &shell.executable;
        log::info!(
            "Spawning terminal {} with shell: {} (args: {:?})",
            id,
            shell_path,
            shell.args,
        );

        // ── 1. Create the PTY ──────────────────────────────────────────
        let pty_system = native_pty_system();
        let size = PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        };

        let pair = pty_system
            .openpty(size)
            .map_err(|e| {
                AppError::new(
                    "PTY_OPEN_ERROR",
                    &format!("Cannot create pseudo-terminal: {}", e),
                )
            })?;

        // ── 2. Build the shell command ──────────────────────────────────
        let mut cmd = CommandBuilder::new(shell_path);
        for arg in &shell.args {
            cmd.arg(arg);
        }
        if !cwd.is_empty() {
            cmd.cwd(std::path::Path::new(cwd));
        }

        // ── 3. Spawn the child process inside the PTY ───────────────────
        let child = pair.slave.spawn_command(cmd).map_err(|e| {
            AppError::new(
                "PTY_SPAWN_ERROR",
                &format!("Cannot spawn shell '{}': {}", shell_path, e),
            )
        })?;

        // ── 4. Get the master-end reader + writer ──────────────────────
        let writer = pair.master.take_writer().map_err(|e| {
            AppError::new(
                "PTY_WRITER_ERROR",
                &format!("Cannot get PTY writer: {}", e),
            )
        })?;

        let reader = pair.master.try_clone_reader().map_err(|e| {
            AppError::new(
                "PTY_READER_ERROR",
                &format!("Cannot get PTY reader: {}", e),
            )
        })?;

        // ── 5. Store session & PTY state ───────────────────────────────
        let session = TerminalSession {
            id: id.clone(),
            project_id: project_id.to_string(),
            title: "Terminal".to_string(),
            cwd: cwd.to_string(),
            shell: shell.clone(),
            status: TerminalStatus::Running,
            exit_code: None,
            created_at: now,
        };

        let pty_session = PtySession {
            master_pty: pair.master,
            writer: Arc::new(Mutex::new(writer)),
        };

        self.sessions
            .lock()
            .unwrap()
            .insert(id.clone(), session.clone());
        self.pty_sessions
            .lock()
            .unwrap()
            .insert(id.clone(), pty_session);

        // ── 6. I/O reader thread ───────────────────────────────────────
        let id_clone = id.clone();
        let app_clone = app_handle.clone();
        let session_count = self.sessions.lock().unwrap().len();

        thread::spawn(move || {
            let mut reader = reader;
            let mut child = child;
            let mut buf = [0u8; 8192];

            log::info!("Reader thread started for terminal {}", id_clone);

            loop {
                match reader.read(&mut buf) {
                    Ok(0) => {
                        // EOF → child has exited
                        log::info!("EOF on terminal {} – process exited", id_clone);
                        let exit_code: Option<i32> = child
                            .try_wait()
                            .ok()
                            .flatten()
                            .map(|s| s.exit_code() as i32);
                        let payload = TerminalExitEvent {
                            terminal_id: id_clone.clone(),
                            exit_code,
                            reason: "exited".to_string(),
                            message: None,
                        };
                        let _ = app_clone.emit(
                            &format!("terminal://exit/{}", id_clone),
                            payload,
                        );
                        break;
                    }
                    Ok(n) => {
                        let data = buf[..n].to_vec();
                        let _ = app_clone.emit(
                            &format!("terminal://output/{}", id_clone),
                            data,
                        );
                    }
                    Err(e) => {
                        log::error!("Read error on terminal {}: {}", id_clone, e);
                        let payload = TerminalExitEvent {
                            terminal_id: id_clone.clone(),
                            exit_code: None,
                            reason: "error".to_string(),
                            message: Some(format!("{}", e)),
                        };
                        let _ = app_clone.emit(
                            &format!("terminal://exit/{}", id_clone),
                            payload,
                        );
                        break;
                    }
                }
            }

            log::info!("Reader thread ended for terminal {}", id_clone);
        });

        log::info!(
            "Terminal {} spawned successfully ({} active sessions)",
            id,
            session_count
        );
        Ok(session)
    }

    /// Write user input to the PTY master.
    pub fn write_input(&self, terminal_id: &str, data: &str) -> Result<(), AppError> {
        let pty_sessions = self.pty_sessions.lock().unwrap();
        if let Some(session) = pty_sessions.get(terminal_id) {
            let mut writer = session.writer.lock().unwrap();
            writer
                .write_all(data.as_bytes())
                .map_err(|e| AppError::new("PTY_WRITE_ERROR", &format!("{}", e)))?;
            writer
                .flush()
                .map_err(|e| AppError::new("PTY_FLUSH_ERROR", &format!("{}", e)))?;
            Ok(())
        } else {
            Err(AppError::new(
                "TERMINAL_NOT_FOUND",
                &format!("Terminal '{}' not found", terminal_id),
            ))
        }
    }

    /// Resize the PTY dimensions.
    pub fn resize_terminal(
        &self,
        terminal_id: &str,
        cols: u16,
        rows: u16,
    ) -> Result<(), AppError> {
        let pty_sessions = self.pty_sessions.lock().unwrap();
        if let Some(session) = pty_sessions.get(terminal_id) {
            session
                .master_pty
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| AppError::new("PTY_RESIZE_ERROR", &format!("{}", e)))?;
            Ok(())
        } else {
            Err(AppError::new(
                "TERMINAL_NOT_FOUND",
                &format!("Terminal '{}' not found", terminal_id),
            ))
        }
    }

    /// Stop a terminal process but keep its session metadata for the UI.
    pub fn stop_terminal(&self, terminal_id: &str) -> Result<(), AppError> {
        self.pty_sessions.lock().unwrap().remove(terminal_id);

        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(terminal_id) {
            session.status = TerminalStatus::Killed;
        } else {
            return Err(AppError::new(
                "TERMINAL_NOT_FOUND",
                &format!("Terminal '{}' not found", terminal_id),
            ));
        }

        log::info!("Terminal {} stopped", terminal_id);
        Ok(())
    }

    /// Close a terminal completely, removing process state and session metadata.
    pub fn close_terminal(&self, terminal_id: &str) -> Result<(), AppError> {
        self.pty_sessions.lock().unwrap().remove(terminal_id);

        let removed = self.sessions.lock().unwrap().remove(terminal_id);
        if removed.is_none() {
            return Err(AppError::new(
                "TERMINAL_NOT_FOUND",
                &format!("Terminal '{}' not found", terminal_id),
            ));
        }

        log::info!("Terminal {} closed", terminal_id);
        Ok(())
    }

    /// Backward-compatible stop operation for existing callers.
    pub fn kill_terminal(&self, terminal_id: &str) -> Result<(), AppError> {
        self.stop_terminal(terminal_id)
    }

    /// Kill all running terminals and clear session metadata.
    pub fn kill_all(&self) -> u32 {
        let pty_count = self.pty_sessions.lock().unwrap().len() as u32;
        self.pty_sessions.lock().unwrap().clear();

        let mut sessions = self.sessions.lock().unwrap();
        let session_count = sessions.len() as u32;
        sessions.clear();

        let count = pty_count.max(session_count);
        log::info!("Killed and cleared all {} terminal(s)", count);
        count
    }

    pub fn get_session(&self, terminal_id: &str) -> Option<TerminalSession> {
        self.sessions.lock().unwrap().get(terminal_id).cloned()
    }

    pub fn list_sessions(&self, project_id: &str) -> Vec<TerminalSession> {
        self.sessions
            .lock()
            .unwrap()
            .values()
            .filter(|s| s.project_id == project_id)
            .cloned()
            .collect()
    }
}
