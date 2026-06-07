use crate::memory::ram::Ram;
use crate::memory::readable::*;
use crate::types::*;

use super::constants::*;
use super::*;

/// A composited sprite pixel: its colour, BG-relative priority and whether it
/// is a semi-transparent (alpha-blended) OBJ.
#[derive(Clone, Copy)]
struct ObjPixel {
    color: BGR,
    priority: u16,
    semi: bool,
}

#[derive(Debug, PartialEq, Clone, Copy)]
struct BGR(HalfWord);

impl BGR {
    fn new(c: HalfWord) -> Self {
        BGR(c)
    }

    fn blue(&self) -> Byte {
        (self.0.wrapping_shr(10) as Byte).wrapping_shl(3)
    }

    fn green(&self) -> Byte {
        ((self.0.wrapping_shr(5) & 0x1F) as Byte).wrapping_shl(3)
    }

    fn red(&self) -> Byte {
        (self.0 & 0x1F).wrapping_shl(3) as Byte
    }
}

pub struct LCDController {
    cycles: usize,
    lines: usize,
    // registers
    dispcnt: DISPCNT,
    dispstat: DISPSTAT,
    bg0cnt: BGCNT,
    bg1cnt: BGCNT,
    bg2cnt: BGCNT,
    bg3cnt: BGCNT,
    // BG scroll registers
    bg0hofs: HalfWord, // BG0 X-Offset
    bg0vofs: HalfWord, // BG0 Y-Offset
    bg1hofs: HalfWord, // BG1 X-Offset
    bg1vofs: HalfWord, // BG1 Y-Offset
    bg2hofs: HalfWord, // BG2 X-Offset
    bg2vofs: HalfWord, // BG2 Y-Offset
    bg3hofs: HalfWord, // BG3 X-Offset
    bg3vofs: HalfWord, // BG3 Y-Offset
    // Mosaic register
    mosaic: HalfWord, // MOSAIC Size register (0x400004C)
    // BG2/BG3 reference point registers (32-bit)
    bg2x: Word, // BG2 Reference Point X-Coordinate (0x4000028-0x400002A)
    bg2y: Word, // BG2 Reference Point Y-Coordinate (0x400002C-0x400002E)
    // BG2 rotation/scaling parameters (16-bit each)
    bg2pa: HalfWord, // BG2 Rotation/Scaling Parameter A (dx) (0x4000020)
    bg2pb: HalfWord, // BG2 Rotation/Scaling Parameter B (dmx) (0x4000022)
    bg2pc: HalfWord, // BG2 Rotation/Scaling Parameter C (dy) (0x4000024)
    bg2pd: HalfWord, // BG2 Rotation/Scaling Parameter D (dmy) (0x4000026)
    // BG3 reference point registers (32-bit)
    bg3x: Word, // BG3 Reference Point X-Coordinate (0x4000038-0x400003A)
    bg3y: Word, // BG3 Reference Point Y-Coordinate (0x400003C-0x400003E)
    // BG3 rotation/scaling parameters (16-bit each)
    bg3pa: HalfWord, // BG3 Rotation/Scaling Parameter A (dx) (0x4000030)
    bg3pb: HalfWord, // BG3 Rotation/Scaling Parameter B (dmx) (0x4000032)
    bg3pc: HalfWord, // BG3 Rotation/Scaling Parameter C (dy) (0x4000034)
    bg3pd: HalfWord, // BG3 Rotation/Scaling Parameter D (dmy) (0x4000036)
    // Window registers
    win0h: HalfWord,  // WIN0H - Window 0 Horizontal Dimensions (0x4000040)
    win1h: HalfWord,  // WIN1H - Window 1 Horizontal Dimensions (0x4000042)
    win0v: HalfWord,  // WIN0V - Window 0 Vertical Dimensions (0x4000044)
    win1v: HalfWord,  // WIN1V - Window 1 Vertical Dimensions (0x4000046)
    winin: HalfWord,  // WININ - Control of Inside of Window(s) (0x4000048)
    winout: HalfWord, // WINOUT - Control of Outside of Windows & Inside of OBJ Window (0x400004A)
    // Color special effects registers
    bldcnt: HalfWord,   // BLDCNT - Color Special Effects Selection (0x4000050)
    bldalpha: HalfWord, // BLDALPHA - Alpha Blending Coefficients (0x4000052)
    bldy: HalfWord,     // BLDY - Brightness (Fade-In/Out) Coefficient (0x4000054)
}

impl LCDController {
    #[inline]
    fn trace_lcd() -> bool {
        std::env::var("AGB_TRACE_LCD").ok().as_deref() == Some("1")
    }

    pub fn new() -> LCDController {
        LCDController {
            cycles: 0,
            lines: 0,
            dispcnt: DISPCNT::new(),
            dispstat: DISPSTAT::new(),
            bg0cnt: BGCNT::new(),
            bg1cnt: BGCNT::new(),
            bg2cnt: BGCNT::new(),
            bg3cnt: BGCNT::new(),
            // Initialize BG scroll registers
            bg0hofs: 0,
            bg0vofs: 0,
            bg1hofs: 0,
            bg1vofs: 0,
            bg2hofs: 0,
            bg2vofs: 0,
            bg3hofs: 0,
            bg3vofs: 0,
            // Initialize mosaic register
            mosaic: 0,
            // Initialize BG2/BG3 reference point registers
            bg2x: 0,
            bg2y: 0,
            // Initialize BG2 rotation/scaling parameters
            bg2pa: 0x0100, // Default scaling factor (1.0 in 8.8 fixed point)
            bg2pb: 0,
            bg2pc: 0,
            bg2pd: 0x0100, // Default scaling factor (1.0 in 8.8 fixed point)
            // Initialize BG3 reference point registers
            bg3x: 0,
            bg3y: 0,
            // Initialize BG3 rotation/scaling parameters
            bg3pa: 0x0100, // Default scaling factor (1.0 in 8.8 fixed point)
            bg3pb: 0,
            bg3pc: 0,
            bg3pd: 0x0100, // Default scaling factor (1.0 in 8.8 fixed point)
            // Initialize window registers
            win0h: 0,    // WIN0H - Window 0 Horizontal Dimensions
            win1h: 0,    // WIN1H - Window 1 Horizontal Dimensions
            win0v: 0,    // WIN0V - Window 0 Vertical Dimensions
            win1v: 0,    // WIN1V - Window 1 Vertical Dimensions
            winin: 0,    // WININ - Control of Inside of Window(s)
            winout: 0,   // WINOUT - Control of Outside of Windows & Inside of OBJ Window
            bldcnt: 0,   // BLDCNT - Color Special Effects Selection
            bldalpha: 0, // BLDALPHA - Alpha Blending Coefficients
            bldy: 0,     // BLDY - Brightness (Fade-In/Out) Coefficient
        }
    }

    pub fn get_bg_mode(&self) -> BgMode { self.dispcnt.mode() }

    pub fn run(&mut self, cycles: usize) -> (bool, bool) {
        self.cycles += cycles;
        let mut vblank_irq_requested = false;

        loop {
            if self.cycles < CYCLES_PER_LINE {
                return (false, vblank_irq_requested);
            }
            self.cycles -= CYCLES_PER_LINE;
            let prev_lines = self.lines;
            self.lines += 1;

            // Check for VBlank transition (entering VBlank period)
            if prev_lines == 159 && self.lines == 160 {
                if self.dispstat.vblank_irq_enable() {
                    vblank_irq_requested = true;
                }
            }

            if self.lines >= LINES_PER_FRAME {
                self.lines -= LINES_PER_FRAME;
                return (true, vblank_irq_requested);
            }
        }
    }

    pub fn read_halfword(&self, addr: Word) -> HalfWord {
        match addr {
            0x0000 => self.dispcnt.read(),
            0x0004 => {
                self.dispstat.read(self.cycles, self.lines)
            },
            0x0006 => self.lines as HalfWord,
            0x0008 => self.bg0cnt.read(),
            0x000A => self.bg1cnt.read(),
            0x000C => self.bg2cnt.read(),
            0x000E => self.bg3cnt.read(),
            0x0020 => self.bg2pa,    // BG2PA - BG2 Rotation/Scaling Parameter A (dx)
            0x0022 => self.bg2pb,    // BG2PB - BG2 Rotation/Scaling Parameter B (dmx)
            0x0024 => self.bg2pc,    // BG2PC - BG2 Rotation/Scaling Parameter C (dy)
            0x0026 => self.bg2pd,    // BG2PD - BG2 Rotation/Scaling Parameter D (dmy)
            0x0030 => self.bg3pa,    // BG3PA - BG3 Rotation/Scaling Parameter A (dx)
            0x0032 => self.bg3pb,    // BG3PB - BG3 Rotation/Scaling Parameter B (dmx)
            0x0034 => self.bg3pc,    // BG3PC - BG3 Rotation/Scaling Parameter C (dy)
            0x0036 => self.bg3pd,    // BG3PD - BG3 Rotation/Scaling Parameter D (dmy)
            0x0040 => self.win0h,    // WIN0H - Window 0 Horizontal Dimensions
            0x0042 => self.win1h,    // WIN1H - Window 1 Horizontal Dimensions
            0x0044 => self.win0v,    // WIN0V - Window 0 Vertical Dimensions
            0x0046 => self.win1v,    // WIN1V - Window 1 Vertical Dimensions
            0x0048 => self.winin,    // WININ - Control of Inside of Window(s)
            0x004A => self.winout,   // WINOUT - Control of Outside of Windows & Inside of OBJ Window
            0x0050 => self.bldcnt,   // BLDCNT - Color Special Effects Selection
            0x0052 => self.bldalpha, // BLDALPHA - Alpha Blending Coefficients
            0x0054 => 0,             // BLDY - Brightness Coefficient (Write Only, reads as 0)
            // Write-only registers (BG scroll, BG2/BG3 reference points, MOSAIC, etc.)
            // and any unmapped LCD I/O read back as 0 rather than panicking.
            _ => 0,
        }
    }

    pub fn read_word(&self, addr: Word) -> Word {
        match addr {
            0x0000 => self.dispcnt.read() as Word,
            0x0004 => {
                self.dispstat.read(self.cycles, self.lines) as Word
            },
            0x0006 => self.lines as Word,

            0x0008 => self.bg0cnt.read() as Word,
            0x000A => self.bg1cnt.read() as Word,
            0x000C => self.bg2cnt.read() as Word,
            0x000E => self.bg3cnt.read() as Word,
            0x0020 => self.bg2pa as Word,    // BG2PA - BG2 Rotation/Scaling Parameter A (dx)
            0x0022 => self.bg2pb as Word,    // BG2PB - BG2 Rotation/Scaling Parameter B (dmx)
            0x0024 => self.bg2pc as Word,    // BG2PC - BG2 Rotation/Scaling Parameter C (dy)
            0x0026 => self.bg2pd as Word,    // BG2PD - BG2 Rotation/Scaling Parameter D (dmy)
            0x0028 => self.bg2x,             // BG2X - BG2 Reference Point X-Coordinate
            0x002C => self.bg2y,             // BG2Y - BG2 Reference Point Y-Coordinate
            0x0030 => self.bg3pa as Word,    // BG3PA - BG3 Rotation/Scaling Parameter A (dx)
            0x0032 => self.bg3pb as Word,    // BG3PB - BG3 Rotation/Scaling Parameter B (dmx)
            0x0034 => self.bg3pc as Word,    // BG3PC - BG3 Rotation/Scaling Parameter C (dy)
            0x0036 => self.bg3pd as Word,    // BG3PD - BG3 Rotation/Scaling Parameter D (dmy)
            0x0038 => self.bg3x,             // BG3X - BG3 Reference Point X-Coordinate
            0x003C => self.bg3y,             // BG3Y - BG3 Reference Point Y-Coordinate
            0x0040 => self.win0h as Word,    // WIN0H - Window 0 Horizontal Dimensions
            0x0042 => self.win1h as Word,    // WIN1H - Window 1 Horizontal Dimensions
            0x0044 => self.win0v as Word,    // WIN0V - Window 0 Vertical Dimensions
            0x0046 => self.win1v as Word,    // WIN1V - Window 1 Vertical Dimensions
            0x0048 => self.winin as Word,    // WININ - Control of Inside of Window(s)
            0x004A => self.winout as Word,   // WINOUT - Control of Outside of Windows & Inside of OBJ Window
            0x0050 => self.bldcnt as Word,   // BLDCNT - Color Special Effects Selection
            0x0052 => self.bldalpha as Word, // BLDALPHA - Alpha Blending Coefficients
            0x0054 => 0,                     // BLDY - Brightness Coefficient (Write Only, reads as 0)
            _ => todo!(),
        }
    }

    pub fn write_halfword(&mut self, addr: Word, data: HalfWord) {
        match addr {
            0x0000 => {
                self.dispcnt.write(data);
                if Self::trace_lcd() {
                    println!(
                        "🔧 DISPCNT write: 0x{:04x} (Mode: {}, BG0: {}, BG1: {}, BG2: {}, BG3: {}, OBJ: {})",
                        data,
                        self.dispcnt.mode() as u8,
                        (data & 0x0100) != 0,
                        (data & 0x0200) != 0,
                        (data & 0x0400) != 0,
                        (data & 0x0800) != 0,
                        (data & 0x1000) != 0
                    );
                }
            }
            0x0004 => {
                // DISPSTAT - General LCD Status (Read/Write)
                self.dispstat.write(data);
                if Self::trace_lcd() {
                    println!(
                        "🔧 DISPSTAT write: 0x{:04x} -> masked: 0x{:04x} (VBlank IRQ: {}, HBlank IRQ: {}, VCounter IRQ: {})",
                        data,
                        self.dispstat.0 & 0x0038,
                        (data & 0x0008) != 0,
                        (data & 0x0010) != 0,
                        (data & 0x0020) != 0
                    );
                }
            }
            0x0008 => {
                self.bg0cnt.write(data);
                if Self::trace_lcd() {
                    println!(
                        "🔧 BG0CNT write: 0x{:04x} (Priority: {}, CharBase: {}, MapBase: {}, Colors: {}, Size: {})",
                        data,
                        self.bg0cnt.bg_priority(),
                        self.bg0cnt.character_base_block(),
                        self.bg0cnt.screen_base_block(),
                        if self.bg0cnt.colors_palettes() { "256/1" } else { "16/16" },
                        self.bg0cnt.screen_size()
                    );
                }
            }
            0x000A => {
                self.bg1cnt.write(data);
                if Self::trace_lcd() {
                    println!(
                        "🔧 BG1CNT write: 0x{:04x} (Priority: {}, CharBase: {}, MapBase: {}, Colors: {}, Size: {})",
                        data,
                        self.bg1cnt.bg_priority(),
                        self.bg1cnt.character_base_block(),
                        self.bg1cnt.screen_base_block(),
                        if self.bg1cnt.colors_palettes() { "256/1" } else { "16/16" },
                        self.bg1cnt.screen_size()
                    );
                }
            }
            0x000C => {
                self.bg2cnt.write(data);
                if Self::trace_lcd() {
                    println!(
                        "🔧 BG2CNT write: 0x{:04x} (Priority: {}, CharBase: {}, MapBase: {}, Colors: {}, Size: {})",
                        data,
                        self.bg2cnt.bg_priority(),
                        self.bg2cnt.character_base_block(),
                        self.bg2cnt.screen_base_block(),
                        if self.bg2cnt.colors_palettes() { "256/1" } else { "16/16" },
                        self.bg2cnt.screen_size()
                    );
                }
            }
            0x000E => {
                self.bg3cnt.write(data);
                if Self::trace_lcd() {
                    println!(
                        "🔧 BG3CNT write: 0x{:04x} (Priority: {}, CharBase: {}, MapBase: {}, Colors: {}, Size: {})",
                        data,
                        self.bg3cnt.bg_priority(),
                        self.bg3cnt.character_base_block(),
                        self.bg3cnt.screen_base_block(),
                        if self.bg3cnt.colors_palettes() { "256/1" } else { "16/16" },
                        self.bg3cnt.screen_size()
                    );
                }
            }
            0x0010 => {
                // BG0HOFS - BG0 X-Offset (Write Only)
                self.bg0hofs = data & 0x01FF; // 9-bit mask
                                              // println!("BG0HOFS write: 0x{:04x}", self.bg0hofs);
            }
            0x0012 => {
                // BG0VOFS - BG0 Y-Offset (Write Only)
                self.bg0vofs = data & 0x01FF; // 9-bit mask
                                              // println!("BG0VOFS write: 0x{:04x}", self.bg0vofs);
            }
            0x0014 => {
                // BG1HOFS - BG1 X-Offset (Write Only)
                self.bg1hofs = data & 0x01FF;
            }
            0x0016 => {
                // BG1VOFS - BG1 Y-Offset (Write Only)
                self.bg1vofs = data & 0x01FF;
            }
            0x0018 => {
                // BG2HOFS - BG2 X-Offset (Write Only)
                self.bg2hofs = data & 0x01FF;
            }
            0x001A => {
                // BG2VOFS - BG2 Y-Offset (Write Only)
                self.bg2vofs = data & 0x01FF;
            }
            0x001C => {
                // BG3HOFS - BG3 X-Offset (Write Only)
                self.bg3hofs = data & 0x01FF;
            }
            0x001E => {
                // BG3VOFS - BG3 Y-Offset (Write Only)
                self.bg3vofs = data & 0x01FF;
            }
            0x0020 => {
                // BG2PA - BG2 Rotation/Scaling Parameter A (dx) (Write Only)
                self.bg2pa = data;
                // println!("BG2PA write: 0x{:04x}", data);
            }
            0x0022 => {
                // BG2PB - BG2 Rotation/Scaling Parameter B (dmx) (Write Only)
                self.bg2pb = data;
                // println!("BG2PB write: 0x{:04x}", data);
            }
            0x0024 => {
                // BG2PC - BG2 Rotation/Scaling Parameter C (dy) (Write Only)
                self.bg2pc = data;
                // println!("BG2PC write: 0x{:04x}", data);
            }
            0x0026 => {
                // BG2PD - BG2 Rotation/Scaling Parameter D (dmy) (Write Only)
                self.bg2pd = data;
                // println!("BG2PD write: 0x{:04x}", data);
            }
            0x0028 => {
                // BG2X_L - BG2 Reference Point X-Coordinate, lower 16 bit (Write Only)
                self.bg2x = (self.bg2x & 0xFFFF_0000) | (data as Word);
                // println!("BG2X_L write: 0x{:04x}, BG2X now: 0x{:08x}", data, self.bg2x);
            }
            0x002A => {
                // BG2X_H - BG2 Reference Point X-Coordinate, upper 12 bit (Write Only)
                self.bg2x = (self.bg2x & 0x0000_FFFF) | ((data as Word & 0x0FFF) << 16);
                // Sign extend if bit 27 is set (28-bit signed value)
                if (self.bg2x & 0x0800_0000) != 0 {
                    self.bg2x |= 0xF000_0000;
                }
                // println!("BG2X_H write: 0x{:04x}, BG2X now: 0x{:08x}", data, self.bg2x);
            }
            0x002C => {
                // BG2Y_L - BG2 Reference Point Y-Coordinate, lower 16 bit (Write Only)
                self.bg2y = (self.bg2y & 0xFFFF_0000) | (data as Word);
                // println!("BG2Y_L write: 0x{:04x}, BG2Y now: 0x{:08x}", data, self.bg2y);
            }
            0x002E => {
                // BG2Y_H - BG2 Reference Point Y-Coordinate, upper 12 bit (Write Only)
                self.bg2y = (self.bg2y & 0x0000_FFFF) | ((data as Word & 0x0FFF) << 16);
                // Sign extend if bit 27 is set (28-bit signed value)
                if (self.bg2y & 0x0800_0000) != 0 {
                    self.bg2y |= 0xF000_0000;
                }
                // println!("BG2Y_H write: 0x{:04x}, BG2Y now: 0x{:08x}", data, self.bg2y);
            }
            0x0030 => {
                // BG3PA - BG3 Rotation/Scaling Parameter A (dx) (Write Only)
                self.bg3pa = data;
                // println!("BG3PA write: 0x{:04x}", data);
            }
            0x0032 => {
                // BG3PB - BG3 Rotation/Scaling Parameter B (dmx) (Write Only)
                self.bg3pb = data;
                // println!("BG3PB write: 0x{:04x}", data);
            }
            0x0034 => {
                // BG3PC - BG3 Rotation/Scaling Parameter C (dy) (Write Only)
                self.bg3pc = data;
                // println!("BG3PC write: 0x{:04x}", data);
            }
            0x0036 => {
                // BG3PD - BG3 Rotation/Scaling Parameter D (dmy) (Write Only)
                self.bg3pd = data;
                // println!("BG3PD write: 0x{:04x}", data);
            }
            0x0038 => {
                // BG3X_L - BG3 Reference Point X-Coordinate, lower 16 bit (Write Only)
                self.bg3x = (self.bg3x & 0xFFFF_0000) | (data as Word);
                // println!("BG3X_L write: 0x{:04x}, BG3X now: 0x{:08x}", data, self.bg3x);
            }
            0x003A => {
                // BG3X_H - BG3 Reference Point X-Coordinate, upper 12 bit (Write Only)
                self.bg3x = (self.bg3x & 0x0000_FFFF) | ((data as Word & 0x0FFF) << 16);
                // Sign extend if bit 27 is set (28-bit signed value)
                if (self.bg3x & 0x0800_0000) != 0 {
                    self.bg3x |= 0xF000_0000;
                }
                // println!("BG3X_H write: 0x{:04x}, BG3X now: 0x{:08x}", data, self.bg3x);
            }
            0x003C => {
                // BG3Y_L - BG3 Reference Point Y-Coordinate, lower 16 bit (Write Only)
                self.bg3y = (self.bg3y & 0xFFFF_0000) | (data as Word);
                // println!("BG3Y_L write: 0x{:04x}, BG3Y now: 0x{:08x}", data, self.bg3y);
            }
            0x003E => {
                // BG3Y_H - BG3 Reference Point Y-Coordinate, upper 12 bit (Write Only)
                self.bg3y = (self.bg3y & 0x0000_FFFF) | ((data as Word & 0x0FFF) << 16);
                // Sign extend if bit 27 is set (28-bit signed value)
                if (self.bg3y & 0x0800_0000) != 0 {
                    self.bg3y |= 0xF000_0000;
                }
                // println!("BG3Y_H write: 0x{:04x}, BG3Y now: 0x{:08x}", data, self.bg3y);
            }
            0x0040 => {
                // WIN0H - Window 0 Horizontal Dimensions (Write Only)
                self.win0h = data;
                let x1 = (data >> 8) & 0xFF; // Bit 8-15: X1, Leftmost coordinate
                let x2 = data & 0xFF; // Bit 0-7: X2, Rightmost coordinate + 1
                                      // println!("WIN0H write: 0x{:04x} (X1:{}, X2:{})", data, x1, x2);
            }
            0x0042 => {
                // WIN1H - Window 1 Horizontal Dimensions (Write Only)
                self.win1h = data;
                let x1 = (data >> 8) & 0xFF; // Bit 8-15: X1, Leftmost coordinate
                let x2 = data & 0xFF; // Bit 0-7: X2, Rightmost coordinate + 1
                                      // println!("WIN1H write: 0x{:04x} (X1:{}, X2:{})", data, x1, x2);
            }
            0x0044 => {
                // WIN0V - Window 0 Vertical Dimensions (Write Only)
                self.win0v = data;
                let y1 = (data >> 8) & 0xFF; // Bit 8-15: Y1, Top-most coordinate
                let y2 = data & 0xFF; // Bit 0-7: Y2, Bottom-most coordinate + 1
                                      // println!("WIN0V write: 0x{:04x} (Y1:{}, Y2:{})", data, y1, y2);
            }
            0x0046 => {
                // WIN1V - Window 1 Vertical Dimensions (Write Only)
                self.win1v = data;
                let y1 = (data >> 8) & 0xFF; // Bit 8-15: Y1, Top-most coordinate
                let y2 = data & 0xFF; // Bit 0-7: Y2, Bottom-most coordinate + 1
                                      // println!("WIN1V write: 0x{:04x} (Y1:{}, Y2:{})", data, y1, y2);
            }
            0x0048 => {
                // WININ - Control of Inside of Window(s) (Read/Write)
                self.winin = data;
                let win0_bg0_3 = data & 0x000F; // Bit 0-3: Window 0 BG0-BG3 Enable
                let win0_obj = (data & 0x0010) != 0; // Bit 4: Window 0 OBJ Enable
                let win0_effect = (data & 0x0020) != 0; // Bit 5: Window 0 Color Special Effect
                let win1_bg0_3 = (data & 0x0F00) >> 8; // Bit 8-11: Window 1 BG0-BG3 Enable
                let win1_obj = (data & 0x1000) != 0; // Bit 12: Window 1 OBJ Enable
                let win1_effect = (data & 0x2000) != 0; // Bit 13: Window 1 Color Special Effect
                                                        // println!("WININ write: 0x{:04x} (Win0: BG:{:04b} OBJ:{} FX:{}, Win1: BG:{:04b} OBJ:{} FX:{})",
                                                        //     data, win0_bg0_3, win0_obj, win0_effect, win1_bg0_3, win1_obj, win1_effect);
            }
            0x004A => {
                // WINOUT - Control of Outside of Windows & Inside of OBJ Window (Read/Write)
                self.winout = data;
                let out_bg0_3 = data & 0x000F; // Bit 0-3: Outside BG0-BG3 Enable
                let out_obj = (data & 0x0010) != 0; // Bit 4: Outside OBJ Enable
                let out_effect = (data & 0x0020) != 0; // Bit 5: Outside Color Special Effect
                let objwin_bg0_3 = (data & 0x0F00) >> 8; // Bit 8-11: OBJ Window BG0-BG3 Enable
                let objwin_obj = (data & 0x1000) != 0; // Bit 12: OBJ Window OBJ Enable
                let objwin_effect = (data & 0x2000) != 0; // Bit 13: OBJ Window Color Special Effect
                                                          // println!("WINOUT write: 0x{:04x} (Out: BG:{:04b} OBJ:{} FX:{}, ObjWin: BG:{:04b} OBJ:{} FX:{})",
                                                          //     data, out_bg0_3, out_obj, out_effect, objwin_bg0_3, objwin_obj, objwin_effect);
            }
            0x004C => {
                // MOSAIC - Mosaic Size (Write Only)
                self.mosaic = data;
            }
            0x0050 => {
                // BLDCNT - Color Special Effects Selection (Read/Write)
                self.bldcnt = data;
                let first_target = data & 0x003F; // Bit 0-5: 1st Target (BG0-3, OBJ, BD)
                let effect_type = (data >> 6) & 0x0003; // Bit 6-7: Effect Type
                let second_target = (data >> 8) & 0x003F; // Bit 8-13: 2nd Target (BG0-3, OBJ, BD)

                let effect_name = match effect_type {
                    0 => "None",
                    1 => "Alpha Blending",
                    2 => "Brightness Increase",
                    3 => "Brightness Decrease",
                    _ => "Unknown",
                };

                // println!("BLDCNT write: 0x{:04x} (1st:{:06b}, Effect:{}, 2nd:{:06b})",
                //     data, first_target, effect_name, second_target);
            }
            0x0052 => {
                // BLDALPHA - Alpha Blending Coefficients (Read/Write)
                self.bldalpha = data;
                let eva = data & 0x001F; // Bit 0-4: EVA Coefficient (1st Target)
                let evb = (data >> 8) & 0x001F; // Bit 8-12: EVB Coefficient (2nd Target)

                // Clamp coefficients to valid range (0-16)
                let eva_clamped = if eva > 16 { 16 } else { eva };
                let evb_clamped = if evb > 16 { 16 } else { evb };

                // println!("BLDALPHA write: 0x{:04x} (EVA:{}/{}, EVB:{}/{})",
                //     data, eva, eva_clamped, evb, evb_clamped);
            }
            0x0054 => {
                // BLDY - Brightness (Fade-In/Out) Coefficient (Write Only)
                self.bldy = data;
                let evy = data & 0x001F; // Bit 0-4: EVY Coefficient (Brightness)

                // Clamp coefficient to valid range (0-16)
                let evy_clamped = if evy > 16 { 16 } else { evy };

                // println!("BLDY write: 0x{:04x} (EVY:{}/{})", data, evy, evy_clamped);
            }
            _ => {
                todo!("Unhandled LCD register write: addr=0x{:04x}, data=0x{:04x}", addr, data);
            }
        }
    }

    pub fn write_word(&mut self, addr: Word, data: Word) {
        let data = data as HalfWord;
        match addr {
            0x0000 => self.dispcnt.write(data),
            0x0004 => {
                // DISPSTAT - General LCD Status (Read/Write)
                self.dispstat.write(data);
                if Self::trace_lcd() {
                    println!("🔧 DISPSTAT word write: 0x{:04x} -> masked: 0x{:04x}", data, self.dispstat.0 & 0x0038);
                }
            }
            0x0008 => self.bg0cnt.write(data),
            0x000A => self.bg1cnt.write(data),
            0x000C => self.bg2cnt.write(data),
            0x000E => self.bg3cnt.write(data),
            0x0010 => {
                // BG0HOFS - BG0 X-Offset (Write Only)
                self.bg0hofs = data & 0x01FF;
            }
            0x0012 => {
                // BG0VOFS - BG0 Y-Offset (Write Only)
                self.bg0vofs = data & 0x01FF;
            }
            0x0014 => {
                // BG1HOFS - BG1 X-Offset (Write Only)
                self.bg1hofs = data & 0x01FF;
            }
            0x0016 => {
                // BG1VOFS - BG1 Y-Offset (Write Only)
                self.bg1vofs = data & 0x01FF;
            }
            0x0018 => {
                // BG2HOFS - BG2 X-Offset (Write Only)
                self.bg2hofs = data & 0x01FF;
            }
            0x001A => {
                // BG2VOFS - BG2 Y-Offset (Write Only)
                self.bg2vofs = data & 0x01FF;
            }
            0x001C => {
                // BG3HOFS - BG3 X-Offset (Write Only)
                self.bg3hofs = data & 0x01FF;
            }
            0x001E => {
                // BG3VOFS - BG3 Y-Offset (Write Only)
                self.bg3vofs = data & 0x01FF;
            }
            0x0020 => {
                // BG2PA - BG2 Rotation/Scaling Parameter A (dx) (Write Only)
                self.bg2pa = data;
            }
            0x0022 => {
                // BG2PB - BG2 Rotation/Scaling Parameter B (dmx) (Write Only)
                self.bg2pb = data;
            }
            0x0024 => {
                // BG2PC - BG2 Rotation/Scaling Parameter C (dy) (Write Only)
                self.bg2pc = data;
            }
            0x0026 => {
                // BG2PD - BG2 Rotation/Scaling Parameter D (dmy) (Write Only)
                self.bg2pd = data;
            }
            0x0028 => {
                // BG2X_L - BG2 Reference Point X-Coordinate, lower 16 bit (Write Only)
                self.bg2x = (self.bg2x & 0xFFFF_0000) | (data as Word);
            }
            0x002A => {
                // BG2X_H - BG2 Reference Point X-Coordinate, upper 12 bit (Write Only)
                self.bg2x = (self.bg2x & 0x0000_FFFF) | ((data as Word & 0x0FFF) << 16);
                // Sign extend if bit 27 is set (28-bit signed value)
                if (self.bg2x & 0x0800_0000) != 0 {
                    self.bg2x |= 0xF000_0000;
                }
            }
            0x002C => {
                // BG2Y_L - BG2 Reference Point Y-Coordinate, lower 16 bit (Write Only)
                self.bg2y = (self.bg2y & 0xFFFF_0000) | (data as Word);
            }
            0x002E => {
                // BG2Y_H - BG2 Reference Point Y-Coordinate, upper 12 bit (Write Only)
                self.bg2y = (self.bg2y & 0x0000_FFFF) | ((data as Word & 0x0FFF) << 16);
                // Sign extend if bit 27 is set (28-bit signed value)
                if (self.bg2y & 0x0800_0000) != 0 {
                    self.bg2y |= 0xF000_0000;
                }
            }
            0x0030 => {
                // BG3PA - BG3 Rotation/Scaling Parameter A (dx) (Write Only)
                self.bg3pa = data;
            }
            0x0032 => {
                // BG3PB - BG3 Rotation/Scaling Parameter B (dmx) (Write Only)
                self.bg3pb = data;
            }
            0x0034 => {
                // BG3PC - BG3 Rotation/Scaling Parameter C (dy) (Write Only)
                self.bg3pc = data;
            }
            0x0036 => {
                // BG3PD - BG3 Rotation/Scaling Parameter D (dmy) (Write Only)
                self.bg3pd = data;
            }
            0x0038 => {
                // BG3X_L - BG3 Reference Point X-Coordinate, lower 16 bit (Write Only)
                self.bg3x = (self.bg3x & 0xFFFF_0000) | (data as Word);
            }
            0x003A => {
                // BG3X_H - BG3 Reference Point X-Coordinate, upper 12 bit (Write Only)
                self.bg3x = (self.bg3x & 0x0000_FFFF) | ((data as Word & 0x0FFF) << 16);
                // Sign extend if bit 27 is set (28-bit signed value)
                if (self.bg3x & 0x0800_0000) != 0 {
                    self.bg3x |= 0xF000_0000;
                }
            }
            0x003C => {
                // BG3Y_L - BG3 Reference Point Y-Coordinate, lower 16 bit (Write Only)
                self.bg3y = (self.bg3y & 0xFFFF_0000) | (data as Word);
            }
            0x003E => {
                // BG3Y_H - BG3 Reference Point Y-Coordinate, upper 12 bit (Write Only)
                self.bg3y = (self.bg3y & 0x0000_FFFF) | ((data as Word & 0x0FFF) << 16);
                // Sign extend if bit 27 is set (28-bit signed value)
                if (self.bg3y & 0x0800_0000) != 0 {
                    self.bg3y |= 0xF000_0000;
                }
            }
            0x0040 => {
                // WIN0H - Window 0 Horizontal Dimensions (Write Only)
                self.win0h = data;
                let x1 = (data >> 8) & 0xFF; // Bit 8-15: X1, Leftmost coordinate
                let x2 = data & 0xFF; // Bit 0-7: X2, Rightmost coordinate + 1
            }
            0x0042 => {
                // WIN1H - Window 1 Horizontal Dimensions (Write Only)
                self.win1h = data;
                let x1 = (data >> 8) & 0xFF; // Bit 8-15: X1, Leftmost coordinate
                let x2 = data & 0xFF; // Bit 0-7: X2, Rightmost coordinate + 1
            }
            0x0044 => {
                // WIN0V - Window 0 Vertical Dimensions (Write Only)
                self.win0v = data;
                let y1 = (data >> 8) & 0xFF; // Bit 8-15: Y1, Top-most coordinate
                let y2 = data & 0xFF; // Bit 0-7: Y2, Bottom-most coordinate + 1
            }
            0x0046 => {
                // WIN1V - Window 1 Vertical Dimensions (Write Only)
                self.win1v = data;
                let y1 = (data >> 8) & 0xFF; // Bit 8-15: Y1, Top-most coordinate
                let y2 = data & 0xFF; // Bit 0-7: Y2, Bottom-most coordinate + 1
            }
            0x0048 => {
                // WININ - Control of Inside of Window(s) (Read/Write)
                self.winin = data;
            }
            0x004A => {
                // WINOUT - Control of Outside of Windows & Inside of OBJ Window (Read/Write)
                self.winout = data;
            }
            0x004C => {
                // MOSAIC - Mosaic Size (Write Only)
                self.mosaic = data;
                let bg_h_size = data & 0x000F;
                let bg_v_size = (data & 0x00F0) >> 4;
                let obj_h_size = (data & 0x0F00) >> 8;
                let obj_v_size = (data & 0xF000) >> 12;
                println!(
                    "MOSAIC word write: 0x{:04x} (BG H:{} V:{}, OBJ H:{} V:{})",
                    data,
                    bg_h_size + 1,
                    bg_v_size + 1,
                    obj_h_size + 1,
                    obj_v_size + 1
                );
            }
            0x0050 => {
                // BLDCNT - Color Special Effects Selection (Read/Write)
                self.bldcnt = data;
            }
            0x0052 => {
                // BLDALPHA - Alpha Blending Coefficients (Read/Write)
                self.bldalpha = data;
            }
            0x0054 => {
                // BLDY - Brightness (Fade-In/Out) Coefficient (Write Only)
                self.bldy = data;
            }
            _ => {
                todo!("Unhandled LCD register word write: addr=0x{:04x}, data=0x{:08x}", addr, data);
            }
        }
    }

    // Helper function to check if a pixel is inside a window
    fn is_pixel_in_window(&self, x: Word, y: Word, win_h: HalfWord, win_v: HalfWord) -> bool {
        if win_h == 0 && win_v == 0 {
            return false; // Window disabled if both dimensions are 0
        }

        // Extract window coordinates
        let x1 = (win_h >> 8) & 0xFF; // Left coordinate
        let x2 = win_h & 0xFF; // Right coordinate + 1
        let y1 = (win_v >> 8) & 0xFF; // Top coordinate
        let y2 = win_v & 0xFF; // Bottom coordinate + 1

        // Handle coordinate wrapping
        let x_in_range = if x2 > x1 {
            x >= x1 as Word && x < x2 as Word
        } else {
            x >= x1 as Word || x < x2 as Word // Wrapped around
        };

        let y_in_range = if y2 > y1 {
            y >= y1 as Word && y < y2 as Word
        } else {
            y >= y1 as Word || y < y2 as Word // Wrapped around
        };

        x_in_range && y_in_range
    }

    // Helper function to determine which layers should be rendered for a pixel
    fn get_window_control(&self, x: Word, y: Word, in_objwin: bool) -> (bool, bool, bool, bool, bool, bool) {
        // If no windows are enabled, fall back to DISPCNT's layer enable bits.
        // DISPCNT bit 15 (here `obj_display_flag`) enables the OBJ window.
        let any_window_enabled = self.dispcnt.window0_display_flag()
            || self.dispcnt.window1_display_flag()
            || self.dispcnt.obj_display_flag();
        if !any_window_enabled {
            let bg0_enable = self.dispcnt.screen_display_bg0();
            let bg1_enable = self.dispcnt.screen_display_bg1();
            let bg2_enable = self.dispcnt.screen_display_bg2();
            let bg3_enable = self.dispcnt.screen_display_bg3();
            let obj_enable = self.dispcnt.screen_display_obj();
            // Enable color effect flag only if some effect is selected in BLDCNT
            let effect_enable = ((self.bldcnt >> 6) & 0x0003) != 0;
            return (bg0_enable, bg1_enable, bg2_enable, bg3_enable, obj_enable, effect_enable);
        }

        // Check which window the pixel is in (Window 0 has highest priority).
        // Only consider win0/win1 if they are actually enabled in DISPCNT.
        let in_win0 = self.dispcnt.window0_display_flag()
            && self.is_pixel_in_window(x, y, self.win0h, self.win0v);
        let in_win1 = self.dispcnt.window1_display_flag()
            && self.is_pixel_in_window(x, y, self.win1h, self.win1v);
        let in_objwin = in_objwin && self.dispcnt.obj_display_flag();

        let control_bits = if in_win0 {
            // Inside Window 0 - use WININ bits 0-5
            self.winin & 0x003F
        } else if in_win1 {
            // Inside Window 1 - use WININ bits 8-13
            (self.winin >> 8) & 0x003F
        } else if in_objwin {
            // Inside OBJ Window - use WINOUT bits 8-13
            (self.winout >> 8) & 0x003F
        } else {
            // Outside all windows - use WINOUT bits 0-5
            self.winout & 0x003F
        };

        // Extract individual layer enable bits
        let bg0_enable = (control_bits & 0x0001) != 0;
        let bg1_enable = (control_bits & 0x0002) != 0;
        let bg2_enable = (control_bits & 0x0004) != 0;
        let bg3_enable = (control_bits & 0x0008) != 0;
        let obj_enable = (control_bits & 0x0010) != 0;
        let effect_enable = (control_bits & 0x0020) != 0;

        (bg0_enable, bg1_enable, bg2_enable, bg3_enable, obj_enable, effect_enable)
    }

    // Helper function to perform alpha blending between two colors
    fn alpha_blend(&self, first_color: BGR, second_color: BGR) -> BGR {
        let eva = self.bldalpha & 0x001F; // Bit 0-4: EVA Coefficient (1st Target)
        let evb = (self.bldalpha >> 8) & 0x001F; // Bit 8-12: EVB Coefficient (2nd Target)

        // Clamp coefficients to valid range (0-16)
        let eva_clamped = if eva > 16 { 16 } else { eva };
        let evb_clamped = if evb > 16 { 16 } else { evb };

        // Extract RGB components from first color (convert from 5-bit to 8-bit)
        let r1 = (first_color.red() >> 3) as u16; // Convert 8-bit back to 5-bit
        let g1 = (first_color.green() >> 3) as u16;
        let b1 = (first_color.blue() >> 3) as u16;

        // Extract RGB components from second color (convert from 5-bit to 8-bit)
        let r2 = (second_color.red() >> 3) as u16; // Convert 8-bit back to 5-bit
        let g2 = (second_color.green() >> 3) as u16;
        let b2 = (second_color.blue() >> 3) as u16;

        // Perform alpha blending: I = MIN(31, I1st*EVA + I2nd*EVB) / 16
        let r_blended = ((r1 * eva_clamped + r2 * evb_clamped) / 16).min(31);
        let g_blended = ((g1 * eva_clamped + g2 * evb_clamped) / 16).min(31);
        let b_blended = ((b1 * eva_clamped + b2 * evb_clamped) / 16).min(31);

        // Convert back to BGR format (5-bit per component)
        let bgr_value = (b_blended << 10) | (g_blended << 5) | r_blended;
        BGR::new(bgr_value as HalfWord)
    }

    fn is_1st_target(&self, target: u8) -> bool {
        (self.bldcnt & (1 << target)) != 0
    }
    fn is_2nd_target(&self, target: u8) -> bool {
        ((self.bldcnt >> 8) & (1 << target)) != 0
    }

    /// Brightness-increase (fade to white) by EVY.
    fn brighten(&self, color: BGR) -> BGR {
        let evy = (self.bldy & 0x1F).min(16);
        let r = (color.red() >> 3) as u16;
        let g = (color.green() >> 3) as u16;
        let b = (color.blue() >> 3) as u16;
        let r = (r + (31 - r) * evy / 16).min(31);
        let g = (g + (31 - g) * evy / 16).min(31);
        let b = (b + (31 - b) * evy / 16).min(31);
        BGR::new(((b << 10) | (g << 5) | r) as HalfWord)
    }

    /// Brightness-decrease (fade to black) by EVY.
    fn darken(&self, color: BGR) -> BGR {
        let evy = (self.bldy & 0x1F).min(16);
        let r = (color.red() >> 3) as u16;
        let g = (color.green() >> 3) as u16;
        let b = (color.blue() >> 3) as u16;
        let r = r - r * evy / 16;
        let g = g - g * evy / 16;
        let b = b - b * evy / 16;
        BGR::new(((b << 10) | (g << 5) | r) as HalfWord)
    }

    /// Apply colour special effects to the top-most pixel given the pixel below.
    /// `top` is (colour, BLDCNT target id, semi-transparent OBJ); `below` is the
    /// colour/target of the next pixel down (the 2nd-target candidate).
    fn blend_pixel(&self, top: (BGR, u8, bool), below: Option<(BGR, u8)>, effect_enable: bool) -> BGR {
        let (c1, t1, semi) = top;
        // A semi-transparent OBJ always alpha-blends with a 2nd target below it,
        // regardless of the BLDCNT effect selection.
        if semi {
            if let Some((c2, t2)) = below {
                if self.is_2nd_target(t2) {
                    return self.alpha_blend(c1, c2);
                }
            }
            return c1;
        }
        if !effect_enable {
            return c1;
        }
        match (self.bldcnt >> 6) & 0x3 {
            1 => {
                if self.is_1st_target(t1) {
                    if let Some((c2, t2)) = below {
                        if self.is_2nd_target(t2) {
                            return self.alpha_blend(c1, c2);
                        }
                    }
                }
                c1
            }
            2 if self.is_1st_target(t1) => self.brighten(c1),
            3 if self.is_1st_target(t1) => self.darken(c1),
            _ => c1,
        }
    }

    /// BG mosaic block size (h, v) in pixels (MOSAIC bits 0-7, value + 1).
    fn bg_mosaic(&self) -> (Word, Word) {
        ((self.mosaic & 0xF) as Word + 1, ((self.mosaic >> 4) & 0xF) as Word + 1)
    }

    /// OBJ mosaic block size (h, v) in pixels (MOSAIC bits 8-15, value + 1).
    fn obj_mosaic(&self) -> (u32, u32) {
        (((self.mosaic >> 8) & 0xF) as u32 + 1, ((self.mosaic >> 12) & 0xF) as u32 + 1)
    }

    /// Sprite (OBJ) sizes indexed by [shape][size] -> (width, height) in pixels.
    fn obj_size(shape: u16, size: u16) -> (u32, u32) {
        const T: [[(u32, u32); 4]; 3] = [
            [(8, 8), (16, 16), (32, 32), (64, 64)],   // square
            [(16, 8), (32, 8), (32, 16), (64, 32)],   // horizontal
            [(8, 16), (8, 32), (16, 32), (32, 64)],   // vertical
        ];
        let s = (shape as usize).min(2);
        T[s][(size & 3) as usize]
    }

    /// Sample one texel of an OBJ. `tx`/`ty` are texture coordinates inside the
    /// sprite (already flipped). Returns None for the transparent palette index.
    fn obj_tile_texel(
        &self,
        vram: &Ram,
        palette: &Ram,
        base_tile: u32,
        tx: u32,
        ty: u32,
        sprite_w: u32,
        is_8bpp: bool,
        one_d: bool,
        palbank: u32,
        bitmap_mode: bool,
    ) -> Option<BGR> {
        let tile_x = tx / 8;
        let tile_y = ty / 8;
        let in_x = tx % 8;
        let in_y = ty % 8;
        let step = if is_8bpp { 2 } else { 1 };
        let tile_id = if one_d {
            base_tile + (tile_y * (sprite_w / 8) + tile_x) * step
        } else {
            base_tile + tile_y * 32 + tile_x * step
        };
        // In bitmap modes (3-5) only the upper OBJ char block (tiles >= 512) is
        // usable; lower tiles overlap the BG bitmap and are not displayed.
        if bitmap_mode && tile_id < 512 {
            return None;
        }
        let char_base: u32 = 0x1_0000;
        if is_8bpp {
            let addr = (char_base + tile_id * 32 + in_y * 8 + in_x) & 0x1_7FFF;
            let v = vram.read_byte(addr);
            if v == 0 {
                return None;
            }
            Some(BGR::new(palette.read_halfword(0x200 + v as u32 * 2)))
        } else {
            let addr = (char_base + tile_id * 32 + in_y * 4 + in_x / 2) & 0x1_7FFF;
            let byte = vram.read_byte(addr);
            let nib = if in_x & 1 == 0 { byte & 0x0F } else { (byte >> 4) & 0x0F };
            if nib == 0 {
                return None;
            }
            Some(BGR::new(palette.read_halfword(0x200 + (palbank * 16 + nib as u32) * 2)))
        }
    }

    /// Render all sprites into a full-screen overlay buffer. Each entry is the
    /// top-most (lowest OBJ index) opaque sprite pixel, with its BG-priority and
    /// a semi-transparent flag.
    /// Render the sprite colour layer and the OBJ-window coverage mask. Mode-2
    /// sprites contribute only to the window mask (they are not drawn).
    fn render_obj(&self, vram: &Ram, palette: &Ram, oam: &Ram) -> (Vec<Option<ObjPixel>>, Vec<bool>) {
        let mut buf: Vec<Option<ObjPixel>> = vec![None; 240 * 160];
        let mut objwin: Vec<bool> = vec![false; 240 * 160];
        if !self.dispcnt.screen_display_obj() {
            return (buf, objwin);
        }
        let one_d = self.dispcnt.obj_char_mapping();
        let bitmap_mode = matches!(self.dispcnt.mode(), BgMode::Mode3 | BgMode::Mode4 | BgMode::Mode5);

        for idx in 0..128u32 {
            let base = idx * 8;
            let a0 = oam.read_halfword(base);
            let a1 = oam.read_halfword(base + 2);
            let a2 = oam.read_halfword(base + 4);

            let affine = (a0 & 0x0100) != 0;
            let double = (a0 & 0x0200) != 0;
            if !affine && double {
                continue; // disable bit
            }
            let mode = (a0 >> 10) & 0x3;
            if mode == 3 {
                continue; // prohibited
            }
            // Mode 2 sprites define the OBJ window region instead of being drawn.
            let is_window = mode == 2;
            let semi = mode == 1;
            let mosaic = (a0 & 0x1000) != 0;
            let is_8bpp = (a0 & 0x2000) != 0;
            let shape = (a0 >> 14) & 0x3;
            let size = (a1 >> 14) & 0x3;
            let (w, h) = Self::obj_size(shape, size);

            let y0 = (a0 & 0xFF) as i32;
            let mut x0 = (a1 & 0x1FF) as i32;
            if x0 >= 256 {
                x0 -= 512; // 9-bit signed
            }
            let prio = (a2 >> 10) & 0x3;
            let base_tile = (a2 & 0x3FF) as u32;
            let palbank = ((a2 >> 12) & 0xF) as u32;

            let (bbw, bbh) = if double { (w * 2, h * 2) } else { (w, h) };

            // Affine matrix (1/256 fixed point). Identity for non-affine sprites.
            let (pa, pb, pc, pd) = if affine {
                let g = ((a1 >> 9) & 0x1F) as u32;
                let rd = |i: u32| oam.read_halfword(g * 32 + i * 8 + 6) as i16 as i32;
                (rd(0), rd(1), rd(2), rd(3))
            } else {
                (0x100, 0, 0, 0x100)
            };
            let hflip = !affine && (a1 & 0x1000) != 0;
            let vflip = !affine && (a1 & 0x2000) != 0;

            for oy in 0..bbh as i32 {
                let sy = (y0 + oy) & 0xFF;
                if sy >= 160 {
                    continue;
                }
                for ox in 0..bbw as i32 {
                    let sx = x0 + ox;
                    if sx < 0 || sx >= 240 {
                        continue;
                    }
                    // Map screen offset -> texture coordinate.
                    let (tx, ty) = if affine {
                        let cx = bbw as i32 / 2;
                        let cy = bbh as i32 / 2;
                        let dx = ox - cx;
                        let dy = oy - cy;
                        let tx = ((pa * dx + pb * dy) >> 8) + w as i32 / 2;
                        let ty = ((pc * dx + pd * dy) >> 8) + h as i32 / 2;
                        (tx, ty)
                    } else {
                        let mut tx = ox;
                        let mut ty = oy;
                        if hflip {
                            tx = w as i32 - 1 - tx;
                        }
                        if vflip {
                            ty = h as i32 - 1 - ty;
                        }
                        (tx, ty)
                    };
                    if tx < 0 || ty < 0 || tx >= w as i32 || ty >= h as i32 {
                        continue;
                    }
                    // OBJ mosaic: snap the texel coordinate to the mosaic block.
                    let (tx, ty) = if mosaic {
                        let (mh, mv) = self.obj_mosaic();
                        (tx - tx % mh as i32, ty - ty % mv as i32)
                    } else {
                        (tx, ty)
                    };

                    let dst = (sy * 240 + sx) as usize;
                    // Lower OBJ index has priority: keep the first opaque writer
                    // (window sprites only need coverage, so let them re-mark).
                    if !is_window && buf[dst].is_some() {
                        continue;
                    }
                    if let Some(color) = self.obj_tile_texel(
                        vram, palette, base_tile, tx as u32, ty as u32, w, is_8bpp, one_d, palbank,
                        bitmap_mode,
                    ) {
                        if is_window {
                            objwin[dst] = true;
                        } else {
                            buf[dst] = Some(ObjPixel { color, priority: prio, semi });
                        }
                    }
                }
            }
        }
        (buf, objwin)
    }

    pub fn render(&self, vram: &Ram, palette: &Ram, oam: &Ram) -> Vec<u8> {
        // Forced blank (DISPCNT bit 7): the screen outputs white.
        if self.dispcnt.forced_vlank() {
            return vec![0xFF; 240 * 160 * 4];
        }
        match self.dispcnt.mode() {
            BgMode::Mode0 | BgMode::Mode1 | BgMode::Mode2 => {
                self.render_tiled(self.dispcnt.mode(), vram, palette, oam)
            }
            BgMode::Mode3 => self.render_with_mode3(vram, palette, oam),
            BgMode::Mode4 => self.render_with_mode4(vram, palette, oam),
            BgMode::Mode5 => self.render_with_mode5(vram, palette, oam),
        }
    }

    /// Composite an OBJ pixel over an already-resolved BG pixel.
    /// `bg` is (priority, color); `bg_is_2nd_target` enables semi-transparent
    /// blending. Returns the final colour to display.
    fn composite_obj(&self, obj: Option<ObjPixel>, bg_priority: u16, bg_color: BGR) -> BGR {
        match obj {
            Some(o) if o.priority <= bg_priority => {
                if o.semi {
                    // Semi-transparent OBJ blends with the layer below.
                    self.alpha_blend(o.color, bg_color)
                } else {
                    o.color
                }
            }
            _ => bg_color,
        }
    }

    fn render_with_mode0(&self, vram: &Ram, palette: &Ram, oam: &Ram) -> Vec<u8> {
        self.render_tiled(BgMode::Mode0, vram, palette, oam)
    }

    /// Generic compositor for the tiled BG modes (0/1/2): per pixel, sample each
    /// enabled BG layer, pick the highest priority, apply colour effects, then
    /// overlay sprites.
    fn render_tiled(&self, mode: BgMode, vram: &Ram, palette: &Ram, oam: &Ram) -> Vec<u8> {
        let mut buf = vec![0; 240 * 160 * 4];
        let (obj, objwin) = self.render_obj(vram, palette, oam);

        // Render pixel by pixel with window control and BG priority composition.
        for screen_y in 0..160 {
            for screen_x in 0..240 {
                // Get window control for this pixel
                let in_objwin = objwin[(screen_y * 240 + screen_x) as usize];
                let (bg0_enable, bg1_enable, bg2_enable, bg3_enable, obj_enable, effect_enable) = self.get_window_control(screen_x, screen_y, in_objwin);

                let buf_index = ((screen_y * 240 + screen_x) * 4) as usize;
                let window_enabled = [bg0_enable, bg1_enable, bg2_enable, bg3_enable];

                // Collect candidate pixels: (priority, kind, target, color, semi).
                // kind orders ties: 0=OBJ, 1=BG, 2=backdrop (OBJ wins BG ties).
                let mut layers: Vec<(u16, u8, u8, BGR, bool)> = Vec::with_capacity(6);
                for layer in 0..4 {
                    if !window_enabled[layer] {
                        continue;
                    }
                    if let Some((priority, color)) =
                        self.sample_bg_layer(mode, layer, screen_x, screen_y, vram, palette)
                    {
                        layers.push((priority, 1, layer as u8, color, false));
                    }
                }
                if obj_enable {
                    if let Some(o) = obj[(screen_y * 240 + screen_x) as usize] {
                        layers.push((o.priority, 0, 4, o.color, o.semi));
                    }
                }
                // Backdrop is always present, below everything.
                layers.push((5, 2, 5, BGR::new(palette.read_halfword(0)), false));
                // Top-most first: by priority, then OBJ-before-BG, then BG layer index.
                layers.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));

                let (_, _, t1, c1, semi) = layers[0];
                let below = layers.get(1).map(|l| (l.3, l.2));
                let color = self.blend_pixel((c1, t1, semi), below, effect_enable);

                buf[buf_index] = color.red();
                buf[buf_index + 1] = color.green();
                buf[buf_index + 2] = color.blue();
                buf[buf_index + 3] = 0xFF;
            }
        }
        buf
    }

    fn sample_text_bg_pixel(
        &self,
        layer: usize,
        screen_x: Word,
        screen_y: Word,
        vram: &Ram,
        palette: &Ram,
    ) -> Option<(u16, BGR)> {
        let (enabled, bgcnt, scroll_x, scroll_y) = match layer {
            0 => (
                self.dispcnt.screen_display_bg0(),
                self.bg0cnt,
                self.bg0hofs as Word,
                self.bg0vofs as Word,
            ),
            1 => (
                self.dispcnt.screen_display_bg1(),
                self.bg1cnt,
                self.bg1hofs as Word,
                self.bg1vofs as Word,
            ),
            2 => (
                self.dispcnt.screen_display_bg2(),
                self.bg2cnt,
                self.bg2hofs as Word,
                self.bg2vofs as Word,
            ),
            3 => (
                self.dispcnt.screen_display_bg3(),
                self.bg3cnt,
                self.bg3hofs as Word,
                self.bg3vofs as Word,
            ),
            _ => return None,
        };

        if !enabled {
            return None;
        }

        // BG mosaic: snap the on-screen coordinate to the mosaic block.
        let (screen_x, screen_y) = if bgcnt.mosaic() {
            let (mh, mv) = self.bg_mosaic();
            (screen_x - screen_x % mh, screen_y - screen_y % mv)
        } else {
            (screen_x, screen_y)
        };

        let (width, height) = match bgcnt.screen_size() {
            0 => (256, 256),
            1 => (512, 256),
            2 => (256, 512),
            _ => (512, 512),
        };
        let bg_x = (screen_x.wrapping_add(scroll_x)) % width;
        let bg_y = (screen_y.wrapping_add(scroll_y)) % height;

        let tile_x = bg_x / 8;
        let tile_y = bg_y / 8;
        let local_tile_x = tile_x % 32;
        let local_tile_y = tile_y % 32;
        let block_x = tile_x / 32;
        let block_y = tile_y / 32;

        let map_block_index = match bgcnt.screen_size() {
            0 => 0,
            1 => block_x,
            2 => block_y,
            _ => block_y * 2 + block_x,
        };

        let map_offset = bgcnt.bg_map_offset();
        let tile_map_addr = map_offset + map_block_index * 0x800 + (local_tile_y * 32 + local_tile_x) * 2;
        let tile_attr = vram.read_halfword(tile_map_addr) as Word;

        let tile_number = tile_attr & 0x03FF;
        let hflip = (tile_attr & 0x0400) != 0;
        let vflip = (tile_attr & 0x0800) != 0;
        let palette_bank = (tile_attr >> 12) & 0x000F;
        let mut pixel_x = bg_x % 8;
        let mut pixel_y = bg_y % 8;
        if hflip {
            pixel_x = 7 - pixel_x;
        }
        if vflip {
            pixel_y = 7 - pixel_y;
        }

        let tile_offset = bgcnt.bg_tile_offset();
        let is_8bpp = bgcnt.colors_palettes();
        let (raw_pixel_value, palette_index_word): (u8, Word) = if is_8bpp {
            let pixel_addr = tile_offset + tile_number * 64 + pixel_y * 8 + pixel_x;
            let value = vram.read_byte(pixel_addr);
            (value, value as Word)
        } else {
            let row_offset = pixel_y * 4;
            let byte_addr = tile_offset + tile_number * 32 + row_offset + (pixel_x / 2);
            let byte = vram.read_byte(byte_addr);
            let nibble = if (pixel_x & 1) == 0 {
                byte & 0x0F
            } else {
                (byte >> 4) & 0x0F
            };
            let pal_index = palette_bank * 16 + nibble as Word;
            (nibble, pal_index)
        };

        if raw_pixel_value == 0 {
            return None;
        }

        let palette_addr = palette_index_word * 2;
        let color = BGR::new(palette.read_halfword(palette_addr));
        Some((bgcnt.bg_priority(), color))
    }

    /// Sample an affine (rotation/scaling) background layer (BG2 or BG3). These
    /// are 8bpp tilemaps whose map entries are a single tile-number byte; the
    /// texture coordinate is computed from the PA-PD matrix and the (BGxX, BGxY)
    /// reference point.
    fn sample_affine_bg_pixel(
        &self,
        layer: usize,
        screen_x: Word,
        screen_y: Word,
        vram: &Ram,
        palette: &Ram,
    ) -> Option<(u16, BGR)> {
        let (enabled, bgcnt, pa, pb, pc, pd, refx, refy) = match layer {
            2 => (
                self.dispcnt.screen_display_bg2(),
                self.bg2cnt,
                self.bg2pa as i16 as i32,
                self.bg2pb as i16 as i32,
                self.bg2pc as i16 as i32,
                self.bg2pd as i16 as i32,
                self.bg2x as i32,
                self.bg2y as i32,
            ),
            3 => (
                self.dispcnt.screen_display_bg3(),
                self.bg3cnt,
                self.bg3pa as i16 as i32,
                self.bg3pb as i16 as i32,
                self.bg3pc as i16 as i32,
                self.bg3pd as i16 as i32,
                self.bg3x as i32,
                self.bg3y as i32,
            ),
            _ => return None,
        };
        if !enabled {
            return None;
        }

        let size_pixels = 128i32 << bgcnt.screen_size(); // 128/256/512/1024
        let wrap = (bgcnt.read() & 0x2000) != 0; // display-area overflow: wraparound
        // BG mosaic snaps the on-screen coordinate to the mosaic block.
        let (screen_x, screen_y) = if bgcnt.mosaic() {
            let (mh, mv) = self.bg_mosaic();
            (screen_x - screen_x % mh, screen_y - screen_y % mv)
        } else {
            (screen_x, screen_y)
        };
        let x = screen_x as i32;
        let y = screen_y as i32;
        let mut tx = (refx + pa * x + pb * y) >> 8;
        let mut ty = (refy + pc * x + pd * y) >> 8;
        if wrap {
            tx = tx.rem_euclid(size_pixels);
            ty = ty.rem_euclid(size_pixels);
        } else if tx < 0 || ty < 0 || tx >= size_pixels || ty >= size_pixels {
            return None;
        }

        let tiles = (size_pixels / 8) as u32;
        let map_base = bgcnt.bg_map_offset();
        let char_base = bgcnt.bg_tile_offset();
        let tile_x = (tx as u32) / 8;
        let tile_y = (ty as u32) / 8;
        let tile_num = vram.read_byte(map_base + tile_y * tiles + tile_x) as u32;
        let in_x = (tx as u32) % 8;
        let in_y = (ty as u32) % 8;
        let idx = vram.read_byte((char_base + tile_num * 64 + in_y * 8 + in_x) & 0x1_7FFF);
        if idx == 0 {
            return None;
        }
        let color = BGR::new(palette.read_halfword(idx as Word * 2));
        Some((bgcnt.bg_priority(), color))
    }

    /// Sample one BG layer for the given tiled mode (0/1/2): text layers for
    /// Mode 0, text BG0/BG1 + affine BG2 for Mode 1, affine BG2/BG3 for Mode 2.
    fn sample_bg_layer(
        &self,
        mode: BgMode,
        layer: usize,
        x: Word,
        y: Word,
        vram: &Ram,
        palette: &Ram,
    ) -> Option<(u16, BGR)> {
        match (mode, layer) {
            (BgMode::Mode0, 0..=3) => self.sample_text_bg_pixel(layer, x, y, vram, palette),
            (BgMode::Mode1, 0) | (BgMode::Mode1, 1) => self.sample_text_bg_pixel(layer, x, y, vram, palette),
            (BgMode::Mode1, 2) => self.sample_affine_bg_pixel(2, x, y, vram, palette),
            (BgMode::Mode2, 2) | (BgMode::Mode2, 3) => self.sample_affine_bg_pixel(layer, x, y, vram, palette),
            _ => None,
        }
    }

    fn render_with_mode3(&self, vram: &Ram, palette: &Ram, oam: &Ram) -> Vec<u8> {
        let mut buf = vec![0u8; 240 * 160 * 4];
        let (obj, _objwin) = self.render_obj(vram, palette, oam);
        let bg_prio = self.bg2cnt.bg_priority();
        let bg_on = self.dispcnt.screen_display_bg2();
        for i in 0..(240 * 160) {
            // Mode 3 BG is a direct 15-bit color bitmap (BG2).
            let bg = BGR::new(vram.read_halfword(i as Word * 2));
            let (p, c) = if bg_on { (bg_prio, bg) } else { (4, BGR::new(palette.read_halfword(0))) };
            let color = self.composite_obj(obj[i], p, c);
            let bi = i * 4;
            buf[bi] = color.red();
            buf[bi + 1] = color.green();
            buf[bi + 2] = color.blue();
            buf[bi + 3] = 0xFF;
        }
        buf
    }

    fn render_with_mode4(&self, vram: &Ram, palette: &Ram, oam: &Ram) -> Vec<u8> {
        let mut buf = vec![0u8; 240 * 160 * 4];
        let (obj, _objwin) = self.render_obj(vram, palette, oam);
        let is_frame1 = matches!(self.dispcnt.frame(), Frame::Frame1);
        let offset = if is_frame1 { 0xA000 } else { 0x0000 };
        let bg_prio = self.bg2cnt.bg_priority();
        let bg_on = self.dispcnt.screen_display_bg2();
        for i in 0..(240 * 160) {
            let palette_index = vram.read_byte(i as Word + offset);
            let bg = BGR::new(palette.read_halfword(palette_index as Word * 2));
            let (p, c) = if bg_on { (bg_prio, bg) } else { (4, BGR::new(palette.read_halfword(0))) };
            let color = self.composite_obj(obj[i], p, c);
            let bi = i * 4;
            buf[bi] = color.red();
            buf[bi + 1] = color.green();
            buf[bi + 2] = color.blue();
            buf[bi + 3] = 0xFF;
        }
        buf
    }

    /// Mode 5: a 160x128 15-bit colour bitmap (BG2), affine-transformable, with
    /// two display frames.
    fn render_with_mode5(&self, vram: &Ram, palette: &Ram, oam: &Ram) -> Vec<u8> {
        let mut buf = vec![0u8; 240 * 160 * 4];
        let (obj, _objwin) = self.render_obj(vram, palette, oam);
        let base = if matches!(self.dispcnt.frame(), Frame::Frame1) { 0xA000u32 } else { 0 };
        let bg_on = self.dispcnt.screen_display_bg2();
        let bg_prio = self.bg2cnt.bg_priority();
        let pa = self.bg2pa as i16 as i32;
        let pb = self.bg2pb as i16 as i32;
        let pc = self.bg2pc as i16 as i32;
        let pd = self.bg2pd as i16 as i32;
        let refx = self.bg2x as i32;
        let refy = self.bg2y as i32;
        for y in 0..160i32 {
            for x in 0..240i32 {
                let tx = (refx + pa * x + pb * y) >> 8;
                let ty = (refy + pc * x + pd * y) >> 8;
                let (p, c) = if bg_on && (0..160).contains(&tx) && (0..128).contains(&ty) {
                    let addr = base + (ty as u32 * 160 + tx as u32) * 2;
                    (bg_prio, BGR::new(vram.read_halfword(addr)))
                } else {
                    (4, BGR::new(palette.read_halfword(0)))
                };
                let i = (y * 240 + x) as usize;
                let color = self.composite_obj(obj[i], p, c);
                let bi = i * 4;
                buf[bi] = color.red();
                buf[bi + 1] = color.green();
                buf[bi + 2] = color.blue();
                buf[bi + 3] = 0xFF;
            }
        }
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::writable::*;

    /// Simulate the AGS `run_vblank_status_test` polling loop directly against
    /// the LCD controller: advance time in small (instruction-sized) steps,
    /// poll the VBlank flag, and record VCOUNT at each flag transition. The
    /// VBlank flag must set at line 160 and clear at line 227.
    #[test]
    fn vblank_flag_transitions_at_lines_160_and_227() {
        let mut lcdc = LCDController::new();
        let mut prev = lcdc.read_halfword(0x0004) & 0x0001;
        let mut transitions: Vec<(u16, u16)> = Vec::new(); // (vblank_flag, vcount)
        // Run for a couple of frames worth of small steps.
        for _ in 0..(super::CYCLES_PER_FRAME / 4 * 3) {
            lcdc.run(4);
            let now = lcdc.read_halfword(0x0004) & 0x0001;
            if now != prev {
                let vcount = lcdc.read_halfword(0x0006);
                transitions.push((now, vcount));
                prev = now;
            }
            if transitions.len() >= 4 {
                break;
            }
        }
        assert!(transitions.len() >= 2, "expected VBlank transitions");
        for (flag, vcount) in transitions {
            if flag != 0 {
                assert_eq!(vcount, 160, "VBlank flag set should occur at line 160");
            } else {
                assert_eq!(vcount, 227, "VBlank flag clear should occur at line 227");
            }
        }
    }

    /// Simulate the VCount-match polling: with a VCount setting, the match flag
    /// must set exactly on the configured line and the VCOUNT read at that point
    /// must equal the setting.
    #[test]
    fn vcount_match_flag_sets_on_configured_line() {
        let mut lcdc = LCDController::new();
        lcdc.write_halfword(0x0004, 100 << 8); // VCount setting = line 100
        let mut prev = lcdc.read_halfword(0x0004) & 0x0004;
        let mut saw_match = false;
        for _ in 0..(super::CYCLES_PER_FRAME / 4 * 2) {
            lcdc.run(4);
            let now = lcdc.read_halfword(0x0004) & 0x0004;
            if now != 0 && prev == 0 {
                // rising edge of VCount match
                assert_eq!(lcdc.read_halfword(0x0006), 100, "VCount match must be at line 100");
                saw_match = true;
                break;
            }
            prev = now;
        }
        assert!(saw_match, "expected a VCount match");
    }

    #[test]
    fn obj_window_gates_bg_inside_sprite() {
        let mut lcdc = LCDController::new();
        // mode 0, BG0 on, OBJ on, OBJ-window display on (bit 15).
        lcdc.write_halfword(0x0000, 0x0100 | 0x1000 | 0x8000);
        // BG0CNT: char base 0, map base block 1 (0x800).
        lcdc.write_halfword(0x0008, 1 << 8);
        // WINOUT: outside = all off (0x00); OBJ-window region = BG0 on (bit 8).
        lcdc.write_halfword(0x004A, 0x0100);

        let mut vram = Ram::new(vec![0; 0x1_8000]);
        let mut palette = Ram::new(vec![0; 0x0400]);
        let mut oam = Ram::new(vec![0; 0x0400]);
        // BG0: tile 0 entirely palette index 1 (red), map entry 0.
        for i in 0..32u32 {
            vram.write_byte(i, 0x11);
        }
        vram.write_halfword(0x0800, 0);
        palette.write_halfword(2, 0x001F);
        // OBJ window sprite 0: mode 2 (bits 10-11 = 10 -> 0x0800), 8x8, tile 0.
        oam.write_halfword(0, 0x0800);
        oam.write_halfword(2, 0x0000);
        oam.write_halfword(4, 0x0000);
        // OBJ tile 0 at 0x10000: non-transparent (defines window coverage).
        for i in 0..32u32 {
            vram.write_byte(0x1_0000 + i, 0x11);
        }

        let buf = lcdc.render(&vram, &palette, &oam);
        let red = |x: usize, y: usize| buf[(y * 240 + x) * 4] & 0xF8 == 0xF8;
        assert!(red(0, 0), "inside the OBJ window BG0 is enabled -> red");
        assert!(red(7, 7), "OBJ window covers the 8x8 sprite");
        assert!(!red(10, 10), "outside the OBJ window BG0 is masked -> backdrop");
    }

    #[test]
    fn alpha_blend_top_two_bg_layers() {
        let mut lcdc = LCDController::new();
        lcdc.write_halfword(0x0000, 0x0300); // mode 0, BG0 + BG1 on
        // BG0: priority 0, char base 0, map base block 2 (0x1000).
        lcdc.write_halfword(0x0008, 0x0000 | (2 << 8));
        // BG1: priority 1, char base block 1 (0x4000), map base block 3 (0x1800).
        lcdc.write_halfword(0x000A, 0x0001 | (1 << 2) | (3 << 8));
        // BLDCNT: 1st target BG0 (bit0), alpha effect (bits6-7=01), 2nd target BG1 (bit9).
        lcdc.write_halfword(0x0050, 0x0001 | 0x0040 | 0x0200);
        // BLDALPHA: EVA = 16, EVB = 16.
        lcdc.write_halfword(0x0052, 0x10 | (0x10 << 8));

        let mut vram = Ram::new(vec![0; 0x1_8000]);
        let mut palette = Ram::new(vec![0; 0x0400]);
        let oam = Ram::new(vec![0; 0x0400]);
        // BG0 tile 0 = palette index 1 (red); BG1 tile 0 = index 2 (green).
        vram.write_byte(0x0000, 0x11);
        vram.write_byte(0x4000, 0x22);
        vram.write_halfword(0x1000, 0); // BG0 map entry 0
        vram.write_halfword(0x1800, 0); // BG1 map entry 0
        palette.write_halfword(2, 0x001F); // idx1 = red
        palette.write_halfword(4, 0x03E0); // idx2 = green

        let buf = lcdc.render(&vram, &palette, &oam);
        let (r, g, b) = (buf[0], buf[1], buf[2]);
        // With EVA=EVB=16 the red (BG0, 1st) and green (BG1, 2nd) saturate -> yellow.
        assert!(r & 0xF8 == 0xF8, "blended pixel keeps red (got {r:#x})");
        assert!(g & 0xF8 == 0xF8, "blended pixel gains green from the 2nd target (got {g:#x})");
        assert_eq!(b & 0xF8, 0, "no blue (got {b:#x})");
    }

    #[test]
    fn bg_mosaic_snaps_horizontally() {
        let mut lcdc = LCDController::new();
        lcdc.write_halfword(0x0000, 0x0100); // mode 0, BG0 on
        // BG0CNT: mosaic on (bit6), char base block 0, map base block 1 (0x800).
        lcdc.write_halfword(0x0008, 0x0040 | (1 << 8));
        lcdc.write_halfword(0x004C, 0x0001); // MOSAIC: BG h-size = 2

        let mut vram = Ram::new(vec![0; 0x1_8000]);
        let mut palette = Ram::new(vec![0; 0x0400]);
        let oam = Ram::new(vec![0; 0x0400]);
        // Map entry (0,0) -> tile 0 (map base 0x800).
        vram.write_halfword(0x0800, 0);
        // Tile 0 (4bpp) row 0: col0 = idx1, col1 = idx2 (byte 0 = 0x21).
        vram.write_byte(0x0000, 0x21);
        palette.write_halfword(2, 0x001F); // idx1 = red
        palette.write_halfword(4, 0x03E0); // idx2 = green

        let buf = lcdc.render(&vram, &palette, &oam);
        let r = |x: usize| buf[(x) * 4]; // red channel of pixel (x,0)
        let g = |x: usize| buf[(x) * 4 + 1];
        // With h-mosaic 2, pixel 1 samples pixel 0 -> both red, no green.
        assert!(r(0) & 0xF8 == 0xF8 && g(0) == 0, "pixel 0 is red");
        assert!(r(1) & 0xF8 == 0xF8 && g(1) == 0, "pixel 1 mosaics to red, not green");
    }

    #[test]
    fn obj_size_table() {
        assert_eq!(LCDController::obj_size(0, 0), (8, 8));
        assert_eq!(LCDController::obj_size(0, 3), (64, 64));
        assert_eq!(LCDController::obj_size(1, 0), (16, 8));
        assert_eq!(LCDController::obj_size(2, 2), (16, 32));
    }

    #[test]
    fn render_affine_bg2_identity() {
        let mut lcdc = LCDController::new();
        // Mode 2, BG2 display on.
        lcdc.write_halfword(0x0000, 0x0002 | 0x0400);
        // BG2CNT: char base block 1 (0x4000), map base block 0, screen size 0 (128x128).
        lcdc.write_halfword(0x000C, 0x0004);
        // Identity affine matrix, zero reference point (defaults already set, but be explicit).
        lcdc.write_halfword(0x0020, 0x0100); // PA = 1.0
        lcdc.write_halfword(0x0026, 0x0100); // PD = 1.0
        lcdc.write_word(0x0028, 0); // BG2X
        lcdc.write_word(0x002C, 0); // BG2Y

        let mut vram = Ram::new(vec![0; 0x1_8000]);
        let mut palette = Ram::new(vec![0; 0x0400]);
        let oam = Ram::new(vec![0; 0x0400]);

        // Affine map entry (0,0) -> tile number 1 (1 byte per entry, map base 0).
        vram.write_byte(0x0000, 1);
        // Tile 1 (8bpp) at char base 0x4000 + 1*64 = 0x4040: fill with palette index 1.
        for i in 0..64u32 {
            vram.write_byte(0x4040 + i, 1);
        }
        // BG palette index 1 = red.
        palette.write_halfword(2, 0x001F);

        let buf = lcdc.render(&vram, &palette, &oam);
        let px = |x: usize, y: usize| {
            let i = (y * 240 + x) * 4;
            (buf[i], buf[i + 1], buf[i + 2])
        };
        assert_eq!(px(0, 0).0 & 0xF8, 0xF8, "affine BG2 pixel should be red");
        assert_eq!(px(7, 7).0 & 0xF8, 0xF8, "tile (0,0) covers 8x8");
    }

    #[test]
    fn render_obj_draws_a_4bpp_sprite() {
        let mut lcdc = LCDController::new();
        // Enable OBJ + 1D mapping, mode 0.
        lcdc.write_halfword(0x0000, 0x1000 | 0x0040);

        let mut vram = Ram::new(vec![0; 0x1_8000]);
        let mut palette = Ram::new(vec![0; 0x0400]);
        let mut oam = Ram::new(vec![0; 0x0400]);

        // OBJ tile 0 at VRAM 0x10000: fill an 8x8 4bpp tile with palette index 1.
        // Each byte packs two 4-bit pixels (0x11 = two pixels of index 1).
        for i in 0..32u32 {
            vram.write_byte(0x1_0000 + i, 0x11);
        }
        // OBJ palette entry 1 (palette 0x200 + 1*2) = red (BGR555 0x001F).
        palette.write_halfword(0x200 + 2, 0x001F);

        // OAM sprite 0: y=0, x=0, 8x8 square, 4bpp, tile 0, priority 0.
        oam.write_halfword(0, 0x0000); // attr0: y=0, normal, square
        oam.write_halfword(2, 0x0000); // attr1: x=0, size 0
        oam.write_halfword(4, 0x0000); // attr2: tile 0, prio 0, palbank 0

        let buf = lcdc.render(&vram, &palette, &oam);
        // Top-left 8x8 should be red; pixel at (10,10) should be backdrop (0).
        let px = |x: usize, y: usize| {
            let i = (y * 240 + x) * 4;
            (buf[i], buf[i + 1], buf[i + 2])
        };
        assert_eq!(px(0, 0).0 & 0xF8, 0xF8, "sprite pixel should be red");
        assert_eq!(px(7, 7).0 & 0xF8, 0xF8, "sprite covers 8x8");
        assert_eq!(px(10, 10), (0, 0, 0), "outside the sprite is backdrop");
    }
}
