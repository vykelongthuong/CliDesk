# CliDesk

[![npm version](https://img.shields.io/npm/v/clidesk.svg)](https://www.npmjs.com/package/clidesk)
[![license](https://img.shields.io/npm/l/clidesk.svg)](https://github.com/vykelongthuong/CliDesk/blob/main/LICENSE)
![platform](https://img.shields.io/badge/platform-Windows%20x64-blue)
![built with](https://img.shields.io/badge/built%20with-Tauri%20v2-orange)

**A local desktop dashboard for managing project-based terminals, files, Git status, and AI coding CLI tools** such as Codex CLI, Claude CLI, DeepSeek CLI, and other terminal-based developer agents.

## Installation

```bash
npm i -g clidesk
```

## Launch

```bash
clidesk
```

- `clidesk` opens the desktop app immediately.
- The terminal prompt returns right away.
- Language selection happens inside the app on first launch.
- Users can change language later in **Settings**.

## CLI Commands

| Command | Description |
|---|---|
| `clidesk` | Open the app and return the terminal prompt immediately. |
| `clidesk --wait` | Open the app and keep the terminal attached until the app closes. Useful for debugging. |
| `clidesk --debug-launch` | Print runtime paths and launch diagnostics. |
| `clidesk --version` | Print the installed CliDesk npm package version. |
| `clidesk --update` | Update CliDesk from npm using `npm i -g clidesk@latest`. |

## Features

- **Project-based terminal management** — organize multiple terminals per project.
- **File explorer and editor** — browse and edit project files.
- **Markdown preview** — preview `.md` files directly.
- **Manual Git panel** — view status, stage, commit, and push.
- **Vietnamese / English UI** — full i18n support.
- **Dark / Light theme** — switch anytime in Settings.
- **Local-first desktop app** — no cloud, no account required.
- **npm global launcher** — install once, run anywhere with `clidesk`.

## Requirements

- **Windows x64**
- **Node.js** and **npm**
- **Microsoft Edge WebView2 Runtime** — pre-installed on Windows 11 and Windows 10 (April 2018+). If missing, [download here](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).
- **Git** — optional, needed for the Git panel.
- **AI CLI tools** (Codex CLI, Claude CLI, DeepSeek CLI, etc.) — must be installed separately by the user.

## How It Works

1. During `npm i -g clidesk`, the npm package includes bundled Windows binaries.
2. When you run `clidesk`, the wrapper copies binaries to a per-version runtime cache:

   ```text
   %LOCALAPPDATA%\CliDesk\npm-runtime\<version>\
   ```

3. The app runs from the runtime cache, **not** directly from `node_modules`.
4. This helps avoid Windows file-locking issues during npm update or uninstall.

## Update

```bash
clidesk --update
```

Or manually:

```bash
npm i -g clidesk@latest
```

## Uninstall

```bash
npm uninstall -g clidesk
```

## Privacy & Security

- CliDesk runs **locally** — no data is sent to external servers.
- Terminal output is **not logged** by default.
- CliDesk does **not** bundle API keys.
- External AI CLI tools are configured separately by the user.
- No administrator permission is required by default.

## Links

- **GitHub**: [github.com/vykelongthuong/CliDesk](https://github.com/vykelongthuong/CliDesk)
- **Issues**: [github.com/vykelongthuong/CliDesk/issues](https://github.com/vykelongthuong/CliDesk/issues)
- **npm**: [npmjs.com/package/clidesk](https://www.npmjs.com/package/clidesk)

---

## Tiếng Việt

CliDesk là ứng dụng desktop local để quản lý nhiều terminal theo project, tích hợp trình quản lý file, Git, và hỗ trợ các AI coding CLI tool.

### Cài đặt

```bash
npm i -g clidesk
```

### Chạy

```bash
clidesk
```

- Lệnh `clidesk` sẽ mở app và trả terminal về prompt ngay.
- Chọn ngôn ngữ trong app ở lần chạy đầu tiên.
- Có thể đổi ngôn ngữ trong **Cài đặt**.

### Cập nhật

```bash
clidesk --update
```

Hoặc:

```bash
npm i -g clidesk@latest
```

---

## License

Apache-2.0
