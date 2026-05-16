# Build Release - CliDesk

This guide keeps development and release builds separate. `npm run dev` remains the development command and must keep running Tauri dev.

## Development

```powershell
npm run dev
```

- Uses `tauri dev` from `package.json`.
- Does not build a release binary.
- Does not require Administrator rights.
- Does not use `requireAdministrator`.

## Portable Windows Build

```powershell
npm run build:win
```

This runs:

```powershell
tauri build --target x86_64-pc-windows-msvc --no-bundle
```

Primary portable exe output:

```text
src-tauri/target/x86_64-pc-windows-msvc/release/clidesk.exe
```

If Cargo/Tauri emits a non-target-specific release path on your machine, also check:

```text
src-tauri/target/release/clidesk.exe
```

The portable exe can be copied to another Windows x64 machine and run directly. Do not copy `node_modules`, `src`, `dist`, Cargo files, Rust toolchains, or source code with it.

The default portable script uses `--no-bundle`, so it does not create installers. If you run Tauri without `--no-bundle`, installer bundle paths may be:

```text
src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/
src-tauri/target/x86_64-pc-windows-msvc/release/bundle/msi/
```

## Optional Clean Build

```powershell
npm run clean:build
npm install
npm run typecheck
npm run build:win
```

`clean:build` removes only generated build output:

```powershell
Remove-Item -Recurse -Force dist -ErrorAction SilentlyContinue
Remove-Item -Recurse -Force src-tauri\target -ErrorAction SilentlyContinue
```

Do not commit `src-tauri/target`, `dist`, `node_modules`, or generated `.exe` files.

## Windows Manifest

`src-tauri/app.manifest` uses:

```xml
<requestedExecutionLevel level="asInvoker" uiAccess="false" />
```

This means launching `clidesk.exe` should not show a UAC elevation prompt. The manifest also keeps `Microsoft.Windows.Common-Controls` v6 for Windows common controls compatibility.

## Test Checklist

After building, test the portable exe:

- Run `clidesk.exe`.
- Confirm the app opens without a UAC prompt.
- Add a project.
- Open a new terminal.
- Open a file from the Files tab.
- Verify the Git tab works when the project is a Git repository.
- Verify Settings and tray behavior.

## Build Machine Requirements

- Node.js
- npm
- Rust stable MSVC toolchain
- Microsoft C++ Build Tools
- Windows SDK

## Runtime Machine Requirements

- Windows x64.
- Microsoft Edge WebView2 Runtime if it is not already installed.
- Git, Node.js, and any AI CLI tools installed separately if you want to use them from CliDesk terminals.

## Separation Rules

- `npm run dev` is for development.
- `npm run build:win` is for release builds.
- Release builds must not change how `npm run dev` behaves.
- The app is not forced to run as Administrator.

