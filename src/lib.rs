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

pub mod cpu;
pub mod gba;
pub mod interrupt;
pub mod io;
pub mod lcd;
pub mod memory;
pub mod types;

#[cfg(target_arch = "wasm32")]
mod wasm;
