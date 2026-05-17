# CliDesk — npm Launcher

> Desktop app launcher for [CliDesk](https://github.com/vykelongthuong/CliDesk) — install with a single command.

## What is CliDesk?

CliDesk is a local desktop dashboard for managing terminal sessions, files, and Git repositories — built with **Tauri v2** + **React** + **TypeScript** + **Rust**.

This npm package contains the CliDesk desktop app binary and a native Windows launcher.

## Install

```bash
npm i -g clidesk
```

## Usage

```bash
clidesk
```

The launcher copies the bundled binaries into a per-version runtime cache and opens CliDesk from there.

## Requirements

- **Windows x64**
- **Microsoft Edge WebView2 Runtime** — pre-installed on Windows 11 and Windows 10 (April 2018+). If missing, [download here](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).
- **Git**, **Node.js**, and any **AI CLI tools** must be installed separately if you want to use them inside CliDesk's embedded terminals.

## How it works

1. During `npm i -g clidesk`, the postinstall script checks that `vendor/clidesk.exe` and `vendor/clidesk-launcher.exe` are bundled.
2. When you run `clidesk`, the npm wrapper copies both binaries to `%LOCALAPPDATA%\CliDesk\npm-runtime\<version>\`.
3. The native launcher starts CliDesk from that runtime directory, not from `node_modules`.

The CliDesk desktop app itself is a Tauri application built from the [source repository](https://github.com/vykelongthuong/CliDesk).

## License

Apache-2.0
