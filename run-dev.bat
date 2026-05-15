@echo off
REM CliDesk - Quick dev runner (Windows CMD)
REM Double-click this file to run CliDesk in dev mode

echo === CliDesk Dev Runner ===
echo.

REM ---- Configure PATH ----
set CARGO_HOME=%USERPROFILE%\.cargo
set PATH=%CARGO_HOME%\bin;%CARGO_HOME%\registry\bin;%PATH%

set WINLIBS_DIR=%USERPROFILE%\AppData\Local\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin
set LLVM_DIR=%USERPROFILE%\AppData\Local\Microsoft\WinGet\Packages\MartinStorsjo.LLVM-MinGW.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\llvm-mingw-20260505-ucrt-x86_64\bin

if exist "%WINLIBS_DIR%\gcc.exe" (
    set PATH=%WINLIBS_DIR%;%PATH%
    echo [OK] WinLibs GCC
) else (
    echo [WARN] WinLibs GCC not found
)

if exist "%LLVM_DIR%\ld.lld.exe" (
    set PATH=%LLVM_DIR%;%PATH%
    echo [OK] LLD linker
) else (
    echo [WARN] LLD not found
)

REM ---- Verify tools ----
echo.
echo Toolchain:
rustc --version >nul 2>&1
if %ERRORLEVEL% EQU 0 (echo [OK] Rust) else (echo [ERR] Rust not found!)
node --version >nul 2>&1
if %ERRORLEVEL% EQU 0 (echo [OK] Node) else (echo [ERR] Node not found!)

REM ---- Quick TypeScript check ----
echo.
echo --- TypeScript check ---
cd /d "%~dp0"
call npx tsc --noEmit
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [FAIL] TypeScript errors found. Fix them first!
    pause
    exit /b 1
)
echo [OK] TypeScript passed

REM ---- Launch Tauri ----
echo.
echo === Launching CliDesk ===
npx tauri dev
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [ERROR] CliDesk closed unexpectedly (code: %ERRORLEVEL%)
    pause
)
