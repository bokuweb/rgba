use crate::cpu::bus;
use crate::gba::dma::DMAController;
use crate::gba::eeprom::{EEPROM, EEPROMSize};
use crate::gba::interrupt::InterruptController;
use crate::gba::timer::TimerController;
use crate::io;
use crate::lcd;
use crate::types::*;

pub(crate) use bus::accessor::*;

use crate::memory::ram::Ram;
use crate::memory::readable::*;
use crate::memory::rom::Rom;
use crate::memory::writable::*;

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

    pub fn update(&mut self, _waitcnt: ()) {
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
    cycle_lut: CycleLUT,
    lcdc: lcd::LCDController,
    bios: Rom,
    rom: Rom,
    wram: Ram,
    eram: Ram,
    vram: Ram,
    palette: Ram,
    oam: Ram,
    key: io::Key,
    interrupt_controller: InterruptController,
    dma_controller: DMAController,
    timer_controller: TimerController,
    eeprom: Option<EEPROM>,
}

impl BusAccessor for CpuBus {
    fn read_byte(&self, addr: u32) -> Byte {
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_byte(addr),
            0x0200_0000..=0x02FF_FFFF => self.eram.read_byte((addr - 0x0200_0000) % 0x40000),
            0x0300_0000..=0x0300_7FFF => self.wram.read_byte(addr - 0x0300_0000),
            0x0300_8000..=0x03FF_FFFF => 0,
            0x0400_0000..=0x0400_005F => unreachable!("A lcdc bus width should be halfword."),
            0x0400_0060..=0x0400_03FF => 0,
            0x0500_0000..=0x0500_03FF => self.palette.read_byte(addr - 0x0500_0000),
            0x0600_0000..=0x0601_7FFF => self.vram.read_byte(addr - 0x0600_0000),
            0x0700_0000..=0x0700_03FF => self.oam.read_byte(addr - 0x0700_0000),
            0x0800_0000..=0x09FF_FFFF => self.rom.read_byte(addr - 0x0800_0000),
            0x0A00_0000..=0x0BFF_FFFF => self.rom.read_byte((addr - 0x0A00_0000) % self.rom.size() as u32), // GamePak WS1 - mirror ROM
            0x0C00_0000..=0x0CFF_FFFF => self.rom.read_byte((addr - 0x0C00_0000) % self.rom.size() as u32), // GamePak WS2 - mirror ROM
            0x0D00_0000..=0x0DFF_FFFF => {

                self.rom.read_byte((addr - 0x0D00_0000) % self.rom.size() as u32) // EEPROM range - currently mirror ROM
            }
            0x0E00_0000..=0x0FFF_FFFF => {

                0 // SRAM - return 0 for now
            }
            _ => {
                let a = format!("read byte addr = {:x}", addr);
                panic!("TODO: {:?}", a);
            }
        }
    }

    fn read_halfword(&self, addr: u32) -> HalfWord {
        // dbg!(format!("read half word addr = {:x}", addr));
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_halfword(addr),
            0x0200_0000..=0x02FF_FFFF => self.eram.read_halfword((addr - 0x0200_0000) % 0x40000),
            0x0300_0000..=0x0300_7FFF => {
                if addr == 0x03007FF8 {
                    // BIOS IF work area - used by IntrWait/VBlankIntrWait
                    self.interrupt_controller.read_bios_if_work()
                } else {
                    self.wram.read_halfword(addr - 0x0300_0000)
                }
            },
            0x0300_8000..=0x03FF_FFFF => 0,
            0x0400_0000..=0x0400_005F => self.lcdc.read_halfword(addr - 0x0400_0000),
            0x0400_0130 => {
                let key_value = self.key.read();
                println!("KEYINPUT read: 0x{:04x}", key_value);
                key_value
            },
            0x0400_0060..=0x0400_03FF => {
                match addr {
                    0x0400_0100..=0x0400_010F => self.timer_controller.read(addr), // Timer registers
                    0x0400_00B0..=0x0400_00DF => self.dma_controller.read_register(addr), // DMA registers
                    0x0400_0200 => self.interrupt_controller.read_ie(), // IE - Interrupt Enable Register
                    0x0400_0202 => {
                        let if_value = self.interrupt_controller.read_if();
                        println!("🔵 IF register read at PC location, returning: 0x{:04x}", if_value);
                        if_value
                    } // IF - Interrupt Request Flags / IRQ Acknowledge
                    0x0400_0204 => 0, // WAITCNT - Game Pak Waitstate Control
                    0x0400_0208 => self.interrupt_controller.read_ime(), // IME - Interrupt Master Enable Register
                    _ => {

                        0
                    }
                }
            },
            0x0500_0000..=0x0500_03FF => self.palette.read_halfword(addr - 0x0500_0000),
            0x0600_0000..=0x0601_7FFF => self.vram.read_halfword(addr - 0x0600_0000),
            0x0700_0000..=0x0700_03FF => self.oam.read_halfword(addr - 0x0700_0000),
            0x0800_0000..=0x09FF_FFFF => self.rom.read_halfword(addr - 0x0800_0000),
            0x0A00_0000..=0x0BFF_FFFF => self.rom.read_halfword((addr - 0x0A00_0000) % self.rom.size() as u32), // GamePak WS1 - mirror ROM
            0x0C00_0000..=0x0CFF_FFFF => self.rom.read_halfword((addr - 0x0C00_0000) % self.rom.size() as u32), // GamePak WS2 - mirror ROM
            0x0D00_0000..=0x0DFF_FFFF => {
                // EEPROM range - typically accessed at 0xDFFFF00-0xDFFFFFF
                if let Some(ref eeprom) = self.eeprom {
                    // Check DMA3 status for proper EEPROM operation
                    let dma3_enabled = self.dma_controller.get_channel(3)
                        .map(|ch| ch.is_enabled())
                        .unwrap_or(false);
                    eeprom.peek_read_halfword(addr, dma3_enabled)
                } else {

                    1 // Return 1 when no EEPROM is present
                }
            }
            0x0E00_0000..=0x0FFF_FFFF => {

                0 // SRAM - return 0 for now
            }
            _ => panic!("TODO: {:x}", addr),
        }
    }

    fn read_word(&self, addr: u32) -> Word {
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_word(addr),
            0x0200_0000..=0x02FF_FFFF => self.eram.read_word((addr - 0x0200_0000) % 0x40000),
            0x0300_0000..=0x0300_7FFF => self.wram.read_word(addr - 0x0300_0000),
            0x0300_8000..=0x03FF_FFFF => 0,
            0x0400_0000..=0x0400_005F => self.lcdc.read_word(addr - 0x0400_0000),
            0x0400_0060..=0x0400_03FF => {
                match addr {
                    0x0400_00B0..=0x0400_00DF => {
                        // DMA registers: compose from two halfwords
                        let lo = self.dma_controller.read_register(addr) as u32;
                        let hi = self.dma_controller.read_register(addr + 2) as u32;
                        (lo | (hi << 16))
                    }
                    0x0400_0100..=0x0400_010F => {
                        // Timer registers: compose from two halfwords
                        let lo = self.timer_controller.read(addr) as u32;
                        let hi = self.timer_controller.read(addr + 2) as u32;
                        (lo | (hi << 16))
                    }
                    0x0400_0200 => {
                        // IE | IF
                        let lo = self.interrupt_controller.read_ie() as u32;
                        let hi = self.interrupt_controller.read_if() as u32;
                        (lo | (hi << 16))
                    }
                    0x0400_0208 => self.interrupt_controller.read_ime() as u32,
                    _ => 0,
                }
            },
            0x0500_0000..=0x0500_03FF => self.palette.read_word(addr - 0x0500_0000),
            0x0600_0000..=0x0601_7FFF => self.vram.read_word(addr - 0x0600_0000),
            0x0700_0000..=0x0700_03FF => self.oam.read_word(addr - 0x0700_0000),
            0x0800_0000..=0x09FF_FFFF => self.rom.read_word(addr - 0x0800_0000),
            0x0A00_0000..=0x0BFF_FFFF => self.rom.read_word((addr - 0x0A00_0000) % self.rom.size() as u32), // GamePak WS1 - mirror ROM
            0x0C00_0000..=0x0CFF_FFFF => self.rom.read_word((addr - 0x0C00_0000) % self.rom.size() as u32), // GamePak WS2 - mirror ROM
            0x0D00_0000..=0x0DFF_FFFF => {

                self.rom.read_word((addr - 0x0D00_0000) % self.rom.size() as u32) // EEPROM range - currently mirror ROM
            }
            0x0E00_0000..=0x0FFF_FFFF => {

                0 // SRAM - return 0 for now
            }
            _ => {
                if addr == 0xc8002489 {

                }
                panic!("TODO: addr = 0x{:x}", addr)
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
            0x0200_0000..=0x02FF_FFFF => self.eram.write_byte((addr - 0x0200_0000) % 0x40000, data),
            0x0300_0000..=0x0300_7FFF => {
                if addr == 0x03007dd9 {
                    // dbg!("write to 0x03007dd9", data);
                }
                self.wram.write_byte(addr - 0x0300_0000, data);
            }
            0x0400_0000..=0x0400_005F => unreachable!("A lcdc bus width should be halfword."),
            0x0400_0060..=0x0400_03FF => {}
            0x0500_0000..=0x0500_03FF => self.palette.write_byte(addr - 0x0500_0000, data),
            0x0600_0000..=0x0601_7FFF => self.vram.write_byte(addr - 0x0600_0000, data),
            0x0700_0000..=0x0700_03FF => self.oam.write_byte(addr - 0x0700_0000, data),
            0x0D00_0000..=0x0DFF_FFFF => {
                println!("[EEPROM] Write byte access at 0x{:08X} = 0x{:02X}", addr, data);
                // EEPROM range - ignore writes for now (should not be used for byte access)
            }
            0x0E00_0000..=0x0FFF_FFFF => {
                println!("[SRAM] Write byte access at 0x{:08X} = 0x{:02X}", addr, data);
                // SRAM - ignore writes for now
            }
            _ => panic!("TODO: 0x{:x} 0x{:x}", addr, data),
        };
    }

    fn write_halfword(&mut self, addr: u32, data: HalfWord) {
        // Monitor test result output (AGS patch writes to 0x0004)
        if addr == 0x0000_0004 {
            println!("🎯 TEST RESULT OUTPUT at 0x{:08x}: 0x{:04x}", addr, data);
        }
        
        // debug!("write half word addr = 0x{:x} data = 0x{:x}", addr, data);
        if addr == 0x0300_0008 {
            // dbg!(data);
            // dbg!("h");
        }
        match addr {
            // I/O Register
            0x0200_0000..=0x02FF_FFFF => self.eram.write_halfword((addr - 0x0200_0000) % 0x40000, data),
            // WRAM
            0x0300_0000..=0x0300_7FFF => {
                if addr == 0x03007FF8 {
                    // BIOS IF work area - used by IntrWait/VBlankIntrWait
                    self.interrupt_controller.write_bios_if_work(data);
                } else {
                // info!("wram addr = {:x} {:x}", addr, data);
                self.wram.write_halfword(addr - 0x0300_0000, data);
                }
            }
            0x0400_0000..=0x0400_005F => {
                if addr == 0x04000000 {
                    println!("📺 DISPCNT write: 0x{:04x} (Mode:{}, BG0-3:{}{}{}{}, OBJ:{}, Win0-2/OBJ:{}{}{}, ForceBlank:{})", 
                        data, 
                        data & 0x7,
                        if data & 0x100 != 0 {"✓"} else {"-"},
                        if data & 0x200 != 0 {"✓"} else {"-"},
                        if data & 0x400 != 0 {"✓"} else {"-"},
                        if data & 0x800 != 0 {"✓"} else {"-"},
                        if data & 0x1000 != 0 {"✓"} else {"-"},
                        if data & 0x2000 != 0 {"✓"} else {"-"},
                        if data & 0x4000 != 0 {"✓"} else {"-"},
                        if data & 0x8000 != 0 {"✓"} else {"-"},
                        if data & 0x80 != 0 {"ON"} else {"OFF"}
                    );
                } else if addr >= 0x04000008 && addr <= 0x0400001E {
                    println!("🖼️  BGxCNT write: 0x{:08x} = 0x{:04x}", addr, data);
                } else {
                    println!("LCD reg write HW: 0x{:08x} = 0x{:04x}", addr, data);
                }
                self.lcdc.write_halfword(addr - 0x0400_0000, data)
            },
            0x0400_0060..=0x0400_03FF => {
                match addr {
                    0x0400_0100..=0x0400_010F => self.timer_controller.write(addr, data), // Timer registers
                    0x0400_00B0..=0x0400_00DF => { // DMA registers
                        println!("DMA reg write: 0x{:08x} = 0x{:04x}", addr, data);
                        self.dma_controller.write_register(addr, data);

                        // If CNT_H (offset 10) written and start timing is immediate (0), execute now
                        let ch_id = ((addr - 0x0400_00B0) / 12) as usize;
                        let reg_offset = (addr - 0x0400_00B0) % 12;
                        if reg_offset == 10 {
                            if let Some(ch) = self.dma_controller.get_channel(ch_id) {
                                let timing = ch.get_start_timing();
                                if ch.is_enabled() && timing == 0 {
                                    println!("Kick Immediate DMA{}", ch_id);
                                    let (mut src, mut dst, mut count, tr32, src_ctrl, dst_ctrl, irq) = (
                                        ch.internal_source,
                                        ch.internal_destination,
                                        ch.get_count(),
                                        ch.get_transfer_type(),
                                        ch.get_source_addr_control(),
                                        ch.get_dest_addr_control(),
                                        ch.is_irq_enabled(),
                                    );
                                    let mut transfers = 0usize;
                                    while count > 0 && transfers < 0x4000 {
                                        let sdata = self.read_word(src);
                                        if tr32 { self.write_word(dst, sdata); }
                                        else { self.write_halfword(dst, (sdata & 0xFFFF) as HalfWord); }
                                        let step = if tr32 { 4 } else { 2 };
                                        match src_ctrl { 0 => src += step, 1 => src -= step, _ => {} }
                                        match dst_ctrl { 0 => dst += step, 1 => dst -= step, 3 => dst += step, _ => {} }
                                        count -= 1;
                                        transfers += 1;
                                    }
                                    if let Some(chm) = self.dma_controller.get_channel_mut(ch_id) {
                                        chm.internal_source = src;
                                        chm.internal_destination = dst;
                                        chm.set_count(count);
                                        if count == 0 {
                                            chm.enable = false;
                                            if irq {
                                                let it = match ch_id { 0=>crate::gba::interrupt::InterruptType::DMA0, 1=>crate::gba::interrupt::InterruptType::DMA1, 2=>crate::gba::interrupt::InterruptType::DMA2, _=>crate::gba::interrupt::InterruptType::DMA3 };
                                                self.interrupt_controller.request_interrupt(it);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    0x0400_0200 => self.interrupt_controller.write_ie(data), // IE - Interrupt Enable Register
                    0x0400_0202 => {
                        println!("DEBUG: write_halfword to IF (0x0400_0202) with data: 0x{:04x}", data);
                        self.interrupt_controller.write_if(data)
                    }, // IF - Interrupt Request Flags / IRQ Acknowledge
                    0x0400_0204 => {}, // WAITCNT - Game Pak Waitstate Control
                    0x0400_0208 => self.interrupt_controller.write_ime(data), // IME - Interrupt Master Enable Register
                    _ => {
                        println!("I/O write halfword: 0x{:08x} = 0x{:04x}", addr, data);
                    }
                }
            }
            0x0500_0000..=0x0500_03FF => self.palette.write_halfword(addr - 0x0500_0000, data),
            0x0600_0000..=0x0601_7FFF => {
                println!("VRAM write at 0x{:08x}: 0x{:04x}", addr, data);
                self.vram.write_halfword(addr - 0x0600_0000, data);
            }
            0x0700_0000..=0x0700_03FF => self.oam.write_halfword(addr - 0x0700_0000, data),
            0x0D00_0000..=0x0DFF_FFFF => {
                // EEPROM range - typically accessed at 0xDFFFF00-0xDFFFFFF
                if let Some(ref mut eeprom) = self.eeprom {
                    // Get DMA3 count for proper EEPROM operation
                    let dma_count = self.dma_controller.get_channel(3)
                        .map(|ch| ch.get_count())
                        .unwrap_or(64); // Default for read operations
                    eeprom.write_halfword(addr, data, dma_count);
                } else {
                    println!("[EEPROM] Write halfword access at 0x{:08X} = 0x{:04X} (no EEPROM)", addr, data);
                }
            }
            0x0E00_0000..=0x0FFF_FFFF => {
                println!("[SRAM] Write halfword access at 0x{:08X} = 0x{:04X}", addr, data);
                // SRAM - ignore writes for now
            }
            _ => panic!("TODO: "),
        };
    }

    fn write_word(&mut self, addr: u32, data: Word) {
        // Monitor test result output (AGS patch writes to 0x0004)
        if addr == 0x0000_0004 {
            println!("🎯 TEST RESULT OUTPUT at 0x{:08x}: 0x{:08x}", addr, data);
        }
        
        // dbg!(format!("write word addr 0x{:x} data = 0x{:x}", addr, data));
        if addr == 0x0300_0008 {
            // dbg!(data);
            // dbg!("h");
            // panic!();
        }
        match addr {
            0x0000_0000..=0x0007_FFFF => {
                // Allow writes to BIOS region for our custom IRQ handler
                // Ignore writes to BIOS area (read-only)
            }
            0x0200_0000..=0x02FF_FFFF => self.eram.write_word((addr - 0x0200_0000) % 0x40000, data),
            // WRAM
            0x0300_0000..=0x0300_7FFF => {
                if addr == 0x0300_7FFC {
                    println!("🧭 User IRQ handler set @0x03007FFC = 0x{:08x}", data);
                }
                self.wram.write_word(addr - 0x0300_0000, data);
            }
            // Unused
            0x0300_8000..=0x03FF_FFFF => {
                // // dbg!(format!("{:x}", addr));
                panic!("unused")
            }
            0x0400_0000..=0x0400_005F => self.lcdc.write_word(addr - 0x0400_0000, data),
            0x0400_0060..=0x0400_03FF => {
                match addr {
                    0x0400_0200 => { // IE|IF as word write
                        let ie: u16 = (data & 0xFFFF) as u16;
                        let if_mask: u16 = ((data >> 16) & 0xFFFF) as u16;
                        self.interrupt_controller.write_ie(ie);
                        self.interrupt_controller.write_if(if_mask);
                    }
                    0x0400_0208 => { // IME as word: take low half
                        let ime: u16 = (data & 0xFFFF) as u16;
                        self.interrupt_controller.write_ime(ime);
                    }
                    _ => {
                        // Fallback: split into two halfword writes for unknown IO
                        let lo: u16 = (data & 0xFFFF) as u16;
                        let hi: u16 = ((data >> 16) & 0xFFFF) as u16;
                        println!("I/O write word: 0x{:08x} = 0x{:08x}", addr, data);
                        self.write_halfword(addr, lo);
                        self.write_halfword(addr + 2, hi);
                    }
                }
            }
            0x0500_0000..=0x0500_03FF => self.palette.write_word(addr - 0x0500_0000, data),
            0x0600_0000..=0x0601_7FFF => {
                println!("VRAM write word at 0x{:08x}: 0x{:08x}", addr, data);
                self.vram.write_word(addr - 0x0600_0000, data);
            }
            0x0700_0000..=0x0700_03FF => self.oam.write_word(addr - 0x0700_0000, data),
            0x0D00_0000..=0x0DFF_FFFF => {
                println!("[EEPROM] Write word access at 0x{:08X} = 0x{:08X}", addr, data);
                // EEPROM range - ignore writes for now (should not be used for word access)
            }
            0x0E00_0000..=0x0FFF_FFFF => {
                println!("[SRAM] Write word access at 0x{:08X} = 0x{:08X}", addr, data);
                // SRAM - ignore writes for now
            }
            _ => panic!("TODO: addr = {:x} data = {:x}", addr, data),
        };
    }

    fn compute_cycle(&self, addr: Word, access_type: AccessType) -> Cycle {
        let page = (addr >> 24) as usize;
        if page > 0xF {
            return 1;
        }
        match access_type {
            AccessType::NonSeq(AccessWidth::Byte) | AccessType::NonSeq(AccessWidth::HalfWord) => self.cycle_lut.n16[page],
            AccessType::NonSeq(AccessWidth::Word) => self.cycle_lut.n32[page],
            AccessType::Seq(AccessWidth::Byte) | AccessType::Seq(AccessWidth::HalfWord) => self.cycle_lut.s16[page],
            AccessType::Seq(AccessWidth::Word) => self.cycle_lut.s32[page],
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
        key: io::Key,
    ) -> CpuBus {
        // Auto-detect EEPROM based on ROM content or size
        let eeprom = Self::detect_eeprom(&rom);
        
        CpuBus {
            cycle_lut: CycleLUT::new(),
            lcdc,
            bios,
            rom,
            wram,
            eram,
            vram,
            palette,
            oam,
            key,
            interrupt_controller: InterruptController::new(),
            dma_controller: DMAController::new(),
            timer_controller: TimerController::new(),
            eeprom,
        }
    }

    pub(crate) fn update_key(&mut self, key: io::Key) {
        self.key = key;
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

    pub(crate) fn borrow_mut_interrupt_controller(&mut self) -> &mut InterruptController {
        &mut self.interrupt_controller
    }

    pub(crate) fn borrow_interrupt_controller(&self) -> &InterruptController {
        &self.interrupt_controller
    }

    pub(crate) fn update_lcd(&mut self, cycles: usize) -> bool {
        // Update timers and handle timer interrupts
        let timer_interrupts = self.timer_controller.update(cycles as u32);
        for interrupt_type in timer_interrupts {
            self.interrupt_controller.request_interrupt(interrupt_type);
        }

        // Update LCD and handle VBlank/HBlank interrupts
        let frame_ready = self.lcdc.run(cycles, &mut self.interrupt_controller);

        // Kick DMA with start-timing VBlank (1) on VBlank start edge
        if self.lcdc.take_vblank_edge() {
            for ch_id in 0..4 {
                let start_on_vblank = self.dma_controller.get_channel(ch_id)
                    .map(|ch| ch.is_enabled() && ch.get_start_timing() == 1)
                    .unwrap_or(false);
                if !start_on_vblank { continue; }
                println!("Kick VBlank DMA{}", ch_id);
                // Minimal transfer: perform up to 0x4000 units
                let (mut src, mut dst, mut count, tr32, src_ctrl, dst_ctrl, irq) = {
                    let ch = self.dma_controller.get_channel(ch_id).unwrap();
                    (
                        ch.internal_source,
                        ch.internal_destination,
                        ch.get_count(),
                        ch.get_transfer_type(),
                        ch.get_source_addr_control(),
                        ch.get_dest_addr_control(),
                        ch.is_irq_enabled(),
                    )
                };
                let mut transfers = 0usize;
                while count > 0 && transfers < 0x4000 {
                    let sdata = self.read_word(src);
                    if tr32 { self.write_word(dst, sdata); }
                    else { self.write_halfword(dst, (sdata & 0xFFFF) as HalfWord); }
                    let step = if tr32 { 4 } else { 2 };
                    match src_ctrl { 0 => src += step, 1 => src -= step, _ => {} }
                    match dst_ctrl { 0 => dst += step, 1 => dst -= step, 3 => dst += step, _ => {} }
                    count -= 1;
                    transfers += 1;
                }
                if let Some(ch) = self.dma_controller.get_channel_mut(ch_id) {
                    ch.internal_source = src;
                    ch.internal_destination = dst;
                    ch.set_count(count);
                    if count == 0 {
                        ch.enable = false;
                        if irq {
                            let it = match ch_id { 0=>crate::gba::interrupt::InterruptType::DMA0, 1=>crate::gba::interrupt::InterruptType::DMA1, 2=>crate::gba::interrupt::InterruptType::DMA2, _=>crate::gba::interrupt::InterruptType::DMA3 };
                            self.interrupt_controller.request_interrupt(it);
                        }
                    }
                }
            }
        }

        // Kick DMA with start-timing HBlank (2) for each HBlank edge
        let hblanks = self.lcdc.take_hblank_edges();
        if hblanks > 0 {
            for _ in 0..hblanks {
                for ch_id in 0..4 {
                    let start_on_hblank = self.dma_controller.get_channel(ch_id)
                        .map(|ch| ch.is_enabled() && ch.get_start_timing() == 2)
                        .unwrap_or(false);
                    if !start_on_hblank { continue; }
                    println!("Kick HBlank DMA{}", ch_id);
                    let (mut src, mut dst, mut count, tr32, src_ctrl, dst_ctrl, irq) = {
                        let ch = self.dma_controller.get_channel(ch_id).unwrap();
                        (
                            ch.internal_source,
                            ch.internal_destination,
                            ch.get_count(),
                            ch.get_transfer_type(),
                            ch.get_source_addr_control(),
                            ch.get_dest_addr_control(),
                            ch.is_irq_enabled(),
                        )
                    };
                    let mut transfers = 0usize;
                    while count > 0 && transfers < 0x400 {
                        let sdata = self.read_word(src);
                        if tr32 { self.write_word(dst, sdata); }
                        else { self.write_halfword(dst, (sdata & 0xFFFF) as HalfWord); }
                        let step = if tr32 { 4 } else { 2 };
                        match src_ctrl { 0 => src += step, 1 => src -= step, _ => {} }
                        match dst_ctrl { 0 => dst += step, 1 => dst -= step, 3 => dst += step, _ => {} }
                        count -= 1;
                        transfers += 1;
                    }
                    if let Some(ch) = self.dma_controller.get_channel_mut(ch_id) {
                        ch.internal_source = src;
                        ch.internal_destination = dst;
                        ch.set_count(count);
                        if count == 0 {
                            ch.enable = false;
                            if irq {
                                let it = match ch_id { 0=>crate::gba::interrupt::InterruptType::DMA0, 1=>crate::gba::interrupt::InterruptType::DMA1, 2=>crate::gba::interrupt::InterruptType::DMA2, _=>crate::gba::interrupt::InterruptType::DMA3 };
                                self.interrupt_controller.request_interrupt(it);
                            }
                        }
                    }
                }
            }
        }

        frame_ready
    }

    /// Detect EEPROM based on ROM content
    /// Based on gbatek documentation and JS implementation
    fn detect_eeprom(rom: &Rom) -> Option<EEPROM> {
        // Check for EEPROM identifier strings in ROM
        let rom_data = rom.data();
        
        // Look for EEPROM_V strings as mentioned in gbatek
        if let Some(_) = Self::find_string_in_rom(rom_data, b"EEPROM_V") {
            println!("[EEPROM] Detected EEPROM identifier in ROM");
            // Default to 512 bytes, can be auto-detected later based on DMA count
            Some(EEPROM::new(EEPROMSize::Size512))
        } else {
            // For testing purposes, create EEPROM for certain ROM sizes or patterns
            // This can be removed or made more sophisticated later
            if rom.size() > 0x1000000 { // 16MB+ ROMs often use EEPROM
                println!("[EEPROM] Large ROM detected, assuming EEPROM present");
                Some(EEPROM::new(EEPROMSize::Size8K))
            } else {
                None
            }
        }
    }

    /// Find a string pattern in ROM data
    fn find_string_in_rom(data: &[u8], pattern: &[u8]) -> Option<usize> {
        data.windows(pattern.len())
            .position(|window| window == pattern)
    }

    /// Get mutable reference to EEPROM (for save/load operations)
    pub fn get_eeprom_mut(&mut self) -> Option<&mut EEPROM> {
        self.eeprom.as_mut()
    }

    /// Get reference to EEPROM (for save/load operations)
    pub fn get_eeprom(&self) -> Option<&EEPROM> {
        self.eeprom.as_ref()
    }

    /// Get mutable reference to DMA controller
    pub fn get_dma_controller_mut(&mut self) -> &mut DMAController {
        &mut self.dma_controller
    }

    /// Get reference to DMA controller
    pub fn get_dma_controller(&self) -> &DMAController {
        &self.dma_controller
    }

    /// Handle EEPROM read with proper state management
    /// This should be called from a context where mutable access is available
    pub fn eeprom_read_with_state_update(&mut self, addr: u32) -> HalfWord {
        if let Some(ref mut eeprom) = self.eeprom {
            let dma3_enabled = self.dma_controller.get_channel(3)
                .map(|ch| ch.is_enabled())
                .unwrap_or(false);
            eeprom.read_halfword(addr, dma3_enabled)
        } else {
            1 // Return 1 when no EEPROM is present
        }
    }
}
