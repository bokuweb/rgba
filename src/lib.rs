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

#[macro_use]
extern crate bitfield;

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
