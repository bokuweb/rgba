mod bus;
mod constants;
mod cpu;
mod decoder;
mod instructions;
mod registers;
mod types;

pub(crate) use bus::accessor::*;

// mod error;
// mod instructions;
// mod memory;
// mod registers;
// mod types;

// use constants::*;
// use error::*;
use super::memory::ram::Ram;
use super::memory::rom::Rom;

use super::memory::readable::*;
use super::memory::writable::*;

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use types::*;

struct CpuBus {
    bios: Rom,
    rom: Rom,
    wram: Ram,
    eram: Ram,
    vram: Ram,
}

impl BusAccessor for CpuBus {
    fn read_byte(&self, addr: u32) -> Byte {
        debug!("read byte addr = {:x}", addr);
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_byte(addr),
            0x0800_0000..=0x09FF_FFFF => self.rom.read_byte(addr - 0x0800_0000),
            _ => panic!("TODO: "),
        }
    }

    fn read_halfword(&self, addr: u32) -> HalfWord {
        debug!("read half word addr = {:x}", addr);
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_halfword(addr),
            0x0800_0000..=0x09FF_FFFF => self.rom.read_halfword(addr - 0x0800_0000),
            _ => panic!("TODO: "),
        }
    }

    fn read_word(&self, addr: u32) -> Word {
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_word(addr),
            // WRAM
            0x0300_0000..=0x0300_7FFF => {
                info!("wram addr = {:x}", addr);
                self.wram.read_word(addr - 0x0300_0000)
            }
            0x0800_0000..=0x09FF_FFFF => self.rom.read_word(addr - 0x0800_0000),
            _ => panic!(format!("TODO: addr = 0x{:x}", addr)),
        }
    }

    fn write_byte(&mut self, addr: u32, data: Byte) {
        info!("write byte addr = 0x{:x} data = 0x{:x}", addr, data);
        match addr {
            // 0x0000_0000...0x0007_FFFF => self.rom.borrow().read_word(addr),
            _ => panic!("TODO: "),
        };
    }

    fn write_halfword(&mut self, addr: u32, data: HalfWord) {
        info!("write half word addr = 0x{:x} data = 0x{:x}", addr, data);
        match addr {
            0x0600_0000..=0x0601_7FFF => self.vram.write_halfword(addr - 0x0600_0000, data),
            _ => panic!("TODO: "),
        };
    }

    fn write_word(&mut self, addr: u32, data: Word) {
        // info!("write word addr = 0x{:x} data = 0x{:x}", addr, data);
        match addr {
            0x0000_0000..=0x0007_FFFF => panic!("illegal write access."),
            0x0200_0000..=0x0203_FFFF => self.eram.write_word(addr - 0x0200_0000, data),
            // WRAM
            0x0300_0000..=0x0300_7FFF => {
                info!("wram addr = {:x} {:x}", addr, data);
                self.wram.write_word(addr - 0x0300_0000, data);
            }
            // Unused
            0x0300_8000..=0x03FF_FFFF => {
                // dbg!(format!("{:x}", addr));
            }
            // I/O Register
            0x0400_0000..=0x0400_03FE => {
                dbg!("I/O register is not implemented yet.");
            }
            _ => panic!("TODO: addr = {:x} data = {:x}", addr, data),
        };
    }
}

impl CpuBus {
    fn new(bios: Rom, rom: Rom, wram: Ram, eram: Ram, vram: Ram) -> CpuBus {
        CpuBus {
            bios,
            rom,
            wram,
            eram,
            vram,
        }
    }
}

fn load_bin(bin: String) -> Result<Vec<u8>, std::io::Error> {
    let path = Path::new(&bin);
    let mut fd = File::open(path)?;
    let mut buf = Vec::new();
    fd.read_to_end(&mut buf)?;
    Ok(buf)
}

pub fn run() {
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

    for _ in 0..1000000 {
        arm.tick(&mut bus);
    }
}
