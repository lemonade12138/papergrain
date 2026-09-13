#!/bin/bash
# PaperGrain — reproducible Windows build (cross-compiled from Linux).
# Toolchain: rustup (x86_64-pc-windows-gnullvm) + LLVM-MinGW + windres.
set -euo pipefail

TC="${PAPERGRAIN_TC:-$HOME/.papergrain-toolchain}"
REPO=/home/z/my-project/papergrain
source "$TC/env.sh"
cd "$REPO"

# llvm-mingw ships libunwind.dll.a which would add a runtime DLL dependency
# that does not exist on Windows. Hide it once so -lunwind resolves to the
# static libunwind.a (safe: this is our private toolchain copy).
if [ -f "$TC/llvm-mingw/x86_64-w64-mingw32/lib/libunwind.dll.a" ]; then
    mv "$TC/llvm-mingw/x86_64-w64-mingw32/lib/libunwind.dll.a" \
       "$TC/llvm-mingw/x86_64-w64-mingw32/lib/libunwind.dll.a.hidden"
fi

# 1. resources (icon + manifest + version info)
mkdir -p build
x86_64-w64-mingw32-windres assets/app.rc -O coff -o build/res.o

# 2. release build with resources
touch src/main.rs   # ensure relink picks up fresh resources
cargo rustc --release --target x86_64-pc-windows-gnullvm -- \
    -C link-arg="$REPO/build/res.o"

# 3. finalize
cp target/x86_64-pc-windows-gnullvm/release/papergrain.exe build/PaperGrain.exe
x86_64-w64-mingw32-strip build/PaperGrain.exe

echo "--- result ---"
ls -la build/PaperGrain.exe
x86_64-w64-mingw32-objdump -p build/PaperGrain.exe | grep "DLL Name" | sort -u
