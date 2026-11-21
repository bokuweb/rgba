# Flappy Enaga (Rust)

[Flappy Enaga](../flappy-enaga) rewritten in `no_std` Rust for the Game Boy Advance. Press **A** or **B** to flap and **Start** to pause.

It is a line-for-line port of the C game, so it plays the same and draws the same picture every frame. `flappy_enaga_rust_port_matches_c` in [`src/gba/mod.rs`](../../src/gba/mod.rs) checks this by running both ROMs side by side in the emulator:

```bash
cargo test --release -- --ignored flappy_enaga
```

Only the very first frame differs, because the two builds finish setting up at different points in it. Sound can also differ slightly when an effect starts, because the C build and the Rust build mix at different speeds.

## Layout

- [`src/main.rs`](src/main.rs): the game, from `main.c`.
- [`src/gba.rs`](src/gba.rs): registers, DMA, OAM and the BIOS VBlank wait, from the C headers.
- [`src/audio.rs`](src/audio.rs): the DirectSound mixer and the VBlank interrupt handler, from `audio.c` and `irq.c`.
- [`src/sin_lut.rs`](src/sin_lut.rs): the sine table for rotating the bird.

Some parts come straight from the C version: the startup code [`../flappy-enaga/rt/crt0.s`](../flappy-enaga/rt/crt0.s) (pulled in with `global_asm!`), the linker script [`../flappy-enaga/rt/gba.ld`](../flappy-enaga/rt/gba.ld), and the art and sound. [`build.rs`](build.rs) draws the art and sound by running [`../flappy-enaga/tools/gen_assets.py`](../flappy-enaga/tools/gen_assets.py) with `--rust`.

The interrupt handler is built as ARM code (`#[instruction_set(arm::a32)]`) and placed in IWRAM. The BIOS jumps to it with `ldr pc`, which can't switch the ARM7TDMI into Thumb state.

## Building

```bash
./build.sh
```

This needs a nightly toolchain with the `rust-src` and `llvm-tools` components, plus Python 3. `thumbv4t-none-eabi` has no prebuilt `core`, so [`.cargo/config.toml`](.cargo/config.toml) builds it from source with `build-std`. Set `TOOLCHAIN` to use a nightly other than `nightly`. The script turns the ELF into a raw ROM image with `llvm-objcopy`, then fills in the cartridge header with `../flappy-enaga/tools/fixrom.py`. The ROM is committed, so the Pages build does not need this toolchain.

The code is adapted from Tim Ullrich's [flappybird-gba](https://github.com/tullrich/flappybird-gba) (MIT, see [`LICENSE.txt`](LICENSE.txt)), like the C version.
