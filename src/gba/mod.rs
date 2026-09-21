mod apu;
mod backup;
mod bus;
mod dma;
mod eeprom;
mod rtc;
mod timer;

use backup::{Backup, SaveKind};
use eeprom::Eeprom;

use crate::io;
use crate::lcd;

use bus::CpuBus;

use crate::cpu::cpu;

// mod error;
// mod instructions;
// mod memory;
// mod registers;
// mod types;

// use constants::*;
// use error::*;
use crate::memory::ram::Ram;
use crate::memory::rom::Rom;

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};


// Visible     240 dots,  57.221 us,    960 cycles - 78% of h-time
// H-Blanking   68 dots,  16.212 us,    272 cycles - 22% of h-time
// Total       308 dots,  73.433 us,   1232 cycles - ca. 13.620 kHz
// Visible (*) 160 lines, 11.749 ms, 197120 cycles - 70% of v-time
// V-Blanking   68 lines,  4.994 ms,  83776 cycles - 30% of v-time
// Total       228 lines, 16.743 ms, 280896 cycles - ca. 59.737 Hz
const CYCLES_PER_FRAME: usize = 280_896;

pub struct GBA {
    pub cycles: usize,
    pub arm: cpu::ARM,
    // pub lcdc: lcd::LCDController,
    pub bus: CpuBus,
    /// `<rom>.sav` path used to persist backup memory (None for embedded ROMs).
    save_path: Option<PathBuf>,
    /// Instruction addresses the debugger should halt on before executing.
    breakpoints: std::collections::HashSet<u32>,
}

impl GBA {
    pub fn new() -> Self {
        let bin_path = env::args().nth(1).expect("Specify bin filename to build.");
        let bin = Self::load_bin(bin_path.clone()).expect("faild to read bin");
        // debug!("read bin data = {:?}", bin);
        let bios = Rom::new(0x4000, &include_bytes!("../../bios/bios.bin")[..]);
        let rom = Rom::new(0x80000, &bin);
        let wram = Ram::new(vec![0; 0x8000]);
        let eram = Ram::new(vec![0; 0x4_0000]);
        let vram = Ram::new(vec![0; 0x1_8000]);
        let palette = Ram::new(vec![0; 0x0400]);
        let oam = Ram::new(vec![0; 0x0400]);

        // Pick the backup type from the ROM's SDK marker and restore any
        // existing `.sav` file next to the ROM. EEPROM carts use a separate
        // serial device in the 0x0D region; everything else is SRAM/Flash.
        let is_eeprom = backup::is_eeprom(&bin);
        let kind = SaveKind::detect(&bin);
        let save_path = PathBuf::from(&bin_path).with_extension("sav");
        let saved = std::fs::read(&save_path).unwrap_or_default();
        let label = if is_eeprom { "EEPROM" } else { "SRAM/Flash" };
        if saved.is_empty() {
            tracing::info!("Backup type: {label} ({kind:?}) (save file: {})", save_path.display());
        } else {
            tracing::info!("Loaded save ({label}, {} bytes) from {}", saved.len(), save_path.display());
        }

        // A Backup is always constructed (it backs the 0x0E region); for EEPROM
        // carts it stays unused and the EEPROM device holds the save instead.
        let backup = Backup::new(kind, if is_eeprom { &[] } else { &saved });

        let lcdc = lcd::LCDController::new();
        let key = io::Key::new();
        let mut bus = CpuBus::new(bios, lcdc, rom, wram, eram, vram, palette, oam, backup, key);
        if is_eeprom {
            bus.attach_eeprom(Eeprom::new(&saved));
        }
        let mut arm = cpu::ARM::new();

        arm.reset();

        Self { cycles: 0, arm, bus, save_path: Some(save_path), breakpoints: std::collections::HashSet::new() }
    }

    /// Build a GBA directly from ROM bytes, with no filesystem or process
    /// arguments. This is the entry point used by non-native frontends (the
    /// WASM debugger), where there is no `.sav` file to read or write.
    ///
    /// Backup memory starts empty; persistence is the frontend's job (e.g.
    /// export [`GBA::backup_snapshot`] to IndexedDB).
    pub fn from_rom(bin: &[u8]) -> Self {
        let bios = Rom::new(0x4000, &include_bytes!("../../bios/bios.bin")[..]);
        let rom = Rom::new(0x80000, bin);
        let wram = Ram::new(vec![0; 0x8000]);
        let eram = Ram::new(vec![0; 0x4_0000]);
        let vram = Ram::new(vec![0; 0x1_8000]);
        let palette = Ram::new(vec![0; 0x0400]);
        let oam = Ram::new(vec![0; 0x0400]);

        // Mirror `new()`'s save-memory wiring, but with no persisted data: an
        // EEPROM cart gets a fresh EEPROM device, everything else plain SRAM.
        let is_eeprom = backup::is_eeprom(bin);
        let kind = SaveKind::detect(bin);
        let backup = Backup::new(kind, &[]);

        let lcdc = lcd::LCDController::new();
        let key = io::Key::new();
        let mut bus = CpuBus::new(bios, lcdc, rom, wram, eram, vram, palette, oam, backup, key);
        if is_eeprom {
            bus.attach_eeprom(Eeprom::new(&[]));
        }
        let mut arm = cpu::ARM::new();
        arm.reset();

        Self { cycles: 0, arm, bus, save_path: None, breakpoints: std::collections::HashSet::new() }
    }

    // ---- Breakpoints ------------------------------------------------------

    pub fn add_breakpoint(&mut self, addr: u32) {
        self.breakpoints.insert(addr);
    }

    pub fn remove_breakpoint(&mut self, addr: u32) {
        self.breakpoints.remove(&addr);
    }

    pub fn clear_breakpoints(&mut self) {
        self.breakpoints.clear();
    }

    pub fn breakpoint_list(&self) -> Vec<u32> {
        let mut v: Vec<u32> = self.breakpoints.iter().copied().collect();
        v.sort_unstable();
        v
    }

    /// Run until a full video frame completes OR the PC reaches a breakpoint.
    /// Returns `true` when stopped at a breakpoint, `false` on frame end.
    ///
    /// The instruction currently at the PC is always executed first, so this is
    /// safe to call repeatedly when already parked on a breakpoint (it makes
    /// progress instead of re-triggering the same one immediately).
    pub fn run_frame_or_break(&mut self) -> bool {
        let mut first = true;
        loop {
            if !first && !self.breakpoints.is_empty() && self.breakpoints.contains(&self.arm.gpr[15]) {
                return true;
            }
            first = false;

            let cycles = self.arm.step(&mut self.bus, false).unwrap();
            self.cycles = self.cycles.wrapping_add(cycles);
            self.bus.advance_clock(cycles);
            self.bus.execute_dma_transfers();
            if self.bus.should_service_interrupt() {
                self.arm.request_irq();
            }
            if self.bus.take_frame_ready() {
                return false;
            }
        }
    }

    // ---- Debugger surface -------------------------------------------------
    //
    // These let a frontend single-step and inspect state, which `frame()`
    // (which runs a whole frame at once) does not expose.

    /// Execute exactly one CPU instruction, advancing timers / DMA / IRQ the
    /// same way [`GBA::frame`] does for one loop iteration. Returns the cycle
    /// cost of the instruction. The returned `bool` is `true` when this step
    /// completed a video frame (so the frontend knows to repaint).
    pub fn step_instruction(&mut self) -> bool {
        let cycles = self.arm.step(&mut self.bus, false).unwrap();
        self.cycles = self.cycles.wrapping_add(cycles);
        self.bus.advance_clock(cycles);
        self.bus.execute_dma_transfers();
        if self.bus.should_service_interrupt() {
            self.arm.request_irq();
        }
        self.bus.take_frame_ready()
    }

    /// Snapshot of the 16 general-purpose registers (r0–r15; r15 is PC).
    pub fn registers(&self) -> [u32; 16] {
        self.arm.gpr
    }

    /// Raw CPSR bits (flags in the high nibble, mode/T/I in the low byte).
    pub fn cpsr_bits(&self) -> u32 {
        self.arm.get_cpsr().get()
    }

    /// `true` when the CPU is currently in THUMB state (16-bit instructions).
    pub fn is_thumb(&self) -> bool {
        use crate::cpu::registers::psr::CpuState;
        self.arm.get_cpsr().get_cpu_state() == CpuState::Thumb
    }

    /// Read `len` bytes of bus-visible memory starting at `addr`. Goes through
    /// the normal bus, so it sees ROM, WRAM, VRAM, I/O mirrors, etc.
    pub fn read_memory(&self, addr: u32, len: u32) -> Vec<u8> {
        use crate::cpu::bus::accessor::BusAccessor;
        (0..len).map(|i| self.bus.read_byte(addr.wrapping_add(i))).collect()
    }

    /// Write a single byte to bus-visible memory (used by the assembler pane to
    /// patch instructions into RAM).
    pub fn write_memory_byte(&mut self, addr: u32, value: u8) {
        use crate::cpu::bus::accessor::BusAccessor;
        self.bus.write_byte(addr, value);
    }

    /// Set the program counter (r15). Used to jump execution to assembled code.
    pub fn set_pc(&mut self, addr: u32) {
        self.arm.gpr[15] = addr;
    }

    /// Current backup (save) memory contents, for the frontend to persist.
    pub fn backup_snapshot(&self) -> Vec<u8> {
        self.bus.backup_bytes().to_vec()
    }

    /// Current framebuffer (RGBA, 240×160) without advancing emulation.
    pub fn read_framebuffer(&self) -> Vec<u8> {
        self.bus.framebuffer().to_vec()
    }

    /// Current BG/video mode (DISPCNT mode field) as a 0..=5 index.
    pub fn video_mode(&self) -> u8 {
        self.bus.borrow_lcdc().bg_mode_index()
    }

    /// DISPCNT layer-enable bits: bit0..bit3 = BG0..BG3, bit4 = OBJ.
    pub fn layer_flags(&self) -> u8 {
        self.bus.borrow_lcdc().dispcnt_layer_flags()
    }

    /// Override the visible layer set for the debugger. `None` renders normally;
    /// `Some(mask)` shows only the layers in `mask` (bit0..3 = BG0..3, bit4 = OBJ).
    pub fn set_debug_layer_mask(&mut self, mask: Option<u8>) {
        self.bus.borrow_mut_lcdc().set_debug_layer_mask(mask);
    }

    /// Persist save memory (SRAM/Flash or EEPROM) to the `.sav` file if it
    /// changed since the last flush. Cheap to call every frame: it is a no-op
    /// unless the save is dirty.
    pub fn flush_save_if_dirty(&mut self) {
        let eeprom_dirty = self.bus.eeprom_is_dirty();
        let backup_dirty = self.bus.backup_is_dirty();
        if !eeprom_dirty && !backup_dirty {
            return;
        }
        let bytes = if eeprom_dirty {
            self.bus.eeprom_bytes().to_vec()
        } else {
            self.bus.backup_bytes().to_vec()
        };
        if let Some(path) = &self.save_path {
            match std::fs::write(path, &bytes) {
                Ok(()) => {
                    self.bus.eeprom_clear_dirty();
                    self.bus.backup_clear_dirty();
                }
                Err(e) => tracing::error!("failed to write save {}: {}", path.display(), e),
            }
        } else {
            // No backing file (embedded ROM): drop the dirty flags so we don't
            // keep retrying.
            self.bus.eeprom_clear_dirty();
            self.bus.backup_clear_dirty();
        }
    }

    fn load_bin(bin: String) -> Result<Vec<u8>, std::io::Error> {
        let path = Path::new(&bin);
        let mut fd = File::open(path)?;
        let mut buf = Vec::new();
        fd.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn frame(&mut self, started: bool) -> Vec<u8> {
        loop {
            let cycles = self.arm.step(&mut self.bus, started).unwrap();

            // Advance the single master clock by this instruction's cycles. This
            // drives the timers and the LCD (and raises the VBlank IRQ) off one
            // delta. DMA advances the same clock per transfer, so timers/LCD stay
            // in lockstep with the CPU even mid-DMA.
            self.bus.advance_clock(cycles);

            // DMA + HBlank/VCounter edge detection. Runs after the clock update so
            // it observes the freshly-updated DISPSTAT edges.
            self.bus.execute_dma_transfers();

            // Request an IRQ on the CPU if the IE/IF/IME combination allows servicing.
            if self.bus.should_service_interrupt() {
                self.arm.request_irq();
            }

            if self.bus.take_frame_ready() {
                break;
            }
        }

        // The framebuffer was filled scanline-by-scanline during the frame.
        self.bus.framebuffer().to_vec()
    }

    pub fn update_key(&mut self, key: io::Key) {
        // dbg!("update_key", key);
        self.bus.update_key(key);
    }

    /// Output sample rate of the audio stream returned by [`GBA::take_audio`].
    pub const AUDIO_SAMPLE_RATE: u32 = apu::SAMPLE_RATE;

    /// Drain queued interleaved L/R audio samples produced since the last call.
    pub fn take_audio(&mut self) -> Vec<i16> {
        self.bus.take_audio()
    }
}

#[cfg(test)]
mod repro {
    use super::*;
    use crate::io::{Key, KeyStatus};
    use std::io::Write;

    fn write_bmp(path: &str, rgba: &[u8]) {
        let (w, h) = (240usize, 160usize);
        let row_padded = (w * 3 + 3) & !3;
        let data_size = row_padded * h;
        let file_size = 54 + data_size;
        let mut f = std::fs::File::create(path).unwrap();
        let mut hdr = Vec::new();
        hdr.extend_from_slice(b"BM");
        hdr.extend_from_slice(&(file_size as u32).to_le_bytes());
        hdr.extend_from_slice(&0u32.to_le_bytes());
        hdr.extend_from_slice(&54u32.to_le_bytes());
        hdr.extend_from_slice(&40u32.to_le_bytes());
        hdr.extend_from_slice(&(w as i32).to_le_bytes());
        hdr.extend_from_slice(&(h as i32).to_le_bytes());
        hdr.extend_from_slice(&1u16.to_le_bytes());
        hdr.extend_from_slice(&24u16.to_le_bytes());
        hdr.extend_from_slice(&0u32.to_le_bytes());
        hdr.extend_from_slice(&(data_size as u32).to_le_bytes());
        hdr.extend_from_slice(&2835i32.to_le_bytes());
        hdr.extend_from_slice(&2835i32.to_le_bytes());
        hdr.extend_from_slice(&0u32.to_le_bytes());
        hdr.extend_from_slice(&0u32.to_le_bytes());
        f.write_all(&hdr).unwrap();
        let mut row = vec![0u8; row_padded];
        for y in (0..h).rev() {
            for x in 0..w {
                let i = (y * w + x) * 4;
                row[x * 3] = rgba[i + 2];
                row[x * 3 + 1] = rgba[i + 1];
                row[x * 3 + 2] = rgba[i];
            }
            f.write_all(&row).unwrap();
        }
    }

    #[test]
    #[ignore = "manual frame-capture harness; requires a local fixtures/m3/m3.gba"]
    fn capture_m3() {
        let bin = std::fs::read("fixtures/m3/m3.gba").expect("rom");
        let mut gba = GBA::from_rom(&bin);
        let total: usize = std::env::var("FRAMES").ok().and_then(|s| s.parse().ok()).unwrap_or(2000);
        let dump_every: usize = std::env::var("EVERY").ok().and_then(|s| s.parse().ok()).unwrap_or(30);
        let dump_from: usize = std::env::var("FROM").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
        let name_until: usize = std::env::var("NAME_UNTIL").ok().and_then(|s| s.parse().ok()).unwrap_or(5000);
        let _ = std::fs::create_dir_all("target/m3");
        for fr in 0..total {
            let mut key = Key::new();
            if fr >= name_until {
                // Past name entry: optionally advance dialogue with A. A_EVERY=0
                // sends no input so the prologue cinematic plays untouched.
                let a_every: usize =
                    std::env::var("A_EVERY").ok().and_then(|s| s.parse().ok()).unwrap_or(12);
                if a_every != 0 && fr % a_every < 3 {
                    key.set_A(KeyStatus::ON);
                }
                gba.update_key(key);
                let buf = gba.frame(false);
                if fr % dump_every == 0 && fr >= dump_from {
                    write_bmp(&format!("target/m3/f{fr:05}.bmp"), &buf);
                }
                continue;
            }
            // Name-entry macro (period 100): hold DOWN to drop the cursor onto the
            // left-most bottom-menu cell "omakase" (random name), confirm, then
            // hold RIGHT to reach "owari" (done) and confirm the "yoroshii desu ka" dialog.
            // Cursor starts in the left column and typing doesn't move it, so a
            // straight DOWN lands on omakase. Looping this clears every name screen.
            let f = fr % 100;
            if f < 16 {
                key.set_DOWN(KeyStatus::ON);
            } else if f == 24 || f == 25 {
                key.set_A(KeyStatus::ON); // select omakase
            } else if (32..48).contains(&f) {
                key.set_RIGHT(KeyStatus::ON); // move to owari
            } else if matches!(f, 54 | 55 | 64 | 65 | 74 | 75 | 84 | 85) {
                key.set_A(KeyStatus::ON); // owari + "yoroshii desu ka"/dialogue advance
            }
            gba.update_key(key);
            let buf = gba.frame(false);
            if fr % dump_every == 0 {
                write_bmp(&format!("target/m3/f{fr:05}.bmp"), &buf);
            }
        }
    }

    #[test]
    #[ignore = "manual frame-capture harness; requires fixtures/beat_beast/BeatBeast_jam.gba"]
    fn capture_beat_beast() {
        use crate::cpu::bus::accessor::BusAccessor;
        let path =
            std::env::var("ROM").unwrap_or_else(|_| "fixtures/beat_beast/BeatBeast_jam.gba".into());
        let bin = std::fs::read(&path).expect("rom");
        let mut gba = GBA::from_rom(&bin);
        let total: usize = std::env::var("FRAMES").ok().and_then(|s| s.parse().ok()).unwrap_or(300);
        let _ = std::fs::create_dir_all("target/bb");
        for fr in 0..total {
            gba.update_key(Key::new());
            let buf = gba.frame(false);
            // Report whether this frame has any visible variation (not a flat color).
            let first = &buf[0..3];
            let varied = buf.as_chunks::<4>().0.iter().any(|px| px[0..3] != *first);
            if fr % 30 == 0 || fr == total - 1 {
                write_bmp(&format!("target/bb/f{fr:05}.bmp"), &buf);
                let dispcnt = gba.bus.read_halfword(0x0400_0000);
                tracing::debug!(
                    "frame {fr}: varied={varied} px0={:02x}{:02x}{:02x} dispcnt={:04x}",
                    buf[0], buf[1], buf[2], dispcnt
                );
            }
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use crate::cpu::bus::accessor::BusAccessor; // brings `read_halfword` into scope for CpuBus
    use crate::memory::ram::Ram;
    use crate::memory::rom::Rom;

    pub fn run_with_step(step: u64, bin: &[u8]) -> (cpu::ARM, CpuBus) {
        // env_logger::init();
        let bios = Rom::new(0x4000, &include_bytes!("../../bios/bios.bin")[..]);
        let rom = Rom::new(0x80000, bin);
        let wram = Ram::new(vec![0; 0x8000]);
        let eram = Ram::new(vec![0; 0x4_0000]);
        let vram = Ram::new(vec![0; 0x1_8000]);
        let palette = Ram::new(vec![0; 0x0400]);
        let oam = Ram::new(vec![0; 0x0400]);
        let backup = Backup::new(SaveKind::Sram, &[]); // default save memory for tests
        let lcdc = lcd::LCDController::new();
        let key = io::Key::new();
        let mut bus = CpuBus::new(bios, lcdc, rom, wram, eram, vram, palette, oam, backup, key);
        let mut arm = cpu::ARM::new();
        for _ in 0..step {
            arm.step(&mut bus, false).expect("should step");
        }
        (arm, bus)
    }

    #[test]
    // step
    fn test_hello_rom() {
        let bin = include_bytes!("../../fixtures/hello/hello.gba");
        let (cpu, bus) = run_with_step(400_000, bin);
        assert_eq!(
            cpu.gpr,
            [
                0,
                0,
                0x1F,
                0x0600_96F0,
                0x0200_0000,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0x0300_7F00,
                0x0800_0187,
                0x0800_02E6
            ]
        );
        assert_eq!(bus.read_halfword(0x0600_96F0), 0x001F);
    }

    #[test]
    // step
    fn test_dot_rom() {
        let bin = include_bytes!("../../fixtures/dot_rs/dot.gba");
        let (_cpu, bus) = run_with_step(100, bin);
        assert_eq!(bus.read_halfword(0x0600_96F0), 0x001F);
    }
}
