//! Library crate root.
//!
//! This exposes the emulator core (CPU, bus, LCD, ...) as a library so it can
//! be driven by frontends other than the native SDL2 binary — in particular the
//! WebAssembly debugger frontend in [`wasm`].
//!
//! The native binary (`src/main.rs`) intentionally declares the same modules
//! itself; the two crate roots compile the shared `src/**` tree independently,
//! which keeps the existing `pub(crate)` visibility working without any
//! refactor.

// On non-wasm targets this library has no entry point: its only public surface,
// the `wasm` module, is `#[cfg(target_arch = "wasm32")]`, so the whole emulator
// core looks "dead" to the native lib build even though the native binary and
// the wasm frontend both exercise it. Silence dead-code analysis for that one
// configuration; genuine dead code is still caught by the `rgba` binary
// build (which has a real `main`) and by the wasm build.
#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

#[macro_use]
mod macros;

// These modules are crate-internal: the only public surface of this library is
// the wasm-bindgen API in [`wasm`]. Keeping them private (rather than `pub`)
// means the emulator's many `fn -> Result<_, ()>` helpers and bare `new()`
// constructors are not treated as exported API, so `clippy::result_unit_err`,
// `new_without_default`, `len_without_is_empty`, etc. don't fire on code that
// was written for the native binary.
mod cpu;
mod gba;
mod interrupt;
mod io;
mod lcd;
mod memory;
mod types;

#[cfg(target_arch = "wasm32")]
mod wasm;
