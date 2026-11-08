#!/bin/sh
# Builds flappy-enaga.gba with LLVM. devkitARM is not needed.
set -e
cd "$(dirname "$0")"
LLVM="${LLVM:-/opt/homebrew/opt/llvm@21/bin}"
LLD="${LLD:-/opt/homebrew/opt/lld@21/bin}"
CFLAGS="--target=armv4t-none-eabi -mthumb -mfloat-abi=soft -std=gnu99 -O2 -ffreestanding \
 -fomit-frame-pointer -fno-common -fno-unwind-tables -fno-asynchronous-unwind-tables -Wall \
 -Dinline=static\ inline -Irt/include -Isrc -Ibuild"

rm -rf build
mkdir -p build
python3 tools/gen_assets.py build

objs=""
for src in src/*.c rt/libc.c build/assets.c; do
    obj="build/$(basename "$src" .c).o"
    eval "$LLVM/clang" $CFLAGS -c "$src" -o "$obj"
    objs="$objs $obj"
done
for src in src/systemcall.s rt/crt0.s; do
    obj="build/$(basename "$src" .s).o"
    "$LLVM/clang" --target=armv4t-none-eabi -c "$src" -o "$obj"
    objs="$objs $obj"
done
"$LLD/ld.lld" -T rt/gba.ld -o build/flappy-enaga.elf $objs
"$LLVM/llvm-objcopy" -O binary build/flappy-enaga.elf flappy-enaga.gba
python3 tools/fixrom.py flappy-enaga.gba ../hello/hello.gba
ls -l flappy-enaga.gba
