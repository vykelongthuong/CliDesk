#!/usr/bin/env node
// CliDesk npm wrapper — spawns the native launcher
//
// Usage:
//   clidesk                 → interactive menu
//   clidesk --hidden        → skip menu, hide terminal
//   clidesk --visible       → skip menu, keep terminal
//   clidesk --app <path>    → explicit path to clidesk.exe

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// ── Locate launcher ────────────────────────────────────────────
const vendorDir = path.join(__dirname, '..', 'vendor');
const launcherPath = path.join(vendorDir, 'clidesk-launcher.exe');

if (!fs.existsSync(launcherPath)) {
    console.error('[CliDesk] Launcher not found:', launcherPath);
    process.exit(1);
}

// ── Forward all args ───────────────────────────────────────────
const args = process.argv.slice(2);

// ── Spawn launcher ─────────────────────────────────────────────
//
// stdio: "inherit" — launcher reads stdin for menu choice,
//                     writes menu/stdout to this terminal.
// windowsHide: false — let the launcher manage its own console.

const child = spawn(launcherPath, args, {
    stdio: 'inherit',
    windowsHide: false,
});

// ── Forward exit code ─────────────────────────────────────────
child.on('exit', (code) => {
    process.exit(code ?? 0);
});

child.on('error', (err) => {
    console.error('[CliDesk] Failed to start launcher:', err.message);
    process.exit(1);
});
