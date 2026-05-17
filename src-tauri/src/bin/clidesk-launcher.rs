// CliDesk Launcher — Native Windows console app (Tauri bin variant)
//
// This file is a copy of src-launcher/src/main.rs that lives inside src-tauri/
// so it can link against Tauri's build environment when bundled as part of the
// main Tauri build.
//
// Responsibilities and flags are identical to the standalone launcher.
// See src-launcher/src/main.rs for full documentation.

#![windows_subsystem = "console"]
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use std::env;
use std::io::{self, Write};
use std::mem;
use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::process;

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
const SW_HIDE: i32 = 0;
const SW_MINIMIZE: i32 = 6;
const PROCESS_QUERY_LIMITED_INFORMATION: DWORD = 0x1000;
const TH32CS_SNAPPROCESS: DWORD = 0x00000002;
const JobObjectExtendedLimitInformation: u32 = 9;
const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: DWORD = 0x00002000;

#[repr(C)]
struct JOBOBJECT_BASIC_LIMIT_INFORMATION { PerProcessUserTimeLimit: i64, PerJobUserTimeLimit: i64, LimitFlags: DWORD, MinimumWorkingSetSize: SIZE_T, MaximumWorkingSetSize: SIZE_T, ActiveProcessLimit: DWORD, Affinity: ULONG_PTR, PriorityClass: DWORD, SchedulingClass: DWORD }
#[repr(C)]
struct IO_COUNTERS { ReadOperationCount: u64, WriteOperationCount: u64, OtherOperationCount: u64, ReadTransferCount: u64, WriteTransferCount: u64, OtherTransferCount: u64 }
#[repr(C)]
struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION { BasicLimitInformation: JOBOBJECT_BASIC_LIMIT_INFORMATION, IoInfo: IO_COUNTERS, ProcessMemoryLimit: SIZE_T, JobMemoryLimit: SIZE_T, PeakProcessMemoryUsed: SIZE_T, PeakJobMemoryUsed: SIZE_T }
#[repr(C)]
struct SECURITY_ATTRIBUTES { nLength: DWORD, lpSecurityDescriptor: LPVOID, bInheritHandle: BOOL }
#[repr(C)]
struct PROCESSENTRY32W { dwSize: DWORD, cntUsage: DWORD, th32ProcessID: DWORD, th32DefaultHeapID: ULONG_PTR, th32ModuleID: DWORD, cntThreads: DWORD, th32ParentProcessID: DWORD, pcPriClassBase: LONG, dwFlags: DWORD, szExeFile: [u16; MAX_PATH_W] }
#[repr(C)]
struct EnumContext { target_pid: DWORD, found_hwnd: HWND }
#[derive(Clone, Copy, PartialEq)]
enum HideResult { Hidden, Minimized, Failed }

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

#[derive(Clone, Copy)]
enum Language { Vi, En }
#[derive(Clone, Copy, PartialEq)]
enum Mode { Hidden, Visible, Detached }
struct VersionInfo { current: String, latest: Option<String>, update_available: bool, update_command: String }

impl Language {
    fn code(self) -> &'static str { match self { Language::Vi => "vi", Language::En => "en" } }
    fn from_code(value: &str) -> Option<Self> { match value.trim().to_lowercase().as_str() { "vi" | "vietnamese" => Some(Language::Vi), "en" | "english" => Some(Language::En), _ => None } }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let debug_mode = args.iter().any(|a| a == "--debug-launch");
    let no_menu = args.iter().any(|a| a == "--no-menu");
    let interactive = args.iter().any(|a| a == "--interactive");

    let lang = if interactive {
        language_from_args(&args).or_else(language_from_env).unwrap_or_else(|| { print_language_menu(); read_language_choice() })
    } else {
        language_from_args(&args).or_else(language_from_env).unwrap_or(Language::Vi)
    };

    let version_info = VersionInfo::from_env();
    if debug_mode { print_debug_info(lang); }

    let detached = args.iter().any(|a| a == "--detached");
    let hidden = args.iter().any(|a| a == "--hidden");
    let visible = args.iter().any(|a| a == "--visible");

    let mode = if detached { Mode::Detached }
    else if hidden { Mode::Hidden }
    else if visible || no_menu { Mode::Visible }
    else if interactive { print_version_status(lang, &version_info); print_mode_menu(lang, &version_info); read_mode_choice(lang) }
    else { Mode::Visible };

    let app_path = match find_app_path(lang) { Ok(p) => p, Err(m) => { eprintln!("{}", m); process::exit(1); } };

    match mode { Mode::Hidden => run_hidden(&app_path, lang), Mode::Visible => run_visible(&app_path, lang), Mode::Detached => run_detached(&app_path, lang), }
}

impl VersionInfo {
    fn from_env() -> Self {
        let current = env::var("CLIDESK_VERSION").ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| "dev".to_string());
        let latest = env::var("CLIDESK_LATEST_VERSION").ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        let update_available = env::var("CLIDESK_UPDATE_AVAILABLE").ok().map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false);
        let update_command = env::var("CLIDESK_UPDATE_COMMAND").ok().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| "npm i -g clidesk".to_string());
        Self { current, latest, update_available, update_command }
    }
}

fn language_from_args(args: &[String]) -> Option<Language> {
    for a in args { if a == "--vi" { return Some(Language::Vi); } if a == "--en" { return Some(Language::En); } if let Some(v) = a.strip_prefix("--lang=") { return Language::from_code(v); } }
    args.iter().position(|a| a == "--lang").and_then(|p| args.get(p + 1)).and_then(|v| Language::from_code(v))
}
fn language_from_env() -> Option<Language> { env::var("CLIDESK_LAUNCH_LANG").ok().and_then(|v| Language::from_code(&v)) }
fn print_language_menu() { println!("╔════════════════════════╗\n║  1. Tiếng Việt        ║\n║  2. English            ║\n╚════════════════════════╝"); print!("Chọn (1 or 2): "); let _ = io::stdout().flush(); }
fn read_language_choice() -> Language { loop { let input = read_line_or_exit(""); match input.trim() { "1" => return Language::Vi, "2" => return Language::En, _ => { print!("1 or 2: "); let _ = io::stdout().flush(); } } } }
fn print_version_status(lang: Language, info: &VersionInfo) { match lang { Language::Vi => { println!("[CliDesk] Version: {}", info.current); if info.update_available { println!("[CliDesk] Update: {} {}", info.latest.as_deref().unwrap_or(""), info.update_command); } } Language::En => { println!("[CliDesk] Version: {}", info.current); if info.update_available { println!("[CliDesk] Update: {} {}", info.latest.as_deref().unwrap_or(""), info.update_command); } } } }
fn print_mode_menu(lang: Language, _info: &VersionInfo) { match lang { Language::Vi => { println!("╔════════════════════════╗\n║  1. Ẩn terminal       ║\n║  2. Giữ terminal       ║\n╚════════════════════════╝"); print!("Chọn (1 or 2): "); } Language::En => { println!("╔════════════════════════╗\n║  1. Hide terminal      ║\n║  2. Keep terminal      ║\n╚════════════════════════╝"); print!("Choose (1 or 2): "); } } let _ = io::stdout().flush(); }
fn read_mode_choice(lang: Language) -> Mode { loop { let input = read_line_or_exit(""); match input.trim() { "1" => return Mode::Hidden, "2" => return Mode::Visible, _ => { print!("{}", match lang { Language::Vi => "1 or 2: ", Language::En => "1 or 2: " }); let _ = io::stdout().flush(); } } } }
fn read_line_or_exit(msg: &str) -> String { let mut i = String::new(); match io::stdin().read_line(&mut i) { Ok(0) => { eprintln!("{}", msg); process::exit(1); } Ok(_) => i, Err(e) => { eprintln!("[CliDesk] {}", e); process::exit(1); } } }

fn find_app_path(_lang: Language) -> Result<std::path::PathBuf, String> {
    let p = env::current_exe().map_err(|e| format!("[CliDesk] {}", e))?;
    let d = p.parent().ok_or_else(|| "[CliDesk] Cannot resolve dir.".to_string())?.join("clidesk.exe");
    if !d.exists() { return Err(format!("[CliDesk] Not found: {}", d.display())); }
    Ok(d)
}

fn to_wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() }
fn from_wide(w: &[u16]) -> String { let e = w.iter().position(|&c| c == 0).unwrap_or(w.len()); String::from_utf16_lossy(&w[..e]) }

fn get_parent_pid(pid: DWORD) -> Option<DWORD> {
    unsafe {
        let h = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if h.is_null() || h as isize == -1 { return None; }
        let mut e: PROCESSENTRY32W = mem::zeroed(); e.dwSize = mem::size_of::<PROCESSENTRY32W>() as DWORD;
        if Process32FirstW(h, &mut e) == FALSE { CloseHandle(h); return None; }
        let mut r = None; loop { if e.th32ProcessID == pid { r = Some(e.th32ParentProcessID); break; } if Process32NextW(h, &mut e) == FALSE { break; } }
        CloseHandle(h); r
    }
}
fn get_process_name(pid: DWORD) -> Option<String> {
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
        if h.is_null() { return None; }
        let mut b = [0u16; MAX_PATH_W]; let mut s = MAX_PATH_W as DWORD;
        let r = QueryFullProcessImageNameW(h, 0, b.as_mut_ptr(), &mut s); CloseHandle(h);
        if r == FALSE { return None; }
        Path::new(&from_wide(&b[..s as usize])).file_name()?.to_str().map(|n| n.to_lowercase())
    }
}
fn find_window_by_pid(tp: DWORD) -> Option<HWND> {
    let mut c = EnumContext { target_pid: tp, found_hwnd: std::ptr::null_mut() };
    unsafe { EnumWindows(Some(ecb), &mut c as *mut EnumContext as LPARAM); }
    if c.found_hwnd.is_null() { None } else { Some(c.found_hwnd) }
}
unsafe extern "system" fn ecb(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let c = &mut *(lparam as *mut EnumContext); let mut p: DWORD = 0;
    GetWindowThreadProcessId(hwnd, &mut p);
    if p == c.target_pid && IsWindowVisible(hwnd) != FALSE { c.found_hwnd = hwnd; return FALSE; }
    TRUE
}
fn find_terminal_window() -> Option<HWND> {
    let mut cp = process::id() as DWORD;
    for _ in 0..8 {
        let p = get_parent_pid(cp)?;
        if let Some(n) = get_process_name(p) {
            if n == "windowsterminal.exe" || n == "wt.exe" { return find_window_by_pid(p); }
            if n == "conhost.exe" || n == "openconsole.exe" { cp = p; continue; }
        }
        cp = p;
    }
    None
}

fn print_debug_info(lang: Language) {
    let pid = process::id();
    let exe = env::current_exe().map(|p| p.display().to_string()).unwrap_or_else(|_| "?".to_string());
    let wt = is_windows_terminal();
    let hw = unsafe { GetConsoleWindow() };
    let hs = if hw.is_null() { "null".to_string() } else { format!("{:p}", hw) };
    let pp = get_parent_pid(pid as DWORD);
    let pn = pp.and_then(get_process_name);
    let th = if wt { "Windows Terminal" } else if !hw.is_null() { "conhost" } else { "none" };
    match lang {
        Language::Vi => { println!("[Debug] PID:{} Path:{} WT_SESSION:{} Console:{} Host:{} Parent:{} {:?}", pid, exe, if wt { "yes" } else { "no" }, hs, th, pp.unwrap_or(0), pn); }
        Language::En => { println!("[Debug] PID:{} Path:{} WT_SESSION:{} Console:{} Host:{} Parent:{} {:?}", pid, exe, if wt { "yes" } else { "no" }, hs, th, pp.unwrap_or(0), pn); }
    }
}
fn is_windows_terminal() -> bool { env::var("WT_SESSION").is_ok() }
fn hide_console_window_enhanced() -> HideResult {
    let h = unsafe { GetConsoleWindow() };
    if h.is_null() { return HideResult::Failed; }
    unsafe { ShowWindow(h, SW_HIDE); }
    if unsafe { IsWindowVisible(h) } == FALSE { return HideResult::Hidden; }
    unsafe { ShowWindow(h, SW_MINIMIZE); }
    if unsafe { IsWindowVisible(h) } == FALSE { return HideResult::Minimized; }
    HideResult::Failed
}

fn spawn_in_job(app: &Path, lang: Language) -> Result<(HANDLE, process::Child), String> {
    let d = app.parent().ok_or_else(|| format!("[CliDesk] Cannot resolve dir."))?;
    let mut c = process::Command::new(app).current_dir(d).env("CLIDESK_LAUNCH_LANG", lang.code()).spawn().map_err(|e| format!("{}", e))?;
    unsafe {
        let jn = to_wide(&format!("Local\\CliDeskJob_{}", process::id()));
        let j = CreateJobObjectW(std::ptr::null(), jn.as_ptr());
        if j.is_null() { let _ = c.kill(); let _ = c.wait(); return Err(format!("[CliDesk] Cannot create Job Object.")); }
        let mut i: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = mem::zeroed();
        i.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(j, JobObjectExtendedLimitInformation, &mut i as *mut _ as LPVOID, mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as DWORD) == FALSE { let _ = c.kill(); let _ = c.wait(); CloseHandle(j); return Err(format!("[CliDesk] Cannot set Job Object.")); }
        if AssignProcessToJobObject(j, c.as_raw_handle() as HANDLE) == FALSE { let _ = c.kill(); let _ = c.wait(); CloseHandle(j); return Err(format!("[CliDesk] Cannot assign to Job Object.")); }
        Ok((j, c))
    }
}

fn run_hidden(app: &Path, lang: Language) {
    if is_windows_terminal() {
        if let Some(w) = find_terminal_window() { unsafe { ShowWindow(w, SW_MINIMIZE); } } else { match lang { Language::Vi => println!("[CliDesk] Cannot hide WT."), Language::En => println!("[CliDesk] Cannot hide WT.") } }
    } else { match hide_console_window_enhanced() { HideResult::Hidden => {}, _ => {} } }
    match spawn_in_job(app, lang) { Ok((j, mut c)) => { let _ = c.wait(); unsafe { CloseHandle(j); } } Err(m) => { eprintln!("{}", m); process::exit(1); } }
}
fn run_visible(app: &Path, lang: Language) {
    match spawn_in_job(app, lang) { Ok((j, mut c)) => { let _ = c.wait(); unsafe { CloseHandle(j); } } Err(m) => { eprintln!("{}", m); process::exit(1); } }
}
fn run_detached(app: &Path, lang: Language) {
    let d = app.parent().expect("[CliDesk] Cannot resolve dir.");
    let _ = process::Command::new(app).current_dir(d).env("CLIDESK_LAUNCH_LANG", lang.code()).spawn();
}
