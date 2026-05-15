use crate::models::{AppError, GitStatus, GitDiffResult};
use crate::services::git_service::GitService;

#[tauri::command]
pub fn git_status(project_id: String) -> Result<GitStatus, AppError> {
    GitService::get_status(&project_id)
}

#[tauri::command]
pub fn git_diff(
    project_id: String,
    relative_path: String,
    staged: Option<bool>,
) -> Result<GitDiffResult, AppError> {
    GitService::get_diff(&project_id, &relative_path, staged.unwrap_or(false))
}
