#!/bin/sh
# Builds flappy-enaga-rs.gba (cartridge) and flappy-enaga-rs_mb.gba (multiboot,
# for sending over a link cable). Needs a nightly toolchain with the rust-src
# and llvm-tools components, and Python 3 for the asset generator and header fix.
set -e
cd "$(dirname "$0")"
TOOLCHAIN="${TOOLCHAIN:-nightly}"
SYSROOT="$(rustc +"$TOOLCHAIN" --print sysroot)"
OBJCOPY="$(find "$SYSROOT/lib/rustlib" -name llvm-objcopy -type f | head -n 1)"
[ -n "$OBJCOPY" ] || { echo "llvm-objcopy not found; run: rustup +$TOOLCHAIN component add llvm-tools" >&2; exit 1; }
ELF=target/thumbv4t-none-eabi/release/flappy-enaga-rs

cargo +"$TOOLCHAIN" build --release
"$OBJCOPY" -O binary "$ELF" flappy-enaga-rs.gba
python3 ../flappy-enaga/tools/fixrom.py flappy-enaga-rs.gba ../hello/hello.gba

cargo +"$TOOLCHAIN" build --release --features multiboot
"$OBJCOPY" -O binary "$ELF" flappy-enaga-rs_mb.gba
python3 ../flappy-enaga/tools/fixrom.py flappy-enaga-rs_mb.gba ../hello/hello.gba

ls -l flappy-enaga-rs.gba flappy-enaga-rs_mb.gba
