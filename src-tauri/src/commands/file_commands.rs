use crate::models::{AppError, FileTreeItem, FileReadResult, FileWriteResult};
use crate::services::file_service::FileService;

#[tauri::command]
pub fn fs_list_dir(
    project_id: String,
    relative_path: String,
) -> Result<Vec<FileTreeItem>, AppError> {
    FileService::list_directory(&project_id, &relative_path)
}

#[tauri::command]
pub fn fs_read_file(
    project_id: String,
    relative_path: String,
) -> Result<FileReadResult, AppError> {
    FileService::read_file(&project_id, &relative_path)
}

#[tauri::command]
pub fn fs_write_file(
    project_id: String,
    relative_path: String,
    content: String,
) -> Result<FileWriteResult, AppError> {
    FileService::write_file(&project_id, &relative_path, &content)
}
