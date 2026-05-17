# CliDesk

![Phiên bản](https://img.shields.io/badge/phi%C3%AAn_b%E1%BA%A3n-0.1.0-blue)
![Stack](https://img.shields.io/badge/stack-Tauri_v2_%7C_React_%7C_TypeScript_%7C_Rust-blue)

**CliDesk** là ứng dụng desktop local giúp quản lý nhiều terminal/AI CLI coding agent theo dự án. Được xây dựng với **Tauri v2** + **React** + **TypeScript** + **Rust**.

---

## Tính năng chính

- **Quản lý nhiều dự án** — Thêm, chọn, xóa dự án từ sidebar.
- **Terminal nhúng** — Mở nhiều terminal theo từng dự án, mỗi terminal có màu sắc riêng theo dự án.
- **File Explorer / Editor** — Duyệt cây thư mục, mở và chỉnh sửa file với Monaco Editor (hỗ trợ syntax highlighting cho nhiều ngôn ngữ).
- **Markdown Preview** — Xem trước file Markdown với chế độ Edit / Preview / Split.
- **Git** — Xem trạng thái Git theo yêu cầu (Load Git thủ công, không tự động tải).
- **Cài đặt** — Hỗ trợ giao diện Tiếng Việt / English, tùy chỉnh font size, theme, bảo mật, system tray.
- **System Tray** — Ẩn xuống tray khi đóng cửa sổ (tùy chọn).

---

## Công nghệ sử dụng

| Thành phần | Công nghệ |
|------------|-----------|
| Giao diện | React 19, TypeScript, Vite 7 |
| Nền tảng | Tauri v2 |
| Backend | Rust (portable-pty, rusqlite) |
| Terminal | xterm.js với fit addon |
| Editor | Monaco Editor (@monaco-editor/react) |
| Cơ sở dữ liệu | SQLite (qua rusqlite, nhúng sẵn) |
| Launcher (Windows) | Rust WinAPI (Job Object) |

---

## Yêu cầu môi trường phát triển

- **Node.js** 18+ và **npm** 9+
- **Rust** stable (MSVC toolchain trên Windows)
- **Microsoft C++ Build Tools** (Windows) — workload "Desktop development with C++"
- **Windows SDK** (Windows)
- **WebView2 Runtime** — Có sẵn trên Windows 11 và Windows 10 (April 2018+)
- **Git**

---

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
|------|-------|
| `npm run dev` | Chạy app ở chế độ phát triển |
| `npm run dev:web` | Chạy frontend web (không Tauri) |
| `npm run typecheck` | Kiểm tra lỗi TypeScript |
| `npm run build` | Build frontend |
| `npm run build:win` | Build Windows release (portable) |
| `npm run clean:build` | Xóa build artifacts (dist, target) |

---

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
├── package.json
└── README.md
```

---

## Bảo mật

- **Không lưu terminal output/log** — Tất cả dữ liệu terminal là tạm thời.
- **Không yêu cầu quyền Administrator** — Ứng dụng chạy với `asInvoker`.
- **Project boundary** — Backend Rust kiểm tra đường dẫn file, ngăn truy cập ngoài thư mục dự án.
- **Không hardcode secret/token** — Không lưu mật khẩu hay API key trong mã nguồn.
- **Atomic write** — Ghi file dùng cơ chế temp + rename để tránh hỏng dữ liệu.

---

## Giấy phép

Dự án này được cấp phép theo [Apache License 2.0](LICENSE).
