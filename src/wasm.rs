//! WebAssembly bindings for the browser debugger frontend.
//!
//! This is a thin shim over [`crate::gba::GBA`]: it owns the emulator, runs
//! frames / single steps on demand, and surfaces register + memory state as
//! plain values JavaScript can consume. Rendering, audio output, input and the
//! UI all live on the JS side — this module never touches the DOM.

use wasm_bindgen::prelude::*;

use crate::gba::GBA;
use crate::io::{Key, KeyStatus};

/// Native GBA screen dimensions; the framebuffer is RGBA, 4 bytes per pixel.
const WIDTH: usize = 240;
const HEIGHT: usize = 160;
const FB_LEN: usize = WIDTH * HEIGHT * 4;

#[wasm_bindgen]
pub struct GbaHandle {
    gba: GBA,
    /// Scratch framebuffer reused every frame so we don't reallocate; alpha is
    /// forced to 0xFF here because canvas `ImageData` treats 0 as transparent.
    framebuf: Vec<u8>,
    key: Key,
}

#[wasm_bindgen]
impl GbaHandle {
    /// Construct an emulator from raw ROM bytes (a `.gba` file).
    #[wasm_bindgen(constructor)]
    pub fn new(rom: &[u8]) -> GbaHandle {
        console_error_panic_hook::set_once();
        GbaHandle {
            gba: GBA::from_rom(rom),
            framebuf: vec![0; FB_LEN],
            key: Key::new(),
        }
    }

    /// Run one full video frame and return the RGBA framebuffer
    /// (240×160×4 bytes), ready to hand to `ctx.putImageData`.
    #[wasm_bindgen(js_name = runFrame)]
    pub fn run_frame(&mut self) -> Vec<u8> {
        self.gba.update_key(self.key);
        let buf = self.gba.frame(false);
        self.copy_framebuffer(&buf);
        self.framebuf.clone()
    }

    /// Execute a single CPU instruction. Returns `true` if that step finished a
    /// video frame (so the caller may want to repaint).
    pub fn step(&mut self) -> bool {
        self.gba.step_instruction()
    }

    /// Run until the next frame boundary OR a breakpoint is hit. Returns `true`
    /// if it stopped on a breakpoint. The caller then reads `framebuffer()`,
    /// `pc()` and `takeAudioF32()`.
    #[wasm_bindgen(js_name = runUntilBreak)]
    pub fn run_until_break(&mut self) -> bool {
        self.gba.update_key(self.key);
        self.gba.run_frame_or_break()
    }

    // ---- Breakpoints ------------------------------------------------------

    #[wasm_bindgen(js_name = setBreakpoint)]
    pub fn set_breakpoint(&mut self, addr: u32) {
        self.gba.add_breakpoint(addr);
    }

    #[wasm_bindgen(js_name = clearBreakpoint)]
    pub fn clear_breakpoint(&mut self, addr: u32) {
        self.gba.remove_breakpoint(addr);
    }

    #[wasm_bindgen(js_name = clearBreakpoints)]
    pub fn clear_breakpoints(&mut self) {
        self.gba.clear_breakpoints();
    }

    /// Sorted list of active breakpoint addresses.
    pub fn breakpoints(&self) -> Vec<u32> {
        self.gba.breakpoint_list()
    }

    // ---- Audio ------------------------------------------------------------

    /// Output sample rate (Hz) of the interleaved stereo stream.
    #[wasm_bindgen(js_name = sampleRate)]
    pub fn sample_rate(&self) -> u32 {
        GBA::AUDIO_SAMPLE_RATE
    }

    /// Drain queued audio as interleaved L/R `f32` in [-1, 1], ready to post to
    /// an AudioWorklet.
    #[wasm_bindgen(js_name = takeAudioF32)]
    pub fn take_audio_f32(&mut self) -> Vec<f32> {
        self.gba.take_audio().into_iter().map(|s| f32::from(s) / 32768.0).collect()
    }

    /// Latest framebuffer (RGBA), without advancing emulation — useful after a
    /// burst of single steps.
    #[wasm_bindgen(js_name = framebuffer)]
    pub fn framebuffer(&mut self) -> Vec<u8> {
        let buf = self.gba.read_framebuffer();
        self.copy_framebuffer(&buf);
        self.framebuf.clone()
    }

    fn copy_framebuffer(&mut self, buf: &[u8]) {
        let n = buf.len().min(FB_LEN);
        self.framebuf[..n].copy_from_slice(&buf[..n]);
        for px in self.framebuf.chunks_mut(4) {
            px[3] = 0xFF;
        }
    }

    // ---- Debugger state ---------------------------------------------------

    /// 16 general-purpose registers (r0–r15) as a `Uint32Array`.
    pub fn registers(&self) -> Vec<u32> {
        self.gba.registers().to_vec()
    }

    /// Raw CPSR bits.
    pub fn cpsr(&self) -> u32 {
        self.gba.cpsr_bits()
    }

    /// Program counter (r15).
    pub fn pc(&self) -> u32 {
        self.gba.registers()[15]
    }

    /// Total CPU cycles executed so far (monotonic). Returned as `f64` to avoid
    /// JS `BigInt` ergonomics; precise well past any realistic session length.
    pub fn cycles(&self) -> f64 {
        self.gba.cycles as f64
    }

    /// `true` when executing 16-bit THUMB instructions, `false` for ARM.
    #[wasm_bindgen(js_name = isThumb)]
    pub fn is_thumb(&self) -> bool {
        self.gba.is_thumb()
    }

    /// Read `len` bytes from bus-visible memory at `addr`.
    #[wasm_bindgen(js_name = readMemory)]
    pub fn read_memory(&self, addr: u32, len: u32) -> Vec<u8> {
        self.gba.read_memory(addr, len)
    }

    /// Write one byte to bus-visible memory.
    #[wasm_bindgen(js_name = writeByte)]
    pub fn write_byte(&mut self, addr: u32, value: u8) {
        self.gba.write_memory_byte(addr, value);
    }

    /// Write a slice of bytes starting at `addr` (used by the assembler pane).
    #[wasm_bindgen(js_name = writeBytes)]
    pub fn write_bytes(&mut self, addr: u32, bytes: &[u8]) {
        for (i, b) in bytes.iter().enumerate() {
            self.gba.write_memory_byte(addr.wrapping_add(i as u32), *b);
        }
    }

    // ---- Disassembly ------------------------------------------------------

    /// Disassemble `count` instructions starting at `addr`. Returns a JSON
    /// array string: `[{"addr":..,"bytes":"..","text":".."}, ...]`. When
    /// `thumb` is true, decodes 16-bit THUMB; otherwise 32-bit ARM.
    pub fn disassemble(&self, addr: u32, count: u32, thumb: bool) -> String {
        use crate::cpu::disasm;
        let step = if thumb { 2 } else { 4 };
        let mut out = String::from("[");
        let mut a = addr;
        for i in 0..count {
            let bytes = self.gba.read_memory(a, step);
            let (text, hex) = if thumb {
                let raw = u16::from(bytes[0]) | (u16::from(bytes[1]) << 8);
                (disasm::thumb(raw, a), format!("{raw:04x}"))
            } else {
                let raw = u32::from(bytes[0])
                    | (u32::from(bytes[1]) << 8)
                    | (u32::from(bytes[2]) << 16)
                    | (u32::from(bytes[3]) << 24);
                (disasm::arm(raw, a), format!("{raw:08x}"))
            };
            if i > 0 {
                out.push(',');
            }
            // text is emulator-generated and contains no JSON metacharacters
            // except possibly a tab; escape the tab for valid JSON.
            let text = text.replace('\t', " ");
            out.push_str(&format!("{{\"addr\":{a},\"bytes\":\"{hex}\",\"text\":\"{text}\"}}"));
            a = a.wrapping_add(step);
        }
        out.push(']');
        out
    }

    // ---- Assembler --------------------------------------------------------

    /// Assemble ARM `source` for load address `addr` and, on success, write the
    /// encoded bytes into memory. Returns a JSON object:
    /// `{"ok":true,"bytes":"e3a00001 .."}` or `{"ok":false,"error":".."}`.
    pub fn assemble(&mut self, addr: u32, source: &str) -> String {
        use crate::cpu::asm;
        match asm::assemble(source, addr) {
            Ok(bytes) => {
                for (i, b) in bytes.iter().enumerate() {
                    self.gba.write_memory_byte(addr.wrapping_add(i as u32), *b);
                }
                let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
                format!("{{\"ok\":true,\"len\":{},\"bytes\":\"{}\"}}", bytes.len(), hex.join(" "))
            }
            // asm error messages contain quotes; escape them for valid JSON.
            Err(e) => format!("{{\"ok\":false,\"error\":\"{}\"}}", e.replace('"', "'").replace('\\', "")),
        }
    }

    /// Set the program counter (r15) — lets the frontend jump execution to
    /// freshly-assembled code.
    #[wasm_bindgen(js_name = setPc)]
    pub fn set_pc(&mut self, addr: u32) {
        self.gba.set_pc(addr);
    }

    // ---- Input ------------------------------------------------------------

    /// Set a key's pressed state. `code` matches the GBA key bit order:
    /// 0=A 1=B 2=Select 3=Start 4=Right 5=Left 6=Up 7=Down 8=R 9=L.
    #[wasm_bindgen(js_name = setKey)]
    pub fn set_key(&mut self, code: u8, pressed: bool) {
        let s = if pressed { KeyStatus::ON } else { KeyStatus::OFF };
        match code {
            0 => self.key.set_A(s),
            1 => self.key.set_B(s),
            2 => self.key.set_SELECT(s),
            3 => self.key.set_START(s),
            4 => self.key.set_RIGHT(s),
            5 => self.key.set_LEFT(s),
            6 => self.key.set_UP(s),
            7 => self.key.set_DOWN(s),
            8 => self.key.set_R(s),
            9 => self.key.set_L(s),
            _ => {}
        }
    }
}
