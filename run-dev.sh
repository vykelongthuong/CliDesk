#!/bin/bash
# CliDesk - Quick dev runner (Git Bash)
# Usage: bash run-dev.sh  or  ./run-dev.sh
# Sets up PATH and launches Tauri dev mode

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== CliDesk Dev Runner ==="
echo ""

# === Configure PATH for Rust + MinGW toolchain ===
export CARGO_HOME="$HOME/.cargo"
export PATH="$CARGO_HOME/bin:$PATH"

WINLIBS_DIR="/c/Users/PC/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin"
LLVM_DIR="/c/Users/PC/AppData/Local/Microsoft/WinGet/Packages/MartinStorsjo.LLVM-MinGW.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/llvm-mingw-20260505-ucrt-x86_64/bin"

if [ -d "$WINLIBS_DIR" ]; then
    export PATH="$WINLIBS_DIR:$PATH"
    echo "[OK] GCC: $(gcc --version 2>/dev/null | head -1)"
else
    echo "[WARN] WinLibs GCC not found at: $WINLIBS_DIR"
fi

if [ -d "$LLVM_DIR" ]; then
    export PATH="$LLVM_DIR:$PATH"
    echo "[OK] LLD linker available"
else
    echo "[WARN] LLVM MinGW not found at: $LLVM_DIR"
fi

# === Tool versions ===
echo ""
echo "Toolchain:"
echo "  Rust: $(rustc --version 2>/dev/null || echo 'NOT FOUND')"
echo "  Node: $(node --version 2>/dev/null || echo 'NOT FOUND')"

# === Fast TypeScript type-check (takes ~5s) ===
echo ""
echo ">>> Quick type-check (TypeScript)..."
npx tsc --noEmit
echo "[OK] TypeScript passed"
echo ""

# === Launch Tauri dev ===
echo "=== Launching CliDesk ==="
echo "Running: npm run tauri dev"
npx tauri dev
