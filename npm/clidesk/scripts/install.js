// CliDesk postinstall script.
//
// It only validates that the package contains the Windows binaries needed by
// bin/clidesk.js. It does not launch the app, copy runtime files, download
// binaries, request admin rights, or use credentials.

const fs = require('fs');
const path = require('path');

const vendorDir = path.join(__dirname, '..', 'vendor');
const requiredBinaries = [
    path.join(vendorDir, 'clidesk.exe'),
    path.join(vendorDir, 'clidesk-launcher.exe'),
];

function main() {
    if (process.platform !== 'win32') {
        console.warn('[CliDesk] This package only supports Windows x64.');
        return;
    }

    if (process.arch !== 'x64') {
        console.warn('[CliDesk] This package only supports x64 architecture.');
        return;
    }

    const missing = requiredBinaries.filter((filePath) => !fs.existsSync(filePath));
    if (missing.length > 0) {
        console.error('[CliDesk] Package is missing required binaries:');
        for (const filePath of missing) {
            console.error('[CliDesk] Missing:', filePath);
        }
        console.error('[CliDesk] Please reinstall with: npm i -g clidesk');
        process.exit(1);
    }

    console.log('[CliDesk] Vendor binaries found.');
}

main();
