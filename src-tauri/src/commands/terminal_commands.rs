use crate::commands::project_commands::DbState;
use crate::models::{AppError, TerminalSession, ShellConfig};
use crate::services::project_service::ProjectService;
use crate::services::terminal_service::TerminalService;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, State};

pub struct TerminalState(pub TerminalService);

#[tauri::command]
pub fn terminal_spawn(
    app: AppHandle,
    db: State<DbState>,
    terminal: State<TerminalState>,
    project_id: String,
    cwd_relative_path: Option<String>,
    shell_id: Option<String>,
    cols: u16,
    rows: u16,
    elevated: Option<bool>,
) -> Result<TerminalSession, AppError> {
    let shell = shell_id.unwrap_or_else(|| {
        #[cfg(target_os = "windows")]
        { "powershell".to_string() }
        #[cfg(not(target_os = "windows"))]
        { "default".to_string() }
    });
    let is_elevated = elevated.unwrap_or(false);

    let project = {
        let conn = db
            .0
            .lock()
            .map_err(|_| AppError::new("LOCK_ERROR", "Failed to lock database"))?;
        let conn_ref = conn
            .as_ref()
            .ok_or_else(|| AppError::new("DB_NOT_INIT", "Database not initialized"))?;
        ProjectService::select_project(conn_ref, &project_id)?
    };

    let cwd_path = resolve_terminal_cwd(&project.path, cwd_relative_path.as_deref())?;
    let cwd = cwd_path.to_string_lossy().to_string();

    log::info!(
        "Spawning terminal for project_id={} project_path={} resolved_cwd={} shell={} elevated={}",
        project_id,
        project.path,
        cwd,
        shell,
        is_elevated
    );

    let session = terminal.0.spawn_terminal(&project_id, &cwd, &shell, cols, rows, is_elevated, app)?;
    Ok(session)
}

fn resolve_terminal_cwd(
    project_path: &str,
    cwd_relative_path: Option<&str>,
) -> Result<PathBuf, AppError> {
    let project_root = PathBuf::from(project_path);
    if !project_root.exists() {
        return Err(AppError::new(
            "TERMINAL_CWD_NOT_FOUND",
            &format!("Project path does not exist: {}", project_path),
        ));
    }
    if !project_root.is_dir() {
        return Err(AppError::new(
            "TERMINAL_CWD_NOT_DIRECTORY",
            &format!("Project path is not a directory: {}", project_path),
        ));
    }

    let canonical_root = project_root.canonicalize().map_err(|e| {
        AppError::new(
            "TERMINAL_CWD_NOT_FOUND",
            &format!("Cannot resolve project path '{}': {}", project_path, e),
        )
    })?;

    let relative = cwd_relative_path.unwrap_or_default().trim();
    let candidate = if relative.is_empty() {
        canonical_root.clone()
    } else {
        let relative_path = Path::new(relative);
        if relative_path.is_absolute() {
            return Err(AppError::new(
                "TERMINAL_CWD_OUTSIDE_PROJECT",
                "Terminal cwd must be relative to the selected project",
            ));
        }
        canonical_root.join(relative_path)
    };

    if !candidate.exists() {
        return Err(AppError::new(
            "TERMINAL_CWD_NOT_FOUND",
            &format!("Terminal cwd does not exist: {}", candidate.display()),
        ));
    }
    if !candidate.is_dir() {
        return Err(AppError::new(
            "TERMINAL_CWD_NOT_DIRECTORY",
            &format!("Terminal cwd is not a directory: {}", candidate.display()),
        ));
    }

    let canonical_cwd = candidate.canonicalize().map_err(|e| {
        AppError::new(
            "TERMINAL_CWD_NOT_FOUND",
            &format!("Cannot resolve terminal cwd '{}': {}", candidate.display(), e),
        )
    })?;

    if !canonical_cwd.starts_with(&canonical_root) {
        return Err(AppError::new(
            "TERMINAL_CWD_OUTSIDE_PROJECT",
            &format!(
                "Terminal cwd '{}' is outside project '{}',",
                canonical_cwd.display(),
                canonical_root.display()
            ),
        ));
    }

    Ok(canonical_cwd)
}

#[tauri::command]
pub fn terminal_write(
    terminal: State<TerminalState>,
    terminal_id: String,
    data: String,
) -> Result<serde_json::Value, AppError> {
    terminal.0.write_input(&terminal_id, &data)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
pub fn terminal_resize(
    terminal: State<TerminalState>,
    terminal_id: String,
    cols: u16,
    rows: u16,
) -> Result<serde_json::Value, AppError> {
    terminal.0.resize_terminal(&terminal_id, cols, rows)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
pub fn terminal_kill(
    terminal: State<TerminalState>,
    terminal_id: String,
) -> Result<serde_json::Value, AppError> {
    terminal.0.stop_terminal(&terminal_id)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
pub fn terminal_close(
    terminal: State<TerminalState>,
    terminal_id: String,
) -> Result<serde_json::Value, AppError> {
    terminal.0.close_terminal(&terminal_id)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
pub fn terminal_kill_all(
    terminal: State<TerminalState>,
) -> Result<serde_json::Value, AppError> {
    let killed = terminal.0.kill_all();
    Ok(serde_json::json!({ "ok": true, "killed": killed }))
}

#[tauri::command]
pub fn terminal_list(
    terminal: State<TerminalState>,
    project_id: String,
) -> Result<Vec<TerminalSession>, AppError> {
    Ok(terminal.0.list_sessions(&project_id))
}

#[tauri::command]
pub fn shell_list() -> Vec<ShellConfig> {
    TerminalService::detect_shells()
}

#[tauri::command]
pub fn is_elevated() -> bool {
    TerminalService::is_elevated()
}

#[tauri::command]
pub fn restart_as_admin(
    terminal: State<TerminalState>,
) -> Result<(), AppError> {
    // Kill all terminal child processes first
    terminal.0.kill_all();
    // Then restart as admin
    TerminalService::restart_as_admin()
}
