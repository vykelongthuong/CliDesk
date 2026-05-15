use tauri::Manager;

#[tauri::command]
pub fn window_hide(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn window_show(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) -> Result<(), String> {
    // Kill all terminal child processes first, then exit
    let terminal_state = app.state::<crate::commands::terminal_commands::TerminalState>();
    terminal_state.0.kill_all();
    app.exit(0);
    Ok(())
}
