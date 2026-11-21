#!/bin/sh
# Builds flappy-enaga.gba (cartridge) and flappy-enaga_mb.gba (multiboot, for
# sending over a link cable) with LLVM. devkitARM is not needed.
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
"$LLVM/clang" --target=armv4t-none-eabi -c src/systemcall.s -o build/systemcall.o
objs="$objs build/systemcall.o"
"$LLVM/clang" --target=armv4t-none-eabi -c rt/crt0.s -o build/crt0.o
"$LLVM/clang" --target=armv4t-none-eabi -Wa,-defsym,MULTIBOOT=1 -c rt/crt0.s -o build/crt0_mb.o

# Cartridge image, linked at 0x08000000.
"$LLD/ld.lld" -T rt/gba.ld -o build/flappy-enaga.elf $objs build/crt0.o
"$LLVM/llvm-objcopy" -O binary build/flappy-enaga.elf flappy-enaga.gba
python3 tools/fixrom.py flappy-enaga.gba ../hello/hello.gba

# Multiboot image, linked at 0x02000000 (EWRAM, at most 256 KiB).
"$LLD/ld.lld" -T rt/gba_mb.ld -o build/flappy-enaga_mb.elf $objs build/crt0_mb.o
"$LLVM/llvm-objcopy" -O binary build/flappy-enaga_mb.elf flappy-enaga_mb.gba
python3 tools/fixrom.py flappy-enaga_mb.gba ../hello/hello.gba

ls -l flappy-enaga.gba flappy-enaga_mb.gba
