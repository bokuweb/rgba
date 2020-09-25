mod bus;

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

use types::*;

fn load_bin(bin: String) -> Result<Vec<u8>, std::io::Error> {
    let path = Path::new(&bin);
    let mut fd = File::open(path)?;
    let mut buf = Vec::new();
    fd.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn frame() -> Vec<u8> {
    // env_logger::init();
    // let elf_path = env::args().nth(1).expect("");
    // let result = load_elf(elf_path);
    let bin_path = env::args().nth(1).expect("Specify bin filename to build.");
    let bin = load_bin(bin_path).expect("faild to read bin");
    // debug!("read bin data = {:?}", bin);
    let bios = Rom::new(0x4000, &include_bytes!("../../bios/bios.bin")[..]);
    let rom = Rom::new(0x80000, &bin);
    let wram = Ram::new(vec![0; 0x8000]);
    let eram = Ram::new(vec![0; 0x4_0000]);
    let vram = Ram::new(vec![0; 0x1_8000]);
    let mut bus = CpuBus::new(bios, rom, wram, eram, vram);
    let mut arm = cpu::ARM::new();

    for _ in 0..4000000 {
        arm.step(&mut bus);
    }

    let mut buf = vec![];
    for offset in 0..(240 * 160) {
        let p = bus.read_halfword(0x0600_0000 + offset * 2);
        buf.push((((p & 0x001F) as f32 / 0x1F as f32) * 0xFF as f32) as u8);
        buf.push((((p & 0x03E0).wrapping_shr(5) as f32 / 0x1F as f32) * 0xFF as f32) as u8);
        buf.push((((p & 0xEC00).wrapping_shr(10) as f32 / 0x1F as f32) * 0xFF as f32) as u8);
        buf.push(255);
    }
    dbg!(buf.len());
    buf
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
        let mut bus = CpuBus::new(bios, rom, wram, eram, vram);
        let mut arm = cpu::ARM::new();
        for _ in 0..step {
            arm.step(&mut bus).expect("should step");
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
                0x0800_02E4
            ]
        );
        self::assert_eq!(bus.read_halfword(0x0600_96F0), 0x001F);
    }
}
