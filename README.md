# CliDesk

[![npm version](https://img.shields.io/npm/v/clidesk.svg)](https://www.npmjs.com/package/clidesk)
[![license](https://img.shields.io/npm/l/clidesk.svg)](https://github.com/vykelongthuong/CliDesk/blob/main/LICENSE)
![platform](https://img.shields.io/badge/platform-Windows%20x64-blue)
![built with](https://img.shields.io/badge/built%20with-Tauri%20v2-orange)

**CliDesk** is a local desktop app for managing multiple terminals and AI coding CLI agents per project. Built with **Tauri v2** + **React** + **TypeScript** + **Rust**.

> 🇻🇳 [Phiên bản tiếng Việt ở bên dưới / Vietnamese version below](#tiếng-việt)

---

## Install via npm

```bash
npm i -g clidesk
```

Then launch:

```bash
clidesk
```

## Features

- **Project-based terminal management** — open multiple terminals per project, each with its own color.
- **File explorer & editor** — browse directories, open and edit files with Monaco Editor (syntax highlighting for many languages).
- **Markdown preview** — preview `.md` files with Edit / Preview / Split modes.
- **Git panel** — view Git status on demand (manual load, no auto-fetch).
- **Settings** — Vietnamese / English UI, font size, theme, security, system tray options.
- **System tray** — optionally minimize to tray when closing the window.

## CLI Commands

| Command | Description |
|---|---|
| `clidesk` | Open the app and return the terminal prompt immediately. |
| `clidesk --wait` | Open the app and keep the terminal attached until the app closes. |
| `clidesk --debug-launch` | Print runtime paths and launch diagnostics. |
| `clidesk --version` | Print the installed npm package version. |
| `clidesk --update` | Update CliDesk via `npm i -g clidesk@latest`. |

## Tech Stack

| Component | Technology |
|---|---|
| Frontend | React 19, TypeScript, Vite 7 |
| Platform | Tauri v2 |
| Backend | Rust (portable-pty, rusqlite) |
| Terminal | xterm.js with fit addon |
| Editor | Monaco Editor (@monaco-editor/react) |
| Database | SQLite (embedded via rusqlite) |
| Launcher (Windows) | Rust WinAPI (Job Object) |

## Development Requirements

- **Node.js** 18+ and **npm** 9+
- **Rust** stable (MSVC toolchain on Windows)
- **Microsoft C++ Build Tools** (Windows) — "Desktop development with C++" workload
- **Windows SDK**
- **WebView2 Runtime** — pre-installed on Windows 11 and Windows 10 (April 2018+)
- **Git**

## Getting Started (Development)

```bash
# 1. Install dependencies
npm install

# 2. Run in development mode
npm run dev
```

`npm run dev` starts the Vite dev server at `http://localhost:1420` and opens the Tauri desktop window.

### Useful Commands

| Command | Description |
|---|---|
| `npm run dev` | Run app in development mode |
| `npm run dev:web` | Run frontend web only (no Tauri) |
| `npm run typecheck` | TypeScript type checking |
| `npm run build` | Build frontend |
| `npm run build:win` | Build Windows release (portable) |
| `npm run clean:build` | Clean build artifacts (dist, target) |

## Project Structure

```
clidesk/
├── src/                    # React frontend
│   ├── components/         # UI components
│   ├── lib/                # Utilities, i18n, commands
│   ├── types/              # TypeScript type definitions
│   ├── App.tsx             # Main component
│   └── main.tsx            # Entry point
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── commands/       # Tauri commands
│   │   └── services/       # Business logic
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src-launcher/           # Windows Launcher (Job Object)
├── npm/clidesk/            # npm package wrapper
├── package.json
└── README.md
```

## Security

- **No terminal output logging** — all terminal data is ephemeral.
- **No administrator privileges required** — runs with `asInvoker`.
- **Project boundary enforcement** — Rust backend validates file paths to prevent access outside project directories.
- **No hardcoded secrets/tokens** — no passwords or API keys in source code.
- **Atomic file writes** — uses temp + rename to prevent data corruption.

## License

This project is licensed under the [Apache License 2.0](LICENSE).

---

# Tiếng Việt

**CliDesk** là ứng dụng desktop local giúp quản lý nhiều terminal/AI CLI coding agent theo dự án. Được xây dựng với **Tauri v2** + **React** + **TypeScript** + **Rust**.

## Cài đặt qua npm

```bash
npm i -g clidesk
```

Sau đó chạy:

```bash
clidesk
```

## Tính năng chính

- **Quản lý nhiều dự án** — Thêm, chọn, xóa dự án từ sidebar.
- **Terminal nhúng** — Mở nhiều terminal theo từng dự án, mỗi terminal có màu sắc riêng theo dự án.
- **File Explorer / Editor** — Duyệt cây thư mục, mở và chỉnh sửa file với Monaco Editor (hỗ trợ syntax highlighting cho nhiều ngôn ngữ).
- **Markdown Preview** — Xem trước file Markdown với chế độ Edit / Preview / Split.
- **Git** — Xem trạng thái Git theo yêu cầu (Load Git thủ công, không tự động tải).
- **Cài đặt** — Hỗ trợ giao diện Tiếng Việt / English, tùy chỉnh font size, theme, bảo mật, system tray.
- **System Tray** — Ẩn xuống tray khi đóng cửa sổ (tùy chọn).

## Công nghệ sử dụng

| Thành phần | Công nghệ |
|---|---|
| Giao diện | React 19, TypeScript, Vite 7 |
| Nền tảng | Tauri v2 |
| Backend | Rust (portable-pty, rusqlite) |
| Terminal | xterm.js với fit addon |
| Editor | Monaco Editor (@monaco-editor/react) |
| Cơ sở dữ liệu | SQLite (qua rusqlite, nhúng sẵn) |
| Launcher (Windows) | Rust WinAPI (Job Object) |

## Yêu cầu môi trường phát triển

- **Node.js** 18+ và **npm** 9+
- **Rust** stable (MSVC toolchain trên Windows)
- **Microsoft C++ Build Tools** (Windows) — workload "Desktop development with C++"
- **Windows SDK** (Windows)
- **WebView2 Runtime** — Có sẵn trên Windows 11 và Windows 10 (April 2018+)
- **Git**

## Cài đặt và chạy thử

```bash
# 1. Cài đặt dependencies
npm install

# 2. Chạy ở chế độ phát triển
npm run dev
```

Lệnh `npm run dev` sẽ khởi động Vite dev server tại `http://localhost:1420` và mở cửa sổ Tauri desktop.

### Các lệnh hữu ích

| Lệnh | Mô tả |
|---|---|
| `npm run dev` | Chạy app ở chế độ phát triển |
| `npm run dev:web` | Chạy frontend web (không Tauri) |
| `npm run typecheck` | Kiểm tra lỗi TypeScript |
| `npm run build` | Build frontend |
| `npm run build:win` | Build Windows release (portable) |
| `npm run clean:build` | Xóa build artifacts (dist, target) |

## Cấu trúc thư mục

```
clidesk/
├── src/                    # React frontend
│   ├── components/         # Components UI
│   ├── lib/                # Tiện ích, i18n, commands
│   ├── types/              # Định nghĩa kiểu TypeScript
│   ├── App.tsx             # Component chính
│   └── main.tsx            # Điểm vào
├── src-tauri/              # Backend Rust
│   ├── src/
│   │   ├── commands/       # Lệnh Tauri
│   │   └── services/       # Logic nghiệp vụ
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src-launcher/           # Windows Launcher (Job Object)
├── npm/clidesk/            # npm package wrapper
├── package.json
└── README.md
```

## Bảo mật

- **Không lưu terminal output/log** — Tất cả dữ liệu terminal là tạm thời.
- **Không yêu cầu quyền Administrator** — Ứng dụng chạy với `asInvoker`.
- **Project boundary** — Backend Rust kiểm tra đường dẫn file, ngăn truy cập ngoài thư mục dự án.
- **Không hardcode secret/token** — Không lưu mật khẩu hay API key trong mã nguồn.
- **Atomic write** — Ghi file dùng cơ chế temp + rename để tránh hỏng dữ liệu.

## Giấy phép

Dự án này được cấp phép theo [Apache License 2.0](LICENSE).
