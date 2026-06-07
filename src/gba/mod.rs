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

        Self { cycles: 0, arm, bus, save_path: Some(save_path) }
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
