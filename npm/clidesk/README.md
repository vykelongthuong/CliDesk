# CliDesk — npm Launcher

> Desktop app launcher for [CliDesk](https://github.com/vykelongthuong/CliDesk) — install with a single command.

## What is CliDesk?

CliDesk is a local desktop dashboard for managing terminal sessions, files, and Git repositories — built with **Tauri v2** + **React** + **TypeScript** + **Rust**.

This npm package is the launcher/downloader for the CliDesk desktop app.

## Install

```bash
npm i -g clidesk
```

## Usage

```bash
clidesk
```

The launcher will find and open the CliDesk desktop app.

## Requirements

- **Windows x64**
- **Microsoft Edge WebView2 Runtime** — pre-installed on Windows 11 and Windows 10 (April 2018+). If missing, [download here](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).
- **Git**, **Node.js**, and any **AI CLI tools** must be installed separately if you want to use them inside CliDesk's embedded terminals.

## How it works

1. During `npm i -g clidesk`, the postinstall script checks for the CliDesk binary in `vendor/clidesk.exe`.
2. If the binary is missing, instructions are shown to download it from GitHub Releases.
3. When you run `clidesk`, the launcher spawns the desktop app.

This npm package is only the launcher. The CliDesk desktop app itself is a Tauri application built from the [source repository](https://github.com/vykelongthuong/CliDesk).

## License

MIT
