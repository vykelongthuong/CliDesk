use crate::models::{AppError, GitStatus, GitFileStatus, GitDiffResult};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;
use std::thread;

const GIT_TIMEOUT_SECS: u64 = 5;

/// Run git command with timeout by executing std::process::Command::output()
/// in a separate thread so we can enforce a deadline.
fn run_git_output_with_timeout(
    project_root: &str,
    args: &[&str],
    label: &str,
) -> Result<(bool, String), AppError> {
    let project = project_root.to_string();
    let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let result = Command::new("git")
            .args(&args_owned)
            .current_dir(&project)
            .output();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_secs(GIT_TIMEOUT_SECS)) {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Ok((output.status.success(), stdout))
        }
        Ok(Err(e)) => {
            Err(AppError::new(
                "GIT_COMMAND_ERROR",
                &format!("Failed to run {}. Is Git installed and in PATH? {}", label, e),
            ))
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err(AppError::new(
                "GIT_TIMEOUT",
                &format!("{} took too long (>{}s)", label, GIT_TIMEOUT_SECS),
            ))
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err(AppError::new("GIT_COMMAND_ERROR", &format!("{} process panicked", label)))
        }
    }
}

pub struct GitService;

impl GitService {
    pub fn new() -> Self {
        Self
    }

    pub fn get_status(project_root: &str) -> Result<GitStatus, AppError> {
        // Check if it's a git repo (rev-parse is fast, no timeout needed normally)
        let (_ok, _out) = run_git_output_with_timeout(
            project_root,
            &["rev-parse", "--is-inside-work-tree"],
            "git rev-parse",
        )?;
        let is_repo = _ok;

        if !is_repo {
            return Ok(GitStatus {
                is_repo: false,
                branch: None,
                ahead: None,
                behind: None,
                files: vec![],
                slow_mode: false,
                skipped_untracked: false,
            });
        }

        // Get branch name
        let branch = run_git_output_with_timeout(
            project_root,
            &["rev-parse", "--abbrev-ref", "HEAD"],
            "git rev-parse (branch)",
        )
            .ok()
            .filter(|(ok, _)| *ok)
            .map(|(_, out)| out.trim().to_string());

        // Try normal status first with --untracked-files=normal
        let status_result = run_git_output_with_timeout(
            project_root,
            &["status", "--porcelain=v1", "-b", "--untracked-files=normal"],
            "git status",
        );

        let (output_str, slow_mode, skipped_untracked) = match status_result {
            Ok((true, out)) => (out, false, false),
            Ok((false, out)) => (out, false, false),
            Err(err) if err.code == "GIT_TIMEOUT" => {
                // Retry without untracked files — much faster on large repos
                match run_git_output_with_timeout(
                    project_root,
                    &["status", "--porcelain=v1", "-b", "--untracked-files=no"],
                    "git status (retry without untracked)",
                ) {
                    Ok((true, out)) => (out, true, true),
                    Ok((false, out)) => (out, true, true),
                    Err(_) => return Err(err), // Both failed — return original timeout
                }
            }
            Err(err) => return Err(err),
        };

        let lines: Vec<&str> = output_str.lines().collect();
        let mut files = Vec::new();
        let mut ahead: Option<i32> = None;
        let mut behind: Option<i32> = None;

        for line in &lines {
            if line.starts_with("## ") {
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
                let path = if line.len() > 3 {
                    line[3..].trim().to_string()
                } else {
                    String::new()
                };

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
            slow_mode,
            skipped_untracked,
        })
    }

    pub fn get_diff(
        project_root: &str,
        relative_path: &str,
        staged: bool,
    ) -> Result<GitDiffResult, AppError> {
        let mut args = vec!["diff"];
        if staged {
            args.push("--cached");
        }
        args.push("--");
        args.push(relative_path);

        let label = if staged { "git diff --cached" } else { "git diff" };
        let (_ok, out) = run_git_output_with_timeout(project_root, &args, label)?;

        Ok(GitDiffResult {
            path: relative_path.to_string(),
            diff: out,
        })
    }
}
