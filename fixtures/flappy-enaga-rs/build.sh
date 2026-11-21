#!/bin/sh
# Builds flappy-enaga-rs.gba. Needs a nightly toolchain with the rust-src and
# llvm-tools components, and Python 3 for the asset generator and header fix.
set -e
cd "$(dirname "$0")"
TOOLCHAIN="${TOOLCHAIN:-nightly}"
cargo +"$TOOLCHAIN" build --release
SYSROOT="$(rustc +"$TOOLCHAIN" --print sysroot)"
OBJCOPY="$(find "$SYSROOT/lib/rustlib" -name llvm-objcopy -type f | head -n 1)"
[ -n "$OBJCOPY" ] || { echo "llvm-objcopy not found; run: rustup +$TOOLCHAIN component add llvm-tools" >&2; exit 1; }
"$OBJCOPY" -O binary target/thumbv4t-none-eabi/release/flappy-enaga-rs flappy-enaga-rs.gba
python3 ../flappy-enaga/tools/fixrom.py flappy-enaga-rs.gba ../hello/hello.gba
ls -l flappy-enaga-rs.gba
