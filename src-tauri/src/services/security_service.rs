use std::path::{Path, PathBuf};
use crate::models::AppError;

pub struct SecurityService;

impl SecurityService {
    pub fn resolve_project_path_no_fs(project_root: &Path, relative_path: &str) -> Result<PathBuf, AppError> {
        // Basic path traversal check without requiring canonicalization
        let root = project_root.to_path_buf();

        // Normalize relative path
        let clean_relative = relative_path
            .replace('\\', "/")
            .split('/')
            .fold(String::new(), |acc, part| {
                if part == ".." {
                    // Remove last component
                    let mut parts: Vec<&str> = acc.split('/').collect();
                    parts.pop();
                    parts.join("/")
                } else if part == "." || part.is_empty() {
                    acc
                } else {
                    if acc.is_empty() {
                        part.to_string()
                    } else {
                        format!("{}/{}", acc, part)
                    }
                }
            });

        let target = root.join(&clean_relative);

        // Check that the target starts with root (basic traversal prevention)
        let target_str = target.to_string_lossy().to_lowercase();
        let root_str = root.to_string_lossy().to_lowercase();
        if !target_str.starts_with(&root_str) {
            return Err(AppError::new("PATH_TRAVERSAL", "Path escapes project root"));
        }

        Ok(target)
    }

    pub fn is_binary_file(path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            matches!(
                ext.as_str(),
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico"
                    | "webp"
                    | "exe" | "dll" | "so" | "dylib"
                    | "zip" | "tar" | "gz" | "rar" | "7z"
                    | "pdf" | "doc" | "docx" | "xls" | "xlsx"
                    | "mp3" | "mp4" | "avi" | "mov" | "wav"
                    | "ttf" | "otf" | "woff" | "woff2"
                    | "o" | "obj" | "class" | "pyc"
            )
        } else {
            false
        }
    }
}
