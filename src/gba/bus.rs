use crate::cpu::bus;
use crate::cpu::types;
use crate::io;
use crate::lcd;
use crate::types::*;

pub(crate) use bus::accessor::*;

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
use super::dma::DMAController;
use std::sync::{OnceLock, atomic::{AtomicUsize, Ordering}};

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

    pub fn update_waitcnt(&mut self, waitcnt: HalfWord) {
        // GBATEK: WAITCNT fields
        const S_GAMEPAK_NSEQ_CYCLES: [usize; 4] = [4, 3, 2, 8];
        const S_GAMEPAK_WS0_SEQ_CYCLES: [usize; 2] = [2, 1];
        const S_GAMEPAK_WS1_SEQ_CYCLES: [usize; 2] = [4, 1];
        const S_GAMEPAK_WS2_SEQ_CYCLES: [usize; 2] = [8, 1];

        let ws0_first = ((waitcnt >> 2) & 0x3) as usize;
        let ws0_second = ((waitcnt >> 4) & 0x1) as usize;
        let ws1_first = ((waitcnt >> 5) & 0x3) as usize;
        let ws1_second = ((waitcnt >> 7) & 0x1) as usize;
        let ws2_first = ((waitcnt >> 8) & 0x3) as usize;
        let ws2_second = ((waitcnt >> 10) & 0x1) as usize;
        let prefetch = ((waitcnt >> 14) & 1) != 0;

        for i in 0..2 {
            self.n16[PAGE_GAMEPAK_WS0 + i] = 1 + S_GAMEPAK_NSEQ_CYCLES[ws0_first];
            self.s16[PAGE_GAMEPAK_WS0 + i] = 1 + if prefetch { 1 } else { S_GAMEPAK_WS0_SEQ_CYCLES[ws0_second] };

            self.n16[PAGE_GAMEPAK_WS1 + i] = 1 + S_GAMEPAK_NSEQ_CYCLES[ws1_first];
            self.s16[PAGE_GAMEPAK_WS1 + i] = 1 + if prefetch { 1 } else { S_GAMEPAK_WS1_SEQ_CYCLES[ws1_second] };

            self.n16[PAGE_GAMEPAK_WS2 + i] = 1 + S_GAMEPAK_NSEQ_CYCLES[ws2_first];
            self.s16[PAGE_GAMEPAK_WS2 + i] = 1 + if prefetch { 1 } else { S_GAMEPAK_WS2_SEQ_CYCLES[ws2_second] };

            // 32bit ROM access = 1N + 1S
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
    dma: DMAController,
    bios: Rom,
    rom: Rom,
    wram: Ram,
    eram: Ram,
    vram: Ram,
    palette: Ram,
    oam: Ram,
    sram: Ram, // SRAM/FRAM/Flash save memory (0x0E000000-0x0E00FFFF, 64KB max)
    key: io::Key,
    interrupt_controller: std::cell::RefCell<crate::interrupt::InterruptController>,
    timers: crate::gba::timer::Timers,
    waitcnt: HalfWord,
}

impl BusAccessor for CpuBus {
    fn read_byte(&self, addr: u32) -> Byte {
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_byte(addr),
            // EWRAM 256KB mirrors across 0x0200_0000-0x02FF_FFFF
            0x0200_0000..=0x02FF_FFFF => self.eram.read_byte((addr - 0x0200_0000) & 0x3FFFF),
            // IWRAM 32KB mirrors across 0x0300_0000-0x03FF_FFFF
            0x0300_0000..=0x03FF_FFFF => self.wram.read_byte((addr - 0x0300_0000) & 0x7FFF),
            0x0400_0000..=0x0400_005F => unreachable!("A lcdc bus width should be halfword."),
            0x0400_0100..=0x0400_010F => {
                let ofs = (addr - 0x0400_0100) as u32;
                let v = self.timers.read(ofs);
                // Debug: 最初の数十回だけタイマー読み出しをログ
                static TIMER_RD_CNT: OnceLock<AtomicUsize> = OnceLock::new();
                let c = TIMER_RD_CNT.get_or_init(|| AtomicUsize::new(0)).fetch_add(1, Ordering::Relaxed);
                if c < 64 {
                    let ch = (ofs / 4) as usize;
                    println!("⏱ TM{} READ ofs=0x{:02X} -> 0x{:02X}", ch, ofs & 0xFF, v);
                }
                v
            }
            0x0400_0060..=0x0400_03FF => 0,
            // Palette 1KB mirrors
            0x0500_0000..=0x05FF_FFFF => self.palette.read_byte((addr - 0x0500_0000) & 0x3FF),
            // 修正(004): VRAMミラー (0x20000で折り返し、0x18000-0x1FFFFは0x10000-0x17FFFへ)
            0x0600_0000..=0x06FF_FFFF => self.vram.read_byte(Self::map_vram_offset(addr)),
            // OAM 1KB mirrors
            0x0700_0000..=0x07FF_FFFF => self.oam.read_byte((addr - 0x0700_0000) & 0x3FF),
            // GamePak ROM mirrors across WS0/WS1/WS2 (0x08000000-0x0DFFFFFF)
            0x0800_0000..=0x0DFF_FFFF => self.rom.read_byte(Self::map_gamepak_offset(addr)),
            0x0E00_0000..=0x0E00_FFFF => {
                // SRAM/FRAM/Flash save memory (byte access only)
                self.sram.read_byte(addr - 0x0E00_0000)
            }
            _ => {
                println!("⚠️  WARNING: Invalid read_byte access to 0x{:08x} (not in GBA memory map) - returning 0xFF", addr);
                0xFF // Return invalid/unconnected bus value
            }
        }
    }

    fn read_halfword(&self, addr: u32) -> HalfWord {
        // dbg!(format!("read half word addr = {:x}", addr));
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_halfword(addr),
            // EWRAM mirrors
            0x0200_0000..=0x02FF_FFFF => self.eram.read_halfword((addr - 0x0200_0000) & 0x3FFFF),
            // IWRAM mirrors
            0x0300_0000..=0x03FF_FFFF => self.wram.read_halfword((addr - 0x0300_0000) & 0x7FFF),
            0x0400_0000..=0x0400_005F => self.lcdc.read_halfword(addr - 0x0400_0000),
            0x0400_0130 => self.key.read(),
            0x0400_0200 => self.interrupt_controller.borrow().read_ie(), // IE register
            0x0400_0202 => self.interrupt_controller.borrow().read_if(), // IF register
            0x0400_0204 => self.waitcnt,
            0x0400_0208 => self.interrupt_controller.borrow().read_ime(), // IME register
            0x0400_0100..=0x0400_010F => {
                let lo = self.read_byte(addr) as u16;
                let hi = self.read_byte(addr + 1) as u16;
                (hi << 8) | lo
            }
            0x0400_0060..=0x0400_03FF => {
                match addr {
                    // DMAxCNT_H reads
                    0x0400_00BA => self.dma.channels[0].control,
                    0x0400_00C6 => self.dma.channels[1].control,
                    0x0400_00D2 => self.dma.channels[2].control,
                    0x0400_00DE => self.dma.channels[3].control,
                    // DMAxCNT_L reads
                    0x0400_00B8 => self.dma.channels[0].count,
                    0x0400_00C4 => self.dma.channels[1].count,
                    0x0400_00D0 => self.dma.channels[2].count,
                    0x0400_00DC => self.dma.channels[3].count,
                    _ => 0,
                }
            },
            0x0500_0000..=0x05FF_FFFF => self.palette.read_halfword((addr - 0x0500_0000) & 0x3FF),
            // 修正(004): VRAMミラー対応
            0x0600_0000..=0x06FF_FFFF => self.vram.read_halfword(Self::map_vram_offset(addr)),
            // 修正(006): OAM 1KB ミラー（16bit 読み）
            0x0700_0000..=0x07FF_FFFF => self.oam.read_halfword((addr - 0x0700_0000) & 0x3FF),
            0x0800_0000..=0x0DFF_FFFF => self.rom.read_halfword(Self::map_gamepak_offset(addr)),
            0x0E00_0000..=0x0E00_FFFF => {
                println!("⚠️  WARNING: Invalid halfword access to SRAM at 0x{:08x} (SRAM is byte-access only) - returning 0xFFFF", addr);
                0xFFFF
            }
            _ => {
                println!("⚠️  WARNING: Invalid read_halfword access to 0x{:08x} (not in GBA memory map) - returning 0xFFFF", addr);
                0xFFFF
            }
        }
    }

    fn read_word(&self, addr: u32) -> Word {
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_word(addr),
            0x0200_0000..=0x02FF_FFFF => self.eram.read_word((addr - 0x0200_0000) & 0x3FFFF),
            0x0400_0100..=0x0400_010F => {
                // 組み合わせて32bit値を返す（TMxCNT_L/CNT_H）
                let base = (addr - 0x0400_0100) as u32;
                let b0 = self.timers.read(base + 0) as u32;
                let b1 = self.timers.read(base + 1) as u32;
                let b2 = self.timers.read(base + 2) as u32;
                let b3 = self.timers.read(base + 3) as u32;
                b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
            }
            0x0300_0000..=0x03FF_FFFF => {
                let off = (addr - 0x0300_0000) & 0x7FFF;
                let v = self.wram.read_word(off);
                if off == 0 || off == 4 || off == 8 {
                    println!("IWRAM read_word [0x{:08x}] -> 0x{:08x}", addr, v);
                }
                v
            },
            // 修正(004): VRAMミラー（0x20000で折り返し、0x18000..は0x10000..へ）
            0x0600_0000..=0x06FF_FFFF => {
                let vram_addr = Self::map_vram_offset(addr);
                self.vram.read_word(vram_addr)
            }
            // 修正(006): OAM 1KB ミラー（32bit 読み）
            0x0700_0000..=0x07FF_FFFF => self.oam.read_word((addr - 0x0700_0000) & 0x3FF),
            0x0400_0000..=0x0400_005F => self.lcdc.read_word(addr - 0x0400_0000),
            0x0400_0060..=0x0400_03FF => {
                match addr {
                    // DMAxSAD (source)
                    0x0400_00B0 => self.dma.channels[0].source,
                    0x0400_00BC => self.dma.channels[1].source,
                    0x0400_00C8 => self.dma.channels[2].source,
                    0x0400_00D4 => self.dma.channels[3].source,
                    // DMAxDAD (destination)
                    0x0400_00B4 => self.dma.channels[0].destination,
                    0x0400_00C0 => self.dma.channels[1].destination,
                    0x0400_00CC => self.dma.channels[2].destination,
                    0x0400_00D8 => self.dma.channels[3].destination,
                    // DMAxCNT (count | control<<16)
                    0x0400_00B8 => (self.dma.channels[0].count as u32) | ((self.dma.channels[0].control as u32) << 16),
                    0x0400_00C4 => (self.dma.channels[1].count as u32) | ((self.dma.channels[1].control as u32) << 16),
                    0x0400_00D0 => (self.dma.channels[2].count as u32) | ((self.dma.channels[2].control as u32) << 16),
                    0x0400_00DC => (self.dma.channels[3].count as u32) | ((self.dma.channels[3].control as u32) << 16),
                    _ => 0,
                }
            },
            0x0500_0000..=0x05FF_FFFF => self.palette.read_word((addr - 0x0500_0000) & 0x3FF),
            0x0800_0000..=0x0DFF_FFFF => self.rom.read_word(Self::map_gamepak_offset(addr)),
            0x0E00_0000..=0x0E00_FFFF => {
                println!("⚠️  WARNING: Invalid word access to SRAM at 0x{:08x} (SRAM is byte-access only) - returning 0xFFFFFFFF", addr);
                0xFFFFFFFF
            }
            _ => {
                println!("⚠️  WARNING: Invalid read_word access to 0x{:08x} (not in GBA memory map) - returning 0xFFFFFFFF", addr);
                0xFFFFFFFF
            }
        }
    }

    fn write_byte(&mut self, addr: u32, data: Byte) {
        // debug!("write byte addr = 0x{:x} data = 0x{:x}", addr, data);
        if addr == 0x0300_0008 {
            // dbg!(data);
            // dbg!("h");
            // panic!();
        }
        match addr {
            // 0x0000_0000...0x0007_FFFF => self.rom.borrow().read_word(addr),
            0x0200_0000..=0x02FF_FFFF => self.eram.write_byte((addr - 0x0200_0000) & 0x3FFFF, data),
            0x0300_0000..=0x03FF_FFFF => {
                if addr == 0x03007dd9 {
                    // dbg!("write to 0x03007dd9", data);
                }
                self.wram.write_byte((addr - 0x0300_0000) & 0x7FFF, data);
            }
            0x0400_0000..=0x0400_005F => unreachable!("A lcdc bus width should be halfword."),
            0x0400_0100..=0x0400_010F => {
                let ofs = (addr - 0x0400_0100) as u32;
                self.timers.write(ofs, data);
            }
            0x0400_0060..=0x0400_03FF => {
                // TODO: Implement DMA, Timer, and other I/O registers
                match addr {
                    // Sound FIFO A/B (accept data; no-op sink for now)
                    0x0400_00A0 | 0x0400_00A4 => {
                        println!("🎵 Sound FIFO write (byte): 0x{:08x} = 0x{:02x}", addr, data);
                    }
                    _ if (addr >= 0x0400_00B0 && addr <= 0x0400_00DE) => {
                        println!("🔧 DMA register byte write: 0x{:08x} = 0x{:02x} (not implemented)", addr, data);
                    }
                    _ => {}
                }
            }
            // Palette: byte store behaves as halfword store replicated (0xVV -> 0xVVVV)
            0x0500_0000..=0x05FF_FFFF => {
                let off = ((addr - 0x0500_0000) & 0x3FF) & !1; // align to halfword
                let hw = (data as HalfWord as u16) | (((data as HalfWord) as u16) << 8);
                self.palette.write_halfword(off, hw as HalfWord);
            }
            // VRAM: in bitmap modes (3/4/5), byte store behaves as halfword replicate; in tiled modes (0/1/2), ignore
            0x0600_0000..=0x06FF_FFFF => {
                let mode = self.lcdc.get_bg_mode();
                match mode {
                    crate::lcd::BgMode::Mode3 |
                    crate::lcd::BgMode::Mode4 |
                    crate::lcd::BgMode::Mode5 => {
                        let vram_addr = Self::map_vram_offset(addr) & !1; // align to halfword
                        let hw = (data as HalfWord as u16) | (((data as HalfWord) as u16) << 8);
                        if vram_addr < 0x10000 { // Log first 64KB of VRAM writes
                            println!("📝 VRAM byte-as-halfword write: 0x{:08x} (VRAM+0x{:04x}) = 0x{:04x}", addr, vram_addr, hw);
                        }
                        self.vram.write_halfword(vram_addr, hw as HalfWord);
                    }
                    _ => {
                        // Tiled modes: ignore byte writes to VRAM
                    }
                }
            }
            // OAM: byte stores are ignored
            0x0700_0000..=0x07FF_FFFF => {
                // ignore
            }
            0x0E00_0000..=0x0E00_FFFF => {
                // SRAM/FRAM/Flash save memory (byte access only)
                self.sram.write_byte(addr - 0x0E00_0000, data);
            }
            _ => {
                println!("⚠️  WARNING: Invalid write_byte to 0x{:08x} = 0x{:02x} (ignored)", addr, data);
            }
        };
    }

    fn write_halfword(&mut self, addr: u32, data: HalfWord) {
        // debug!("write half word addr = 0x{:x} data = 0x{:x}", addr, data);
        if addr == 0x0300_0008 {
            // dbg!(data);
            // dbg!("h");
        }
        match addr {
            // I/O Register
            0x0200_0000..=0x02FF_FFFF => self.eram.write_halfword((addr - 0x0200_0000) & 0x3F_FFFF, data),
            // WRAM
            0x0300_0000..=0x03FF_FFFF => {
                // info!("wram addr = {:x} {:x}", addr, data);
                self.wram.write_halfword((addr - 0x0300_0000) & 0x7FFF, data);
            }
            0x0400_0000..=0x0400_005F => self.lcdc.write_halfword(addr - 0x0400_0000, data),
            0x0400_0200 => self.interrupt_controller.borrow_mut().write_ie(data), // IE register
            0x0400_0202 => self.interrupt_controller.borrow_mut().write_if(data), // IF register  
            0x0400_0208 => self.interrupt_controller.borrow_mut().write_ime(data), // IME register
            0x0400_0204 => { // WAITCNT
                self.waitcnt = data;
                self.cycleLUT.update_waitcnt(data);
            }
            0x0400_0100..=0x0400_010F => {
                let lo = (data & 0x00FF) as u8;
                let hi = ((data >> 8) & 0x00FF) as u8;
                let base = (addr - 0x0400_0100) as u32;
                self.timers.write(base, lo);
                self.timers.write(base + 1, hi);
            }
            0x0400_0060..=0x0400_03FF => {
                // TODO: Implement DMA, Timer, and other I/O registers
                match addr {
                    // Sound FIFO A/B (accept data; no-op sink for now)
                    0x0400_00A0 | 0x0400_00A4 => {
                        println!("🎵 Sound FIFO write (halfword): 0x{:08x} = 0x{:04x}", addr, data);
                    }
                    _ if (addr >= 0x0400_00B0 && addr <= 0x0400_00DE) => {
                        println!("🔧 DMA register write: 0x{:08x} = 0x{:04x} (not implemented)", addr, data);
                    }
                    _ => {}
                }
            }
            0x0500_0000..=0x05FF_FFFF => self.palette.write_halfword((addr - 0x0500_0000) & 0x3FF, data),
            0x0600_0000..=0x06FF_FFFF => {
                // 修正(004): halfword write もミラー
                let vram_addr = Self::map_vram_offset(addr);
                if vram_addr < 0x10000 { // Log first 64KB of VRAM writes
                    println!("📝 VRAM halfword write: 0x{:08x} (VRAM+0x{:04x}) = 0x{:04x}", addr, vram_addr, data);
                }
                self.vram.write_halfword(vram_addr, data);
            }
            // 修正(006): OAM 1KB ミラー（16bit 書き）
            0x0700_0000..=0x07FF_FFFF => self.oam.write_halfword((addr - 0x0700_0000) & 0x3FF, data),
            0x0E00_0000..=0x0E00_FFFF => {
                println!("⚠️  WARNING: Invalid halfword write to SRAM at 0x{:08x} = 0x{:04x} (SRAM is byte-access only, ignored)", addr, data);
            }
            _ => {
                println!("⚠️  WARNING: Invalid write_halfword to 0x{:08x} = 0x{:04x} (ignored)", addr, data);
            }
        };
    }

    fn write_word(&mut self, addr: u32, data: Word) {
        // dbg!(format!("write word addr 0x{:x} data = 0x{:x}", addr, data));
        if addr == 0x0300_0008 {
            // dbg!(data);
            // dbg!("h");
            // panic!();
        }
        match addr {
            0x0000_0000..=0x0007_FFFF => panic!("illegal write access."),
            0x0200_0000..=0x02FF_FFFF => self.eram.write_word((addr - 0x0200_0000) & 0x3FFFF, data),
            // WRAM
            0x0300_0000..=0x0300_7FFF => {
                // info!("wram addr = {:x} {:x}", addr, data);
                let off = addr - 0x0300_0000;
                if off == 0 || off == 4 || off == 8 {
                    println!("IWRAM write_word [0x{:08x}] = 0x{:08x}", addr, data);
                }
                self.wram.write_word(off, data);
            }
            // Unused
            0x0300_8000..=0x03FF_FFFF => {
                println!("⚠️  WARNING: Write to unused area 0x{:08x} = 0x{:08x} (ignored)", addr, data);
            }
            0x0400_0000..=0x0400_005F => self.lcdc.write_word(addr - 0x0400_0000, data),
            0x0400_0100..=0x0400_010F => {
                // Split into 4 byte writes
                let base = (addr - 0x0400_0100) as u32;
                self.timers.write(base + 0, (data & 0x000000FF) as u8);
                self.timers.write(base + 1, ((data >> 8) & 0x000000FF) as u8);
                self.timers.write(base + 2, ((data >> 16) & 0x000000FF) as u8);
                self.timers.write(base + 3, ((data >> 24) & 0x000000FF) as u8);
            }
            0x0400_0204 => {
                // WAITCNT is 16-bit at 0x04000204; word writes may target it
                let hw = (data & 0xFFFF) as HalfWord;
                self.waitcnt = hw;
                self.cycleLUT.update_waitcnt(hw);
            }
            0x0400_0060..=0x0400_03FF => {
                // DMA and other I/O registers
                match addr {
                    // DMA0 registers
                    0x0400_00B0 => self.dma.write_source(0, data),
                    0x0400_00B4 => self.dma.write_destination(0, data),
                    0x0400_00B8 => {
                        // DMA0CNT_L (count) and DMA0CNT_H (control) are written together as 32-bit
                        let count = (data & 0xFFFF) as HalfWord;
                        let control = ((data >> 16) & 0xFFFF) as HalfWord;
                        self.dma.write_count(0, count);
                        self.dma.write_control(0, control);
                        // Immediate start: execute DMA right away
                        self.execute_dma_transfers();
                    }
                    0x0400_00BA => {
                        self.dma.write_control(0, data as HalfWord);
                        self.execute_dma_transfers();
                    }
                    
                    // DMA1 registers
                    0x0400_00BC => self.dma.write_source(1, data),
                    0x0400_00C0 => self.dma.write_destination(1, data),
                    0x0400_00C4 => {
                        let count = (data & 0xFFFF) as HalfWord;
                        let control = ((data >> 16) & 0xFFFF) as HalfWord;
                        self.dma.write_count(1, count);
                        self.dma.write_control(1, control);
                        self.execute_dma_transfers();
                    }
                    0x0400_00C6 => {
                        self.dma.write_control(1, data as HalfWord);
                        self.execute_dma_transfers();
                    }
                    
                    // DMA2 registers
                    0x0400_00C8 => self.dma.write_source(2, data),
                    0x0400_00CC => self.dma.write_destination(2, data),
                    0x0400_00D0 => {
                        let count = (data & 0xFFFF) as HalfWord;
                        let control = ((data >> 16) & 0xFFFF) as HalfWord;
                        self.dma.write_count(2, count);
                        self.dma.write_control(2, control);
                        self.execute_dma_transfers();
                    }
                    0x0400_00D2 => {
                        self.dma.write_control(2, data as HalfWord);
                        self.execute_dma_transfers();
                    }
                    
                    // DMA3 registers (most commonly used)
                    0x0400_00D4 => self.dma.write_source(3, data),
                    0x0400_00D8 => self.dma.write_destination(3, data),
                    0x0400_00DC => {
                        // DMA3CNT_L (count) and DMA3CNT_H (control) are written together as 32-bit
                        let count = (data & 0xFFFF) as HalfWord;
                        let control = ((data >> 16) & 0xFFFF) as HalfWord;
                        self.dma.write_count(3, count);
                        self.dma.write_control(3, control);
                        self.execute_dma_transfers();
                    }
                    0x0400_00DE => {
                        self.dma.write_control(3, data as HalfWord);
                        self.execute_dma_transfers();
                    }
                    
                    _ => {
                        // Other I/O registers (Timer, etc.)
                        if addr >= 0x0400_00B0 && addr <= 0x0400_00DE {
                            println!("🔧 Unknown DMA register word write: 0x{:08x} = 0x{:08x} (ignored)", addr, data);
                        }
                    }
                }
            }
            0x0500_0000..=0x05FF_FFFF => self.palette.write_word((addr - 0x0500_0000) & 0x3FF, data),
            0x0600_0000..=0x06FF_FFFF => {
                // 修正(004): 書き込み側もミラー
                let vram_addr = Self::map_vram_offset(addr);
                if vram_addr < 0x10000 {
                    println!("📝 VRAM word write: 0x{:08x} (VRAM+0x{:04x}) = 0x{:08x}", addr, vram_addr, data);
                }
                self.vram.write_word(vram_addr, data);
            }
            // 修正(006): OAM 1KB ミラー（32bit 書き）
            0x0700_0000..=0x07FF_FFFF => self.oam.write_word((addr - 0x0700_0000) & 0x3FF, data),
            0x0E00_0000..=0x0E00_FFFF => {
                println!("⚠️  WARNING: Invalid word write to SRAM at 0x{:08x} = 0x{:08x} (SRAM is byte-access only, ignored)", addr, data);
            }
            _ => {
                println!("⚠️  WARNING: Invalid write_word to 0x{:08x} = 0x{:08x} (ignored)", addr, data);
            }
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
    pub(crate) fn new(
        bios: Rom,
        lcdc: lcd::LCDController,
        rom: Rom,
        wram: Ram,
        eram: Ram,
        vram: Ram,
        palette: Ram,
        oam: Ram,
        sram: Ram, // SRAM/FRAM/Flash save memory
        key: io::Key,
    ) -> CpuBus {
        CpuBus {
            cycleLUT: CycleLUT::new(),
            lcdc,
            dma: DMAController::new(),
            bios,
            rom,
            wram,
            eram,
            vram,
            palette,
            oam,
            sram,
            key,
            interrupt_controller: std::cell::RefCell::new(crate::interrupt::InterruptController::new()),
            timers: crate::gba::timer::Timers::default(),
            waitcnt: 0,
        }
    }

    pub(crate) fn update_key(&mut self, key: io::Key) {
        self.key = key;
    }
    
    pub(crate) fn request_vblank_interrupt(&mut self) {
        self.interrupt_controller.borrow_mut().request_interrupt(crate::interrupt::InterruptType::VBlank);
    }
    
    pub(crate) fn should_service_interrupt(&self) -> bool {
        self.interrupt_controller.borrow().should_service_interrupt()
    }

    pub(crate) fn execute_dma_transfers(&mut self) {
        // Check for pending DMA transfers and execute them
        for channel in 0..4 {
            if let Some((source, dest, count, transfer_size)) = self.dma.get_pending_transfer(channel) {
                self.perform_dma_transfer(channel, source, dest, count, transfer_size);
                self.dma.complete_transfer(channel);
            }
        }
    }

    pub(crate) fn tick_timers(&mut self, now_cycles: u64) {
        let irq = &mut *self.interrupt_controller.borrow_mut();
        self.timers.tick(now_cycles, irq);
    }

    // VRAM mirror mapping helper
    // GBA VRAM is 96KB (0x18000) within a 128KB window (0x20000).
    // 0x06000000-0x06FFFFFF should wrap every 0x20000, and offsets >= 0x18000
    // mirror the last 32KB (map to 0x10000-0x17FFF).
    fn map_vram_offset(addr: u32) -> u32 {
        // 128KB window wrap first (0x20000-1 = 0x1_FFFF)
        let off_20000 = (addr - 0x0600_0000) & 0x1_FFFF;
        if off_20000 >= 0x18000 { off_20000 - 0x8000 } else { off_20000 }
    }

    // Map GamePak address (0x08000000-0x0DFFFFFF) to ROM offset
    // WS0: 0x08000000-0x09FFFFFF, WS1: 0x0A000000-0x0BFFFFFF, WS2: 0x0C000000-0x0DFFFFFF
    #[inline]
    fn map_gamepak_offset(addr: u32) -> u32 {
        match addr {
            0x0800_0000..=0x09FF_FFFF => addr - 0x0800_0000,
            0x0A00_0000..=0x0BFF_FFFF => addr - 0x0A00_0000,
            0x0C00_0000..=0x0DFF_FFFF => addr - 0x0C00_0000,
            _ => unreachable!("address out of gamepak range"),
        }
    }

    fn perform_dma_transfer(&mut self, channel: usize, mut source: Word, mut dest: Word, count: usize, transfer_size: usize) {
        println!("🚀 Executing DMA{} transfer: 0x{:08x} -> 0x{:08x}, {} words of {} bytes", 
            channel, source, dest, count, transfer_size);

        for i in 0..count {
            if transfer_size == 4 {
                // 32-bit transfer
                let data = self.read_word_internal(source);
                self.write_word_internal(dest, data);
                if i < 5 { // Log first few transfers
                    println!("  DMA{} [{}]: 0x{:08x} -> 0x{:08x} = 0x{:08x}", channel, i, source, dest, data);
                }
            } else {
                // 16-bit transfer
                let data = self.read_halfword_internal(source);
                self.write_halfword_internal(dest, data);
                if i < 5 { // Log first few transfers
                    println!("  DMA{} [{}]: 0x{:08x} -> 0x{:08x} = 0x{:04x}", channel, i, source, dest, data);
                }
            }

            // Update addresses based on control settings
            let src_control = self.dma.channels[channel].get_source_control();
            let dest_control = self.dma.channels[channel].get_dest_control();

            match src_control {
                0 => source += transfer_size as u32, // Increment
                1 => source -= transfer_size as u32, // Decrement
                2 => {}, // Fixed
                3 => {}, // Prohibited
                _ => {}
            }

            match dest_control {
                0 => dest += transfer_size as u32, // Increment
                1 => dest -= transfer_size as u32, // Decrement
                2 => {}, // Fixed
                3 => dest += transfer_size as u32, // Increment/Reload
                _ => {}
            }
        }

        println!("✅ DMA{} transfer completed: {} transfers", channel, count);
    }

    // Internal memory access methods that bypass DMA triggering
    fn read_word_internal(&self, addr: Word) -> Word {
        match addr {
            0x0800_0000..=0x0DFF_FFFF => self.rom.read_word(Self::map_gamepak_offset(addr)),
            0x0300_0000..=0x0300_7FFF => self.wram.read_word(addr - 0x0300_0000),
            0x0200_0000..=0x0203_FFFF => self.eram.read_word(addr - 0x0200_0000),
            0x0600_0000..=0x06FF_FFFF => self.vram.read_word(Self::map_vram_offset(addr)),
            _ => {
                println!("⚠️  DMA read_word from unsupported address: 0x{:08x}", addr);
                0
            }
        }
    }

    fn read_halfword_internal(&self, addr: Word) -> HalfWord {
        match addr {
            0x0800_0000..=0x0DFF_FFFF => self.rom.read_halfword(Self::map_gamepak_offset(addr)),
            0x0300_0000..=0x0300_7FFF => self.wram.read_halfword(addr - 0x0300_0000),
            0x0200_0000..=0x0203_FFFF => self.eram.read_halfword(addr - 0x0200_0000),
            0x0600_0000..=0x06FF_FFFF => self.vram.read_halfword(Self::map_vram_offset(addr)),
            _ => {
                println!("⚠️  DMA read_halfword from unsupported address: 0x{:08x}", addr);
                0
            }
        }
    }

    fn write_word_internal(&mut self, addr: Word, data: Word) {
        match addr {
            0x0400_00A0 | 0x0400_00A4 => {
                println!("🎵 DMA Sound FIFO write (word): 0x{:08x} = 0x{:08x}", addr, data);
            }
            0x0600_0000..=0x06FF_FFFF => {
                let vram_addr = Self::map_vram_offset(addr);
                if vram_addr < 0x10000 { // Log first 64KB of VRAM writes
                    println!("📝 DMA VRAM word write: 0x{:08x} (VRAM+0x{:04x}) = 0x{:08x}", addr, vram_addr, data);
                }
                self.vram.write_word(vram_addr, data);
            }
            0x0500_0000..=0x0500_03FF => {
                println!("📝 DMA Palette word write: 0x{:08x} = 0x{:08x}", addr, data);
                self.palette.write_word(addr - 0x0500_0000, data);
            }
            0x0300_0000..=0x0300_7FFF => self.wram.write_word(addr - 0x0300_0000, data),
            0x0200_0000..=0x02FF_FFFF => self.eram.write_word((addr - 0x0200_0000) & 0x3FFFF, data),
            _ => {
                println!("⚠️  DMA write_word to unsupported address: 0x{:08x} = 0x{:08x}", addr, data);
            }
        }
    }

    fn write_halfword_internal(&mut self, addr: Word, data: HalfWord) {
        match addr {
            0x0400_00A0 | 0x0400_00A4 => {
                println!("🎵 DMA Sound FIFO write (halfword): 0x{:08x} = 0x{:04x}", addr, data);
            }


            0x0600_0000..=0x06FF_FFFF => {
                let vram_addr = Self::map_vram_offset(addr);
                if vram_addr < 0x10000 { // Log first 64KB of VRAM writes
                    println!("📝 DMA VRAM halfword write: 0x{:08x} (VRAM+0x{:04x}) = 0x{:04x}", addr, vram_addr, data);
                }
                self.vram.write_halfword(vram_addr, data);
            }
            0x0500_0000..=0x0500_03FF => {
                println!("📝 DMA Palette halfword write: 0x{:08x} = 0x{:04x}", addr, data);
                self.palette.write_halfword(addr - 0x0500_0000, data);
            }
            0x0300_0000..=0x0300_7FFF => self.wram.write_halfword(addr - 0x0300_0000, data),
            0x0200_0000..=0x0203_FFFF => self.eram.write_halfword(addr - 0x0200_0000, data),
            _ => {
                println!("⚠️  DMA write_halfword to unsupported address: 0x{:08x} = 0x{:04x}", addr, data);
            }
        }
    }

    pub(crate) fn borrow_mut_lcdc(&mut self) -> &mut lcd::LCDController {
        &mut self.lcdc
    }

    pub(crate) fn borrow_lcdc(&self) -> &lcd::LCDController {
        &self.lcdc
    }

    pub(crate) fn borrow_vram(&self) -> &Ram {
        &self.vram
    }

    pub(crate) fn borrow_palette(&self) -> &Ram {
        &self.palette
    }

    pub(crate) fn borrow_oam(&self) -> &Ram {
        &self.oam
    }
}
