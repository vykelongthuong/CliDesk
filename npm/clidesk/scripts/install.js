// CliDesk postinstall script
//
// Responsibilities:
//   1. Detect platform (win32 x64)
//   2. Ensure vendor/ directory exists
//   3. Check if vendor/clidesk.exe exists
//   4. If not, provide clear download instructions
//
// No admin required. No process spawned. No downloads.

const fs = require('fs');
const path = require('path');

const VENDOR_DIR = path.join(__dirname, '..', 'vendor');
const APP_BINARY = path.join(VENDOR_DIR, 'clidesk.exe');

function main() {
    // ── Platform check ──────────────────────────────────────
    if (process.platform !== 'win32') {
        console.warn('[CliDesk] This package only supports Windows x64.');
        console.warn('[CliDesk] Skipping binary setup.');
        return;
    }

    if (process.arch !== 'x64') {
        console.warn('[CliDesk] This package only supports x64 architecture.');
        console.warn('[CliDesk] Skipping binary setup.');
        return;
    }

    // ── Ensure vendor directory ─────────────────────────────
    if (!fs.existsSync(VENDOR_DIR)) {
        fs.mkdirSync(VENDOR_DIR, { recursive: true });
        console.log('[CliDesk] Created vendor directory.');
    }

    // ── Check for existing binary ───────────────────────────
    if (fs.existsSync(APP_BINARY)) {
        console.log('[CliDesk] Binary found in vendor/.');
        return;
    }

    // ── Binary missing — instructions ───────────────────────
    console.log('');
    console.log('╔══════════════════════════════════════════════╗');
    console.log('║          CliDesk Binary Setup                ║');
    console.log('╠══════════════════════════════════════════════╣');
    console.log('║                                              ║');
    console.log('║  CliDesk binary is not bundled yet.           ║');
    console.log('║                                              ║');
    console.log('║  To complete setup:                           ║');
    console.log('║                                              ║');
    console.log('║  1. Download clidesk.exe from:                ║');
    console.log('║     GitHub Releases                           ║');
    console.log('║     https://github.com/vykelongthuong/CliDesk  ║');
    console.log('║                                              ║');
    console.log('║  2. Place it in:                              ║');
    console.log('║     ' + VENDOR_DIR.padEnd(42) + '║');
    console.log('║                                              ║');
    console.log('║  3. Run: clidesk                              ║');
    console.log('║                                              ║');
    console.log('╚══════════════════════════════════════════════╝');
    console.log('');
}

main();
