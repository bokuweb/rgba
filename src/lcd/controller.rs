use crate::memory::ram::Ram;
use crate::memory::readable::*;
use crate::types::*;

use super::constants::*;
use super::*;

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
                    println!("🔥 VBlank IRQ requested! (lines: {} -> {})", prev_lines, self.lines);
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
            0x0004 => self.dispstat.read(self.cycles, self.lines),
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
            _ => todo!(),
        }
    }

    pub fn read_word(&self, addr: Word) -> Word {
        match addr {
            0x0000 => self.dispcnt.read() as Word,
            0x0004 => self.dispstat.read(self.cycles, self.lines) as Word,
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
            0x0004 => {
                // DISPSTAT - General LCD Status (Read/Write)
                self.dispstat.write(data);
                println!(
                    "🔧 DISPSTAT write: 0x{:04x} -> masked: 0x{:04x} (VBlank IRQ: {}, HBlank IRQ: {}, VCounter IRQ: {})",
                    data,
                    self.dispstat.0 & 0x0038,
                    (data & 0x0008) != 0,
                    (data & 0x0010) != 0,
                    (data & 0x0020) != 0
                );
            }
            0x0008 => {
                self.bg0cnt.write(data);
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
            0x000A => {
                self.bg1cnt.write(data);
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
            0x000C => {
                self.bg2cnt.write(data);
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
            0x000E => {
                self.bg3cnt.write(data);
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
                let bg_h_size = data & 0x000F; // Bit 0-3: BG Mosaic H-Size (minus 1)
                let bg_v_size = (data & 0x00F0) >> 4; // Bit 4-7: BG Mosaic V-Size (minus 1)
                let obj_h_size = (data & 0x0F00) >> 8; // Bit 8-11: OBJ Mosaic H-Size (minus 1)
                let obj_v_size = (data & 0xF000) >> 12; // Bit 12-15: OBJ Mosaic V-Size (minus 1)
                println!(
                    "MOSAIC register write: 0x{:04x} (BG H:{} V:{}, OBJ H:{} V:{})",
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
                println!("🔧 DISPSTAT word write: 0x{:04x} -> masked: 0x{:04x}", data, self.dispstat.0 & 0x0038);
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
    fn get_window_control(&self, x: Word, y: Word) -> (bool, bool, bool, bool, bool, bool) {
        // Check which window the pixel is in (Window 0 has highest priority)
        let in_win0 = self.is_pixel_in_window(x, y, self.win0h, self.win0v);
        let in_win1 = self.is_pixel_in_window(x, y, self.win1h, self.win1v);

        // For now, we don't implement OBJ Window, so in_objwin is always false
        let in_objwin = false;

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

    // Helper function to apply color special effects
    fn apply_color_effect(&self, color: BGR, layer_id: u8, effect_enable: bool) -> BGR {
        if !effect_enable {
            return color;
        }

        let first_target = self.bldcnt & 0x003F; // Bit 0-5: 1st Target
        let effect_type = (self.bldcnt >> 6) & 0x0003; // Bit 6-7: Effect Type
        let second_target = (self.bldcnt >> 8) & 0x003F; // Bit 8-13: 2nd Target

        // Check if current layer is a first target
        let is_first_target = (first_target & (1 << layer_id)) != 0;

        if !is_first_target {
            return color; // No effect if not a first target
        }

        match effect_type {
            0 => color, // None - no effect
            1 => {
                // Alpha Blending - blend with backdrop color as a simple demonstration
                // In a full implementation, this would blend with the actual 2nd target layer
                let second_target = (self.bldcnt >> 8) & 0x003F; // Bit 8-13: 2nd Target
                let backdrop_is_second_target = (second_target & 0x0020) != 0; // BD bit

                if backdrop_is_second_target {
                    // Create a simple backdrop color (dark gray)
                    let backdrop_color = BGR::new(0x4210); // Dark gray in BGR555 format
                    self.alpha_blend(color, backdrop_color)
                } else {
                    // No valid 2nd target, return original color
                    color
                }
            }
            2 => {
                // Brightness Increase (fade to white)
                // Formula: I = I1st + (31-I1st)*EVY/16
                let evy = self.bldy & 0x001F; // Bit 0-4: EVY Coefficient
                let evy_clamped = if evy > 16 { 16 } else { evy };

                // Extract RGB components (convert from 8-bit back to 5-bit)
                let r1 = (color.red() >> 3) as u16; // Convert 8-bit to 5-bit
                let g1 = (color.green() >> 3) as u16;
                let b1 = (color.blue() >> 3) as u16;

                // Apply brightness increase formula: I = I1st + (31-I1st)*EVY/16
                let r_bright = r1 + ((31 - r1) * evy_clamped / 16);
                let g_bright = g1 + ((31 - g1) * evy_clamped / 16);
                let b_bright = b1 + ((31 - b1) * evy_clamped / 16);

                // Clamp to 5-bit range and convert back to BGR format
                let r_final = r_bright.min(31);
                let g_final = g_bright.min(31);
                let b_final = b_bright.min(31);

                let bgr_value = (b_final << 10) | (g_final << 5) | r_final;
                BGR::new(bgr_value as HalfWord)
            }
            3 => {
                // Brightness Decrease (fade to black)
                // Formula: I = I1st - I1st*EVY/16
                let evy = self.bldy & 0x001F; // Bit 0-4: EVY Coefficient
                let evy_clamped = if evy > 16 { 16 } else { evy };

                // Extract RGB components (convert from 8-bit back to 5-bit)
                let r1 = (color.red() >> 3) as u16; // Convert 8-bit to 5-bit
                let g1 = (color.green() >> 3) as u16;
                let b1 = (color.blue() >> 3) as u16;

                // Apply brightness decrease formula: I = I1st - I1st*EVY/16
                let r_dark = r1 - (r1 * evy_clamped / 16);
                let g_dark = g1 - (g1 * evy_clamped / 16);
                let b_dark = b1 - (b1 * evy_clamped / 16);

                // Convert back to BGR format (no need to clamp as subtraction won't exceed range)
                let bgr_value = (b_dark << 10) | (g_dark << 5) | r_dark;
                BGR::new(bgr_value as HalfWord)
            }
            _ => color, // Unknown effect
        }
    }

    pub fn render(&self, vram: &Ram, palette: &Ram, oam: &Ram) -> Vec<u8> {
        match self.dispcnt.mode() {
            BgMode::Mode0 => self.render_with_mode0(vram, palette),
            BgMode::Mode3 => self.render_with_mode3(vram),
            BgMode::Mode4 => self.render_with_mode4(vram, palette),
            _ => todo!(),
        }
    }

    fn render_with_mode0(&self, vram: &Ram, palette: &Ram) -> Vec<u8> {
        let mut buf = vec![0; 240 * 160 * 4];
        let tile_offset = self.bg0cnt.bg_tile_offset();
        let map_offset = self.bg0cnt.bg_map_offset();

        // Apply scroll offsets (9-bit values, can be 0-511)
        let scroll_x = self.bg0hofs as Word;
        let scroll_y = self.bg0vofs as Word;

        // Render pixel by pixel with scroll support and proper window control
        for screen_y in 0..160 {
            for screen_x in 0..240 {
                // Get window control for this pixel
                let (bg0_enable, _bg1_enable, _bg2_enable, _bg3_enable, _obj_enable, _effect_enable) = self.get_window_control(screen_x, screen_y);

                let buf_index = ((screen_y * 240 + screen_x) * 4) as usize;

                if bg0_enable {
                    // BG0 is enabled for this pixel - render normally
                    // Calculate the actual position in the background map considering scroll
                    let bg_x = (screen_x + scroll_x) & 0x1FF; // Wrap at 512 pixels (64 tiles * 8 pixels)
                    let bg_y = (screen_y + scroll_y) & 0x1FF; // Wrap at 512 pixels (64 tiles * 8 pixels)

                    // Convert pixel coordinates to tile coordinates
                    let tile_x = bg_x / 8;
                    let tile_y = bg_y / 8;
                    let pixel_x = bg_x % 8;
                    let pixel_y = bg_y % 8;

                    // Calculate tile map address (32x32 tile map)
                    let tile_map_addr = (tile_y * VIRTUAL_DISPLAY_TILE_WIDTH + tile_x) * 2 + map_offset;
                    let tile_index = vram.read_halfword(tile_map_addr) as Word;

                    // Calculate pixel address within the tile
                    let pixel_addr = tile_offset + tile_index * 64 + pixel_y * 8 + pixel_x;
                    let palette_index = vram.read_byte(pixel_addr);

                    // Skip transparent pixels (palette index 0)
                    if palette_index != 0 {
                        // Get color from palette
                        let mut color = BGR::new(palette.read_halfword(palette_index as Word * 2));

                        // Apply color special effects (BG0 = layer_id 0)
                        color = self.apply_color_effect(color, 0, _effect_enable);

                        // Set pixel in output buffer
                        buf[buf_index] = color.red();
                        buf[buf_index + 1] = color.green();
                        buf[buf_index + 2] = color.blue();
                        buf[buf_index + 3] = 0xFF;
                    } else {
                        // Transparent pixel - render backdrop color (palette index 0)
                        let mut backdrop_color = BGR::new(palette.read_halfword(0));

                        // Apply color special effects to backdrop (BD = layer_id 5)
                        backdrop_color = self.apply_color_effect(backdrop_color, 5, _effect_enable);

                        buf[buf_index] = backdrop_color.red();
                        buf[buf_index + 1] = backdrop_color.green();
                        buf[buf_index + 2] = backdrop_color.blue();
                        buf[buf_index + 3] = 0xFF;
                    }
                } else {
                    // BG0 is disabled for this pixel - render backdrop color or black
                    // In a full implementation, other layers (BG1-3, OBJ) would be checked here
                    buf[buf_index] = 0x00; // Black
                    buf[buf_index + 1] = 0x00;
                    buf[buf_index + 2] = 0x00;
                    buf[buf_index + 3] = 0xFF;
                }
            }
        }
        buf
    }

    fn render_with_mode3(&self, vram: &Ram) -> Vec<u8> {
        let mut buf = vec![];
        for offset in 0..(240 * 160) {
            let p = vram.read_halfword(offset * 2);
            buf.push((((p & 0x001F) as f32 / 0x1F as f32) * 0xFF as f32) as u8);
            buf.push((((p & 0x03E0).wrapping_shr(5) as f32 / 0x1F as f32) * 0xFF as f32) as u8);
            buf.push((((p & 0xEC00).wrapping_shr(10) as f32 / 0x1F as f32) * 0xFF as f32) as u8);
            buf.push(255);
        }
        buf
    }

    fn render_with_mode4(&self, vram: &Ram, palette: &Ram) -> Vec<u8> {
        let mut buf = vec![];
        let is_frame1 = matches!(self.dispcnt.frame(), Frame::Frame1);
        let offset = if is_frame1 { 0xA000 } else { 0x0000 };
        for addr in 0..(240 * 160) {
            let palette_index = vram.read_byte(addr + offset);
            let color = BGR::new(palette.read_halfword(palette_index as Word * 2));
            buf.push(color.red());
            buf.push(color.green());
            buf.push(color.blue());
            buf.push(0xFF);
        }
        buf
    }
}
