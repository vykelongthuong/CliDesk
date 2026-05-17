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

use std::env;
use std::io::{self, Write};
use std::mem;
use std::path::PathBuf;
use std::process;

// ── Windows type aliases ───────────────────────────────────────
type BOOL = i32;
type DWORD = u32;
type HANDLE = *mut std::ffi::c_void;
type LPVOID = *mut std::ffi::c_void;
type LPCVOID = *const std::ffi::c_void;
type LPCWSTR = *const u16;
type PCWSTR = *const u16;
type HWND = HANDLE;
type SIZE_T = usize;
type ULONG_PTR = usize;
const FALSE: BOOL = 0;
const TRUE: BOOL = 1;

// ── Constants ──────────────────────────────────────────────────
const SW_HIDE: i32 = 0;
const CREATE_NO_WINDOW: DWORD = 0x08000000;
const INFINITE: DWORD = 0xFFFFFFFF;
const WAIT_OBJECT_0: DWORD = 0;

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
struct STARTUPINFOW {
    cb: DWORD,
    lpReserved: LPCWSTR,
    lpDesktop: LPCWSTR,
    lpTitle: LPCWSTR,
    dwX: DWORD,
    dwY: DWORD,
    dwXSize: DWORD,
    dwYSize: DWORD,
    dwXCountChars: DWORD,
    dwYCountChars: DWORD,
    dwFillAttribute: DWORD,
    dwFlags: DWORD,
    wShowWindow: u16,
    cbReserved2: u16,
    lpReserved2: *mut u8,
    hStdInput: HANDLE,
    hStdOutput: HANDLE,
    hStdError: HANDLE,
}

#[repr(C)]
struct PROCESS_INFORMATION {
    hProcess: HANDLE,
    hThread: HANDLE,
    dwProcessId: DWORD,
    dwThreadId: DWORD,
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

    fn CreateProcessW(
        lpApplicationName: LPCWSTR,
        lpCommandLine: LPCWSTR,
        lpProcessAttributes: *const SECURITY_ATTRIBUTES,
        lpThreadAttributes: *const SECURITY_ATTRIBUTES,
        bInheritHandles: BOOL,
        dwCreationFlags: DWORD,
        lpEnvironment: LPVOID,
        lpCurrentDirectory: LPCWSTR,
        lpStartupInfo: *const STARTUPINFOW,
        lpProcessInformation: *mut PROCESS_INFORMATION,
    ) -> BOOL;

    fn WaitForSingleObject(
        hHandle: HANDLE,
        dwMilliseconds: DWORD,
    ) -> DWORD;

    fn CloseHandle(hObject: HANDLE) -> BOOL;

    fn TerminateProcess(
        hProcess: HANDLE,
        uExitCode: u32,
    ) -> BOOL;

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

fn main() {
    let args: Vec<String> = env::args().collect();

    // ── CLI flags ──────────────────────────────────────────────
    let mode = if args.iter().any(|a| a == "--hidden") {
        Mode::Hidden
    } else if args.iter().any(|a| a == "--visible") {
        Mode::Visible
    } else {
        print_menu();
        read_choice()
    };

    // ── Locate clidesk.exe ─────────────────────────────────────
    let app_path = find_app_path(&args);

    // ── Run ────────────────────────────────────────────────────
    match mode {
        Mode::Hidden => run_hidden(&app_path),
        Mode::Visible => run_visible(&app_path),
    }
}

// ── Menu ───────────────────────────────────────────────────────

fn print_menu() {
    println!("╔══════════════════════════════════╗");
    println!("║        CliDesk Launcher          ║");
    println!("╠══════════════════════════════════╣");
    println!("║  1. Ẩn terminal, chỉ hiện app    ║");
    println!("║  2. Giữ terminal hiển thị        ║");
    println!("╚══════════════════════════════════╝");
    print!("Chọn (1 hoặc 2): ");
    let _ = io::stdout().flush();
}

fn read_choice() -> Mode {
    loop {
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            process::exit(0);
        }
        match input.trim() {
            "1" => return Mode::Hidden,
            "2" => return Mode::Visible,
            _ => {
                print!("Nhập sai. Chọn 1 hoặc 2: ");
                let _ = io::stdout().flush();
            }
        }
    }
}

// ── Path resolution ───────────────────────────────────────────

fn find_app_path(args: &[String]) -> String {
    // 1. --app flag
    if let Some(pos) = args.iter().position(|a| a == "--app") {
        if let Some(path) = args.get(pos + 1) {
            return path.trim_matches('"').replace('/', "\\");
        }
    }

    // 2. CLIDESK_APP_PATH env
    if let Ok(path) = env::var("CLIDESK_APP_PATH") {
        return path;
    }

    // 3. Sibling in same directory as launcher (npm vendor layout)
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            let sibling = parent.join("clidesk.exe");
            if sibling.exists() {
                return sibling.to_string_lossy().to_string();
            }
        }
    }

    // 4. Dev fallback — look for cargo-built binary
    let dev_patterns: &[&str] = &[
        "src-tauri\\target\\x86_64-pc-windows-msvc\\release\\clidesk.exe",
        "src-tauri\\target\\x86_64-pc-windows-msvc\\debug\\clidesk.exe",
        "src-tauri\\target\\release\\clidesk.exe",
        "src-tauri\\target\\debug\\clidesk.exe",
        "target\\x86_64-pc-windows-msvc\\release\\clidesk.exe",
        "target\\release\\clidesk.exe",
    ];
    for p in dev_patterns {
        let path = PathBuf::from(p);
        if path.exists() {
            return p.to_string();
        }
    }

    eprintln!("[CliDesk Launcher] Không tìm thấy clidesk.exe.");
    eprintln!("Dùng --app <path> hoặc đặt biến môi trường CLIDESK_APP_PATH.");
    process::exit(1);
}

// ── Wide string helper ────────────────────────────────────────
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ── Job Object helpers ─────────────────────────────────────────
unsafe fn spawn_in_job(app_path: &str) -> (HANDLE, HANDLE) {
    // Create Job Object with unique name
    let job_name = format!("Local\\CliDeskJob_{}", process::id());
    let job_name_wide = to_wide(&job_name);
    let job = CreateJobObjectW(std::ptr::null(), job_name_wide.as_ptr());
    if job.is_null() {
        panic!("CreateJobObjectW failed");
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
        CloseHandle(job);
        panic!("SetInformationJobObject failed");
    }

    // Create process
    let app_cmd = format!("\"{}\"", app_path);
    let app_wide = to_wide(&app_cmd);

    let mut si: STARTUPINFOW = mem::zeroed();
    si.cb = mem::size_of::<STARTUPINFOW>() as DWORD;

    let mut pi: PROCESS_INFORMATION = mem::zeroed();

    let ret = CreateProcessW(
        app_wide.as_ptr(),
        std::ptr::null(),
        std::ptr::null(),
        std::ptr::null(),
        FALSE,
        CREATE_NO_WINDOW,
        std::ptr::null_mut(),
        std::ptr::null(),
        &si,
        &mut pi,
    );
    if ret == FALSE {
        CloseHandle(job);
        panic!("CreateProcessW failed for '{}'", app_path);
    }

    // Assign to job
    let ret = AssignProcessToJobObject(job, pi.hProcess);
    if ret == FALSE {
        TerminateProcess(pi.hProcess, 1);
        CloseHandle(pi.hProcess);
        CloseHandle(pi.hThread);
        CloseHandle(job);
        panic!("AssignProcessToJobObject failed");
    }

    // Close thread handle — only need process handle
    CloseHandle(pi.hThread);

    (job, pi.hProcess)
}

// ── Hidden mode ────────────────────────────────────────────────
fn run_hidden(app_path: &str) {
    unsafe {
        let hwnd = GetConsoleWindow();
        if !hwnd.is_null() {
            ShowWindow(hwnd, SW_HIDE);
        }
    }

    let (job, process) = unsafe { spawn_in_job(app_path) };

    unsafe {
        WaitForSingleObject(process, INFINITE);
    }

    unsafe {
        CloseHandle(process);
        CloseHandle(job);
    }
}

// ── Visible mode ───────────────────────────────────────────────
fn run_visible(app_path: &str) {
    println!("[CliDesk] Đang khởi động CliDesk...");
    println!("[CliDesk] Terminal này sẽ đợi CliDesk kết thúc.");
    println!("[CliDesk] Đóng cửa sổ terminal này sẽ buộc đóng CliDesk.");
    println!();

    let (job, process) = unsafe { spawn_in_job(app_path) };

    unsafe {
        WaitForSingleObject(process, INFINITE);
    }

    unsafe {
        CloseHandle(process);
        CloseHandle(job);
    }

    println!();
    println!("[CliDesk] Ứng dụng đã đóng.");
}
