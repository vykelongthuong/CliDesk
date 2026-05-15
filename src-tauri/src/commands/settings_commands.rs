use crate::models::AppError;
use crate::services::settings_service::SettingsService;
use crate::commands::project_commands::DbState;
use std::collections::HashMap;

#[tauri::command]
pub fn settings_get(db: tauri::State<DbState>) -> Result<HashMap<String, String>, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::new("LOCK_ERROR", "Failed to lock database"))?;
    let conn_ref = conn.as_ref().ok_or_else(|| AppError::new("DB_NOT_INIT", "Database not initialized"))?;
    SettingsService::get_all(conn_ref)
}

#[tauri::command]
pub fn settings_set(
    db: tauri::State<DbState>,
    key: String,
    value: String,
) -> Result<serde_json::Value, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::new("LOCK_ERROR", "Failed to lock database"))?;
    let conn_ref = conn.as_ref().ok_or_else(|| AppError::new("DB_NOT_INIT", "Database not initialized"))?;
    SettingsService::set(conn_ref, &key, &value)?;
    Ok(serde_json::json!({ "ok": true }))
}
