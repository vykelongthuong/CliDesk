use std::path::Path;
use std::fs;
use crate::models::{AppError, FileTreeItem, FileReadResult, FileWriteResult};
use crate::services::security_service::SecurityService;
use chrono::Utc;

pub struct FileService;

impl FileService {
    pub fn list_directory(project_root: &str, relative_path: &str) -> Result<Vec<FileTreeItem>, AppError> {
        let target = SecurityService::resolve_project_path_no_fs(Path::new(project_root), relative_path)?;

        if !target.exists() {
            return Err(AppError::new("DIR_NOT_FOUND", "Directory does not exist"));
        }
        if !target.is_dir() {
            return Err(AppError::new("NOT_DIRECTORY", "Path is not a directory"));
        }

        let mut items = Vec::new();
        let mut entries: Vec<_> = fs::read_dir(&target)
            .map_err(|e| AppError::new("IO_ERROR", &format!("Failed to read directory: {}", e)))?
            .filter_map(|e| e.ok())
            .collect();

        entries.sort_by(|a, b| {
            let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
            b_is_dir.cmp(&a_is_dir).then(a.file_name().cmp(&b.file_name()))
        });

        // Ignored paths
        let ignored: Vec<&str> = vec![
            ".git", "node_modules", "dist", "build", ".next", ".nuxt",
            "target", "coverage", ".cache", "vendor",
        ];

        for entry in entries {
            let name = entry.file_name().to_string_lossy().to_string();
            if ignored.contains(&name.as_str()) {
                continue;
            }

            let file_type = entry.file_type().map_err(|e| AppError::new("IO_ERROR", &format!("Failed to get file type: {}", e)))?;
            let metadata = entry.metadata().ok();
            let item_relative_path = if relative_path.is_empty() || relative_path == "." {
                name.clone()
            } else {
                format!("{}/{}", relative_path, name)
            };

            items.push(FileTreeItem {
                name,
                relative_path: item_relative_path,
                kind: if file_type.is_dir() {
                    "directory".to_string()
                } else if file_type.is_symlink() {
                    "symlink".to_string()
                } else {
                    "file".to_string()
                },
                size: metadata.as_ref().map(|m| m.len() as i64),
                modified_at: metadata.as_ref().and_then(|m| {
                    m.modified().ok().map(|t| {
                        let dt: chrono::DateTime<chrono::Utc> = t.into();
                        dt.to_rfc3339()
                    })
                }),
            });
        }

        Ok(items)
    }

    pub fn read_file(project_root: &str, relative_path: &str) -> Result<FileReadResult, AppError> {
        let target = SecurityService::resolve_project_path_no_fs(Path::new(project_root), relative_path)?;

        if !target.exists() {
            return Err(AppError::new("FILE_NOT_FOUND", "File does not exist"));
        }
        if !target.is_file() {
            return Err(AppError::new("NOT_FILE", "Path is not a file"));
        }

        // Check binary
        if SecurityService::is_binary_file(&target) {
            return Err(AppError::new("BINARY_FILE", "Cannot open binary file as text"));
        }

        let metadata = target.metadata()
            .map_err(|e| AppError::new("IO_ERROR", &format!("Failed to get metadata: {}", e)))?;

        let size = metadata.len() as i64;

        // Check file size (2MB limit for editable, 10MB for reading)
        if size > 10_485_760 {
            return Err(AppError::new("FILE_TOO_LARGE", "File exceeds maximum size limit of 10MB"));
        }
        let read_only = size > 2_097_152;

        let content = fs::read_to_string(&target)
            .map_err(|e| AppError::new("IO_ERROR", &format!("Failed to read file: {}", e)))?;

        let language_id = Self::detect_language(&target);

        let modified_at = metadata.modified()
            .ok()
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_else(|| Utc::now().to_rfc3339());

        // Calculate relative path for output
        let rel_path = if relative_path.is_empty() {
            target.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        } else {
            relative_path.to_string()
        };

        Ok(FileReadResult {
            project_id: project_root.to_string(),
            relative_path: rel_path,
            content,
            language_id,
            size,
            modified_at,
            read_only,
        })
    }

    pub fn write_file(project_root: &str, relative_path: &str, content: &str) -> Result<FileWriteResult, AppError> {
        let target = SecurityService::resolve_project_path_no_fs(Path::new(project_root), relative_path)?;

        // Ensure parent directory exists
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AppError::new("IO_ERROR", &format!("Failed to create parent directory: {}", e)))?;
        }

        // Atomic write: write to temp file then rename
        let temp_path = target.with_extension("tmp");
        fs::write(&temp_path, content)
            .map_err(|e| AppError::new("IO_ERROR", &format!("Failed to write file: {}", e)))?;

        fs::rename(&temp_path, &target)
            .map_err(|e| AppError::new("IO_ERROR", &format!("Failed to finalize file write: {}", e)))?;

        let metadata = target.metadata()
            .map_err(|e| AppError::new("IO_ERROR", &format!("Failed to get metadata: {}", e)))?;

        let modified_at = metadata.modified()
            .ok()
            .map(|t| {
                let dt: chrono::DateTime<chrono::Utc> = t.into();
                dt.to_rfc3339()
            })
            .unwrap_or_else(|| Utc::now().to_rfc3339());

        Ok(FileWriteResult {
            ok: true,
            modified_at,
        })
    }

    fn detect_language(path: &Path) -> Option<String> {
        let ext = path.extension()?.to_string_lossy().to_lowercase();
        let lang = match ext.as_str() {
            "ts" | "tsx" => "typescript",
            "js" | "jsx" | "mjs" => "javascript",
            "rs" => "rust",
            "py" => "python",
            "go" => "go",
            "java" => "java",
            "c" | "h" => "c",
            "cpp" | "hpp" | "cc" | "cxx" => "cpp",
            "cs" => "csharp",
            "html" | "htm" => "html",
            "css" | "scss" | "less" => "css",
            "json" => "json",
            "xml" | "svg" => "xml",
            "yaml" | "yml" => "yaml",
            "md" | "markdown" => "markdown",
            "sql" => "sql",
            "sh" | "bash" | "zsh" => "shell",
            "ps1" => "powershell",
            "toml" => "toml",
            "ini" | "cfg" | "conf" => "ini",
            "vue" => "vue",
            "svelte" => "svelte",
            "rb" => "ruby",
            "php" => "php",
            "swift" => "swift",
            "kt" | "kts" => "kotlin",
            "dart" => "dart",
            "lua" => "lua",
            "r" => "r",
            _ => return None,
        };
        Some(lang.to_string())
    }
}
