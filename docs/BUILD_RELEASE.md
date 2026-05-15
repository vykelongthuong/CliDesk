# Build & Release Guide — CliDesk

This document describes how to build CliDesk for distribution on **Windows** and **Linux**.

---

## Requirements

| Tool | Version (min) | Notes |
|------|---------------|-------|
| Node.js | 18+ | [nodejs.org](https://nodejs.org) |
| npm | 9+ | Comes with Node.js |
| Rust | stable MSVC | [rustup.rs](https://rustup.rs) — use `rustup default stable-msvc` on Windows |
| Microsoft C++ Build Tools | 2022 | [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) — select "Desktop development with C++" workload |
| Windows SDK | 10.x | Included with Visual Studio Build Tools |
| WebView2 | — | Pre-installed on Windows 11 / Windows 10 (April 2018+). [Download](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) if missing |

---

## Setup

```bash
# Install frontend dependencies
npm install
```

---

## Build Commands

### Windows — release with Administrator elevation

```bash
npm run build:win
```

Equivalent to:

```bash
npm run tauri build --target windows-msvc
```

### Windows — dev mode

```bash
npm run dev
```

**Note:** Dev mode does **not** embed the administrator manifest. If you need to test admin features during development, run the dev binary **as Administrator** manually:
1. Open a terminal **as Administrator**
2. `npm run dev`

### Linux — release

```bash
npm run tauri build
```

---

## Output Files

### Windows

| File | Description |
|------|-------------|
| `src-tauri/target/release/clidesk.exe` | **Portable executable** — standalone, no installer needed. Run directly. |
| `src-tauri/target/release/bundle/nsis/CliDesk_0.1.0_x64-setup.exe` | NSIS installer (if configured) |
| `src-tauri/target/release/bundle/msi/CliDesk_0.1.0_x64.msi` | MSI installer (if configured) |

### Linux

| File | Description |
|------|-------------|
| `src-tauri/target/release/clidesk` | Portable binary |
| `src-tauri/target/release/bundle/appimage/clidesk_0.1.0_amd64.AppImage` | AppImage (if configured) |
| `src-tauri/target/release/bundle/deb/clidesk_0.1.0_amd64.deb` | Debian/Ubuntu package (if configured) |

---

## Administrator Execution (Windows)

The Windows build embeds a manifest that sets `requestedExecutionLevel="requireAdministrator"`.

**What this means:**
- When you launch `clidesk.exe`, Windows will show a **UAC prompt** before the app starts.
- The app **must** be confirmed by an administrator to run.
- All terminal child processes spawned inside CliDesk **inherit** administrator privileges.
- The existing **Admin Terminal** feature works immediately — no additional UAC prompt.

**How to verify the app is running as administrator:**
1. Launch `clidesk.exe` and accept the UAC prompt.
2. In any terminal inside CliDesk, run:
   ```cmd
   net session
   ```
   - If running as admin, you will see: `There are no entries in the list.` (no error).
   - If **not** running as admin, you will see: `Access is denied.`
3. Or in PowerShell:
   ```powershell
   [Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent() | 
     ForEach-Object { $_.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator) }
   ```
   - Returns `True` if running elevated, `False` otherwise.

---

## Linux Notes

- The Linux build does **not** embed an administrator manifest — Linux GUI apps should not run as root by default.
- If you need root-level terminal access, use `sudo` inside the CliDesk terminal session.
- If you build on Linux, the output is a native binary plus optionally an AppImage, `.deb`, or `.rpm` depending on the Tauri bundle configuration.

---

## Important Caveats

| Issue | Details |
|-------|---------|
| **Code signing** | The executable is **not code-signed**. Windows SmartScreen may show a warning: *"Windows protected your PC"*. This is expected for unsigned apps. Click **"Run anyway"** or **"More info → Run anyway"** to proceed. |
| **WebView2** | Windows 10 (April 2018+) and Windows 11 include WebView2. Older Windows versions may need a [manual install](https://developer.microsoft.com/en-us/microsoft-edge/webview2/). |
| **Portable .exe** | `src-tauri/target/release/clidesk.exe` is fully standalone. Copy it to another Windows machine (same architecture, x64) and run it directly. No installation needed. |
| **Linux builds** | Must be built on Linux. Cross-compilation from Windows to Linux is not covered here. |
| **UAC bypass** | This configuration does **not** bypass UAC. The `requireAdministrator` manifest triggers a standard UAC elevation prompt — the user must confirm. |
