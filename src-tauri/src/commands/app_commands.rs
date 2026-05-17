use crate::models::{AppError, AppRuntimeInfo, UpdateResult};
use std::process::Command;

#[tauri::command]
pub fn app_runtime_info() -> Result<AppRuntimeInfo, AppError> {
    let current_version = std::env::var("CLIDESK_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    let latest_version = std::env::var("CLIDESK_LATEST_VERSION")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let update_available = std::env::var("CLIDESK_UPDATE_AVAILABLE")
        .ok()
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or_else(|| {
            latest_version
                .as_ref()
                .map(|latest| is_version_newer(latest, &current_version))
                .unwrap_or(false)
        });

    let launch_language = std::env::var("CLIDESK_LAUNCH_LANG")
        .ok()
        .filter(|value| value == "vi" || value == "en");

    Ok(AppRuntimeInfo {
        current_version,
        latest_version,
        update_available,
        launch_language,
        update_command: "npm i -g clidesk".to_string(),
    })
}

#[tauri::command]
pub async fn app_update_from_npm() -> Result<UpdateResult, AppError> {
    tauri::async_runtime::spawn_blocking(run_npm_update)
        .await
        .map_err(|err| AppError::new("UPDATE_JOIN_ERROR", &format!("Failed to run update task: {}", err)))?
}

fn run_npm_update() -> Result<UpdateResult, AppError> {
    let mut command = if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/d", "/c", "npm", "i", "-g", "clidesk", "--no-audit", "--no-fund"]);
        command
    } else {
        let mut command = Command::new("npm");
        command.args(["i", "-g", "clidesk", "--no-audit", "--no-fund"]);
        command
    };

    command
        .env("npm_config_loglevel", "error")
        .env("npm_config_logs_max", "0")
        .env("npm_config_update_notifier", "false")
        .env("NO_UPDATE_NOTIFIER", "1");

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    let output = command
        .output()
        .map_err(|err| AppError::new("UPDATE_SPAWN_ERROR", &format!("Failed to start npm: {}", err)))?;

    if output.status.success() {
        return Ok(UpdateResult {
            ok: true,
            message: "CliDesk update completed. Restart CliDesk to use the updated version.".to_string(),
        });
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = first_non_empty_lines(&stderr, &stdout);

    Err(AppError::new(
        "UPDATE_FAILED",
        &format!("npm i -g clidesk failed. {}", detail),
    ))
}

fn first_non_empty_lines(primary: &str, fallback: &str) -> String {
    let mut lines: Vec<String> = primary
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(6)
        .map(ToString::to_string)
        .collect();

    if lines.is_empty() {
        lines = fallback
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .take(6)
            .map(ToString::to_string)
            .collect();
    }

    if lines.is_empty() {
        return "npm exited without an error message.".to_string();
    }

    lines.join(" ")
}

fn is_version_newer(latest: &str, current: &str) -> bool {
    let latest_parts = parse_version_parts(latest);
    let current_parts = parse_version_parts(current);

    for index in 0..3 {
        if latest_parts[index] > current_parts[index] {
            return true;
        }
        if latest_parts[index] < current_parts[index] {
            return false;
        }
    }

    false
}

fn parse_version_parts(version: &str) -> [u64; 3] {
    let mut parts = [0_u64; 3];
    for (index, segment) in version
        .split(|ch| ch == '.' || ch == '-' || ch == '+')
        .take(3)
        .enumerate()
    {
        if let Ok(value) = segment.parse::<u64>() {
            parts[index] = value;
        }
    }
    parts
}
