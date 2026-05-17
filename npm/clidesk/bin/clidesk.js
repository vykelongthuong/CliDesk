#!/usr/bin/env node
// CliDesk npm launcher.
//
// The package stores signed binaries in vendor/, but it never runs them from
// node_modules. Each package version is copied to a runtime cache first so npm
// can update or uninstall the global package without Windows keeping vendor/
// locked by a running desktop process.

const fs = require('fs');
const path = require('path');
const { execFileSync, spawn } = require('child_process');

const args = process.argv.slice(2);
const debug = args.includes('--debug-launch');

const packageRoot = path.resolve(__dirname, '..');
const pkg = require(path.join(packageRoot, 'package.json'));
const version = pkg.version;

const vendorDir = path.join(packageRoot, 'vendor');
const runtimeBase = process.env.LOCALAPPDATA || process.env.TEMP;

if (!runtimeBase) {
    console.error('[CliDesk] Không thể xác định LOCALAPPDATA hoặc TEMP.');
    process.exit(1);
}

const runtimeDir = path.join(runtimeBase, 'CliDesk', 'npm-runtime', version);

const vendorApp = path.join(vendorDir, 'clidesk.exe');
const vendorLauncher = path.join(vendorDir, 'clidesk-launcher.exe');

const runtimeApp = path.join(runtimeDir, 'clidesk.exe');
const runtimeLauncher = path.join(runtimeDir, 'clidesk-launcher.exe');
const latestVersion = readLatestVersion();
const updateAvailable = latestVersion ? isVersionNewer(latestVersion, version) : false;

function ensureFileExists(filePath, label) {
    if (!fs.existsSync(filePath)) {
        throw new Error(`${label} không tồn tại: ${filePath}`);
    }
}

function copyIfMissing(src, dest) {
    ensureFileExists(src, 'Binary nguồn');
    fs.mkdirSync(path.dirname(dest), { recursive: true });

    if (fs.existsSync(dest)) {
        return;
    }

    const tmpDest = `${dest}.tmp-${process.pid}`;
    try {
        fs.copyFileSync(src, tmpDest);
        fs.renameSync(tmpDest, dest);
    } catch (err) {
        try {
            if (fs.existsSync(tmpDest)) {
                fs.unlinkSync(tmpDest);
            }
        } catch (_) {
            // Best-effort cleanup only.
        }
        throw err;
    }
}

function printDebugInfo() {
    console.log('[CliDesk] packageRoot:', packageRoot);
    console.log('[CliDesk] vendorDir:', vendorDir);
    console.log('[CliDesk] runtimeDir:', runtimeDir);
    console.log('[CliDesk] launcherPath:', runtimeLauncher);
    console.log('[CliDesk] appPath:', runtimeApp);
    console.log('[CliDesk] version:', version);
    console.log('[CliDesk] latestVersion:', latestVersion || '(unknown)');
    console.log('[CliDesk] updateAvailable:', updateAvailable ? 'true' : 'false');
}

function readLatestVersion() {
    try {
        const command = process.platform === 'win32' ? (process.env.ComSpec || 'cmd.exe') : 'npm';
        const commandArgs = process.platform === 'win32'
            ? ['/d', '/s', '/c', 'npm view clidesk version --silent']
            : ['view', 'clidesk', 'version', '--silent'];
        const output = execFileSync(command, commandArgs, {
            encoding: 'utf8',
            timeout: 5000,
            windowsHide: true,
            env: {
                ...process.env,
                npm_config_audit: 'false',
                npm_config_fund: 'false',
                npm_config_loglevel: 'silent',
                npm_config_logs_max: '0',
                npm_config_update_notifier: 'false',
                NO_UPDATE_NOTIFIER: '1',
            },
            stdio: ['ignore', 'pipe', 'ignore'],
        });
        const latest = output.trim();
        return latest || null;
    } catch (_) {
        return null;
    }
}

function isVersionNewer(latest, current) {
    const latestParts = parseVersion(latest);
    const currentParts = parseVersion(current);
    for (let index = 0; index < 3; index += 1) {
        if (latestParts[index] > currentParts[index]) {
            return true;
        }
        if (latestParts[index] < currentParts[index]) {
            return false;
        }
    }
    return false;
}

function parseVersion(value) {
    const parts = String(value)
        .split(/[.+-]/)
        .slice(0, 3)
        .map((part) => {
            const parsed = Number.parseInt(part, 10);
            return Number.isFinite(parsed) ? parsed : 0;
        });

    while (parts.length < 3) {
        parts.push(0);
    }

    return parts;
}

function main() {
    copyIfMissing(vendorApp, runtimeApp);
    copyIfMissing(vendorLauncher, runtimeLauncher);

    if (debug) {
        printDebugInfo();
    }

    const child = spawn(runtimeLauncher, args, {
        cwd: runtimeDir,
        env: {
            ...process.env,
            CLIDESK_VERSION: version,
            CLIDESK_LATEST_VERSION: latestVersion || '',
            CLIDESK_UPDATE_AVAILABLE: updateAvailable ? '1' : '0',
            CLIDESK_UPDATE_COMMAND: 'npm i -g clidesk',
        },
        stdio: 'inherit',
        windowsHide: false,
    });

    child.on('exit', (code) => {
        process.exit(code ?? 0);
    });

    child.on('error', (err) => {
        console.error('[CliDesk] Không thể chạy launcher:', err.message);
        console.error('[CliDesk] Path:', runtimeLauncher);
        process.exit(1);
    });
}

try {
    main();
} catch (err) {
    console.error('[CliDesk] Không thể chuẩn bị runtime CliDesk.');
    console.error('[CliDesk] Lỗi:', err && err.message ? err.message : err);
    if (debug) {
        printDebugInfo();
    }
    process.exit(1);
}
