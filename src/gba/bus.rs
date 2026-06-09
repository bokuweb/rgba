use crate::cpu::bus;
use crate::io;
use crate::lcd;
use crate::types::*;

pub use bus::accessor::*;

use crate::memory::ram::Ram;
use crate::memory::readable::*;
use crate::memory::rom::Rom;
use crate::memory::writable::*;

use std::cell::Cell;

use super::apu::Apu;
use super::backup::Backup;
use super::dma::DMAController;
use super::eeprom::Eeprom;
use super::rtc::Rtc;

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
    fn default() -> Self {
        let mut table = Self {
            n32: [1; 0x10],
            s32: [1; 0x10],
            n16: [1; 0x10],
            s16: [1; 0x10],
        };
        table.n32[PAGE_EWRAM] = 6;
        table.s32[PAGE_EWRAM] = 6;
        table.n16[PAGE_EWRAM] = 3;
        table.s16[PAGE_EWRAM] = 3;

        // OAM sits on a 32-bit bus: 8/16/32-bit accesses are all 1 cycle.
        table.n32[PAGE_OAM] = 1;
        table.s32[PAGE_OAM] = 1;
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

        // With the GamePak prefetch unit enabled (WAITCNT bit 14), a *sequential*
        // opcode fetch from ROM is served from the prefetch buffer in a single
        // cycle — "as fast as internal RAM" (GBATEK) — regardless of the
        // configured second-access waitstate. A *non-sequential* access (the N
        // path, e.g. the fetch right after a branch flushes the buffer, or any
        // data access) still pays the full first-access waitstate. Modelling the
        // sequential cost as a flat 1 (rather than the previous `1 + 1`) removes a
        // 2x over-count on every sequential ROM fetch, which otherwise inflates
        // ROM-resident code by ~2x and desyncs cycle-timed raster effects.
        for i in 0..2 {
            self.n16[PAGE_GAMEPAK_WS0 + i] = 1 + S_GAMEPAK_NSEQ_CYCLES[ws0_first];
            self.s16[PAGE_GAMEPAK_WS0 + i] = if prefetch { 1 } else { 1 + S_GAMEPAK_WS0_SEQ_CYCLES[ws0_second] };

            self.n16[PAGE_GAMEPAK_WS1 + i] = 1 + S_GAMEPAK_NSEQ_CYCLES[ws1_first];
            self.s16[PAGE_GAMEPAK_WS1 + i] = if prefetch { 1 } else { 1 + S_GAMEPAK_WS1_SEQ_CYCLES[ws1_second] };

            self.n16[PAGE_GAMEPAK_WS2 + i] = 1 + S_GAMEPAK_NSEQ_CYCLES[ws2_first];
            self.s16[PAGE_GAMEPAK_WS2 + i] = if prefetch { 1 } else { 1 + S_GAMEPAK_WS2_SEQ_CYCLES[ws2_second] };

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
    cycle_lut: CycleLUT,
    lcdc: lcd::LCDController,
    dma: DMAController,
    bios: Rom,
    rom: Rom,
    wram: Ram,
    eram: Ram,
    vram: Ram,
    palette: Ram,
    oam: Ram,
    backup: Backup, // SRAM / Flash save memory (0x0E000000-0x0E00FFFF)
    eeprom: Option<Eeprom>, // serial EEPROM save (0x0D000000 region), when present
    rtc: Rtc, // GamePak GPIO + real-time clock (0x080000C4-C8)
    key: io::Key,
    keycnt: HalfWord,
    siocnt: HalfWord,
    /// Pending serial transfer completion: clear the SIOCNT start/busy bit and
    /// (if enabled) raise the SIO interrupt at the next servicing point.
    sio_irq_latch: bool,
    interrupt_controller: std::cell::RefCell<crate::interrupt::InterruptController>,
    timers: crate::gba::timer::Timers,
    /// Audio processing unit.
    apu: Apu,
    /// Master clock: absolute cycle count shared by all timed components.
    clock: u64,
    /// Set when the LCD finishes a frame during advance_clock.
    frame_ready: bool,
    waitcnt: HalfWord,
    open_bus_pc: Cell<Word>,
    open_bus_instruction_width: Cell<Word>,
    cpu_halted: Cell<bool>,
    /// Interrupt mask the CPU is blocked on via BIOS IntrWait/VBlankIntrWait
    /// (None when not waiting). Drives the re-check loop in the SWI dispatcher.
    intr_wait_mask: Cell<Option<HalfWord>>,
    prev_vblank: bool,
    prev_hblank: bool,
    prev_vcounter: bool,
    dma_irq_latch: [bool; 4],
    trace_dma: bool,
    /// mGBA/no$gba debug-logging interface (0x04FFF600..=0x04FFF7FF). Homebrew
    /// (e.g. Butano with logging enabled) writes a message into the 256-byte
    /// buffer then pokes 0x04FFF700 to flush it. Captured so these writes are
    /// valid (not "invalid write" warnings) and the log is observable.
    mgba_log_buffer: [u8; 256],
    mgba_log_enabled: bool,
}

impl BusAccessor for CpuBus {
    fn set_open_bus_context(&mut self, pc: Word, instruction_width: Word) {
        self.open_bus_pc.set(pc);
        self.open_bus_instruction_width.set(instruction_width);
    }
    fn set_cpu_halted(&mut self, halted: bool) {
        self.cpu_halted.set(halted);
    }
    fn is_cpu_halted(&self) -> bool {
        self.cpu_halted.get()
    }
    fn has_pending_interrupt_flags(&self) -> bool {
        self.interrupt_controller.borrow().read_if() != 0
    }
    fn read_bios_if(&self) -> HalfWord {
        self.interrupt_controller.borrow().read_bios_if_work()
    }
    fn clear_bios_if(&mut self, mask: HalfWord) {
        self.interrupt_controller.borrow_mut().clear_bios_if_work(mask);
    }
    fn set_intr_wait(&mut self, mask: HalfWord) {
        self.intr_wait_mask.set(Some(mask));
    }
    fn clear_intr_wait(&mut self) {
        self.intr_wait_mask.set(None);
    }
    fn intr_wait_mask(&self) -> Option<HalfWord> {
        self.intr_wait_mask.get()
    }

    fn read_byte(&self, addr: u32) -> Byte {
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_byte(addr),
            // EWRAM 256KB mirrors across 0x0200_0000-0x02FF_FFFF
            0x0200_0000..=0x02FF_FFFF => self.eram.read_byte((addr - 0x0200_0000) & 0x3FFFF),
            // BIOS IF work area mirror (0x03007FF8, mirrored in all IWRAM aliases)
            0x0300_0000..=0x03FF_FFFF if ((addr - 0x0300_0000) & 0x7FFF) == 0x7FF8 || ((addr - 0x0300_0000) & 0x7FFF) == 0x7FF9 => {
                let v = self.interrupt_controller.borrow().read_bios_if_work();
                if (addr & 1) == 0 { (v & 0x00FF) as u8 } else { (v >> 8) as u8 }
            }
            // IWRAM 32KB mirrors across 0x0300_0000-0x03FF_FFFF
            0x0300_0000..=0x03FF_FFFF => self.wram.read_byte((addr - 0x0300_0000) & 0x7FFF),
            0x0400_0000..=0x0400_005F => {
                // Byte access to LCD I/O: read the containing halfword and select the byte.
                let aligned = addr & !1;
                let hw = self.lcdc.read_halfword(aligned - 0x0400_0000);
                if (addr & 1) == 0 { (hw & 0x00FF) as u8 } else { (hw >> 8) as u8 }
            }
            0x0400_0130 => (self.key.read() & 0x00FF) as u8,
            0x0400_0131 => ((self.key.read() >> 8) & 0x00FF) as u8,
            0x0400_0132 => (self.keycnt & 0x00FF) as u8,
            0x0400_0133 => ((self.keycnt >> 8) & 0x00FF) as u8,
            0x0400_0100..=0x0400_010F => {
                let ofs = addr - 0x0400_0100 ;
                
                self.timers.read(ofs)
            }
            0x0400_0060..=0x0400_00A7 => self.apu.read_register(addr),
            0x0400_00A8..=0x0400_03FF => 0,
            // Palette 1KB mirrors
            0x0500_0000..=0x05FF_FFFF => self.palette.read_byte((addr - 0x0500_0000) & 0x3FF),
            // Fix(004): VRAM mirror (wraps at 0x20000; 0x18000-0x1FFFF maps to 0x10000-0x17FFF)
            0x0600_0000..=0x06FF_FFFF => self.vram.read_byte(Self::map_vram_offset(addr)),
            // OAM 1KB mirrors
            0x0700_0000..=0x07FF_FFFF => self.oam.read_byte((addr - 0x0700_0000) & 0x3FF),
            // GamePak ROM mirrors across WS0/WS1/WS2 (0x08000000-0x0DFFFFFF)
            0x0800_0000..=0x0DFF_FFFF => self.gamepak_read_byte(addr),
            0x0E00_0000..=0x0E00_FFFF => {
                // SRAM / Flash save memory (byte access only)
                self.backup.read(addr)
            }
            _ => {
                self.read_open_bus_byte(addr)
            }
        }
    }

    fn read_halfword(&self, addr: u32) -> HalfWord {
        // dbg!(format!("read half word addr = {:x}", addr));
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_halfword(addr),
            // mGBA debug enable register: reads back 0x1DEA once enabled so the
            // homebrew runtime detects logging support.
            0x04FF_F780 if self.mgba_log_enabled => 0x1DEA,
            // EWRAM mirrors
            0x0200_0000..=0x02FF_FFFF => self.eram.read_halfword((addr - 0x0200_0000) & 0x3FFFF),
            // BIOS IF work area mirror (0x03007FF8, mirrored in all IWRAM aliases)
            0x0300_0000..=0x03FF_FFFF if ((addr - 0x0300_0000) & 0x7FFF) == 0x7FF8 => self.interrupt_controller.borrow().read_bios_if_work(),
            // IWRAM mirrors
            0x0300_0000..=0x03FF_FFFF => self.wram.read_halfword((addr - 0x0300_0000) & 0x7FFF),
            0x0400_0000..=0x0400_005F => self.lcdc.read_halfword(addr - 0x0400_0000),
            0x0400_0130 => self.key.read(),
            0x0400_0132 => self.keycnt,
            0x0400_0200 => self.interrupt_controller.borrow().read_ie(), // IE register
            0x0400_0202 => self.interrupt_controller.borrow().read_if(), // IF register
            0x0400_0204 => self.waitcnt,
            0x0400_0208 => self.interrupt_controller.borrow().read_ime(), // IME register
            0x0400_0100..=0x0400_010F => {
                let lo = self.read_byte(addr) as u16;
                let hi = self.read_byte(addr + 1) as u16;
                (hi << 8) | lo
            }
            0x0400_0060..=0x0400_00A7 => {
                let lo = self.apu.read_register(addr) as u16;
                let hi = self.apu.read_register(addr + 1) as u16;
                (hi << 8) | lo
            }
            // 0x60..=0xA7 is handled by the APU arm above; this covers the rest.
            0x0400_00A8..=0x0400_03FF => {
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
                    // SIOCNT - Serial Communication Control
                    0x0400_0128 => self.siocnt,
                    _ => 0,
                }
            },
            0x0500_0000..=0x05FF_FFFF => self.palette.read_halfword((addr - 0x0500_0000) & 0x3FF),
            // Fix(004): VRAM mirror handling
            0x0600_0000..=0x06FF_FFFF => self.vram.read_halfword(Self::map_vram_offset(addr)),
            // Fix(006): OAM 1KB mirror (16-bit read)
            0x0700_0000..=0x07FF_FFFF => self.oam.read_halfword((addr - 0x0700_0000) & 0x3FF),
            // GamePak GPIO / RTC registers (only readable once the game enables
            // GPIO reads; otherwise these addresses read ROM).
            0x0800_00C4 | 0x0800_00C6 | 0x0800_00C8 if self.rtc.read_enabled() => self.rtc.read(addr),
            // Direct CPU reads of EEPROM only see the ready flag; real games
            // clock data out via DMA (see `perform_dma_transfer`).
            0x0D00_0000..=0x0DFF_FFFF if self.eeprom.is_some() => 1,
            0x0800_0000..=0x0DFF_FFFF => self.gamepak_read_halfword(addr),
            0x0E00_0000..=0x0E00_FFFF => {
                tracing::warn!("Invalid halfword access to SRAM at 0x{addr:08x} (SRAM is byte-access only) - returning 0xFFFF");
                0xFFFF
            }
            _ => {
                self.read_open_bus_halfword(addr)
            }
        }
    }

    fn read_word(&self, addr: u32) -> Word {
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_word(addr),
            0x0200_0000..=0x02FF_FFFF => self.eram.read_word((addr - 0x0200_0000) & 0x3FFFF),
            // BIOS IF work area mirror (0x03007FF8, mirrored in all IWRAM aliases).
            // Lower halfword is BIOS IF work, upper halfword remains regular IWRAM.
            0x0300_0000..=0x03FF_FFFF if ((addr - 0x0300_0000) & 0x7FFF) == 0x7FF8 => {
                let lo = self.interrupt_controller.borrow().read_bios_if_work() as u32;
                let hi = self.wram.read_halfword(0x7FFA) as u32;
                lo | (hi << 16)
            }
            0x0400_0100..=0x0400_010F => {
                // Combine bytes into a 32-bit value (TMxCNT_L/CNT_H).
                let base = addr - 0x0400_0100 ;
                let b0 = self.timers.read(base) as u32;
                let b1 = self.timers.read(base + 1) as u32;
                let b2 = self.timers.read(base + 2) as u32;
                let b3 = self.timers.read(base + 3) as u32;
                b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
            }
            // IE/IF as a 32-bit pair (low=IE, high=IF)
            0x0400_0200 => {
                let irq = self.interrupt_controller.borrow();
                (irq.read_ie() as u32) | ((irq.read_if() as u32) << 16)
            }
            // KEYINPUT/KEYCNT pair
            0x0400_0130 => (self.key.read() as u32) | ((self.keycnt as u32) << 16),
            0x0400_0204 => self.waitcnt as u32,
            0x0400_0208 => self.interrupt_controller.borrow().read_ime() as u32,
            0x0300_0000..=0x03FF_FFFF => {
                let off = (addr - 0x0300_0000) & 0x7FFF;
                self.wram.read_word(off)
            },
            // Fix(004): VRAM mirror (wraps at 0x20000; 0x18000.. maps to 0x10000..)
            0x0600_0000..=0x06FF_FFFF => {
                let vram_addr = Self::map_vram_offset(addr);
                self.vram.read_word(vram_addr)
            }
            // Fix(006): OAM 1KB mirror (32-bit read)
            0x0700_0000..=0x07FF_FFFF => self.oam.read_word((addr - 0x0700_0000) & 0x3FF),
            0x0400_0000..=0x0400_005F => self.lcdc.read_word(addr - 0x0400_0000),
            0x0400_0060..=0x0400_00A7 => {
                let b0 = self.apu.read_register(addr) as u32;
                let b1 = self.apu.read_register(addr + 1) as u32;
                let b2 = self.apu.read_register(addr + 2) as u32;
                let b3 = self.apu.read_register(addr + 3) as u32;
                b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
            }
            // 0x60..=0xA7 is handled by the APU arm above; this covers the rest.
            0x0400_00A8..=0x0400_03FF => {
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
            0x0800_0000..=0x0DFF_FFFF => self.gamepak_read_word(addr),
            0x0E00_0000..=0x0E00_FFFF => {
                tracing::warn!("Invalid word access to SRAM at 0x{addr:08x} (SRAM is byte-access only) - returning 0xFFFFFFFF");
                0xFFFF_FFFF
            }
            _ => {
                self.read_open_bus_word()
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
            // BIOS IF work area mirror (0x03007FF8, mirrored in all IWRAM aliases)
            0x0300_0000..=0x03FF_FFFF if ((addr - 0x0300_0000) & 0x7FFF) == 0x7FF8 || ((addr - 0x0300_0000) & 0x7FFF) == 0x7FF9 => {
                let mut irq = self.interrupt_controller.borrow_mut();
                let current = irq.read_bios_if_work();
                let next = if (addr & 1) == 0 {
                    (current & 0xFF00) | (data as u16)
                } else {
                    (current & 0x00FF) | ((data as u16) << 8)
                };
                irq.write_bios_if_work(next);
                self.wram.write_byte((addr - 0x0300_0000) & 0x7FFF, data);
            }
            0x0300_0000..=0x03FF_FFFF => {
                if addr == 0x0300_7dd9 {
                    // dbg!("write to 0x03007dd9", data);
                }
                self.wram.write_byte((addr - 0x0300_0000) & 0x7FFF, data);
            }
            0x0400_0000..=0x0400_005F => {
                // Byte access to LCD I/O: read-modify-write the containing halfword.
                let off = (addr & !1) - 0x0400_0000;
                let cur = self.lcdc.read_halfword(off);
                let new = if (addr & 1) == 0 {
                    (cur & 0xFF00) | (data as u16)
                } else {
                    (cur & 0x00FF) | ((data as u16) << 8)
                };
                self.lcdc.write_halfword(off, new);
            }
            0x0400_0132 => {
                // KEYCNT
                self.keycnt = (self.keycnt & 0xFF00) | (data as u16);
                self.check_keypad_interrupt();
            }
            0x0400_0133 => {
                // KEYCNT high byte
                self.keycnt = (self.keycnt & 0x00FF) | ((data as u16) << 8);
                self.check_keypad_interrupt();
            }
            0x0400_0100..=0x0400_010F => {
                let ofs = addr - 0x0400_0100 ;
                self.timers.write(ofs, data);
            }
            0x0400_0060..=0x0400_03FF => {
                // TODO: Implement DMA, Timer, and other I/O registers
                match addr {
                    // Sound registers + FIFO (byte access)
                    0x0400_0060..=0x0400_00A7 => self.apu.write_register(addr, data),
                    _ if (0x0400_00B0..=0x0400_00DE).contains(&addr) => {
                        let _ = data;
                    }
                    _ => {}
                }
            }
            // Palette: byte store behaves as halfword store replicated (0xVV -> 0xVVVV)
            0x0500_0000..=0x05FF_FFFF => {
                let off = ((addr - 0x0500_0000) & 0x3FF) & !1; // align to halfword
                let hw = data as HalfWord | ((data as HalfWord) << 8);
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
                        let hw = data as HalfWord | ((data as HalfWord) << 8);
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
                // SRAM / Flash save memory (byte access only)
                self.backup.write(addr, data);
            }
            0x04FF_F600..=0x04FF_F7FF => {
                self.mgba_debug_write(addr, data as u32, 1);
            }
            _ => {
                tracing::warn!("Invalid write_byte to 0x{addr:08x} = 0x{data:02x} (ignored)");
            }
        }
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
            // BIOS IF work area mirror (0x03007FF8, mirrored in all IWRAM aliases)
            0x0300_0000..=0x03FF_FFFF if ((addr - 0x0300_0000) & 0x7FFF) == 0x7FF8 => {
                self.interrupt_controller.borrow_mut().write_bios_if_work(data);
                self.wram.write_halfword(0x7FF8, data);
            }
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
                self.cycle_lut.update_waitcnt(data);
            }
            0x0400_0132 => {
                // KEYCNT
                self.keycnt = data;
                self.check_keypad_interrupt();
            }
            0x0400_0100..=0x0400_010F => {
                let lo = (data & 0x00FF) as u8;
                let hi = ((data >> 8) & 0x00FF) as u8;
                let base = addr - 0x0400_0100 ;
                self.timers.write(base, lo);
                self.timers.write(base + 1, hi);
            }
            0x0400_0060..=0x0400_03FF => {
                match addr {
                    // Sound registers + FIFO (16-bit access -> two bytes)
                    0x0400_0060..=0x0400_00A7 => {
                        self.apu.write_register(addr, (data & 0xFF) as u8);
                        self.apu.write_register(addr + 1, ((data >> 8) & 0xFF) as u8);
                    }
                    // SIOCNT - Serial Communication Control
                    0x0400_0128 => {
                        self.write_siocnt(data);
                    }
                    // DMAx registers (16-bit writes are common on GBA)
                    0x0400_00B0..=0x0400_00DE => {
                        let channel = ((addr - 0x0400_00B0) / 0x0C) as usize;
                        if channel < 4 {
                            let reg = (addr - (0x0400_00B0 + (channel as u32) * 0x0C)) as u16;
                            let ch = self.dma.channels[channel];
                            if self.trace_dma {
                                tracing::trace!(
                                    "DMA reg16 write ch={channel} reg=0x{reg:02x} addr=0x{addr:08x} data=0x{data:04x}"
                                );
                            }
                            match reg {
                                // DMAxSAD_L
                                0x00 => {
                                    let src = (ch.source & 0xFFFF_0000) | (data as u32);
                                    self.dma.write_source(channel, src);
                                }
                                // DMAxSAD_H
                                0x02 => {
                                    let src = ((data as u32) << 16) | (ch.source & 0x0000_FFFF);
                                    self.dma.write_source(channel, src & 0x0FFF_FFFF);
                                }
                                // DMAxDAD_L
                                0x04 => {
                                    let dst = (ch.destination & 0xFFFF_0000) | (data as u32);
                                    self.dma.write_destination(channel, dst);
                                }
                                // DMAxDAD_H
                                0x06 => {
                                    let dst = ((data as u32) << 16) | (ch.destination & 0x0000_FFFF);
                                    self.dma.write_destination(channel, dst & 0x0FFF_FFFF);
                                }
                                // DMAxCNT_L
                                0x08 => {
                                    let count_mask = if channel == 3 { 0xFFFF } else { 0x3FFF };
                                    self.dma.write_count(channel, data & count_mask);
                                }
                                // DMAxCNT_H
                                0x0A => {
                                    self.dma.write_control(channel, data);
                                    self.maybe_trigger_timed_dma_immediately(channel);
                                    self.execute_dma_transfers();
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            0x0500_0000..=0x05FF_FFFF => self.palette.write_halfword((addr - 0x0500_0000) & 0x3FF, data),
            0x0600_0000..=0x06FF_FFFF => {
                // Fix(004): mirror halfword writes too
                let vram_addr = Self::map_vram_offset(addr);
                self.vram.write_halfword(vram_addr, data);
            }
            // Fix(006): OAM 1KB mirror (16-bit write)
            0x0700_0000..=0x07FF_FFFF => self.oam.write_halfword((addr - 0x0700_0000) & 0x3FF, data),
            // GamePak GPIO / RTC registers.
            0x0800_00C4 | 0x0800_00C6 | 0x0800_00C8 => self.rtc.write(addr, data),
            // Direct CPU writes to EEPROM clock in one command bit (real games
            // use DMA, but handle this for completeness).
            0x0D00_0000..=0x0DFF_FFFF if self.eeprom.is_some() => {
                if let Some(e) = self.eeprom.as_mut() {
                    e.write_bit(data);
                }
            }
            0x0E00_0000..=0x0E00_FFFF => {
                tracing::warn!("Invalid halfword write to SRAM at 0x{addr:08x} = 0x{data:04x} (SRAM is byte-access only, ignored)");
            }
            0x04FF_F600..=0x04FF_F7FF => {
                self.mgba_debug_write(addr, data as u32, 2);
            }
            _ => {
                tracing::warn!("Invalid write_halfword to 0x{addr:08x} = 0x{data:04x} (ignored)");
            }
        }
    }

    fn write_word(&mut self, addr: u32, data: Word) {
        // dbg!(format!("write word addr 0x{:x} data = 0x{:x}", addr, data));
        if addr == 0x0300_0008 {
            // dbg!(data);
            // dbg!("h");
            // panic!();
        }
        match addr {
            0x0000_0000..=0x0007_FFFF => {
                // BIOS area is read-only; ignore writes.
                let _ = data;
            }
            0x0200_0000..=0x02FF_FFFF => self.eram.write_word((addr - 0x0200_0000) & 0x3FFFF, data),
            // BIOS IF work area mirror (0x03007FF8, mirrored in all IWRAM aliases)
            0x0300_0000..=0x03FF_FFFF if ((addr - 0x0300_0000) & 0x7FFF) == 0x7FF8 => {
                self.interrupt_controller.borrow_mut().write_bios_if_work((data & 0xFFFF) as HalfWord);
                self.wram.write_word(0x7FF8, data);
            }
            // WRAM
            0x0300_0000..=0x0300_7FFF => {
                // info!("wram addr = {:x} {:x}", addr, data);
                let off = addr - 0x0300_0000;
                self.wram.write_word(off, data);
            }
            // Unused
            0x0300_8000..=0x03FF_FFFF => {
                tracing::warn!("Write to unused area 0x{addr:08x} = 0x{data:08x} (ignored)");
            }
            0x0400_0000..=0x0400_005F => self.lcdc.write_word(addr - 0x0400_0000, data),
            0x0400_0100..=0x0400_010F => {
                // Split into 4 byte writes
                let base = addr - 0x0400_0100 ;
                self.timers.write(base, (data & 0x0000_00FF) as u8);
                self.timers.write(base + 1, ((data >> 8) & 0x0000_00FF) as u8);
                self.timers.write(base + 2, ((data >> 16) & 0x0000_00FF) as u8);
                self.timers.write(base + 3, ((data >> 24) & 0x0000_00FF) as u8);
            }
            0x0400_0130 => {
                // KEYINPUT (RO) low half ignored; KEYCNT is high halfword on word writes
                self.keycnt = ((data >> 16) & 0xFFFF) as HalfWord;
                self.check_keypad_interrupt();
            }
            0x0400_0200 => {
                // IE/IF as a 32-bit pair (low=IE write, high=IF ACK write)
                self.interrupt_controller.borrow_mut().write_ie((data & 0xFFFF) as HalfWord);
                self.interrupt_controller.borrow_mut().write_if((data >> 16) as HalfWord);
            }
            0x0400_0204 => {
                // WAITCNT is 16-bit at 0x04000204; word writes may target it
                let hw = (data & 0xFFFF) as HalfWord;
                self.waitcnt = hw;
                self.cycle_lut.update_waitcnt(hw);
            }
            0x0400_0208 => {
                self.interrupt_controller.borrow_mut().write_ime((data & 0xFFFF) as HalfWord);
            }
            0x0400_0060..=0x0400_03FF => {
                // DMA and other I/O registers
                match addr {
                    // Sound FIFO (32-bit feed) and other sound registers.
                    0x0400_00A0 => self.apu.push_fifo_a(data),
                    0x0400_00A4 => self.apu.push_fifo_b(data),
                    0x0400_0060..=0x0400_00A7 => {
                        self.apu.write_register(addr, (data & 0xFF) as u8);
                        self.apu.write_register(addr + 1, ((data >> 8) & 0xFF) as u8);
                        self.apu.write_register(addr + 2, ((data >> 16) & 0xFF) as u8);
                        self.apu.write_register(addr + 3, ((data >> 24) & 0xFF) as u8);
                    }
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
                        self.maybe_trigger_timed_dma_immediately(0);
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
                        self.maybe_trigger_timed_dma_immediately(1);
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
                        self.maybe_trigger_timed_dma_immediately(2);
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
                        self.maybe_trigger_timed_dma_immediately(3);
                        self.execute_dma_transfers();
                    }
                    
                    _ => {
                        // Other I/O registers (Timer, etc.)
                        if (0x0400_00B0..=0x0400_00DE).contains(&addr) {
                            let _ = data;
                        }
                    }
                }
            }
            0x0500_0000..=0x05FF_FFFF => self.palette.write_word((addr - 0x0500_0000) & 0x3FF, data),
            0x0600_0000..=0x06FF_FFFF => {
                // Fix(004): mirror the write side too
                let vram_addr = Self::map_vram_offset(addr);
                self.vram.write_word(vram_addr, data);
            }
            // Fix(006): OAM 1KB mirror (32-bit write)
            0x0700_0000..=0x07FF_FFFF => self.oam.write_word((addr - 0x0700_0000) & 0x3FF, data),
            0x0E00_0000..=0x0E00_FFFF => {
                tracing::warn!("Invalid word write to SRAM at 0x{addr:08x} = 0x{data:08x} (SRAM is byte-access only, ignored)");
            }
            0x04FF_F600..=0x04FF_F7FF => {
                self.mgba_debug_write(addr, data, 4);
            }
            _ => {
                tracing::warn!("Invalid write_word to 0x{addr:08x} = 0x{data:08x} (ignored)");
            }
        }
    }

    fn compute_cycle(&self, addr: Word, access_type: AccessType) -> Cycle {
        let page = (addr >> 24) as usize;
        if page > 0xF {
            return 1;
        }
        match access_type {
            AccessType::NonSeq(AccessWidth::Byte | AccessWidth::HalfWord) => self.cycle_lut.n16[page],
            AccessType::NonSeq(AccessWidth::Word) => self.cycle_lut.n32[page],
            AccessType::Seq(AccessWidth::Byte | AccessWidth::HalfWord) => self.cycle_lut.s16[page],
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
        backup: Backup, // SRAM / Flash save memory
        key: io::Key,
    ) -> Self {
        Self {
            cycle_lut: CycleLUT::new(),
            lcdc,
            dma: DMAController::new(),
            bios,
            rom,
            wram,
            eram,
            vram,
            palette,
            oam,
            backup,
            eeprom: None,
            rtc: Rtc::new(),
            key,
            keycnt: 0,
            siocnt: 0,
            sio_irq_latch: false,
            interrupt_controller: std::cell::RefCell::new(crate::interrupt::InterruptController::new()),
            timers: crate::gba::timer::Timers::default(),
            apu: Apu::new(),
            clock: 0,
            frame_ready: false,
            waitcnt: 0,
            open_bus_pc: Cell::new(0),
            open_bus_instruction_width: Cell::new(4),
            cpu_halted: Cell::new(false),
            intr_wait_mask: Cell::new(None),
            prev_vblank: false,
            prev_hblank: false,
            prev_vcounter: false,
            dma_irq_latch: [false; 4],
            trace_dma: std::env::var("AGB_TRACE_DMA").ok().as_deref() == Some("1"),
            mgba_log_buffer: [0; 256],
            mgba_log_enabled: false,
        }
    }

    /// Handle a write to the mGBA/no$gba debug-logging registers
    /// (0x04FFF600..=0x04FFF7FF). Returns `true` if the address belongs to that
    /// region (and was consumed), `false` otherwise.
    ///
    /// Layout: 0x04FFF600..=0x04FFF6FF is a 256-byte message buffer; a write to
    /// 0x04FFF700 flushes it as a log line (bits 0-2 = level); a write of 0xC0DE
    /// to 0x04FFF780 enables the interface (reads then return 0x1DEA).
    fn mgba_debug_write(&mut self, addr: u32, data: u32, width: u32) -> bool {
        match addr {
            0x04FF_F600..=0x04FF_F6FF => {
                let base = (addr - 0x04FF_F600) as usize;
                for i in 0..width as usize {
                    if let Some(slot) = self.mgba_log_buffer.get_mut(base + i) {
                        *slot = ((data >> (i * 8)) & 0xFF) as u8;
                    }
                }
                true
            }
            0x04FF_F700 => {
                // Flush: emit the buffer up to the first NUL as a log line.
                if self.mgba_log_enabled {
                    let end = self.mgba_log_buffer.iter().position(|&b| b == 0).unwrap_or(256);
                    let msg = String::from_utf8_lossy(&self.mgba_log_buffer[..end]);
                    let level = data & 0x7;
                    tracing::debug!("[mGBA log {level}] {msg}");
                }
                self.mgba_log_buffer = [0; 256];
                true
            }
            0x04FF_F780 => {
                // Enable register: 0xC0DE turns the interface on.
                self.mgba_log_enabled = (data & 0xFFFF) == 0xC0DE;
                true
            }
            0x04FF_F701..=0x04FF_F7FF => true, // reserved control bytes; accept silently
            _ => false,
        }
    }

    pub(crate) fn update_key(&mut self, key: io::Key) {
        self.key = key;
        self.check_keypad_interrupt();
    }

    /// Raw backup (save) memory bytes, for persistence to a `.sav` file.
    pub(crate) fn backup_bytes(&self) -> &[u8] {
        self.backup.bytes()
    }

    /// Whether the backup memory changed since the last `backup_clear_dirty`.
    pub(crate) const fn backup_is_dirty(&self) -> bool {
        self.backup.is_dirty()
    }

    pub(crate) const fn backup_clear_dirty(&mut self) {
        self.backup.clear_dirty();
    }

    /// Attach a serial EEPROM device (for carts whose save type is EEPROM).
    pub(crate) fn attach_eeprom(&mut self, eeprom: Eeprom) {
        self.eeprom = Some(eeprom);
    }

    /// EEPROM save bytes for persistence (empty when no EEPROM is present).
    pub(crate) fn eeprom_bytes(&self) -> &[u8] {
        self.eeprom.as_ref().map_or(&[], Eeprom::bytes)
    }

    /// Whether the EEPROM changed since the last `eeprom_clear_dirty`.
    pub(crate) fn eeprom_is_dirty(&self) -> bool {
        self.eeprom.as_ref().is_some_and(Eeprom::is_dirty)
    }

    pub(crate) fn eeprom_clear_dirty(&mut self) {
        if let Some(e) = self.eeprom.as_mut() {
            e.clear_dirty();
        }
    }

    /// Evaluate the KEYCNT keypad interrupt condition against the current key
    /// state and raise a Keypad interrupt when it is satisfied.
    ///
    /// KEYCNT (0x04000132): bit14 = IRQ enable, bit15 = condition (0=logical OR
    /// / any selected key, 1=logical AND / all selected keys), bits 0-9 = key
    /// select mask. KEYINPUT uses 0=pressed, so `pressed = !KEYINPUT & 0x3FF`.
    pub(crate) fn check_keypad_interrupt(&mut self) {
        if (self.keycnt & 0x4000) == 0 {
            return; // IRQ disabled
        }
        let mask = self.keycnt & 0x03FF;
        let pressed = (!self.key.read()) & 0x03FF;
        let triggered = if (self.keycnt & 0x8000) != 0 {
            // AND: all selected keys pressed
            (pressed & mask) == mask
        } else {
            // OR: any selected key pressed
            (pressed & mask) != 0
        };
        if triggered {
            self.interrupt_controller
                .borrow_mut()
                .request_interrupt(crate::interrupt::InterruptType::Keypad);
        }
    }

    /// Handle a write to SIOCNT (0x04000128). We don't emulate an actual serial
    /// peripheral, but starting a transfer with the internal shift clock must
    /// eventually complete and (if enabled) raise the SIO interrupt. The
    /// start/busy bit (bit 7) stays set until the transfer "completes" on the
    /// next servicing point, matching software that polls it.
    const fn write_siocnt(&mut self, data: HalfWord) {
        let starting = (data & 0x0080) != 0 && (self.siocnt & 0x0080) == 0;
        self.siocnt = data;
        // Internal shift clock (bit 0) drives the transfer on this side.
        if starting && (data & 0x0001) != 0 {
            self.sio_irq_latch = true;
        }
    }
    
    pub(crate) fn request_vblank_interrupt(&mut self) {
        self.interrupt_controller.borrow_mut().request_interrupt(crate::interrupt::InterruptType::VBlank);
    }
    
    pub(crate) fn should_service_interrupt(&self) -> bool {
        self.interrupt_controller.borrow().should_service_interrupt()
    }

    fn maybe_trigger_timed_dma_immediately(&mut self, channel: usize) {
        if channel >= 4 || !self.dma.channels[channel].enabled {
            return;
        }
        let timing = self.dma.channels[channel].get_timing();
        if timing == 0 {
            return;
        }
        let dispstat = self.lcdc.read_halfword(0x0004);
        let vcount = self.lcdc.read_halfword(0x0006) as usize;
        let in_vblank = (dispstat & 0x0001) != 0;
        let in_hblank = (dispstat & 0x0002) != 0;
        match timing {
            1 if in_vblank => self.dma.trigger_timing_event(channel),
            2 if in_hblank && vcount < 160 => self.dma.trigger_timing_event(channel),
            _ => {}
        }
    }

    pub(crate) fn execute_dma_transfers(&mut self) {
        // Complete any pending serial transfer: clear the start/busy bit and, if
        // the SIO interrupt is enabled (SIOCNT bit 14), raise it.
        if self.sio_irq_latch {
            self.sio_irq_latch = false;
            self.siocnt &= !0x0080;
            if (self.siocnt & 0x4000) != 0 {
                self.interrupt_controller
                    .borrow_mut()
                    .request_interrupt(crate::interrupt::InterruptType::Serial);
            }
        }

        // Raise DMA IRQs with a small delay (next servicing point), not in the same
        // transfer completion moment. This better matches software expectations around IntrWait.
        for channel in 0..4 {
            if self.dma_irq_latch[channel] {
                self.dma_irq_latch[channel] = false;
                let irq = match channel {
                    0 => crate::interrupt::InterruptType::DMA0,
                    1 => crate::interrupt::InterruptType::DMA1,
                    2 => crate::interrupt::InterruptType::DMA2,
                    3 => crate::interrupt::InterruptType::DMA3,
                    _ => continue,
                };
                self.interrupt_controller.borrow_mut().request_interrupt(irq);
            }
        }

        self.poll_timed_dma_triggers();

        // Check for pending DMA transfers and execute them
        for channel in 0..4 {
            self.run_dma_channel(channel);
        }
    }

    fn poll_timed_dma_triggers(&mut self) {
        // Trigger timed DMAs on blanking edge transitions.
        let dispstat = self.lcdc.read_halfword(0x0004);
        let vcount = self.lcdc.read_halfword(0x0006);
        let vblank = (dispstat & 0x0001) != 0;
        let hblank = (dispstat & 0x0002) != 0;
        let vcounter = (dispstat & 0x0004) != 0;
        let vblank_rising = !self.prev_vblank && vblank;
        let hblank_rising = !self.prev_hblank && hblank;
        let vcounter_rising = !self.prev_vcounter && vcounter;

        if hblank_rising && (dispstat & 0x0010) != 0 {
            self.interrupt_controller.borrow_mut().request_interrupt(crate::interrupt::InterruptType::HBlank);
        }
        if vcounter_rising && (dispstat & 0x0020) != 0 {
            self.interrupt_controller.borrow_mut().request_interrupt(crate::interrupt::InterruptType::VCounter);
        }

        if vblank_rising || hblank_rising {
            let mut has_timed_enabled = false;
            for channel in 0..4 {
                if self.dma.channels[channel].enabled {
                    let t = self.dma.channels[channel].get_timing();
                    if t == 1 || t == 2 || t == 3 {
                        has_timed_enabled = true;
                        break;
                    }
                }
            }
            if self.trace_dma && has_timed_enabled {
                let vcount = self.lcdc.read_halfword(0x0006);
                tracing::trace!(
                    "DMA timing edge vblank_rising={vblank_rising} hblank_rising={hblank_rising} vcount={vcount}"
                );
            }
            for channel in 0..4 {
                if !self.dma.channels[channel].enabled {
                    continue;
                }
                if self.trace_dma && has_timed_enabled {
                    tracing::trace!(
                        "DMA ch{} enabled timing={} pending={}",
                        channel,
                        self.dma.channels[channel].get_timing(),
                        self.dma.channels[channel].pending
                    );
                }
                match self.dma.channels[channel].get_timing() {
                    1 if vblank_rising => self.dma.trigger_timing_event(channel), // VBlank
                    // HBlank DMA fires only during the visible lines (0..159);
                    // it does NOT occur during VBlank.
                    2 if hblank_rising && (vcount as usize) < 160 => self.dma.trigger_timing_event(channel),
                    // Video-capture DMA (DMA3 "special" timing): fires each HBlank
                    // for lines 2..=161, then stops at line 162.
                    3 if channel == 3 && hblank_rising && (2..162).contains(&(vcount as usize)) => {
                        self.dma.trigger_timing_event(channel);
                    }
                    _ => {}
                }
            }
        }

        // Video-capture DMA (DMA3, special timing) is automatically disabled once
        // VCOUNT reaches line 162.
        if hblank_rising && vcount as usize == 162 && self.dma.channels[3].enabled && self.dma.channels[3].get_timing() == 3 {
            self.dma.channels[3].enabled = false;
            self.dma.channels[3].control &= !0x8000;
        }

        self.prev_vblank = vblank;
        self.prev_hblank = hblank;
        self.prev_vcounter = vcounter;
    }

    fn run_dma_channel(&mut self, channel: usize) {
        let request_irq = self.dma.channels[channel].handle_irq();
        if let Some((source, dest, count, transfer_size)) = self.dma.get_pending_transfer(channel) {
            if self.trace_dma {
                tracing::trace!(
                    "DMA start ch={} timing={} src={:08x} dst={:08x} count={} size={} ctrl={:04x}",
                    channel,
                    self.dma.channels[channel].get_timing(),
                    source,
                    dest,
                    count,
                    transfer_size,
                    self.dma.channels[channel].control
                );
            }
            self.perform_dma_transfer(channel, source, dest, count, transfer_size);
            self.dma.complete_transfer(channel);
            if request_irq {
                self.dma_irq_latch[channel] = true;
                if self.trace_dma {
                    tracing::trace!("DMA irq latched ch={channel}");
                }
            }
        }
    }

    pub(crate) fn tick_timers(&mut self, now_cycles: u64) {
        let irq = &mut *self.interrupt_controller.borrow_mut();
        self.timers.tick(now_cycles, irq);
    }

    /// Advance the shared master clock by `cycles` and drive every timed
    /// Drain queued audio samples (interleaved L/R `i16`) for host playback.
    pub(crate) fn take_audio(&mut self) -> Vec<i16> {
        self.apu.take_samples()
    }

    /// Top up a DirectSound FIFO with one 4-word burst from the DMA channel
    /// (1 or 2) programmed to feed `fifo_addr` with special (FIFO) timing.
    fn fifo_dma_refill(&mut self, fifo_addr: u32) {
        for c in 1..=2usize {
            let (enabled, timing, dest, src_ctrl, mut src) = {
                let ch = &self.dma.channels[c];
                (ch.enabled, ch.get_timing(), ch.destination, ch.get_source_control(), ch.next_source)
            };
            if !(enabled && timing == 3 && (dest & 0x0FFF_FFFF) == fifo_addr) {
                continue;
            }
            for _ in 0..4 {
                let word = self.read_word_internal(src & !3);
                if fifo_addr == 0x0400_00A0 {
                    self.apu.push_fifo_a(word);
                } else {
                    self.apu.push_fifo_b(word);
                }
                match src_ctrl {
                    1 => src = src.wrapping_sub(4), // decrement
                    2 => {}                          // fixed
                    _ => src = src.wrapping_add(4), // increment / reload
                }
            }
            self.dma.channels[c].next_source = src;
            if self.dma.channels[c].handle_irq() {
                let kind = if c == 1 {
                    crate::interrupt::InterruptType::DMA1
                } else {
                    crate::interrupt::InterruptType::DMA2
                };
                self.interrupt_controller.borrow_mut().request_interrupt(kind);
            }
            return;
        }
    }

    /// component (timers and the LCD) by the same delta. This is the single
    /// entry point through which time passes, so DMA (which calls it per
    /// transfer) keeps the timers/LCD in lockstep with the CPU.
    pub(crate) fn advance_clock(&mut self, cycles: Cycle) {
        if cycles == 0 {
            return;
        }
        self.clock = self.clock.wrapping_add(cycles as u64);
        let overflows = {
            let irq = &mut *self.interrupt_controller.borrow_mut();
            self.timers.tick(self.clock, irq)
        };
        // DirectSound: timer 0/1 overflows clock the FIFOs; when a FIFO drops
        // below half, top it up from its associated DMA channel.
        for t in 0..2 {
            if overflows[t] > 0 {
                let req = self.apu.on_timer_overflow(t, overflows[t]);
                if req.fifo_a {
                    self.fifo_dma_refill(0x0400_00A0);
                }
                if req.fifo_b {
                    self.fifo_dma_refill(0x0400_00A4);
                }
            }
        }
        self.apu.tick(cycles as u32);
        let (ready, vblank_irq) = self.lcdc.run(cycles);
        // Render any visible scanlines the LCD just entered, using the current
        // register/memory state (scanline-accurate raster).
        let pending = self.lcdc.take_pending_scanlines();
        for y in pending {
            self.lcdc.render_scanline(y, &self.vram, &self.palette, &self.oam);
        }
        if vblank_irq {
            self.request_vblank_interrupt();
        }
        if ready {
            self.frame_ready = true;
        }
    }

    /// Borrow the LCD framebuffer (scanline-accumulated RGBA8).
    pub(crate) fn framebuffer(&self) -> &[u8] {
        self.lcdc.framebuffer()
    }

    /// Returns and clears the "LCD finished a frame" flag.
    pub(crate) const fn take_frame_ready(&mut self) -> bool {
        let r = self.frame_ready;
        self.frame_ready = false;
        r
    }

    // VRAM mirror mapping helper
    // GBA VRAM is 96KB (0x18000) within a 128KB window (0x20000).
    // 0x06000000-0x06FFFFFF should wrap every 0x20000, and offsets >= 0x18000
    // mirror the last 32KB (map to 0x10000-0x17FFF).
    const fn map_vram_offset(addr: u32) -> u32 {
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

    /// Decide the access type of one DMA unit (index `i`) at `addr`.
    ///
    /// DMA units use sequential timing, except in GamePak ROM
    /// (0x08000000-0x0DFFFFFF): when a unit's access ends exactly on a 0x20000
    /// page boundary, the *following* access is forced non-sequential — even in
    /// fixed/decrement addressing mode (see DMA/readme). Because it affects the
    /// following access, the first unit (i == 0) is never upgraded.
    fn dma_access_type(addr: Word, width: AccessWidth, unit: u32, i: usize) -> AccessType {
        let is_rom = (0x0800_0000..=0x0DFF_FFFF).contains(&addr);
        let rom_boundary = is_rom && ((addr & 0x1_FFFF) + unit) >= 0x2_0000;
        if i > 0 && rom_boundary {
            AccessType::NonSeq(width)
        } else {
            AccessType::Seq(width)
        }
    }

    // GamePak reads beyond the end of the cartridge return an "open bus" value:
    // the lower 16 bits of (address / 2) for each addressed halfword. (gba-tests
    // unsafe t002)
    #[inline]
    fn gamepak_read_byte(&self, addr: Word) -> Byte {
        let off = Self::map_gamepak_offset(addr) as usize;
        if off < self.rom.len() {
            self.rom.read_byte(off as u32)
        } else {
            let hw = (addr >> 1) & 0xFFFF;
            if (addr & 1) != 0 { (hw >> 8) as u8 } else { (hw & 0xFF) as u8 }
        }
    }
    #[inline]
    fn gamepak_read_halfword(&self, addr: Word) -> HalfWord {
        let off = Self::map_gamepak_offset(addr) as usize;
        if off < self.rom.len() {
            self.rom.read_halfword(off as u32)
        } else {
            ((addr >> 1) & 0xFFFF) as u16
        }
    }
    #[inline]
    fn gamepak_read_word(&self, addr: Word) -> Word {
        let off = Self::map_gamepak_offset(addr) as usize;
        if off < self.rom.len() {
            self.rom.read_word(off as u32)
        } else {
            let lo = (addr >> 1) & 0xFFFF;
            let hi = ((addr.wrapping_add(2)) >> 1) & 0xFFFF;
            lo | (hi << 16)
        }
    }

    fn read_open_bus_byte(&self, addr: Word) -> Byte {
        let pc = self.open_bus_pc.get();
        let width = self.open_bus_instruction_width.get();
        let base = pc.wrapping_sub(width);
        let src = base.wrapping_add(addr & 0x3);
        self.read_mapped_byte_or_ff(src)
    }

    fn read_open_bus_halfword(&self, addr: Word) -> HalfWord {
        let pc = self.open_bus_pc.get();
        let width = self.open_bus_instruction_width.get();
        let base = pc.wrapping_sub(width);
        let src = base.wrapping_add(addr & 0x2);
        let lo = self.read_mapped_byte_or_ff(src) as u16;
        let hi = self.read_mapped_byte_or_ff(src.wrapping_add(1)) as u16;
        lo | (hi << 8)
    }

    fn read_open_bus_word(&self) -> Word {
        let pc = self.open_bus_pc.get();
        let width = self.open_bus_instruction_width.get();
        let base = pc.wrapping_sub(width);

        if width == 4 {
            let b0 = self.read_mapped_byte_or_ff(base) as u32;
            let b1 = self.read_mapped_byte_or_ff(base.wrapping_add(1)) as u32;
            let b2 = self.read_mapped_byte_or_ff(base.wrapping_add(2)) as u32;
            let b3 = self.read_mapped_byte_or_ff(base.wrapping_add(3)) as u32;
            b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
        } else {
            let lo = self.read_mapped_byte_or_ff(base) as u16;
            let hi = self.read_mapped_byte_or_ff(base.wrapping_add(1)) as u16;
            let hw = lo | (hi << 8);
            (hw as u32) | ((hw as u32) << 16)
        }
    }

    fn read_mapped_byte_or_ff(&self, addr: Word) -> Byte {
        match addr {
            0x0000_0000..=0x0000_3FFF => self.bios.read_byte(addr),
            0x0200_0000..=0x02FF_FFFF => self.eram.read_byte((addr - 0x0200_0000) & 0x3FFFF),
            0x0300_0000..=0x03FF_FFFF => self.wram.read_byte((addr - 0x0300_0000) & 0x7FFF),
            0x0500_0000..=0x05FF_FFFF => self.palette.read_byte((addr - 0x0500_0000) & 0x3FF),
            0x0600_0000..=0x06FF_FFFF => self.vram.read_byte(Self::map_vram_offset(addr)),
            0x0700_0000..=0x07FF_FFFF => self.oam.read_byte((addr - 0x0700_0000) & 0x3FF),
            0x0800_0000..=0x0DFF_FFFF => self.gamepak_read_byte(addr),
            0x0E00_0000..=0x0E00_FFFF => self.backup.read(addr),
            _ => 0xFF,
        }
    }

    fn perform_dma_transfer(&mut self, channel: usize, mut source: Word, mut dest: Word, count: usize, transfer_size: usize) {
        if transfer_size == 4 {
            source &= !3;
            dest &= !3;
        } else {
            source &= !1;
            dest &= !1;
        }

        let src_region = source & 0xFF00_0000;
        let dst_region = dest & 0xFF00_0000;
        let mut src_off = (source & 0x00FF_FFFF) as i32;
        let mut dst_off = (dest & 0x00FF_FFFF) as i32;
        let mut first_value: Option<u32> = None;
        let mut last_value: Option<u32> = None;

        let width = if transfer_size == 4 { AccessWidth::Word } else { AccessWidth::HalfWord };
        let unit = transfer_size as u32;

        // This transfer can be preempted by a higher-priority channel (lower
        // index) only if such a channel is enabled with an HBlank trigger — the
        // case real software relies on to interleave a short repeating HBlank
        // DMA with a long transfer. Restricting to HBlank keeps the common case
        // (and FIFO/VBlank/special-timed channels) on the original exact path.
        let preemptible = (0..channel).any(|c| {
            self.dma.channels[c].enabled && self.dma.channels[c].get_timing() == 2
        });

        // Serial EEPROM (0x0D region) is driven by DMA: prime the device with the
        // transfer length so it can detect its size and frame the command. A
        // write targets EEPROM (dest), a read sources from it (src).
        let eeprom_write = dst_region == GAMEPAK_WS2_HI && self.eeprom.is_some();
        let eeprom_read = src_region == GAMEPAK_WS2_HI && self.eeprom.is_some();
        if let Some(e) = self.eeprom.as_mut() {
            if eeprom_write {
                e.prepare(true, count);
            } else if eeprom_read {
                e.prepare(false, count);
            }
        }

        for i in 0..count {
            let source_addr = src_region | ((src_off as u32) & 0x00FF_FFFF);
            let dest_addr = dst_region | ((dst_off as u32) & 0x00FF_FFFF);

            // A DMA unit normally uses sequential access timing. In GamePak ROM,
            // an access whose unit ends exactly on a 0x20000 page boundary is
            // forced non-sequential — even in fixed/decrement mode — but this
            // only upgrades the *following* accesses, so the first unit is left
            // sequential (see DMA/readme).
            let src_at = Self::dma_access_type(source_addr, width, unit, i);
            let dst_at = Self::dma_access_type(dest_addr, width, unit, i);
            // Advance the shared clock *before* reading so that a fixed source
            // pointing at a running timer/counter is sampled at the correct,
            // advancing value. This is what makes the cycle-timed DMA tests
            // observe the right per-transfer deltas.
            let cost = self.compute_cycle(source_addr, src_at) + self.compute_cycle(dest_addr, dst_at);
            self.advance_clock(cost);

            if transfer_size == 4 {
                // 32-bit transfer
                let data = self.read_word_internal(source_addr);
                self.write_word_internal(dest_addr, data);
                if first_value.is_none() {
                    first_value = Some(data);
                }
                last_value = Some(data);
            } else {
                // 16-bit transfer (the only width EEPROM uses).
                let data = if eeprom_read {
                    self.eeprom.as_mut().map_or(1, Eeprom::read_bit)
                } else {
                    self.read_halfword_internal(source_addr)
                };
                if eeprom_write {
                    if let Some(e) = self.eeprom.as_mut() {
                        e.write_bit(data);
                    }
                } else {
                    self.write_halfword_internal(dest_addr, data);
                }
                let data32 = data as u32;
                if first_value.is_none() {
                    first_value = Some(data32);
                }
                last_value = Some(data32);
            }

            // Update addresses based on control settings
            let src_control = self.dma.channels[channel].get_source_control();
            let dest_control = self.dma.channels[channel].get_dest_control();
            let step = transfer_size as i32;

            match src_control {
                0 => src_off = src_off.wrapping_add(step), // Increment
                1 => src_off = src_off.wrapping_sub(step), // Decrement
                2 => {} // Fixed
                // Prohibited in docs, but many implementations treat it as increment.
                3 => src_off = src_off.wrapping_add(step),
                _ => {}
            }

            match dest_control {
                0 => dst_off = dst_off.wrapping_add(step), // Increment
                1 => dst_off = dst_off.wrapping_sub(step), // Decrement
                2 => {} // Fixed
                3 => dst_off = dst_off.wrapping_add(step), // Increment/Reload
                _ => {}
            }

            // DMA priority/preemption (checked at unit boundaries): time has
            // passed, so a higher-priority channel (lower index) may now be
            // pending via its HBlank/VBlank trigger. Run it to completion before
            // continuing this transfer, matching hardware bus arbitration.
            if preemptible {
                self.poll_timed_dma_triggers();
                for higher in 0..channel {
                    self.run_dma_channel(higher);
                }
            }
        }

        // Update internal DMA progress state; public DMA registers retain the programmed values.
        self.dma.channels[channel].next_source = src_region | (src_off as u32);
        self.dma.channels[channel].next_destination = dst_region | (dst_off as u32);
        self.dma.channels[channel].next_count = 0;

        if self.trace_dma && channel == 0 {
            tracing::trace!(
                "DMA done ch=0 first=0x{:08x} last=0x{:08x} next_src=0x{:08x} next_dst=0x{:08x} pub_src=0x{:08x} pub_dst=0x{:08x} pub_count={} en={}",
                first_value.unwrap_or(0),
                last_value.unwrap_or(0),
                self.dma.channels[channel].next_source,
                self.dma.channels[channel].next_destination,
                self.dma.channels[channel].source,
                self.dma.channels[channel].destination,
                self.dma.channels[channel].count,
                self.dma.channels[channel].enabled
            );
        }
    }

    // Internal memory access methods that bypass DMA triggering
    fn read_word_internal(&self, addr: Word) -> Word {
        match addr {
            0x0800_0000..=0x0DFF_FFFF => self.gamepak_read_word(addr),
            0x0300_0000..=0x0300_7FFF => self.wram.read_word(addr - 0x0300_0000),
            0x0200_0000..=0x0203_FFFF => self.eram.read_word(addr - 0x0200_0000),
            0x0600_0000..=0x06FF_FFFF => self.vram.read_word(Self::map_vram_offset(addr)),
            // Allow DMA reads from I/O (e.g. timers/FIFOs) and other mapped ranges.
            _ => self.read_word(addr),
        }
    }

    fn read_halfword_internal(&self, addr: Word) -> HalfWord {
        match addr {
            0x0800_0000..=0x0DFF_FFFF => self.gamepak_read_halfword(addr),
            0x0300_0000..=0x0300_7FFF => self.wram.read_halfword(addr - 0x0300_0000),
            0x0200_0000..=0x0203_FFFF => self.eram.read_halfword(addr - 0x0200_0000),
            0x0600_0000..=0x06FF_FFFF => self.vram.read_halfword(Self::map_vram_offset(addr)),
            // Allow DMA reads from I/O (e.g. timers/FIFOs) and other mapped ranges.
            _ => self.read_halfword(addr),
        }
    }

    fn write_word_internal(&mut self, addr: Word, data: Word) {
        match addr {
            0x0400_00A0 => self.apu.push_fifo_a(data),
            0x0400_00A4 => self.apu.push_fifo_b(data),
            // GamePak ROM/EEPROM area: writes are typically ignored by ROM,
            // and EEPROM uses serial protocol (not modeled here yet).
            0x0800_0000..=0x0DFF_FFFF => {
                let _ = data;
            }
            0x0600_0000..=0x06FF_FFFF => {
                let vram_addr = Self::map_vram_offset(addr);
                self.vram.write_word(vram_addr, data);
            }
            0x0500_0000..=0x0500_03FF => {
                self.palette.write_word(addr - 0x0500_0000, data);
            }
            0x0300_0000..=0x0300_7FFF => self.wram.write_word(addr - 0x0300_0000, data),
            0x0200_0000..=0x02FF_FFFF => self.eram.write_word((addr - 0x0200_0000) & 0x3FFFF, data),
            0x0700_0000..=0x07FF_FFFF => self.oam.write_word((addr - 0x0700_0000) & 0x3FF, data),
            // I/O registers (DISPCNT, BLDCNT, sound, etc.). A DMA can legitimately
            // target memory-mapped registers (e.g. games update DISPCNT via DMA);
            // route these through the normal register write path instead of
            // silently dropping them.
            0x0400_0000..=0x0400_03FF => self.write_word(addr, data),
            // BIOS / unmapped low memory (0x0-0x01FFFFFF) is read-only on
            // hardware; a DMA targeting it (e.g. an audio engine's first-frame
            // FIFO restart, before its source/dest pointers are programmed)
            // is silently dropped rather than warned about.
            0x0000_0000..=0x01FF_FFFF => {
                let _ = data;
            }
            _ => {
                tracing::warn!("DMA write_word to unsupported address: 0x{addr:08x} = 0x{data:08x}");
            }
        }
    }

    fn write_halfword_internal(&mut self, addr: Word, data: HalfWord) {
        match addr {
            0x0400_00A0 => self.apu.push_fifo_a(data as u32),
            0x0400_00A4 => self.apu.push_fifo_b(data as u32),
            // GamePak ROM/EEPROM area: treat as no-op for now.
            0x0800_0000..=0x0DFF_FFFF => {
                let _ = data;
            }

            0x0600_0000..=0x06FF_FFFF => {
                let vram_addr = Self::map_vram_offset(addr);
                self.vram.write_halfword(vram_addr, data);
            }
            0x0500_0000..=0x0500_03FF => {
                self.palette.write_halfword(addr - 0x0500_0000, data);
            }
            0x0300_0000..=0x0300_7FFF => self.wram.write_halfword(addr - 0x0300_0000, data),
            0x0200_0000..=0x0203_FFFF => self.eram.write_halfword(addr - 0x0200_0000, data),
            0x0700_0000..=0x07FF_FFFF => self.oam.write_halfword((addr - 0x0700_0000) & 0x3FF, data),
            // I/O registers (DISPCNT, BLDCNT, sound, etc.). A DMA can legitimately
            // target memory-mapped registers (e.g. games update DISPCNT via DMA);
            // route these through the normal register write path instead of
            // silently dropping them.
            0x0400_0000..=0x0400_03FF => self.write_halfword(addr, data),
            // BIOS / unmapped low memory is read-only on hardware; drop silently.
            0x0000_0000..=0x01FF_FFFF => {
                let _ = data;
            }
            _ => {
                tracing::warn!("DMA write_halfword to unsupported address: 0x{addr:08x} = 0x{data:04x}");
            }
        }
    }

    pub(crate) const fn borrow_mut_lcdc(&mut self) -> &mut lcd::LCDController {
        &mut self.lcdc
    }

    pub(crate) const fn borrow_lcdc(&self) -> &lcd::LCDController {
        &self.lcdc
    }

    pub(crate) const fn borrow_vram(&self) -> &Ram {
        &self.vram
    }

    pub(crate) const fn borrow_palette(&self) -> &Ram {
        &self.palette
    }

    pub(crate) const fn borrow_oam(&self) -> &Ram {
        &self.oam
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gba::SaveKind;
    use crate::interrupt::InterruptType;

    fn new_bus() -> CpuBus {
        let bios = Rom::new(0x4000, &[0u8; 0x4000][..]);
        let rom = Rom::new(0x80000, &vec![0u8; 0x80000]);
        let wram = Ram::new(vec![0; 0x8000]);
        let eram = Ram::new(vec![0; 0x4_0000]);
        let vram = Ram::new(vec![0; 0x1_8000]);
        let palette = Ram::new(vec![0; 0x0400]);
        let oam = Ram::new(vec![0; 0x0400]);
        let backup = Backup::new(SaveKind::Sram, &[]);
        let lcdc = lcd::LCDController::new();
        let key = io::Key::new();
        CpuBus::new(bios, lcdc, rom, wram, eram, vram, palette, oam, backup, key)
    }

    fn if_flags(bus: &CpuBus) -> u16 {
        bus.interrupt_controller.borrow().read_if()
    }

    /// Build a bus whose GamePak ROM is filled with `insn`, run an ARM core from
    /// 0x0800_0000 with the given WAITCNT, and return the steady-state cycle cost
    /// of one instruction. A DP instruction does no data access, so this measures
    /// the 32-bit sequential ROM *fetch* cost (s32) under that WAITCNT.
    fn rom_fetch_cycles(insn: u32, waitcnt: u16) -> crate::types::Cycle {
        let mut prog = vec![0u8; 0x1_0000];
        for i in (0..prog.len()).step_by(4) {
            prog[i..i + 4].copy_from_slice(&insn.to_le_bytes());
        }
        let bios = Rom::new(0x4000, &[0u8; 0x4000][..]);
        let rom = Rom::new(prog.len(), &prog);
        let wram = Ram::new(vec![0; 0x8000]);
        let eram = Ram::new(vec![0; 0x4_0000]);
        let vram = Ram::new(vec![0; 0x1_8000]);
        let palette = Ram::new(vec![0; 0x0400]);
        let oam = Ram::new(vec![0; 0x0400]);
        let backup = Backup::new(SaveKind::Sram, &[]);
        let lcdc = lcd::LCDController::new();
        let key = io::Key::new();
        let mut bus = CpuBus::new(bios, lcdc, rom, wram, eram, vram, palette, oam, backup, key);
        bus.write_halfword(0x0400_0204, waitcnt);
        let mut arm = crate::cpu::cpu::ARM::new();
        arm.reset();
        arm.set_gpr(15, 0x0800_0000);
        let mut last = 0;
        for _ in 0..16 {
            last = arm.step(&mut bus, false).unwrap();
        }
        last
    }

    #[test]
    fn rom_sequential_fetch_honours_prefetch() {
        // Steady-state cost of a DP instruction fetched sequentially from GamePak
        // ROM (no data access), i.e. the 32-bit sequential ROM fetch cost (s32).
        const MOV_R0_R0: u32 = 0xE1A0_0000;

        // Prefetch ON (WAITCNT bit 14): GBATEK — sequential ROM reads are served
        // by the prefetch buffer at 1 cycle/halfword, so a 32-bit fetch = 2.
        // (Regression guard: this was 4 — a 2x over-count — which inflated all
        // ROM-resident code and desynced Mother 3's cycle-timed raster effects.)
        assert_eq!(rom_fetch_cycles(MOV_R0_R0, 0x4000), 2, "prefetch on: s32 = 1 cycle/halfword");

        // Prefetch OFF, WS0 defaults (second-access = 2 waitstates): s16 = 1+2,
        // so a 32-bit sequential fetch = 2*(1+2) = 6.
        assert_eq!(rom_fetch_cycles(MOV_R0_R0, 0x0000), 6, "prefetch off: s32 = 2*(1+S)");
    }

    #[test]
    fn apu_emits_audio_through_bus() {
        // Drive a square channel entirely through the bus register interface,
        // then advance the clock and confirm the bus->APU path yields audio.
        let mut bus = new_bus();
        bus.write_byte(0x0400_0084, 0x80); // SOUNDCNT_X: master enable
        bus.write_byte(0x0400_0080, 0x77); // SOUNDCNT_L: full L/R master volume
        bus.write_byte(0x0400_0081, 0x11); // ch1 enabled on both sides
        bus.write_byte(0x0400_0062, 0x80); // ch1 duty
        bus.write_byte(0x0400_0063, 0xF0); // ch1 envelope: volume 15, DAC on
        bus.write_byte(0x0400_0064, 0x00); // ch1 frequency low
        bus.write_byte(0x0400_0065, 0x87); // ch1 frequency high + trigger
        bus.advance_clock(300_000);
        let audio = bus.take_audio();
        assert!(!audio.is_empty(), "no samples produced");
        assert!(audio.iter().any(|&s| s != 0), "bus -> APU produced only silence");
    }

    #[test]
    fn keypad_irq_or_condition_fires_when_any_selected_key_pressed() {
        let mut bus = new_bus();
        // KEYCNT: IRQ enable (bit14), OR condition (bit15=0), select START (bit3).
        bus.write_halfword(0x0400_0132, 0x4000 | 0x0008);
        assert_eq!(if_flags(&bus) & (1 << 12), 0, "no key pressed yet");

        // Press START (KEYINPUT 0=pressed).
        let mut key = io::Key::new();
        key.set_START(io::KeyStatus::ON);
        bus.update_key(key);
        assert_ne!(if_flags(&bus) & (1 << 12), 0, "keypad IRQ should fire");
    }

    #[test]
    fn keypad_irq_and_condition_requires_all_selected_keys() {
        let mut bus = new_bus();
        // AND condition (bit15=1), IRQ enable, select A (bit0) and B (bit1).
        bus.write_halfword(0x0400_0132, 0x4000 | 0x8000 | 0x0001 | 0x0002);

        // Only A pressed: AND not satisfied.
        let mut key = io::Key::new();
        key.set_A(io::KeyStatus::ON);
        bus.update_key(key);
        assert_eq!(if_flags(&bus) & (1 << 12), 0, "AND needs all keys");

        // A and B pressed: AND satisfied.
        key.set_B(io::KeyStatus::ON);
        bus.update_key(key);
        assert_ne!(if_flags(&bus) & (1 << 12), 0, "AND satisfied");
    }

    #[test]
    fn keypad_irq_not_raised_when_disabled() {
        let mut bus = new_bus();
        // IRQ enable bit clear.
        bus.write_halfword(0x0400_0132, 0x0008);
        let mut key = io::Key::new();
        key.set_START(io::KeyStatus::ON);
        bus.update_key(key);
        assert_eq!(if_flags(&bus) & (1 << 12), 0, "keypad IRQ disabled");
    }

    #[test]
    fn sio_internal_transfer_raises_serial_irq_after_servicing() {
        let mut bus = new_bus();
        // SIOCNT: internal shift clock (bit0), IRQ enable (bit14), start (bit7).
        bus.write_halfword(0x0400_0128, 0x4000 | 0x0001 | 0x0080);
        // Busy bit stays set until completion (software polls it).
        assert_ne!(bus.read_halfword(0x0400_0128) & 0x0080, 0, "busy bit set right after start");
        assert_eq!(if_flags(&bus) & (1 << 7), 0, "IRQ not raised yet");

        // Completion happens at the next servicing point.
        bus.execute_dma_transfers();
        assert_eq!(bus.read_halfword(0x0400_0128) & 0x0080, 0, "busy bit cleared on completion");
        assert_ne!(if_flags(&bus) & (1 << 7), 0, "serial IRQ raised");
    }

    #[test]
    fn sio_without_irq_enable_does_not_raise() {
        let mut bus = new_bus();
        // Internal clock + start, but no IRQ enable.
        bus.write_halfword(0x0400_0128, 0x0001 | 0x0080);
        bus.execute_dma_transfers();
        assert_eq!(if_flags(&bus) & (1 << 7), 0, "no serial IRQ without IRQ enable");
    }

    #[test]
    fn lcd_io_byte_access_round_trips() {
        let mut bus = new_bus();
        // Byte writes to DISPCNT (0x04000000) must not panic and must compose
        // into the halfword (regression test for the byte-access panic).
        bus.write_byte(0x0400_0000, 0x12);
        bus.write_byte(0x0400_0001, 0x03);
        assert_eq!(bus.read_halfword(0x0400_0000), 0x0312);
        assert_eq!(bus.read_byte(0x0400_0000), 0x12);
        assert_eq!(bus.read_byte(0x0400_0001), 0x03);
    }

    #[test]
    fn request_vblank_interrupt_sets_if_flag() {
        let mut bus = new_bus();
        bus.request_vblank_interrupt();
        assert_ne!(if_flags(&bus) & (1 << 0), 0, "VBlank IF bit set");
        assert_eq!(if_flags(&bus) & !(1 << 0), 0, "only VBlank bit set");
        // request_interrupt path used here must not pre-fill the BIOS work area.
        assert_eq!(bus.interrupt_controller.borrow().read_bios_if_work(), 0);
        let _ = InterruptType::VBlank; // keep import used across cfgs
    }

    #[test]
    fn sram_mirrors_every_32kb() {
        let mut bus = new_bus();
        // SRAM is 32KB and mirrors within the 64KB save region. (gba-tests unsafe t001)
        bus.write_byte(0x0E00_0000, 0xA5);
        assert_eq!(bus.read_byte(0x0E00_8000), 0xA5, "0x8000 mirrors 0x0000");
        bus.write_byte(0x0E00_7FFF, 0x3C);
        assert_eq!(bus.read_byte(0x0E00_FFFF), 0x3C, "0xFFFF mirrors 0x7FFF");
    }

    #[test]
    fn advance_clock_ticks_timers() {
        let mut bus = new_bus();
        bus.write_halfword(0x0400_0100, 0); // TM0 reload = 0
        bus.write_halfword(0x0400_0102, 0x0080); // TM0 enable, prescaler /1
        let before = bus.read_halfword(0x0400_0100);
        bus.advance_clock(100);
        let after = bus.read_halfword(0x0400_0100);
        assert!(after > before, "timer must advance with the master clock: {} -> {}", before, after);
    }

    #[test]
    fn dma_rom_boundary_forces_nonsequential() {
        use crate::types::{AccessType, AccessWidth};
        // First unit is sequential even at a boundary-adjacent ROM address.
        assert!(matches!(
            CpuBus::dma_access_type(0x0801_FFFC, AccessWidth::Word, 4, 0),
            AccessType::Seq(_)
        ));
        // Later units at a ROM 0x20000-boundary-adjacent address are non-seq.
        assert!(matches!(
            CpuBus::dma_access_type(0x0801_FFFC, AccessWidth::Word, 4, 1),
            AccessType::NonSeq(_)
        ));
        // A non-boundary ROM access stays sequential.
        assert!(matches!(
            CpuBus::dma_access_type(0x0800_0000, AccessWidth::Word, 4, 1),
            AccessType::Seq(_)
        ));
        // Non-ROM memory never gets the boundary upgrade.
        assert!(matches!(
            CpuBus::dma_access_type(0x0201_FFFC, AccessWidth::Word, 4, 1),
            AccessType::Seq(_)
        ));
    }

    #[test]
    fn dma_from_running_timer_samples_increasing_values() {
        // A DMA whose fixed source is a running timer must observe the timer
        // advancing during the transfer (cycle-accurate DMA). (jsmolka DMA16 /
        // AGS MEMORY DMA tests.)
        let mut bus = new_bus();
        bus.write_halfword(0x0400_0100, 0); // TM0 reload = 0
        bus.write_halfword(0x0400_0102, 0x0080); // TM0 enable, prescaler /1
        // DMA3: src = TM0CNT (fixed), dst = EWRAM, 16-bit, 8 units.
        bus.write_halfword(0x0400_00D4, 0x0100); // SAD lo
        bus.write_halfword(0x0400_00D6, 0x0400); // SAD hi -> 0x04000100
        bus.write_halfword(0x0400_00D8, 0x0000); // DAD lo
        bus.write_halfword(0x0400_00DA, 0x0200); // DAD hi -> 0x02000000
        bus.write_halfword(0x0400_00DC, 8); // count
        // enable (bit15) + source fixed (bits 7-8 = 0b10), 16-bit -> triggers.
        bus.write_halfword(0x0400_00DE, 0x8000 | 0x0100);

        let vals: Vec<u16> = (0..8).map(|i| bus.read_halfword(0x0200_0000 + i * 2)).collect();
        let delta = vals[2].wrapping_sub(vals[1]);
        assert!(delta > 0, "timer must advance during DMA: {:?}", vals);
        // The per-transfer delta is constant in steady state.
        for i in 2..7 {
            assert_eq!(
                vals[i + 1].wrapping_sub(vals[i]),
                delta,
                "constant per-transfer delta expected: {vals:?}"
            );
        }
    }

    #[test]
    fn hblank_dma_preempts_a_long_lower_priority_transfer() {
        // A higher-priority HBlank DMA (ch0) must interleave into a long
        // immediate DMA running on a lower-priority channel (ch1). (AGS DMA
        // priority test.)
        let mut bus = new_bus();
        bus.write_halfword(0x0400_0100, 0); // TM0 reload = 0
        bus.write_halfword(0x0400_0102, 0x0080); // TM0 enable, /1

        // ch0: src=TM0 (fixed), dst=0x02010000, 16-bit, HBlank timing, 8 units.
        bus.write_halfword(0x0400_00B0, 0x0100);
        bus.write_halfword(0x0400_00B2, 0x0400);
        bus.write_halfword(0x0400_00B4, 0x0000);
        bus.write_halfword(0x0400_00B6, 0x0201);
        bus.write_halfword(0x0400_00B8, 8);
        // enable | HBlank timing (bits 12-13 = 10) | source fixed (bits 7-8 = 10)
        bus.write_halfword(0x0400_00BA, 0x8000 | 0x2000 | 0x0100);

        // ch1: src=TM0 (fixed), dst=0x02000000, 16-bit, immediate, 512 units
        // (long enough to span an HBlank so ch0 preempts).
        bus.write_halfword(0x0400_00BC, 0x0100);
        bus.write_halfword(0x0400_00BE, 0x0400);
        bus.write_halfword(0x0400_00C0, 0x0000);
        bus.write_halfword(0x0400_00C2, 0x0200);
        bus.write_halfword(0x0400_00C4, 512);
        bus.write_halfword(0x0400_00C6, 0x8000 | 0x0100); // triggers the long DMA

        // ch0 must have run during ch1: its destination holds captured timer
        // values (non-zero and increasing).
        let hi: Vec<u16> = (0..8).map(|i| bus.read_halfword(0x0201_0000 + i * 2)).collect();
        assert!(hi[0] != 0, "HBlank DMA did not preempt the long transfer: {:?}", hi);
        assert!(hi[7] > hi[0], "preempted DMA values should increase: {:?}", hi);
    }

    #[test]
    fn gamepak_out_of_bounds_reads_open_bus() {
        // The default test ROM is 0x80000 bytes; reads past the cart return the
        // lower 16 bits of (address / 2) per halfword. (gba-tests unsafe t002)
        let bus = new_bus();
        let addr = 0x0800_0000 + 0x0008_0000; // first byte past the cart
        let hw = bus.read_halfword(addr);
        assert_eq!(hw, ((addr >> 1) & 0xFFFF) as u16);
        // Byte reads select the appropriate half of (addr/2).
        assert_eq!(bus.read_byte(addr), ((addr >> 1) & 0xFF) as u8);
        assert_eq!(bus.read_byte(addr + 1), ((((addr + 1) >> 1) >> 8) as u8));
        // Word reads combine two consecutive open-bus halfwords.
        let lo = (addr >> 1) & 0xFFFF;
        let hi = ((addr + 2) >> 1) & 0xFFFF;
        assert_eq!(bus.read_word(addr), lo | (hi << 16));
    }
}
