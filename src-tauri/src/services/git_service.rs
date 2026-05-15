use crate::models::{AppError, GitStatus, GitFileStatus, GitDiffResult};
use std::process::Command;

pub struct GitService;

impl GitService {
    pub fn new() -> Self {
        Self
    }

    pub fn get_status(project_root: &str) -> Result<GitStatus, AppError> {
        // Check if it's a git repo
        let is_repo = Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .current_dir(project_root)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !is_repo {
            return Ok(GitStatus {
                is_repo: false,
                branch: None,
                ahead: None,
                behind: None,
                files: vec![],
            });
        }

        // Get branch info
        let branch = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(project_root)
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout).ok().map(|s| s.trim().to_string())
                } else {
                    None
                }
            });

        // Get status
        let status_output = Command::new("git")
            .args(["status", "--porcelain=v1", "-b"])
            .current_dir(project_root)
            .output()
            .map_err(|e| AppError::new("GIT_COMMAND_ERROR", &format!("Failed to run git status: {}", e)))?;

        let output_str = String::from_utf8_lossy(&status_output.stdout);
        let lines: Vec<&str> = output_str.lines().collect();

        let mut files = Vec::new();
        let mut ahead: Option<i32> = None;
        let mut behind: Option<i32> = None;

        for line in &lines {
            if line.starts_with("## ") {
                // Branch line: "## main...origin/main [ahead 1, behind 2]"
                let branch_info = &line[3..];
                if let Some(bracket_start) = branch_info.find('[') {
                    if let Some(bracket_end) = branch_info.find(']') {
                        let status_part = &branch_info[bracket_start + 1..bracket_end];
                        if status_part.contains("ahead") {
                            ahead = status_part.split(',').find_map(|s| {
                                let s = s.trim();
                                if s.starts_with("ahead") {
                                    s.split_whitespace().nth(1).and_then(|n| n.parse().ok())
                                } else {
                                    None
                                }
                            });
                        }
                        if status_part.contains("behind") {
                            behind = status_part.split(',').find_map(|s| {
                                let s = s.trim();
                                if s.starts_with("behind") {
                                    s.split_whitespace().nth(1).and_then(|n| n.parse().ok())
                                } else {
                                    None
                                }
                            });
                        }
                    }
                }
            } else if line.len() >= 3 {
                let index_status = line[..1].to_string();
                let working_tree_status = line[1..2].to_string();
                let path = if line.len() > 3 { line[3..].trim().to_string() } else { String::new() };

                let display_status = match (&index_status[..], &working_tree_status[..]) {
                    ("M", _) | (_, "M") => "modified",
                    ("A", _) => "added",
                    ("D", _) | (_, "D") => "deleted",
                    ("R", _) => "renamed",
                    ("?", "?") => "untracked",
                    ("U", _) | (_, "U") => "conflicted",
                    _ => "unknown",
                };

                files.push(GitFileStatus {
                    path,
                    index_status,
                    working_tree_status,
                    display_status: display_status.to_string(),
                });
            }
        }

        Ok(GitStatus {
            is_repo: true,
            branch,
            ahead,
            behind,
            files,
        })
    }

    pub fn get_diff(project_root: &str, relative_path: &str, staged: bool) -> Result<GitDiffResult, AppError> {
        let mut args = vec!["diff"];
        if staged {
            args.push("--cached");
        }
        args.push("--");
        args.push(relative_path);

        let output = Command::new("git")
            .args(&args)
            .current_dir(project_root)
            .output()
            .map_err(|e| AppError::new("GIT_COMMAND_ERROR", &format!("Failed to run git diff: {}", e)))?;

        let diff = String::from_utf8_lossy(&output.stdout).to_string();

        Ok(GitDiffResult {
            path: relative_path.to_string(),
            diff,
        })
    }
}
