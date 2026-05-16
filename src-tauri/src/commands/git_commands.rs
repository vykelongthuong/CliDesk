use crate::commands::project_commands::DbState;
use crate::models::{AppError, GitDiffResult, GitStatus};
use crate::services::git_service::GitService;
use crate::services::project_service::ProjectService;
use std::path::Path;
use tauri::State;

fn resolve_project_root(db: State<DbState>, project_id: &str) -> Result<String, AppError> {
    let conn = db
        .0
        .lock()
        .map_err(|_| AppError::new("LOCK_ERROR", "Failed to lock database"))?;
    let conn_ref = conn
        .as_ref()
        .ok_or_else(|| AppError::new("DB_NOT_INIT", "Database not initialized"))?;
    let project = ProjectService::get_project_by_id(conn_ref, project_id)?;

    let project_path = Path::new(&project.path);
    if !project_path.exists() {
        return Err(AppError::new(
            "PROJECT_PATH_NOT_FOUND",
            &format!("Project path does not exist: {}", project.path),
        ));
    }
    if !project_path.is_dir() {
        return Err(AppError::new(
            "PROJECT_PATH_NOT_FOUND",
            &format!("Project path is not a directory: {}", project.path),
        ));
    }

    Ok(project.path)
}

#[tauri::command]
pub fn git_status(db: State<DbState>, project_id: String) -> Result<GitStatus, AppError> {
    let project_root = resolve_project_root(db, &project_id)?;
    GitService::get_status(&project_root)
}

#[tauri::command]
pub fn git_diff(
    db: State<DbState>,
    project_id: String,
    relative_path: String,
    staged: Option<bool>,
) -> Result<GitDiffResult, AppError> {
    let project_root = resolve_project_root(db, &project_id)?;
    GitService::get_diff(&project_root, &relative_path, staged.unwrap_or(false))
}
