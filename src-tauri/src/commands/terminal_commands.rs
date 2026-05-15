use crate::models::{AppError, TerminalSession, ShellConfig};
use crate::services::terminal_service::TerminalService;
use tauri::{AppHandle, State};

pub struct TerminalState(pub TerminalService);

#[tauri::command]
pub fn terminal_spawn(
    app: AppHandle,
    terminal: State<TerminalState>,
    project_id: String,
    cwd_relative_path: Option<String>,
    shell_id: Option<String>,
    cols: u16,
    rows: u16,
    elevated: Option<bool>,
) -> Result<TerminalSession, AppError> {
    let cwd = cwd_relative_path.unwrap_or_default();
    let shell = shell_id.unwrap_or_else(|| {
        #[cfg(target_os = "windows")]
        { "powershell".to_string() }
        #[cfg(not(target_os = "windows"))]
        { "default".to_string() }
    });
    let is_elevated = elevated.unwrap_or(false);

    let session = terminal.0.spawn_terminal(&project_id, &cwd, &shell, cols, rows, is_elevated, app)?;
    Ok(session)
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
    terminal.0.kill_terminal(&terminal_id)?;
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
