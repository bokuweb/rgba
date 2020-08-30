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
use super::memory::readable::*;
use super::memory::rom::Rom;
// use super::memory::writable::*;

use std::cell::RefCell;
use std::rc::Rc;

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use types::*;

struct CpuBus {
    rom: Rom,
    ram: Ram,
}

impl BusAccessor for CpuBus {
    fn read_byte(&self, addr: u32) -> Byte {
        debug!("read byte addr = {:x}", addr);
        match addr {
            0x0000_0000..=0x0007_FFFF => self.rom.read_byte(addr),
            _ => panic!("TODO: "),
        }
    }
    fn read_word(&self, addr: u32) -> Word {
        debug!("read word addr = {:x}", addr);
        match addr {
            0x0000_0000..=0x0007_FFFF => self.rom.read_word(addr),
            _ => panic!("TODO: "),
        }
    }
    fn write_byte(&mut self, addr: u32, data: u8) {
        info!("write byte addr = 0x{:x} data = 0x{:x}", addr, data);
        match addr {
            // 0x0000_0000...0x0007_FFFF => self.rom.borrow().read_word(addr),
            _ => panic!("TODO: "),
        };
    }

    fn write_word(&mut self, addr: u32, data: u32) {
        info!("write word addr = 0x{:x} data = 0x{:x}", addr, data);
        match addr {
            0x0000_0000..=0x0007_FFFF => {
                self.rom.read_word(addr);
            }
            // I/O Register
            0x0400_0000..=0x0400_03FE => {
                debug!("I/O register is not implemented yet.");
            }
            _ => panic!("TODO: "),
        };
    }
}

impl CpuBus {
    fn new(rom: Rom, ram: Ram) -> CpuBus {
        CpuBus { rom, ram }
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
    let rom = Rom::new(0x80000, bin);
    let ram = Ram::new(vec![0; 0x10000]);
    let mut bus = CpuBus::new(rom, ram);
    let mut arm = cpu::ARM::new();
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
    arm.tick(&mut bus);
}
