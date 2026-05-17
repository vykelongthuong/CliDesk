// CliDesk Launcher — Native Windows console app
//
// Uses raw WinAPI FFI (kernel32.dll + user32.dll) — no external crate.
// This avoids the windows crate's build-script dependency issues while
// keeping the same Windows Job Object lifecycle guarantees.
//
// Responsibilities:
//   1. Present interactive menu (hidden / visible mode) or accept CLI flags
//   2. Spawn clidesk.exe inside a Windows Job Object (KILL_ON_JOB_CLOSE)
//   3. Hide console window for hidden mode (with Windows Terminal fallback)
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
type LPWSTR = *mut u16;
type HWND = HANDLE;
type SIZE_T = usize;
type ULONG_PTR = usize;
type LONG = i32;
type LPARAM = isize;
type LPDWORD = *mut DWORD;
const FALSE: BOOL = 0;
const TRUE: BOOL = 1;

// MAX_PATH from Windows SDK
const MAX_PATH_W: usize = 260;

// ── Constants ──────────────────────────────────────────────────
const SW_HIDE: i32 = 0;
const SW_MINIMIZE: i32 = 6;

// Process access rights
const PROCESS_QUERY_LIMITED_INFORMATION: DWORD = 0x1000;

// Toolhelp flags
const TH32CS_SNAPPROCESS: DWORD = 0x00000002;

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

// ── Process helper structs ─────────────────────────────────────
#[repr(C)]
struct PROCESSENTRY32W {
    dwSize: DWORD,
    cntUsage: DWORD,
    th32ProcessID: DWORD,
    th32DefaultHeapID: ULONG_PTR,
    th32ModuleID: DWORD,
    cntThreads: DWORD,
    th32ParentProcessID: DWORD,
    pcPriClassBase: LONG,
    dwFlags: DWORD,
    szExeFile: [u16; MAX_PATH_W],
}

// Context passed to EnumWindows callback via LPARAM
#[repr(C)]
struct EnumContext {
    target_pid: DWORD,
    found_hwnd: HWND,
}

// Result of hiding the console window
#[derive(Clone, Copy, PartialEq)]
enum HideResult {
    Hidden,
    Minimized,
    Failed,
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

    fn OpenProcess(
        dwDesiredAccess: DWORD,
        bInheritHandle: BOOL,
        dwProcessId: DWORD,
    ) -> HANDLE;

    fn QueryFullProcessImageNameW(
        hProcess: HANDLE,
        dwFlags: DWORD,
        lpExeName: LPWSTR,
        lpdwSize: LPDWORD,
    ) -> BOOL;

    fn CreateToolhelp32Snapshot(
        dwFlags: DWORD,
        th32ProcessID: DWORD,
    ) -> HANDLE;

    fn Process32FirstW(
        hSnapshot: HANDLE,
        lppe: *mut PROCESSENTRY32W,
    ) -> BOOL;

    fn Process32NextW(
        hSnapshot: HANDLE,
        lppe: *mut PROCESSENTRY32W,
    ) -> BOOL;
}

#[link(name = "user32")]
extern "system" {
    fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;

    fn IsWindowVisible(hWnd: HWND) -> BOOL;

    fn GetWindowThreadProcessId(
        hWnd: HWND,
        lpdwProcessId: LPDWORD,
    ) -> DWORD;

    fn EnumWindows(
        lpEnumFunc: Option<
            unsafe extern "system" fn(HWND, LPARAM) -> BOOL,
        >,
        lParam: LPARAM,
    ) -> BOOL;
}

// ── Mode enum ──────────────────────────────────────────────────
enum Mode {
    Hidden,
    Visible,
    Detached,
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
    let debug_mode = args.iter().any(|a| a == "--debug-launch");

    let lang = language_from_args(&args)
        .or_else(language_from_env)
        .unwrap_or_else(|| {
            print_language_menu();
            read_language_choice()
        });
    let version_info = VersionInfo::from_env();
    print_version_status(lang, &version_info);

    // Debug info
    if debug_mode {
        print_debug_info(lang);
    }

    // ── CLI flags ──────────────────────────────────────────────
    let detached_flag = args.iter().any(|a| a == "--detached");
    let mode = if detached_flag {
        Mode::Detached
    } else if args.iter().any(|a| a == "--hidden") {
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
        Mode::Detached => run_detached(&app_path, lang),
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
                print!(
                    "{}",
                    match lang {
                        Language::Vi => "Nhập sai. Chọn 1 hoặc 2: ",
                        Language::En => "Invalid choice. Choose 1 or 2: ",
                    }
                );
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

fn from_wide(wide: &[u16]) -> String {
    let end = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..end])
}

// ── Process tree helpers ───────────────────────────────────────

/// Get parent process PID using Toolhelp32 snapshot.
fn get_parent_pid(pid: DWORD) -> Option<DWORD> {
    unsafe {
        let h_snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if h_snapshot.is_null() || h_snapshot as isize == -1 {
            // INVALID_HANDLE_VALUE
            return None;
        }

        let mut entry: PROCESSENTRY32W = mem::zeroed();
        entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as DWORD;

        if Process32FirstW(h_snapshot, &mut entry) == FALSE {
            CloseHandle(h_snapshot);
            return None;
        }

        let mut parent_pid = None;
        loop {
            if entry.th32ProcessID == pid {
                parent_pid = Some(entry.th32ParentProcessID);
                break;
            }
            if Process32NextW(h_snapshot, &mut entry) == FALSE {
                break;
            }
        }

        CloseHandle(h_snapshot);
        parent_pid
    }
}

/// Get process name (executable filename) for a given PID.
fn get_process_name(pid: DWORD) -> Option<String> {
    unsafe {
        let h_process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
        if h_process.is_null() {
            return None;
        }

        let mut buffer = [0u16; MAX_PATH_W];
        let mut size = MAX_PATH_W as DWORD;
        let ret = QueryFullProcessImageNameW(
            h_process,
            0, // WIN32 path format
            buffer.as_mut_ptr(),
            &mut size,
        );

        CloseHandle(h_process);

        if ret == FALSE {
            return None;
        }

        let name = from_wide(&buffer[..size as usize]);
        // Extract just the filename from the full path
        let path = std::path::Path::new(&name);
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.to_lowercase())
    }
}

/// Find a top-level window that belongs to the given process PID.
fn find_window_by_pid(target_pid: DWORD) -> Option<HWND> {
    let mut ctx = EnumContext {
        target_pid,
        found_hwnd: std::ptr::null_mut(),
    };

    unsafe {
        EnumWindows(
            Some(enum_window_callback),
            &mut ctx as *mut EnumContext as LPARAM,
        );
    }

    if ctx.found_hwnd.is_null() {
        None
    } else {
        Some(ctx.found_hwnd)
    }
}

/// EnumWindows callback — finds a top-level window matching target_pid.
unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam as *mut EnumContext);
    let mut pid: DWORD = 0;

    GetWindowThreadProcessId(hwnd, &mut pid);

    if pid == ctx.target_pid && IsWindowVisible(hwnd) != FALSE {
        ctx.found_hwnd = hwnd;
        return FALSE; // stop enumeration
    }

    TRUE // continue
}

/// Walk the process tree upward from our PID to find a Windows Terminal
/// ancestor. Returns the HWND of the Windows Terminal top-level window,
/// or None if not found or unsafe to act on.
fn find_terminal_window() -> Option<HWND> {
    let our_pid = process::id() as DWORD;
    // Walk up: our PID -> parent -> grandparent -> ...
    let mut current_pid = our_pid;

    for _ in 0..8 {
        // Limit depth to avoid infinite loops
        let pid = get_parent_pid(current_pid)?;

        if let Some(name) = get_process_name(pid) {
            let is_wt =
                name == "windowsterminal.exe" || name == "wt.exe";
            let is_conhost =
                name == "conhost.exe" || name == "openconsole.exe";

            if is_wt {
                // Found Windows Terminal — find its main window
                return find_window_by_pid(pid);
            } else if is_conhost {
                // Continue walking up past conhost
                current_pid = pid;
                continue;
            }
        }

        current_pid = pid;
    }

    None
}

// ── Debug info ─────────────────────────────────────────────────

fn print_debug_info(lang: Language) {
    let pid = process::id();
    let current_exe = env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let in_wt = is_windows_terminal();
    let hwnd = unsafe { GetConsoleWindow() };

    let hwnd_status = if hwnd.is_null() {
        "null".to_string()
    } else {
        format!("{:p}", hwnd)
    };

    let parent_pid = get_parent_pid(pid as DWORD);
    let parent_name = parent_pid.and_then(get_process_name);
    let grandparent_pid = parent_pid.and_then(|ppid| get_parent_pid(ppid));
    let grandparent_name = grandparent_pid.and_then(get_process_name);

    let terminal_host = if in_wt {
        "Windows Terminal"
    } else if !hwnd.is_null() {
        "conhost/CMD/powershell (classic)"
    } else {
        "unknown (no console window)"
    };

    let wt_window = find_terminal_window();
    let wt_window_status = match wt_window {
        Some(hwnd) => format!("found ({:p})", hwnd),
        None => "not found".to_string(),
    };

    match lang {
        Language::Vi => {
            println!("[CliDesk-Debug] === Thông tin debug ===");
            println!("[CliDesk-Debug] PID launcher: {}", pid);
            println!("[CliDesk-Debug] Đường dẫn: {}", current_exe);
            println!("[CliDesk-Debug] WT_SESSION: {}", if in_wt { "có" } else { "không" });
            println!("[CliDesk-Debug] GetConsoleWindow: {}", hwnd_status);
            println!("[CliDesk-Debug] Terminal host: {}", terminal_host);
            println!("[CliDesk-Debug] Parent PID: {}, Name: {:?}", parent_pid.unwrap_or(0), parent_name);
            println!("[CliDesk-Debug] Grandparent PID: {}, Name: {:?}", grandparent_pid.unwrap_or(0), grandparent_name);
            println!("[CliDesk-Debug] Windows Terminal window: {}", wt_window_status);
            println!("[CliDesk-Debug] ==========================");
        }
        Language::En => {
            println!("[CliDesk-Debug] === Debug info ===");
            println!("[CliDesk-Debug] Launcher PID: {}", pid);
            println!("[CliDesk-Debug] Path: {}", current_exe);
            println!("[CliDesk-Debug] WT_SESSION: {}", if in_wt { "yes" } else { "no" });
            println!("[CliDesk-Debug] GetConsoleWindow: {}", hwnd_status);
            println!("[CliDesk-Debug] Terminal host: {}", terminal_host);
            println!("[CliDesk-Debug] Parent PID: {}, Name: {:?}", parent_pid.unwrap_or(0), parent_name);
            println!("[CliDesk-Debug] Grandparent PID: {}, Name: {:?}", grandparent_pid.unwrap_or(0), grandparent_name);
            println!("[CliDesk-Debug] Windows Terminal window: {}", wt_window_status);
            println!("[CliDesk-Debug] =========================");
        }
    }
}

// ── Console hide helper ─────────────────────────────────────────

/// Attempt to hide the console window using multiple strategies.
/// Returns HideResult indicating what was achieved.
fn hide_console_window_enhanced() -> HideResult {
    let hwnd = unsafe { GetConsoleWindow() };

    if hwnd.is_null() {
        return HideResult::Failed;
    }

    // Strategy 1: SW_HIDE (full hide)
    unsafe {
        ShowWindow(hwnd, SW_HIDE);
    }

    // Check if it actually worked
    let still_visible = unsafe { IsWindowVisible(hwnd) } != FALSE;
    if !still_visible {
        return HideResult::Hidden;
    }

    // Strategy 2: SW_MINIMIZE (minimize — fallback)
    unsafe {
        ShowWindow(hwnd, SW_MINIMIZE);
    }

    // Check again
    let still_visible_after_min = unsafe { IsWindowVisible(hwnd) } != FALSE;
    if !still_visible_after_min {
        return HideResult::Minimized;
    }

    HideResult::Failed
}

/// Returns true if running inside Windows Terminal (WT_SESSION is set).
fn is_windows_terminal() -> bool {
    std::env::var("WT_SESSION").is_ok()
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

// ── Hidden mode ────────────────────────────────────────────────
fn run_hidden(app_path: &Path, lang: Language) {
    // If in Windows Terminal, try to find and minimize the WT window first
    let in_wt = is_windows_terminal();

    if in_wt {
        // Try minimizing the Windows Terminal window safely
        if let Some(wt_hwnd) = find_terminal_window() {
            match lang {
                Language::Vi => println!("[CliDesk] Đang thu nhỏ Windows Terminal..."),
                Language::En => println!("[CliDesk] Minimizing Windows Terminal..."),
            }
            unsafe {
                ShowWindow(wt_hwnd, SW_MINIMIZE);
            }
            match lang {
                Language::Vi => println!("[CliDesk] Đã thu nhỏ Windows Terminal."),
                Language::En => println!("[CliDesk] Windows Terminal minimized."),
            }
        } else {
            match lang {
                Language::Vi => println!(
                    "[CliDesk] Không thể ẩn Windows Terminal một cách an toàn.\n\
[CliDesk] CliDesk vẫn sẽ khởi động. Bạn có thể thu nhỏ tab Windows Terminal bằng tay."
                ),
                Language::En => println!(
                    "[CliDesk] Unable to safely hide Windows Terminal.\n\
[CliDesk] CliDesk will still start. You can manually minimize the Windows Terminal tab."
                ),
            }
        }
    } else {
        // Non-Windows Terminal: use enhanced hide (SW_HIDE → SW_MINIMIZE → report)
        match lang {
            Language::Vi => println!("[CliDesk] Đang ẩn terminal..."),
            Language::En => println!("[CliDesk] Hiding terminal..."),
        }

        let hide_result = hide_console_window_enhanced();

        match hide_result {
            HideResult::Hidden => match lang {
                Language::Vi => println!("[CliDesk] Đã ẩn terminal."),
                Language::En => println!("[CliDesk] Terminal hidden."),
            },
            HideResult::Minimized => match lang {
                Language::Vi => println!("[CliDesk] Đã thu nhỏ terminal (không thể ẩn hoàn toàn)."),
                Language::En => {
                    println!("[CliDesk] Terminal minimized (could not fully hide).")
                }
            },
            HideResult::Failed => match lang {
                Language::Vi => println!(
                    "[CliDesk] Không thể ẩn terminal trong môi trường hiện tại.\n\
[CliDesk] CliDesk vẫn sẽ khởi động."
                ),
                Language::En => println!(
                    "[CliDesk] Unable to hide the terminal in this environment.\n\
[CliDesk] CliDesk will still start."
                ),
            },
        }
    }

    println!();

    // Spawn app inside job object so lifecycle is preserved
    match spawn_in_job(app_path, lang) {
        Ok((job, mut child)) => {
            match lang {
                Language::Vi => println!("[CliDesk] CliDesk đang chạy."),
                Language::En => println!("[CliDesk] CliDesk is running."),
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
            println!(
                "{}",
                match lang {
                    Language::Vi => "[CliDesk] Ứng dụng đã đóng.",
                    Language::En => "[CliDesk] App closed.",
                }
            );
        }
        Err(msg) => {
            eprintln!("{}", msg);
            process::exit(1);
        }
    }
}

// ── Detached mode ──────────────────────────────────────────────
/// Spawn app without a Job Object and exit immediately.
/// The app is not bound to the launcher's lifecycle.
fn run_detached(app_path: &Path, lang: Language) {
    let app_dir = app_path
        .parent()
        .expect("[CliDesk] Failed to resolve app directory.");

    match lang {
        Language::Vi => {
            println!("[CliDesk] Đang khởi động CliDesk ở chế độ detached...");
            println!("[CliDesk] Terminal có thể đóng mà không ảnh hưởng đến CliDesk.");
        }
        Language::En => {
            println!("[CliDesk] Starting CliDesk in detached mode...");
            println!("[CliDesk] Terminal can be closed without affecting CliDesk.");
        }
    }

    match process::Command::new(app_path)
        .current_dir(app_dir)
        .env("CLIDESK_LAUNCH_LANG", lang.code())
        .spawn()
    {
        Ok(_) => {
            match lang {
                Language::Vi => println!("[CliDesk] CliDesk đã được khởi động. Terminal có thể đóng an toàn."),
                Language::En => {
                    println!("[CliDesk] CliDesk launched. Terminal can be closed safely.")
                }
            }
            // Do NOT wait for child — exit immediately
        }
        Err(err) => {
            eprintln!(
                "{}",
                match lang {
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
                }
            );
            process::exit(1);
        }
    }
}
