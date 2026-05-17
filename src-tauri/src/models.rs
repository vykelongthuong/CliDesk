use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_opened_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSession {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub cwd: String,
    pub shell: ShellConfig,
    pub status: TerminalStatus,
    pub exit_code: Option<i32>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellConfig {
    pub id: String,
    pub label: String,
    pub executable: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TerminalStatus {
    #[serde(rename = "starting")]
    Starting,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "exited")]
    Exited,
    #[serde(rename = "killed")]
    Killed,
    #[serde(rename = "error")]
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeItem {
    pub name: String,
    pub relative_path: String,
    pub kind: String,
    pub size: Option<i64>,
    pub modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReadResult {
    pub project_id: String,
    pub relative_path: String,
    pub content: String,
    pub language_id: Option<String>,
    pub size: i64,
    pub modified_at: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileWriteResult {
    pub ok: bool,
    pub modified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatus {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub ahead: Option<i32>,
    pub behind: Option<i32>,
    pub files: Vec<GitFileStatus>,
    #[serde(default)]
    pub slow_mode: bool,
    #[serde(default)]
    pub skipped_untracked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFileStatus {
    pub path: String,
    pub index_status: String,
    pub working_tree_status: String,
    pub display_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffResult {
    pub path: String,
    pub diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.to_string(),
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalExitEvent {
    pub terminal_id: String,
    pub exit_code: Option<i32>,
    pub reason: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRuntimeInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub launch_language: Option<String>,
    pub update_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResult {
    pub ok: bool,
    pub message: String,
}
