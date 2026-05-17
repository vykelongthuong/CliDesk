// CliDesk Launcher — Native Windows console app
//
// Uses raw WinAPI FFI (kernel32.dll + user32.dll) — no external crate.
//
// Launcher is only used with --wait or --interactive flags.
// Default npm flow (clidesk without flags) runs clidesk.exe directly.
//
// --no-menu:      Launch app silently (no prompts), keep terminal with Job Object
// --wait:         Same as --no-menu (keeps terminal attached for debugging)
// --interactive:  Show old interactive menus (language + mode) for debugging
// --detached:     Launch app without Job Object, exit immediately
// --hidden:       Hide console then launch (with Job Object)
// --visible:      Keep console visible (with Job Object) [default for --no-menu]
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
use std::path::Path;
use std::process;

// ── Windows type aliases ───────────────────────────────────────
type BOOL = i32;
type DWORD = u32;
type HANDLE = *mut std::ffi::c_void;
type LPVOID = *mut std::ffi::c_void;
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

const MAX_PATH_W: usize = 260;

// ── Constants ──────────────────────────────────────────────────
const SW_HIDE: i32 = 0;
const SW_MINIMIZE: i32 = 6;
const PROCESS_QUERY_LIMITED_INFORMATION: DWORD = 0x1000;
const TH32CS_SNAPPROCESS: DWORD = 0x00000002;
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

#[repr(C)]
struct EnumContext {
    target_pid: DWORD,
    found_hwnd: HWND,
}

#[derive(Clone, Copy, PartialEq)]
enum HideResult {
    Hidden,
    Minimized,
    Failed,
}

// ── WinAPI FFI declarations ────────────────────────────────────

#[link(name = "kernel32")]
extern "system" {
    fn CreateJobObjectW(lpJobAttributes: *const SECURITY_ATTRIBUTES, lpName: LPCWSTR) -> HANDLE;
    fn SetInformationJobObject(hJob: HANDLE, JobObjectInfoClass: u32, lpJobObjectInfo: LPVOID, cbJobObjectInfoLength: DWORD) -> BOOL;
    fn AssignProcessToJobObject(hJob: HANDLE, hProcess: HANDLE) -> BOOL;
    fn CloseHandle(hObject: HANDLE) -> BOOL;
    fn GetConsoleWindow() -> HWND;
    fn OpenProcess(dwDesiredAccess: DWORD, bInheritHandle: BOOL, dwProcessId: DWORD) -> HANDLE;
    fn QueryFullProcessImageNameW(hProcess: HANDLE, dwFlags: DWORD, lpExeName: LPWSTR, lpdwSize: LPDWORD) -> BOOL;
    fn CreateToolhelp32Snapshot(dwFlags: DWORD, th32ProcessID: DWORD) -> HANDLE;
    fn Process32FirstW(hSnapshot: HANDLE, lppe: *mut PROCESSENTRY32W) -> BOOL;
    fn Process32NextW(hSnapshot: HANDLE, lppe: *mut PROCESSENTRY32W) -> BOOL;
}

#[link(name = "user32")]
extern "system" {
    fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
    fn IsWindowVisible(hWnd: HWND) -> BOOL;
    fn GetWindowThreadProcessId(hWnd: HWND, lpdwProcessId: LPDWORD) -> DWORD;
    fn EnumWindows(lpEnumFunc: Option<unsafe extern "system" fn(HWND, LPARAM) -> BOOL>, lParam: LPARAM) -> BOOL;
}

// ── Enums ──────────────────────────────────────────────────────
#[derive(Clone, Copy)]
enum Language { Vi, En }

#[derive(Clone, Copy, PartialEq)]
enum Mode { Hidden, Visible, Detached }

struct VersionInfo {
    current: String,
    latest: Option<String>,
    update_available: bool,
    update_command: String,
}

impl Language {
    fn code(self) -> &'static str {
        match self { Language::Vi => "vi", Language::En => "en" }
    }
    fn from_code(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "vi" | "vietnamese" => Some(Language::Vi),
            "en" | "english" => Some(Language::En),
            _ => None,
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let debug_mode = args.iter().any(|a| a == "--debug-launch");
    let no_menu = args.iter().any(|a| a == "--no-menu");
    let interactive = args.iter().any(|a| a == "--interactive");

    // Language: read from args, env, or prompt only in interactive mode
    let lang = if interactive {
        language_from_args(&args)
            .or_else(language_from_env)
            .unwrap_or_else(|| {
                print_language_menu();
                read_language_choice()
            })
    } else {
        // --no-menu or default: language from env or fallback to vi
        language_from_args(&args)
            .or_else(language_from_env)
            .unwrap_or(Language::Vi)
    };

    let version_info = VersionInfo::from_env();

    // Debug info (always prints if flag is set)
    if debug_mode {
        print_debug_info(lang);
    }

    // ── Determine mode ─────────────────────────────────────────
    let detached_flag = args.iter().any(|a| a == "--detached");
    let hidden_flag = args.iter().any(|a| a == "--hidden");
    let visible_flag = args.iter().any(|a| a == "--visible");

    let mode = if detached_flag {
        Mode::Detached
    } else if hidden_flag {
        Mode::Hidden
    } else if visible_flag || no_menu {
        // --no-menu and --visible use Visible mode by default
        Mode::Visible
    } else if interactive {
        print_version_status(lang, &version_info);
        print_mode_menu(lang, &version_info);
        read_mode_choice(lang)
    } else {
        // Default fallback: Visible (attached)
        Mode::Visible
    };

    // ── Locate clidesk.exe ─────────────────────────────────────
    let app_path = match find_app_path(lang) {
        Ok(path) => path,
        Err(msg) => {
            eprintln!("{}", msg);
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

// ── Helpers ────────────────────────────────────────────────────

impl VersionInfo {
    fn from_env() -> Self {
        let current = env::var("CLIDESK_VERSION")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "dev".to_string());
        let latest = env::var("CLIDESK_LATEST_VERSION")
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
        let update_available = env::var("CLIDESK_UPDATE_AVAILABLE")
            .ok()
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        let update_command = env::var("CLIDESK_UPDATE_COMMAND")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "npm i -g clidesk".to_string());
        Self { current, latest, update_available, update_command }
    }
}

fn language_from_args(args: &[String]) -> Option<Language> {
    for arg in args {
        if arg == "--vi" { return Some(Language::Vi); }
        if arg == "--en" { return Some(Language::En); }
        if let Some(v) = arg.strip_prefix("--lang=") { return Language::from_code(v); }
    }
    if let Some(pos) = args.iter().position(|a| a == "--lang") {
        if let Some(v) = args.get(pos + 1) { return Language::from_code(v); }
    }
    None
}

fn language_from_env() -> Option<Language> {
    env::var("CLIDESK_LAUNCH_LANG").ok().and_then(|v| Language::from_code(&v))
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
        let input = read_line_or_exit("[CliDesk] Không nhận được lựa chọn ngôn ngữ.");
        match input.trim() {
            "1" => return Language::Vi,
            "2" => return Language::En,
            _ => { print!("Chọn 1 hoặc 2: "); let _ = io::stdout().flush(); }
        }
    }
}

fn print_version_status(lang: Language, info: &VersionInfo) {
    println!();
    match lang {
        Language::Vi => {
            println!("[CliDesk] Phiên bản hiện tại: {}", info.current);
            if info.update_available {
                println!("[CliDesk] Có bản mới: {}", info.latest.as_deref().unwrap_or("latest"));
                println!("[CliDesk] Cập nhật: {}", info.update_command);
            }
        }
        Language::En => {
            println!("[CliDesk] Current version: {}", info.current);
            if info.update_available {
                println!("[CliDesk] Update available: {}", info.latest.as_deref().unwrap_or("latest"));
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
            Language::Vi => "[CliDesk] Không nhận được lựa chọn.",
            Language::En => "[CliDesk] No choice received.",
        });
        match input.trim() {
            "1" => return Mode::Hidden,
            "2" => return Mode::Visible,
            _ => {
                print!("{}", match lang {
                    Language::Vi => "Chọn 1 hoặc 2: ",
                    Language::En => "Choose 1 or 2: ",
                });
                let _ = io::stdout().flush();
            }
        }
    }
}

fn read_line_or_exit(exit_msg: &str) -> String {
    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(0) => { eprintln!("{}", exit_msg); process::exit(1); }
        Ok(_) => input,
        Err(err) => { eprintln!("[CliDesk] {}", err); process::exit(1); }
    }
}

// ── Path resolution ───────────────────────────────────────────

fn find_app_path(lang: Language) -> Result<std::path::PathBuf, String> {
    let launcher_path = env::current_exe().map_err(|err| match lang {
        Language::Vi => format!("[CliDesk] Không thể lấy đường dẫn launcher: {}", err),
        Language::En => format!("[CliDesk] Failed to read launcher path: {}", err),
    })?;
    let app_dir = launcher_path.parent().ok_or_else(|| match lang {
        Language::Vi => "[CliDesk] Không thể xác định thư mục launcher.".to_string(),
        Language::En => "[CliDesk] Failed to resolve launcher directory.".to_string(),
    })?;
    let app_path = app_dir.join("clidesk.exe");
    if !app_path.exists() {
        return Err(match lang {
            Language::Vi => format!("[CliDesk] Không tìm thấy app binary.\n[CliDesk] Path: {}", app_path.display()),
            Language::En => format!("[CliDesk] App binary not found.\n[CliDesk] Path: {}", app_path.display()),
        });
    }
    Ok(app_path)
}

// ── Wide string helper ────────────────────────────────────────
fn to_wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }

fn from_wide(wide: &[u16]) -> String {
    let end = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16_lossy(&wide[..end])
}

// ── Process tree helpers ───────────────────────────────────────

fn get_parent_pid(pid: DWORD) -> Option<DWORD> {
    unsafe {
        let h = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if h.is_null() || h as isize == -1 { return None; }
        let mut entry: PROCESSENTRY32W = mem::zeroed();
        entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as DWORD;
        if Process32FirstW(h, &mut entry) == FALSE { CloseHandle(h); return None; }
        let mut parent = None;
        loop {
            if entry.th32ProcessID == pid { parent = Some(entry.th32ParentProcessID); break; }
            if Process32NextW(h, &mut entry) == FALSE { break; }
        }
        CloseHandle(h);
        parent
    }
}

fn get_process_name(pid: DWORD) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
        if h.is_null() { return None; }
        let mut buf = [0u16; MAX_PATH_W];
        let mut size = MAX_PATH_W as DWORD;
        let ret = QueryFullProcessImageNameW(h, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(h);
        if ret == FALSE { return None; }
        let name = from_wide(&buf[..size as usize]);
        std::path::Path::new(&name).file_name()?.to_str().map(|n| n.to_lowercase())
    }
}

fn find_window_by_pid(target_pid: DWORD) -> Option<HWND> {
    let mut ctx = EnumContext { target_pid, found_hwnd: std::ptr::null_mut() };
    unsafe { EnumWindows(Some(enum_window_callback), &mut ctx as *mut EnumContext as LPARAM); }
    if ctx.found_hwnd.is_null() { None } else { Some(ctx.found_hwnd) }
}

unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let ctx = &mut *(lparam as *mut EnumContext);
    let mut pid: DWORD = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == ctx.target_pid && IsWindowVisible(hwnd) != FALSE {
        ctx.found_hwnd = hwnd;
        return FALSE;
    }
    TRUE
}

fn find_terminal_window() -> Option<HWND> {
    let our_pid = process::id() as DWORD;
    let mut current_pid = our_pid;
    for _ in 0..8 {
        let pid = get_parent_pid(current_pid)?;
        if let Some(name) = get_process_name(pid) {
            let is_wt = name == "windowsterminal.exe" || name == "wt.exe";
            let is_conhost = name == "conhost.exe" || name == "openconsole.exe";
            if is_wt { return find_window_by_pid(pid); }
            if is_conhost { current_pid = pid; continue; }
        }
        current_pid = pid;
    }
    None
}

// ── Debug info ─────────────────────────────────────────────────

fn print_debug_info(lang: Language) {
    let pid = process::id();
    let current_exe = env::current_exe().map(|p| p.display().to_string()).unwrap_or_else(|_| "unknown".to_string());
    let in_wt = is_windows_terminal();
    let hwnd = unsafe { GetConsoleWindow() };
    let hwnd_status = if hwnd.is_null() { "null".to_string() } else { format!("{:p}", hwnd) };
    let parent_pid = get_parent_pid(pid as DWORD);
    let parent_name = parent_pid.and_then(get_process_name);
    let terminal_host = if in_wt { "Windows Terminal" } else if !hwnd.is_null() { "conhost/CMD/powershell (classic)" } else { "unknown (no console)" };

    match lang {
        Language::Vi => {
            println!("[CliDesk-Debug] === Thông tin debug ===");
            println!("[CliDesk-Debug] PID launcher: {}", pid);
            println!("[CliDesk-Debug] Đường dẫn: {}", current_exe);
            println!("[CliDesk-Debug] WT_SESSION: {}", if in_wt { "có" } else { "không" });
            println!("[CliDesk-Debug] GetConsoleWindow: {}", hwnd_status);
            println!("[CliDesk-Debug] Terminal host: {}", terminal_host);
            println!("[CliDesk-Debug] Parent PID: {}, Name: {:?}", parent_pid.unwrap_or(0), parent_name);
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
            println!("[CliDesk-Debug] =========================");
        }
    }
}

fn is_windows_terminal() -> bool { env::var("WT_SESSION").is_ok() }

// ── Console hide helper ─────────────────────────────────────────

fn hide_console_window_enhanced() -> HideResult {
    let hwnd = unsafe { GetConsoleWindow() };
    if hwnd.is_null() { return HideResult::Failed; }
    unsafe { ShowWindow(hwnd, SW_HIDE); }
    if unsafe { IsWindowVisible(hwnd) } == FALSE { return HideResult::Hidden; }
    unsafe { ShowWindow(hwnd, SW_MINIMIZE); }
    if unsafe { IsWindowVisible(hwnd) } == FALSE { return HideResult::Minimized; }
    HideResult::Failed
}

// ── Job Object helper ─────────────────────────────────────────

fn spawn_in_job(app_path: &Path, lang: Language) -> Result<(HANDLE, process::Child), String> {
    let app_dir = app_path.parent().ok_or_else(|| match lang {
        Language::Vi => "[CliDesk] Không thể xác định thư mục app.".to_string(),
        Language::En => "[CliDesk] Failed to resolve app directory.".to_string(),
    })?;

    let mut child = process::Command::new(app_path)
        .current_dir(app_dir)
        .env("CLIDESK_LAUNCH_LANG", lang.code())
        .spawn()
        .map_err(|err| match lang {
            Language::Vi => format!("[CliDesk] Không thể mở app CliDesk.\n[CliDesk] Path: {}\n[CliDesk] Lỗi: {}", app_path.display(), err),
            Language::En => format!("[CliDesk] Failed to open CliDesk.\n[CliDesk] Path: {}\n[CliDesk] Error: {}", app_path.display(), err),
        })?;

    unsafe {
        let job_name = format!("Local\\CliDeskJob_{}", process::id());
        let job_name_wide = to_wide(&job_name);
        let job = CreateJobObjectW(std::ptr::null(), job_name_wide.as_ptr());
        if job.is_null() {
            let _ = child.kill(); let _ = child.wait();
            return Err(match lang {
                Language::Vi => format!("[CliDesk] Không thể tạo Windows Job Object."),
                Language::En => format!("[CliDesk] Failed to create Windows Job Object."),
            });
        }
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(job, JobObjectExtendedLimitInformation, &mut info as *mut _ as LPVOID, mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as DWORD) == FALSE {
            let _ = child.kill(); let _ = child.wait(); CloseHandle(job);
            return Err(match lang {
                Language::Vi => format!("[CliDesk] Không thể cấu hình Windows Job Object."),
                Language::En => format!("[CliDesk] Failed to configure Windows Job Object."),
            });
        }
        if AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE) == FALSE {
            let _ = child.kill(); let _ = child.wait(); CloseHandle(job);
            return Err(match lang {
                Language::Vi => format!("[CliDesk] Không thể gán CliDesk vào Windows Job Object."),
                Language::En => format!("[CliDesk] Failed to assign CliDesk to Windows Job Object."),
            });
        }
        Ok((job, child))
    }
}

// ── Run modes ──────────────────────────────────────────────────

fn run_hidden(app_path: &Path, lang: Language) {
    let in_wt = is_windows_terminal();
    if in_wt {
        if let Some(wt_hwnd) = find_terminal_window() {
            match lang { Language::Vi => println!("[CliDesk] Đang thu nhỏ Windows Terminal..."), Language::En => println!("[CliDesk] Minimizing Windows Terminal..."), }
            unsafe { ShowWindow(wt_hwnd, SW_MINIMIZE); }
            match lang { Language::Vi => println!("[CliDesk] Đã thu nhỏ Windows Terminal."), Language::En => println!("[CliDesk] Windows Terminal minimized."), }
        } else {
            match lang {
                Language::Vi => println!("[CliDesk] Không thể ẩn Windows Terminal một cách an toàn.\n[CliDesk] CliDesk vẫn sẽ khởi động."),
                Language::En => println!("[CliDesk] Unable to safely hide Windows Terminal.\n[CliDesk] CliDesk will still start."),
            }
        }
    } else {
        match lang { Language::Vi => println!("[CliDesk] Đang ẩn terminal..."), Language::En => println!("[CliDesk] Hiding terminal..."), }
        match hide_console_window_enhanced() {
            HideResult::Hidden => match lang { Language::Vi => println!("[CliDesk] Đã ẩn terminal."), Language::En => println!("[CliDesk] Terminal hidden."), },
            HideResult::Minimized => match lang { Language::Vi => println!("[CliDesk] Đã thu nhỏ terminal."), Language::En => println!("[CliDesk] Terminal minimized."), },
            HideResult::Failed => match lang { Language::Vi => println!("[CliDesk] Không thể ẩn terminal.\n[CliDesk] CliDesk vẫn sẽ khởi động."), Language::En => println!("[CliDesk] Unable to hide terminal.\n[CliDesk] CliDesk will still start."), },
        }
    }

    match spawn_in_job(app_path, lang) {
        Ok((job, mut child)) => {
            let _ = child.wait();
            unsafe { CloseHandle(job); }
        }
        Err(msg) => { eprintln!("{}", msg); process::exit(1); }
    }
}

fn run_visible(app_path: &Path, lang: Language) {
    match lang {
        Language::Vi => println!("[CliDesk] Terminal sẽ đợi CliDesk kết thúc."),
        Language::En => println!("[CliDesk] Terminal will wait until CliDesk exits."),
    }

    match spawn_in_job(app_path, lang) {
        Ok((job, mut child)) => {
            let _ = child.wait();
            unsafe { CloseHandle(job); }
            println!("{}", match lang { Language::Vi => "[CliDesk] Ứng dụng đã đóng.", Language::En => "[CliDesk] App closed.", });
        }
        Err(msg) => { eprintln!("{}", msg); process::exit(1); }
    }
}

fn run_detached(app_path: &Path, lang: Language) {
    let app_dir = app_path.parent().expect("[CliDesd] Failed to resolve app directory.");
    match process::Command::new(app_path).current_dir(app_dir).env("CLIDESK_LAUNCH_LANG", lang.code()).spawn() {
        Ok(_) => {}
        Err(err) => { eprintln!("{}", match lang { Language::Vi => format!("[CliDesk] Không thể mở CliDesk.\n[CliDesk] Lỗi: {}", err), Language::En => format!("[CliDesk] Failed to open CliDesk.\n[CliDesk] Error: {}", err), }); process::exit(1); }
    }
}
