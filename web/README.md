# rusty-gba — web debugger (WASM)

A browser frontend for the emulator core, No$gba-style:

* run / pause / single-step, with the **current PC** shown live in the toolbar
  and highlighted in the disassembly;
* **breakpoints** — click any disassembly line to toggle (run stops on hit);
* **registers + flags** (changed registers flash) and the live **screen**;
* **memory** hex+ASCII dump with goto-address, *follow SP*, and inline byte
  editing (click a byte, type, Enter);
* **disassemble** (bin → asm) and **assemble** (asm → bin) ARM into memory;
* **audio** via an AudioWorklet (`web/audio-processor.js`).

## Build

```sh
# one-time
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

# build the wasm package into web/pkg
wasm-pack build --target web --out-dir web/pkg --no-typescript
```

## Run

Serve the repo root over HTTP (ES modules + .wasm need a server, not file://):

```sh
python3 -m http.server 8753
# then open http://localhost:8753/web/
```

Pick a `.gba` file (e.g. `fixtures/hello/hello.gba`) to start.

Keys: `Z`=A `X`=B `Enter`=Start `Space`=Select arrows=D-Pad `A`=L `S`=R.

## What lives where

| Concern            | Code                                   |
|--------------------|----------------------------------------|
| WASM bindings      | `src/wasm.rs` (`GbaHandle`)            |
| Emulator core API  | `src/gba/mod.rs` (`from_rom`, `step_instruction`, `run_frame_or_break`, breakpoints, `read_memory`, ...) |
| Disassembler (bin→asm) | `src/cpu/disasm.rs` (reuses the decoder) |
| Assembler (asm→bin)    | `src/cpu/asm.rs` (two-pass, labels) |
| UI / logic         | `web/index.html` + `web/app.js`        |
| Audio              | `web/audio-processor.js` (AudioWorklet) |

## Notes

* The native SDL2 binary (`cargo run`) is unchanged; `src/main.rs` and
  `src/lib.rs` compile the same module tree independently, and SDL2 is gated to
  non-wasm targets in `Cargo.toml`.
* The bundled assembler is a **useful subset** of ARM (data-processing,
  branches, `bx`, word/byte load-store, `swi`, `.word`, labels). For the full
  instruction set + THUMB you can drop in [Keystone](https://www.keystone-engine.org/)
  compiled to WASM and feed its output to `GbaHandle.writeBytes`.
