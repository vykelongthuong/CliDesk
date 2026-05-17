// CliDesk Launcher — Native Windows console app
//
// Uses raw WinAPI FFI (kernel32.dll + user32.dll) — no external crate.
// This avoids the windows crate's build-script dependency issues while
// keeping the same Windows Job Object lifecycle guarantees.
//
// Responsibilities:
//   1. Present interactive menu (hidden / visible mode) or accept CLI flags
//   2. Spawn clidesk.exe inside a Windows Job Object (KILL_ON_JOB_CLOSE)
//   3. Hide console window for hidden mode
//   4. Wait for child exit, then clean up
//
// The Job Object guarantees: if this launcher dies for any reason
// (terminal closed, killed, etc.), Windows kernel will terminate clidesk.exe.
//
// No admin required. No taskkill used. No other processes touched.

#![windows_subsystem = "console"]
#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code
)]

use std::env;
use std::io::{self, Write};
use std::mem;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::process;

// ── Windows type aliases ───────────────────────────────────────
type BOOL = i32;
type DWORD = u32;
type HANDLE = *mut std::ffi::c_void;
type LPVOID = *mut std::ffi::c_void;
type LPCVOID = *const std::ffi::c_void;
type LPCWSTR = *const u16;
type HWND = HANDLE;
type SIZE_T = usize;
type ULONG_PTR = usize;
const FALSE: BOOL = 0;
const TRUE: BOOL = 1;

// ── Constants ──────────────────────────────────────────────────
const SW_HIDE: i32 = 0;

// Job Object info class
const JobObjectExtendedLimitInformation: u32 = 9;
const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: DWORD = 0x00002000;

// ── Structs ────────────────────────────────────────────────────
#[repr(C)]
struct JOBOBJECT_BASIC_LIMIT_INFORMATION {
    PerProcessUserTimeLimit: i64,
    PerJobUserTimeLimit: i64,
    LimitFlags: DWORD,
    MinimumWorkingSetSize: SIZE_T,
    MaximumWorkingSetSize: SIZE_T,
    ActiveProcessLimit: DWORD,
    Affinity: ULONG_PTR,
    PriorityClass: DWORD,
    SchedulingClass: DWORD,
}

#[repr(C)]
struct IO_COUNTERS {
    ReadOperationCount: u64,
    WriteOperationCount: u64,
    OtherOperationCount: u64,
    ReadTransferCount: u64,
    WriteTransferCount: u64,
    OtherTransferCount: u64,
}

#[repr(C)]
struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
    BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION,
    IoInfo: IO_COUNTERS,
    ProcessMemoryLimit: SIZE_T,
    JobMemoryLimit: SIZE_T,
    PeakProcessMemoryUsed: SIZE_T,
    PeakJobMemoryUsed: SIZE_T,
}

#[repr(C)]
struct SECURITY_ATTRIBUTES {
    nLength: DWORD,
    lpSecurityDescriptor: LPVOID,
    bInheritHandle: BOOL,
}

// ── WinAPI FFI declarations ────────────────────────────────────

#[link(name = "kernel32")]
extern "system" {
    fn CreateJobObjectW(
        lpJobAttributes: *const SECURITY_ATTRIBUTES,
        lpName: LPCWSTR,
    ) -> HANDLE;

    fn SetInformationJobObject(
        hJob: HANDLE,
        JobObjectInfoClass: u32,
        lpJobObjectInfo: LPVOID,
        cbJobObjectInfoLength: DWORD,
    ) -> BOOL;

    fn AssignProcessToJobObject(
        hJob: HANDLE,
        hProcess: HANDLE,
    ) -> BOOL;

    fn CloseHandle(hObject: HANDLE) -> BOOL;

    fn GetConsoleWindow() -> HWND;
}

#[link(name = "user32")]
extern "system" {
    fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
}

// ── Mode enum ──────────────────────────────────────────────────
enum Mode {
    Hidden,
    Visible,
}

#[derive(Clone, Copy)]
enum Language {
    Vi,
    En,
}

struct VersionInfo {
    current: String,
    latest: Option<String>,
    update_available: bool,
    update_command: String,
}

impl Language {
    fn code(self) -> &'static str {
        match self {
            Language::Vi => "vi",
            Language::En => "en",
        }
    }

    fn from_code(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "vi" | "vietnamese" | "tieng-viet" | "tiếng-việt" => Some(Language::Vi),
            "en" | "english" => Some(Language::En),
            _ => None,
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let lang = language_from_args(&args)
        .or_else(language_from_env)
        .unwrap_or_else(|| {
            print_language_menu();
            read_language_choice()
        });
    let version_info = VersionInfo::from_env();
    print_version_status(lang, &version_info);

    // ── CLI flags ──────────────────────────────────────────────
    let mode = if args.iter().any(|a| a == "--hidden") {
        Mode::Hidden
    } else if args.iter().any(|a| a == "--visible") {
        Mode::Visible
    } else {
        print_mode_menu(lang, &version_info);
        read_mode_choice(lang)
    };

    // ── Locate clidesk.exe ─────────────────────────────────────
    let app_path = match find_app_path(lang) {
        Ok(path) => path,
        Err(message) => {
            eprintln!("{}", message);
            process::exit(1);
        }
    };

    // ── Run ────────────────────────────────────────────────────
    match mode {
        Mode::Hidden => run_hidden(&app_path, lang),
        Mode::Visible => run_visible(&app_path, lang),
    }
}

// ── Menu ───────────────────────────────────────────────────────

impl VersionInfo {
    fn from_env() -> Self {
        let current = env::var("CLIDESK_VERSION")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "dev".to_string());

        let latest = env::var("CLIDESK_LATEST_VERSION")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());

        let update_available = env::var("CLIDESK_UPDATE_AVAILABLE")
            .ok()
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or_else(|| {
                latest
                    .as_ref()
                    .map(|latest_version| is_version_newer(latest_version, &current))
                    .unwrap_or(false)
            });

        let update_command = env::var("CLIDESK_UPDATE_COMMAND")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "npm i -g clidesk".to_string());

        Self {
            current,
            latest,
            update_available,
            update_command,
        }
    }
}

fn language_from_args(args: &[String]) -> Option<Language> {
    for arg in args {
        if arg == "--vi" {
            return Some(Language::Vi);
        }
        if arg == "--en" {
            return Some(Language::En);
        }
        if let Some(value) = arg.strip_prefix("--lang=") {
            if let Some(lang) = Language::from_code(value) {
                return Some(lang);
            }
        }
    }

    if let Some(pos) = args.iter().position(|arg| arg == "--lang") {
        if let Some(value) = args.get(pos + 1) {
            return Language::from_code(value);
        }
    }

    None
}

fn language_from_env() -> Option<Language> {
    env::var("CLIDESK_LAUNCH_LANG")
        .ok()
        .and_then(|value| Language::from_code(&value))
}

fn print_language_menu() {
    println!("╔══════════════════════════════════╗");
    println!("║        CliDesk Launcher          ║");
    println!("╠══════════════════════════════════╣");
    println!("║  1. Tiếng Việt                   ║");
    println!("║  2. English                      ║");
    println!("╚══════════════════════════════════╝");
    print!("Chọn ngôn ngữ / Select language (1 or 2): ");
    let _ = io::stdout().flush();
}

fn read_language_choice() -> Language {
    loop {
        let input = read_line_or_exit("[CliDesk] Không nhận được lựa chọn ngôn ngữ / No language choice received.");
        match input.trim() {
            "1" => return Language::Vi,
            "2" => return Language::En,
            _ => {
                print!("Nhập sai / Invalid choice. Chọn 1 hoặc 2 / Choose 1 or 2: ");
                let _ = io::stdout().flush();
            }
        }
    }
}

fn print_version_status(lang: Language, info: &VersionInfo) {
    println!();
    match lang {
        Language::Vi => {
            println!("[CliDesk] Phiên bản hiện tại: {}", info.current);
            if info.update_available {
                let latest = info.latest.as_deref().unwrap_or("latest");
                println!("[CliDesk] Có bản mới: {}", latest);
                println!("[CliDesk] Cập nhật: {}", info.update_command);
            }
        }
        Language::En => {
            println!("[CliDesk] Current version: {}", info.current);
            if info.update_available {
                let latest = info.latest.as_deref().unwrap_or("latest");
                println!("[CliDesk] Update available: {}", latest);
                println!("[CliDesk] Update: {}", info.update_command);
            }
        }
    }
}

fn print_mode_menu(lang: Language, info: &VersionInfo) {
    match lang {
        Language::Vi => {
            println!("╔══════════════════════════════════╗");
            println!("║        CliDesk Launcher          ║");
            println!("║        Phiên bản {:<15}║", info.current);
            println!("╠══════════════════════════════════╣");
            println!("║  1. Ẩn terminal, chỉ hiện app    ║");
            println!("║  2. Giữ terminal hiển thị        ║");
            println!("╚══════════════════════════════════╝");
            print!("Chọn (1 hoặc 2): ");
        }
        Language::En => {
            println!("╔══════════════════════════════════╗");
            println!("║        CliDesk Launcher          ║");
            println!("║        Version {:<17}║", info.current);
            println!("╠══════════════════════════════════╣");
            println!("║  1. Hide terminal, app only      ║");
            println!("║  2. Keep terminal visible        ║");
            println!("╚══════════════════════════════════╝");
            print!("Choose (1 or 2): ");
        }
    }
    let _ = io::stdout().flush();
}

fn read_mode_choice(lang: Language) -> Mode {
    loop {
        let input = read_line_or_exit(match lang {
            Language::Vi => "[CliDesk] Không nhận được lựa chọn launcher.",
            Language::En => "[CliDesk] No launcher choice received.",
        });
        match input.trim() {
            "1" => return Mode::Hidden,
            "2" => return Mode::Visible,
            _ => {
                print!("{}", match lang {
                    Language::Vi => "Nhập sai. Chọn 1 hoặc 2: ",
                    Language::En => "Invalid choice. Choose 1 or 2: ",
                });
                let _ = io::stdout().flush();
            }
        }
    }
}

fn read_line_or_exit(eof_message: &str) -> String {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => {
            eprintln!("{}", eof_message);
            process::exit(1);
        }
        Ok(_) => input,
        Err(err) => {
            eprintln!("[CliDesk] {}", err);
            process::exit(1);
        }
    }
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

// ── Path resolution ───────────────────────────────────────────

fn find_app_path(lang: Language) -> Result<PathBuf, String> {
    let launcher_path = env::current_exe()
        .map_err(|err| match lang {
            Language::Vi => format!("[CliDesk] Không thể lấy đường dẫn launcher: {}", err),
            Language::En => format!("[CliDesk] Failed to read launcher path: {}", err),
        })?;

    let app_dir = launcher_path
        .parent()
        .ok_or_else(|| match lang {
            Language::Vi => "[CliDesk] Không thể xác định thư mục launcher.".to_string(),
            Language::En => "[CliDesk] Failed to resolve launcher directory.".to_string(),
        })?;

    let app_path = app_dir.join("clidesk.exe");
    if !app_path.exists() {
        return Err(match lang {
            Language::Vi => format!(
                "[CliDesk] Không tìm thấy app binary.\n[CliDesk] Path: {}",
                app_path.display()
            ),
            Language::En => format!(
                "[CliDesk] App binary not found.\n[CliDesk] Path: {}",
                app_path.display()
            ),
        });
    }

    Ok(app_path)
}

// ── Wide string helper ────────────────────────────────────────
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ── Job Object helpers ─────────────────────────────────────────
/// Spawn app via std::process::Command, then assign it to a Job Object
/// with KILL_ON_JOB_CLOSE so the app is killed when the launcher exits.
fn spawn_in_job(app_path: &Path, lang: Language) -> Result<(HANDLE, process::Child), String> {
    let app_dir = app_path
        .parent()
        .ok_or_else(|| match lang {
            Language::Vi => "[CliDesk] Không thể xác định thư mục app.".to_string(),
            Language::En => "[CliDesk] Failed to resolve app directory.".to_string(),
        })?;

    // Use std::process::Command so Windows path quoting is handled by Rust.
    let mut child = process::Command::new(app_path)
        .current_dir(app_dir)
        .env("CLIDESK_LAUNCH_LANG", lang.code())
        .spawn()
        .map_err(|err| match lang {
            Language::Vi => format!(
                "[CliDesk] Không thể mở app CliDesk.\n[CliDesk] Path: {}\n[CliDesk] Lỗi: {}",
                app_path.display(),
                err
            ),
            Language::En => format!(
                "[CliDesk] Failed to open CliDesk.\n[CliDesk] Path: {}\n[CliDesk] Error: {}",
                app_path.display(),
                err
            ),
        })?;

    unsafe {
        // Create Job Object with unique name
        let job_name = format!("Local\\CliDeskJob_{}", process::id());
        let job_name_wide = to_wide(&job_name);
        let job = CreateJobObjectW(std::ptr::null(), job_name_wide.as_ptr());
        if job.is_null() {
            let err = io::Error::last_os_error();
            let _ = child.kill();
            let _ = child.wait();
            return Err(match lang {
                Language::Vi => format!(
                    "[CliDesk] Không thể tạo Windows Job Object.\n[CliDesk] Lỗi: {}",
                    err
                ),
                Language::En => format!(
                    "[CliDesk] Failed to create Windows Job Object.\n[CliDesk] Error: {}",
                    err
                ),
            });
        }

        // Set KILL_ON_JOB_CLOSE
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let ret = SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &mut info as *mut _ as LPVOID,
            mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as DWORD,
        );
        if ret == FALSE {
            let err = io::Error::last_os_error();
            let _ = child.kill();
            let _ = child.wait();
            CloseHandle(job);
            return Err(match lang {
                Language::Vi => format!(
                    "[CliDesk] Không thể cấu hình Windows Job Object.\n[CliDesk] Lỗi: {}",
                    err
                ),
                Language::En => format!(
                    "[CliDesk] Failed to configure Windows Job Object.\n[CliDesk] Error: {}",
                    err
                ),
            });
        }

        // Assign process to job
        let process_handle = child.as_raw_handle() as HANDLE;
        let ret = AssignProcessToJobObject(job, process_handle);
        if ret == FALSE {
            let err = io::Error::last_os_error();
            let _ = child.kill();
            let _ = child.wait();
            CloseHandle(job);
            return Err(match lang {
                Language::Vi => format!(
                    "[CliDesk] Không thể gán CliDesk vào Windows Job Object.\n[CliDesk] Path: {}\n[CliDesk] Lỗi: {}",
                    app_path.display(),
                    err
                ),
                Language::En => format!(
                    "[CliDesk] Failed to assign CliDesk to Windows Job Object.\n[CliDesk] Path: {}\n[CliDesk] Error: {}",
                    app_path.display(),
                    err
                ),
            });
        }

        Ok((job, child))
    }
}

// ── Console hide helper ─────────────────────────────────────────
/// Attempt to hide the console window. Returns true if the window was
/// successfully hidden, false if GetConsoleWindow() returned null
/// (e.g., in Windows Terminal or when no console is attached).
fn hide_console_window() -> bool {
    unsafe {
        let hwnd = GetConsoleWindow();
        if hwnd.is_null() {
            return false;
        }
        ShowWindow(hwnd, SW_HIDE);
        true
    }
}

/// Returns true if running inside Windows Terminal (WT_SESSION is set).
fn is_windows_terminal() -> bool {
    std::env::var("WT_SESSION").is_ok()
}

// ── Hidden mode ────────────────────────────────────────────────
fn run_hidden(app_path: &Path, lang: Language) {
    match spawn_in_job(app_path, lang) {
        Ok((job, mut child)) => {
            let in_wt = is_windows_terminal();
            let hidden = hide_console_window();

            if hidden {
                match lang {
                    Language::Vi => println!("[CliDesk] Đã ẩn terminal. CliDesk đang chạy."),
                    Language::En => println!("[CliDesk] Terminal hidden. CliDesk is running."),
                }
            } else if in_wt {
                match lang {
                    Language::Vi => println!(
                        "[CliDesk] Không thể ẩn terminal trong Windows Terminal.\n\
[CliDesk] CliDesk vẫn sẽ khởi động. Bạn có thể thu nhỏ tab Windows Terminal bằng tay."
                    ),
                    Language::En => println!(
                        "[CliDesk] Unable to hide the terminal in Windows Terminal.\n\n[CliDesk] CliDesk will still start. You can manually minimize the Windows Terminal tab."
                    ),
                }
            } else {
                match lang {
                    Language::Vi => println!(
                        "[CliDesk] Không thể ẩn terminal trong môi trường hiện tại.\n\n[CliDesk] CliDesk vẫn sẽ khởi động."
                    ),
                    Language::En => println!(
                        "[CliDesk] Unable to hide the terminal in this environment.\n\n[CliDesk] CliDesk will still start."
                    ),
                }
            }

            let _ = child.wait();
            unsafe {
                CloseHandle(job);
            }
        }
        Err(msg) => {
            eprintln!("{}", msg);
            process::exit(1);
        }
    }
}

// ── Visible mode ───────────────────────────────────────────────
fn run_visible(app_path: &Path, lang: Language) {
    match lang {
        Language::Vi => {
            println!("[CliDesk] Đang khởi động CliDesk...");
            println!("[CliDesk] Terminal này sẽ đợi CliDesk kết thúc.");
            println!("[CliDesk] Đóng cửa sổ terminal này sẽ buộc đóng CliDesk.");
        }
        Language::En => {
            println!("[CliDesk] Starting CliDesk...");
            println!("[CliDesk] This terminal will wait until CliDesk exits.");
            println!("[CliDesk] Closing this terminal will close CliDesk.");
        }
    }
    println!();

    match spawn_in_job(app_path, lang) {
        Ok((job, mut child)) => {
            let _ = child.wait();
            unsafe {
                CloseHandle(job);
            }
            println!();
            println!("{}", match lang {
                Language::Vi => "[CliDesk] Ứng dụng đã đóng.",
                Language::En => "[CliDesk] App closed.",
            });
        }
        Err(msg) => {
            eprintln!("{}", msg);
            process::exit(1);
        }
    }
}
