mod apu;
mod backup;
mod bus;
mod dma;
mod timer;

use backup::{Backup, SaveKind};

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
        // existing `.sav` file next to the ROM.
        let kind = SaveKind::detect(&bin);
        let save_path = PathBuf::from(&bin_path).with_extension("sav");
        let saved = std::fs::read(&save_path).unwrap_or_default();
        if saved.is_empty() {
            println!("💾 Backup type: {:?} (save file: {})", kind, save_path.display());
        } else {
            println!("💾 Loaded save ({:?}, {} bytes) from {}", kind, saved.len(), save_path.display());
        }
        let backup = Backup::new(kind, &saved);

        let lcdc = lcd::LCDController::new();
        let key = io::Key::new();
        let bus = CpuBus::new(bios, lcdc, rom, wram, eram, vram, palette, oam, backup, key);
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

        let kind = SaveKind::detect(bin);
        let backup = Backup::new(kind, &[]);

        let lcdc = lcd::LCDController::new();
        let key = io::Key::new();
        let bus = CpuBus::new(bios, lcdc, rom, wram, eram, vram, palette, oam, backup, key);
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
            self.cycles = self.cycles.wrapping_add(cycles as usize);
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
        self.cycles = self.cycles.wrapping_add(cycles as usize);
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

    /// Persist backup memory to the `.sav` file if it changed since the last
    /// flush. Cheap to call every frame: it is a no-op unless the save is dirty.
    pub fn flush_save_if_dirty(&mut self) {
        if !self.bus.backup_is_dirty() {
            return;
        }
        if let Some(path) = &self.save_path {
            match std::fs::write(path, self.bus.backup_bytes()) {
                Ok(()) => self.bus.backup_clear_dirty(),
                Err(e) => eprintln!("⚠️  failed to write save {}: {}", path.display(), e),
            }
        } else {
            // No backing file (embedded ROM): drop the dirty flag so we don't
            // keep retrying.
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

            // IE/IF/IMEの組み合わせでサービス可能ならCPUにIRQ要求
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
    #[ignore]
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
                // Past name entry: only advance dialogue with A so dpad presses
                // don't perturb the cinematic / walking.
                if fr % 12 < 3 {
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
            // left-most bottom-menu cell "おまかせ" (random name), confirm, then
            // hold RIGHT to reach "おわり" (done) and confirm the よろしいですか dialog.
            // Cursor starts in the left column and typing doesn't move it, so a
            // straight DOWN lands on おまかせ. Looping this clears every name screen.
            let f = fr % 100;
            if f < 16 {
                key.set_DOWN(KeyStatus::ON);
            } else if f == 24 || f == 25 {
                key.set_A(KeyStatus::ON); // select おまかせ
            } else if (32..48).contains(&f) {
                key.set_RIGHT(KeyStatus::ON); // move to おわり
            } else if matches!(f, 54 | 55 | 64 | 65 | 74 | 75 | 84 | 85) {
                key.set_A(KeyStatus::ON); // おわり + よろしいですか/dialogue advance
            }
            gba.update_key(key);
            let buf = gba.frame(false);
            if fr % dump_every == 0 {
                write_bmp(&format!("target/m3/f{fr:05}.bmp"), &buf);
            }
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use pretty_assertions::*;

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
        self::assert_eq!(
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
        self::assert_eq!(bus.read_halfword(0x0600_96F0), 0x001F);
    }

    #[test]
    // step
    fn test_dot_rom() {
        let bin = include_bytes!("../../fixtures/dot_rs/dot.gba");
        let (_cpu, bus) = run_with_step(100, bin);
        self::assert_eq!(bus.read_halfword(0x0600_96F0), 0x001F);
    }
}
