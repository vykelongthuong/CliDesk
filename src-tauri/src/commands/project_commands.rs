use crate::models::{AppError, Project};
use crate::services::project_service::ProjectService;
use rusqlite::Connection;
use std::sync::Mutex;
use tauri::State;

pub struct DbState(pub Mutex<Option<Connection>>);

#[tauri::command]
pub fn project_list(db: State<DbState>) -> Result<Vec<Project>, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::new("LOCK_ERROR", "Failed to lock database"))?;
    let conn_ref = conn.as_ref().ok_or_else(|| AppError::new("DB_NOT_INIT", "Database not initialized"))?;
    ProjectService::list_projects(conn_ref)
}

#[tauri::command]
pub fn project_add(db: State<DbState>, path: String) -> Result<Project, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::new("LOCK_ERROR", "Failed to lock database"))?;
    let conn_ref = conn.as_ref().ok_or_else(|| AppError::new("DB_NOT_INIT", "Database not initialized"))?;
    ProjectService::add_project(conn_ref, &path)
}

#[tauri::command]
pub fn project_remove(db: State<DbState>, project_id: String) -> Result<serde_json::Value, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::new("LOCK_ERROR", "Failed to lock database"))?;
    let conn_ref = conn.as_ref().ok_or_else(|| AppError::new("DB_NOT_INIT", "Database not initialized"))?;
    ProjectService::remove_project(conn_ref, &project_id)?;
    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
pub fn project_select(db: State<DbState>, project_id: String) -> Result<Project, AppError> {
    let conn = db.0.lock().map_err(|_| AppError::new("LOCK_ERROR", "Failed to lock database"))?;
    let conn_ref = conn.as_ref().ok_or_else(|| AppError::new("DB_NOT_INIT", "Database not initialized"))?;
    ProjectService::select_project(conn_ref, &project_id)
}
