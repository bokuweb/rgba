# rusty-gba

[![CI](https://github.com/bokuweb/rgba/actions/workflows/rust.yml/badge.svg)](https://github.com/bokuweb/rgba/actions/workflows/rust.yml)
[![Pages](https://github.com/bokuweb/rgba/actions/workflows/pages.yml/badge.svg)](https://github.com/bokuweb/rgba/actions/workflows/pages.yml)

A Game Boy Advance emulator written in Rust, with two frontends that share the
same core:

* **native** — an SDL2 window (`cargo run`)
* **web** — a WASM build with a No$gba-style debugger (disassembly,
  breakpoints, registers, memory editor, ARM assembler, audio scope)

**Live debugger:** https://bokuweb.github.io/rgba/

## Features

| Area | Status |
|------|--------|
| CPU | ARM7TDMI — ARM + THUMB, all modes, IRQ / SWI |
| BIOS | `bios/bios.bin` is embedded at compile time and mapped at `0x0000_0000`; SWI system calls are implemented in HLE (`src/cpu/bios.rs`) |
| Video | Modes 0–5, BG0–3 (text / affine), OBJ, windows, blending, mosaic |
| Audio | 4 PSG channels + 2 DirectSound FIFOs via DMA / timers |
| DMA / Timers | DMA0–3, Timer0–3 with cascading |
| Backup | SRAM / Flash / EEPROM autodetected from the ROM's SDK marker; `.sav` written next to the ROM (native only) |
| Input | Keypad register (KEYINPUT / KEYCNT) |
| Debugger | Single-step, breakpoints, disassembler (bin→asm), assembler (asm→bin) |

## Layout

```
src/
  main.rs        native SDL2 frontend (binary)
  lib.rs         library root used by the WASM build
  wasm.rs        wasm-bindgen shim (`GbaHandle`)
  gba/           system glue: bus, DMA, timers, APU, backup memory, RTC
  cpu/           ARM7TDMI: decoder, instructions, registers, asm/disasm, BIOS
  lcd/           PPU: registers and renderer
  io/            keypad
  interrupt/     IE / IF / IME
  memory/        RAM / ROM primitives
web/             browser debugger (index.html + app.js + AudioWorklet)
bios/            GBA BIOS image (included at compile time)
fixtures/        test ROMs and their sources (see below)
```

## Native frontend

Requirements: Rust stable, SDL2.

```bash
brew install sdl2
```

On Apple Silicon add this to your shell profile so the linker can find SDL2:

```bash
export LIBRARY_PATH="$LIBRARY_PATH:$(brew --prefix)/lib"
```

Then:

```bash
cargo run --release ./fixtures/hello/hello.gba
```

Logging goes through `tracing`; set `RUST_LOG` to change the level
(default `warn`), e.g. `RUST_LOG=info cargo run -- rom.gba`.

Keys: `Z`=A `X`=B `Enter`=Start `Space`=Select arrows=D-Pad `A`=L `S`=R
`Esc`=quit.

## Web frontend (WASM debugger)

The hosted build is deployed automatically from `main` to
https://bokuweb.github.io/rgba/ by
[`.github/workflows/pages.yml`](.github/workflows/pages.yml).

To build and run it locally:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

```bash
wasm-pack build --target web --out-dir web/pkg --no-typescript --release
```

```bash
node web/static-server.js
```

Open http://localhost:8761/web/ and load a `.gba` file (e.g.
`fixtures/hello/hello.gba`). See [`web/README.md`](web/README.md) for the
debugger's features and the JS ↔ WASM API surface.

## Tests

```bash
cargo test
```

```bash
cargo clippy --all-targets
```

Clippy runs with `all` + `pedantic` denied (see `[workspace.lints]` in
`Cargo.toml`), so any new lint finding fails CI.

Unit tests live next to the code (`#[cfg(test)]` modules per instruction /
device). The end-to-end tests in `src/gba/mod.rs` embed `fixtures/hello/hello.gba`
and `fixtures/dot_rs/dot.gba` and check the rendered frame; other ROMs under
`fixtures/` ([gba-tests](https://github.com/jsmolka/gba-tests), armwrestler,
`suite`, ...) are for manual verification through either frontend. The C /
Rust fixtures can be rebuilt from source with:

```bash
cd fixtures && make
```

This uses Docker (`shumon84/gba` for the C fixtures and `bokuweb/rust-gba`,
built from the root `Dockerfile`, for the Rust fixture).

## References

* [GBATEK](https://problemkaputt.de/gbatek.htm)
* [gba-tests](https://github.com/jsmolka/gba-tests)
* [GBA test ROM index](https://emulation.gametechwiki.com/index.php?title=GBA_Tests)
