#!/usr/bin/env node
// CliDesk npm launcher — spawns the CliDesk desktop app
//
// Usage:
//   clidesk           → launch CliDesk
//
// Priority:
//   1. vendor/clidesk-launcher.exe (native launcher with menu)
//   2. vendor/clidesk.exe         (direct app binary)
//
// The launcher offers interactive menu (hidden/visible terminal).
// If the launcher is absent, the app is spawned directly.

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// ── Locate binaries ─────────────────────────────────────────────
const vendorDir = path.join(__dirname, '..', 'vendor');
const launcherPath = path.join(vendorDir, 'clidesk-launcher.exe');
const appPath = path.join(vendorDir, 'clidesk.exe');

let targetPath;
let targetName;

if (fs.existsSync(launcherPath)) {
    targetPath = launcherPath;
    targetName = 'clidesk-launcher.exe';
} else if (fs.existsSync(appPath)) {
    targetPath = appPath;
    targetName = 'clidesk.exe';
} else {
    console.error('[CliDesk] Binary not found.');
    console.error('[CliDesk] Try reinstalling: npm i -g clidesk');
    console.error('[CliDesk] Or download clidesk.exe from GitHub Releases');
    console.error('[CliDesk] and place it in: ' + vendorDir);
    process.exit(1);
}

// ── Forward all args ────────────────────────────────────────────
const args = process.argv.slice(2);

// ── Spawn ───────────────────────────────────────────────────────
//
// stdio: "inherit" — let the launcher/app use this terminal.
// windowsHide: false — don't suppress console windows.
// The native launcher manages its own console window (hidden/visible).

const child = spawn(targetPath, args, {
    stdio: 'inherit',
    windowsHide: false,
});

child.on('exit', (code) => {
    process.exit(code ?? 0);
});

child.on('error', (err) => {
    console.error('[CliDesk] Failed to start:', err.message);
    process.exit(1);
});
