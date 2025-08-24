mod bus;

use crate::io;
use crate::lcd;

use bus::CpuBus;

use crate::cpu::bus::accessor::BusAccessor;
use crate::cpu::constants;
use crate::cpu::cpu;
use crate::cpu::decoder;
use crate::cpu::instructions;
use crate::cpu::registers;
use crate::cpu::types;

// mod error;
// mod instructions;
// mod memory;
// mod registers;
// mod types;

// use constants::*;
// use error::*;
use crate::memory::ram::Ram;
use crate::memory::readable::*;
use crate::memory::rom::Rom;
use crate::memory::writable::*;
use crate::memory::Raw;

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::types::*;

// Visible     240 dots,  57.221 us,    960 cycles - 78% of h-time
// H-Blanking   68 dots,  16.212 us,    272 cycles - 22% of h-time
// Total       308 dots,  73.433 us,   1232 cycles - ca. 13.620 kHz
// Visible (*) 160 lines, 11.749 ms, 197120 cycles - 70% of v-time
// V-Blanking   68 lines,  4.994 ms,  83776 cycles - 30% of v-time
// Total       228 lines, 16.743 ms, 280896 cycles - ca. 59.737 Hz
const CYCLES_PER_FRAME: usize = 280896;

pub struct GBA {
    pub cycles: usize,
    pub arm: cpu::ARM,
    // pub lcdc: lcd::LCDController,
    pub bus: CpuBus,
}

impl GBA {
    pub fn new() -> Self {
        let bin_path = env::args().nth(1).expect("Specify bin filename to build.");
        let bin = GBA::load_bin(bin_path).expect("faild to read bin");
        // debug!("read bin data = {:?}", bin);
        let bios = Rom::new(0x4000, &include_bytes!("../../bios/bios.bin")[..]);
        let rom = Rom::new(0x80000, &bin);
        let wram = Ram::new(vec![0; 0x8000]);
        let eram = Ram::new(vec![0; 0x4_0000]);
        let vram = Ram::new(vec![0; 0x1_8000]);
        let palette = Ram::new(vec![0; 0x0400]);
        let oam = Ram::new(vec![0; 0x0400]);
        let sram = Ram::new(vec![0; 0x1_0000]); // 64KB SRAM/FRAM/Flash save memory
        let lcdc = lcd::LCDController::new();
        let key = io::Key::new();
        let bus = CpuBus::new(bios, lcdc, rom, wram, eram, vram, palette, oam, sram, key);
        let mut arm = cpu::ARM::new();

        arm.reset();

        Self { cycles: 0, arm, bus }
    }

    fn load_bin(bin: String) -> Result<Vec<u8>, std::io::Error> {
        let path = Path::new(&bin);
        let mut fd = File::open(path)?;
        let mut buf = Vec::new();
        fd.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn frame(&mut self, started: bool) -> Vec<u8> {
        let mut step_count = 0;
        let mut total_cycles = 0;
        loop {
            let cpu_cycles = self.arm.step(&mut self.bus, started).unwrap();
            step_count += 1;
            total_cycles += cpu_cycles;
            
            // For VBlank interrupt to occur, we need to ensure LCD progresses
            // Even if CPU is in a wait loop, LCD should continue advancing
            let effective_cycles = if cpu_cycles < 100 {
                // If CPU is waiting (low cycle count), advance LCD at minimum rate
                // VBlank occurs at line 160, need 1232 cycles per line, so boost significantly
                std::cmp::max(cpu_cycles, 100)
            } else {
                cpu_cycles
            };
            
            // Debug: Print cycles every 1000 steps
            if step_count % 5000 == 0 {
                println!("🔧 Step {}: CPU cycles = {}, Total = {}, Effective = {}", 
                    step_count, cpu_cycles, total_cycles, effective_cycles);
            }
            
            // Execute any pending DMA transfers
            // TODO: self.bus.execute_dma_transfers();
            
            let lcdc = self.bus.borrow_mut_lcdc();
            let (ready, vblank_entered) = lcdc.run(effective_cycles);
            
            // Check for VBlank IRQ and trigger CPU interrupt if needed
            if vblank_entered {
                println!("🔥 VBlank detected, requesting interrupt");
                self.bus.request_vblank_interrupt();
                if self.bus.should_service_interrupt() {
                    self.arm.request_irq();
                }
            }
            
            if ready {
                println!("🔧 Frame completed after {} steps", step_count);
                break;
            }
        }
        let lcdc = self.bus.borrow_lcdc();
        let vram = self.bus.borrow_vram();
        let palette = self.bus.borrow_palette();
        let oam = self.bus.borrow_oam();
        lcdc.render(vram, palette, oam)
    }

    pub fn update_key(&mut self, key: io::Key) {
        // dbg!("update_key", key);
        self.bus.update_key(key)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use pretty_assertions::*;

    use crate::memory::ram::Ram;
    use crate::memory::rom::Rom;

    pub fn run_with_step(step: u64, bin: &[u8]) -> (cpu::ARM, CpuBus) {
        // env_logger::init();
        let bios = Rom::new(0x4000, &include_bytes!("../../bios/bios.bin")[..]);
        let rom = Rom::new(0x80000, &bin);
        let wram = Ram::new(vec![0; 0x8000]);
        let eram = Ram::new(vec![0; 0x4_0000]);
        let vram = Ram::new(vec![0; 0x1_8000]);
        let palette = Ram::new(vec![0; 0x0400]);
        let oam = Ram::new(vec![0; 0x0400]);
        let sram = Ram::new(vec![0; 0x1_0000]); // 64KB SRAM/FRAM/Flash save memory
        let lcdc = lcd::LCDController::new();
        let key = io::Key::new();
        let mut bus = CpuBus::new(bios, lcdc, rom, wram, eram, vram, palette, oam, sram, key);
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
        let (cpu, bus) = run_with_step(400000, bin);
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
