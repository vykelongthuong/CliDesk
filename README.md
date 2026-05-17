# CliDesk

A local desktop dashboard for managing terminal sessions, files, and Git repositories — built with **Tauri v2** + **React** + **TypeScript** + **Rust**.

![Tech Stack](https://img.shields.io/badge/stack-Tauri_v2_%7C_React_%7C_TypeScript_%7C_Rust-blue)

---

## Overview

CliDesk provides an integrated development environment in a native desktop window:

- **Terminals** — Embedded PTY terminals via xterm.js + Rust backend (portable-pty)
- **Files** — File explorer and text editor with syntax highlighting (Monaco)
- **Git** — View current branch, changed files, and git status
- **Settings** — Theme, font size, language (Tiếng Việt / English), system tray behavior

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Frontend | React 19, TypeScript, Vite 7 |
| Desktop | Tauri v2 |
| Backend | Rust, portable-pty, rusqlite |
| Terminal | xterm.js with fit addon |
| Editor | Monaco Editor (@monaco-editor/react) |
| Database | SQLite (via rusqlite, bundled) |

---

## Requirements

- **Node.js** 18+ and **npm** 9+
- **Rust** stable (MSVC toolchain on Windows)
- **Microsoft C++ Build Tools** (Windows only) — "Desktop development with C++" workload
- **Windows SDK** (Windows only)

---

## Getting Started

```bash
# Install frontend dependencies
npm install

# Run in development mode
npm run dev
```

This starts the Vite dev server on `http://localhost:1420` and opens the Tauri desktop window.

---

## Build

### Windows — release

```bash
npm run build:win
```

The output executable is at:
```
src-tauri/target/release/clidesk.exe
```

The Windows build embeds a manifest with `requestedExecutionLevel="asInvoker"`, so launching `clidesk.exe` should not show a UAC elevation prompt.

### Linux — release

```bash
npm run tauri build
```

Linux builds do **not** embed an administrator manifest. Use `sudo` inside terminal sessions if root access is needed.

### Output locations

| Platform | File |
|----------|------|
| Windows | `src-tauri/target/release/clidesk.exe` (portable) |
| Windows (installer) | `src-tauri/target/release/bundle/nsis/` or `msi/` |
| Linux | `src-tauri/target/release/clidesk` |
| Linux (AppImage) | `src-tauri/target/release/bundle/appimage/` |

---

## Important Notes

- **No terminal output is saved** — CliDesk does not log or persist terminal output. All session data is ephemeral.
- **On Windows**, the app runs without requiring Administrator privileges. The `.exe` is not code-signed, so Windows SmartScreen may show a warning. Click **"Run anyway"** to proceed.
- **WebView2** is required on Windows. It is pre-installed on Windows 11 and Windows 10 (April 2018+). Older versions may need a [manual install](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).
- **Linux builds** must be built on Linux — cross-compilation from Windows is not covered.

---

## Project Structure

```
clidesk/
├── src/                    # React frontend
│   ├── components/         # UI components
│   ├── lib/                # Utilities, i18n, commands
│   ├── types/              # TypeScript type definitions
│   ├── App.tsx             # Main app component
│   └── main.tsx            # Entry point
├── src-tauri/              # Rust backend
│   ├── src/                # Rust source
│   │   ├── commands/       # Tauri commands
│   │   └── services/       # Business logic
│   ├── Cargo.toml
│   └── tauri.conf.json
├── package.json
└── README.md
```

---

## License

This project is licensed under the Apache License 2.0. See the [LICENSE](LICENSE) file for details.
