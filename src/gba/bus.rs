use crate::cpu::bus;
use crate::cpu::constants;
use crate::cpu::cpu;
use crate::cpu::decoder;
use crate::cpu::instructions;
use crate::cpu::registers;
use crate::cpu::types;
use crate::lcd;
use crate::types::*;

pub(crate) use bus::accessor::*;

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

pub const BIOS_ADDR: u32 = 0x0000_0000;
pub const EWRAM_ADDR: u32 = 0x0200_0000;
pub const IWRAM_ADDR: u32 = 0x0300_0000;
pub const IOMEM_ADDR: u32 = 0x0400_0000;
pub const PALRAM_ADDR: u32 = 0x0500_0000;
pub const VRAM_ADDR: u32 = 0x0600_0000;
pub const OAM_ADDR: u32 = 0x0700_0000;
pub const GAMEPAK_WS0_LO: u32 = 0x0800_0000;
pub const GAMEPAK_WS0_HI: u32 = 0x0900_0000;
pub const GAMEPAK_WS1_LO: u32 = 0x0A00_0000;
pub const GAMEPAK_WS1_HI: u32 = 0x0B00_0000;
pub const GAMEPAK_WS2_LO: u32 = 0x0C00_0000;
pub const GAMEPAK_WS2_HI: u32 = 0x0D00_0000;
pub const SRAM_LO: u32 = 0x0E00_0000;
pub const SRAM_HI: u32 = 0x0F00_0000;

pub const PAGE_BIOS: usize = (BIOS_ADDR >> 24) as usize;
pub const PAGE_EWRAM: usize = (EWRAM_ADDR >> 24) as usize;
pub const PAGE_IWRAM: usize = (IWRAM_ADDR >> 24) as usize;
pub const PAGE_IOMEM: usize = (IOMEM_ADDR >> 24) as usize;
pub const PAGE_PALRAM: usize = (PALRAM_ADDR >> 24) as usize;
pub const PAGE_VRAM: usize = (VRAM_ADDR >> 24) as usize;
pub const PAGE_OAM: usize = (OAM_ADDR >> 24) as usize;
pub const PAGE_GAMEPAK_WS0: usize = (GAMEPAK_WS0_LO >> 24) as usize;
pub const PAGE_GAMEPAK_WS1: usize = (GAMEPAK_WS1_LO >> 24) as usize;
pub const PAGE_GAMEPAK_WS2: usize = (GAMEPAK_WS2_LO >> 24) as usize;
pub const PAGE_SRAM_LO: usize = (SRAM_LO >> 24) as usize;
pub const PAGE_SRAM_HI: usize = (SRAM_HI >> 24) as usize;

#[derive(Clone)]
struct CycleLUT {
    n32: [usize; 0x10],
    s32: [usize; 0x10],
    n16: [usize; 0x10],
    s16: [usize; 0x10],
}

impl Default for CycleLUT {
    fn default() -> CycleLUT {
        let mut table = CycleLUT {
            n32: [1; 0x10],
            s32: [1; 0x10],
            n16: [1; 0x10],
            s16: [1; 0x10],
        };
        table.n32[PAGE_EWRAM] = 6;
        table.s32[PAGE_EWRAM] = 6;
        table.n16[PAGE_EWRAM] = 3;
        table.s16[PAGE_EWRAM] = 3;

        table.n32[PAGE_OAM] = 2;
        table.s32[PAGE_OAM] = 2;
        table.n16[PAGE_OAM] = 1;
        table.s16[PAGE_OAM] = 1;

        table.n32[PAGE_VRAM] = 2;
        table.s32[PAGE_VRAM] = 2;
        table.n16[PAGE_VRAM] = 1;
        table.s16[PAGE_VRAM] = 1;

        table.n32[PAGE_PALRAM] = 2;
        table.s32[PAGE_PALRAM] = 2;
        table.n16[PAGE_PALRAM] = 1;
        table.s16[PAGE_PALRAM] = 1;
        table
    }
}

impl CycleLUT {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, waitcnt: ()) {
        static S_GAMEPAK_NSEQ_CYCLES: [usize; 4] = [4, 3, 2, 8];
        static S_GAMEPAK_WS0_SEQ_CYCLES: [usize; 2] = [2, 1];
        static S_GAMEPAK_WS1_SEQ_CYCLES: [usize; 2] = [4, 1];
        static S_GAMEPAK_WS2_SEQ_CYCLES: [usize; 2] = [8, 1];

        // let ws0_first_access = waitcnt.ws0_first_access() as usize;
        // let ws1_first_access = waitcnt.ws1_first_access() as usize;
        // let ws2_first_access = waitcnt.ws2_first_access() as usize;
        // let ws0_second_access = waitcnt.ws0_second_access() as usize;
        // let ws1_second_access = waitcnt.ws1_second_access() as usize;
        // let ws2_second_access = waitcnt.ws2_second_access() as usize;

        // update SRAM
        // let sram_wait_cycles = 1 + S_GAMEPAK_NSEQ_CYCLES[waitcnt.sram_wait_control() as usize];
        // self.n32[PAGE_SRAM_LO] = sram_wait_cycles;
        // self.n32[PAGE_SRAM_LO] = sram_wait_cycles;
        // self.n16[PAGE_SRAM_HI] = sram_wait_cycles;
        // self.n16[PAGE_SRAM_HI] = sram_wait_cycles;
        // self.s32[PAGE_SRAM_LO] = sram_wait_cycles;
        // self.s32[PAGE_SRAM_LO] = sram_wait_cycles;
        // self.s16[PAGE_SRAM_HI] = sram_wait_cycles;
        // self.s16[PAGE_SRAM_HI] = sram_wait_cycles;

        for i in 0..2 {
            //self.n16[PAGE_GAMEPAK_WS0 + i] = 1 + S_GAMEPAK_NSEQ_CYCLES[ws0_first_access];
            //self.s16[PAGE_GAMEPAK_WS0 + i] = 1 + S_GAMEPAK_WS0_SEQ_CYCLES[ws0_second_access];
            //
            //self.n16[PAGE_GAMEPAK_WS1 + i] = 1 + S_GAMEPAK_NSEQ_CYCLES[ws1_first_access];
            //self.s16[PAGE_GAMEPAK_WS1 + i] = 1 + S_GAMEPAK_WS1_SEQ_CYCLES[ws1_second_access];
            //
            //self.n16[PAGE_GAMEPAK_WS2 + i] = 1 + S_GAMEPAK_NSEQ_CYCLES[ws2_first_access];
            //self.s16[PAGE_GAMEPAK_WS2 + i] = 1 + S_GAMEPAK_WS2_SEQ_CYCLES[ws2_second_access];

            // ROM 32bit accesses are split into two 16bit accesses 1N+1S
            self.n32[PAGE_GAMEPAK_WS0 + i] = self.n16[PAGE_GAMEPAK_WS0 + i] + self.s16[PAGE_GAMEPAK_WS0 + i];
            self.n32[PAGE_GAMEPAK_WS1 + i] = self.n16[PAGE_GAMEPAK_WS1 + i] + self.s16[PAGE_GAMEPAK_WS1 + i];
            self.n32[PAGE_GAMEPAK_WS2 + i] = self.n16[PAGE_GAMEPAK_WS2 + i] + self.s16[PAGE_GAMEPAK_WS2 + i];

            self.s32[PAGE_GAMEPAK_WS0 + i] = 2 * self.s16[PAGE_GAMEPAK_WS0 + i];
            self.s32[PAGE_GAMEPAK_WS1 + i] = 2 * self.s16[PAGE_GAMEPAK_WS1 + i];
            self.s32[PAGE_GAMEPAK_WS2 + i] = 2 * self.s16[PAGE_GAMEPAK_WS2 + i];
        }
    }
}

pub struct CpuBus {
    cycleLUT: CycleLUT,
    lcdc: lcd::LCDController,
    bios: Rom,
    rom: Rom,
    wram: Ram,
    eram: Ram,
    vram: Ram,
    palette: Ram,
    oam: Ram,
}

impl BusAccessor for CpuBus {
    fn read_byte(&self, addr: u32) -> Byte {
        debug!("read byte addr = {:x}", addr);
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_byte(addr),
            0x0300_0000..=0x0300_7FFF => self.wram.read_byte(addr - 0x0300_0000),
            0x0500_0000..=0x0500_03FF => self.palette.read_byte(addr - 0x0500_0000),
            0x0800_0000..=0x09FF_FFFF => self.rom.read_byte(addr - 0x0800_0000),
            _ => panic!("TODO: "),
        }
    }

    fn read_halfword(&self, addr: u32) -> HalfWord {
        debug!("read half word addr = {:x}", addr);
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_halfword(addr),
            0x0300_0000..=0x0300_7FFF => self.wram.read_halfword(addr - 0x0300_0000),
            0x0400_0000..=0x0400_03FE => {
                // dbg!("I/O register is not implemented yet.", format!("addr = {:x}", addr));
                if addr == 0x0400_0006 {
                    return self.lcdc.read();
                }
                0
            }
            0x0500_0000..=0x0500_03FF => self.palette.read_halfword(addr - 0x0500_0000),
            0x0600_0000..=0x0601_7FFF => self.vram.read_halfword(addr - 0x0600_0000),
            0x0800_0000..=0x09FF_FFFF => self.rom.read_halfword(addr - 0x0800_0000),
            _ => panic!("TODO: {:x}", addr),
        }
    }

    fn read_word(&self, addr: u32) -> Word {
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_word(addr),
            0x0200_0000..=0x0203_FFFF => self.eram.read_word(addr - 0x0200_0000),
            // WRAM
            0x0300_0000..=0x0300_7FFF => {
                // info!("wram addr = {:x}", addr);

                self.wram.read_word(addr - 0x0300_0000)
            }
            0x0500_0000..=0x0500_03FF => self.palette.read_word(addr - 0x0500_0000),
            0x0800_0000..=0x09FF_FFFF => self.rom.read_word(addr - 0x0800_0000),
            _ => panic!(format!("TODO: addr = 0x{:x}", addr)),
        }
    }

    fn write_byte(&mut self, addr: u32, data: Byte) {
        debug!("write byte addr = 0x{:x} data = 0x{:x}", addr, data);
        match addr {
            // 0x0000_0000...0x0007_FFFF => self.rom.borrow().read_word(addr),
            0x0300_0000..=0x0300_7FFF => {
                if addr == 0x03007dd9 {
                    // dbg!("write to 0x03007dd9", data);
                }
                self.wram.write_byte(addr - 0x0300_0000, data);
            }
            0x0500_0000..=0x0500_03FF => self.palette.write_byte(addr - 0x0500_0000, data),
            _ => panic!("TODO: "),
        };
    }

    fn write_halfword(&mut self, addr: u32, data: HalfWord) {
        debug!("write half word addr = 0x{:x} data = 0x{:x}", addr, data);
        match addr {
            // I/O Register
            0x0200_0000..=0x0203_FFFF => self.eram.write_halfword(addr - 0x0200_0000, data),
            // WRAM
            0x0300_0000..=0x0300_7FFF => {
                // info!("wram addr = {:x} {:x}", addr, data);
                self.wram.write_halfword(addr - 0x0300_0000, data);
            }
            0x0400_0000..=0x0400_03FE => {
                // dbg!(
                //                    "I/O register is not implemented yet.",
                //                    format!("addr = {:x} data = {:x}", addr, data)
                //                );
            }
            0x0500_0000..=0x0500_03FF => self.palette.write_halfword(addr - 0x0500_0000, data),
            0x0600_0000..=0x0601_7FFF => {
                self.vram.write_halfword(addr - 0x0600_0000, data);
            }

            _ => panic!("TODO: "),
        };
    }

    fn write_word(&mut self, addr: u32, data: Word) {
        // dbg!(format!("write word addr 0x{:x} data = 0x{:x}", addr, data));

        match addr {
            0x0000_0000..=0x0007_FFFF => panic!("illegal write access."),
            0x0200_0000..=0x0203_FFFF => self.eram.write_word(addr - 0x0200_0000, data),
            // WRAM
            0x0300_0000..=0x0300_7FFF => {
                // info!("wram addr = {:x} {:x}", addr, data);
                self.wram.write_word(addr - 0x0300_0000, data);
            }
            // Unused
            0x0300_8000..=0x03FF_FFFF => {
                // // dbg!(format!("{:x}", addr));
            }
            // I/O Register
            0x0400_0000..=0x0400_03FE => {
                // dbg!(
                //    "I/O register is not implemented yet.",
                //    format!("addr = {:x} data = {:x}", addr, data)
                //);
            }
            0x0500_0000..=0x0500_03FF => self.palette.write_word(addr - 0x0500_0000, data),
            _ => panic!("TODO: addr = {:x} data = {:x}", addr, data),
        };
    }

    fn compute_cycle(&self, addr: Word, access_type: AccessType) -> Cycle {
        let page = (addr >> 24) as usize;
        if page > 0xF {
            return 1;
        }
        match access_type {
            AccessType::NonSeq(AccessWidth::Byte) | AccessType::NonSeq(AccessWidth::HalfWord) => self.cycleLUT.n16[page],
            AccessType::NonSeq(AccessWidth::Word) => self.cycleLUT.n32[page],
            AccessType::Seq(AccessWidth::Byte) | AccessType::Seq(AccessWidth::HalfWord) => self.cycleLUT.s16[page],
            AccessType::Seq(AccessWidth::Word) => self.cycleLUT.s32[page],
        }
    }
}

impl CpuBus {
    pub fn new(bios: Rom, lcdc: lcd::LCDController, rom: Rom, wram: Ram, eram: Ram, vram: Ram, palette: Ram, oam: Ram) -> CpuBus {
        CpuBus {
            cycleLUT: CycleLUT::new(),
            lcdc,
            bios,
            rom,
            wram,
            eram,
            vram,
            palette,
            oam,
        }
    }

    pub fn get_mut_lcdc(&mut self) -> &mut lcd::LCDController {
        &mut self.lcdc
    }
}
