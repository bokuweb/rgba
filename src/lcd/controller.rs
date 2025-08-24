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
    mosaic: HalfWord,  // MOSAIC Size register (0x400004C)
    // BG2/BG3 reference point registers (32-bit)
    bg2x: Word,        // BG2 Reference Point X-Coordinate (0x4000028-0x400002A)
    bg2y: Word,        // BG2 Reference Point Y-Coordinate (0x400002C-0x400002E)
    // BG2 rotation/scaling parameters (16-bit each)
    bg2pa: HalfWord,   // BG2 Rotation/Scaling Parameter A (dx) (0x4000020)
    bg2pb: HalfWord,   // BG2 Rotation/Scaling Parameter B (dmx) (0x4000022)
    bg2pc: HalfWord,   // BG2 Rotation/Scaling Parameter C (dy) (0x4000024)
    bg2pd: HalfWord,   // BG2 Rotation/Scaling Parameter D (dmy) (0x4000026)
    // BG3 reference point registers (32-bit)
    bg3x: Word,        // BG3 Reference Point X-Coordinate (0x4000038-0x400003A)
    bg3y: Word,        // BG3 Reference Point Y-Coordinate (0x400003C-0x400003E)
    // BG3 rotation/scaling parameters (16-bit each)
    bg3pa: HalfWord,   // BG3 Rotation/Scaling Parameter A (dx) (0x4000030)
    bg3pb: HalfWord,   // BG3 Rotation/Scaling Parameter B (dmx) (0x4000032)
    bg3pc: HalfWord,   // BG3 Rotation/Scaling Parameter C (dy) (0x4000034)
    bg3pd: HalfWord,   // BG3 Rotation/Scaling Parameter D (dmy) (0x4000036)
    // Window registers
    win0h: HalfWord,   // WIN0H - Window 0 Horizontal Dimensions (0x4000040)
    win1h: HalfWord,   // WIN1H - Window 1 Horizontal Dimensions (0x4000042)
    win0v: HalfWord,   // WIN0V - Window 0 Vertical Dimensions (0x4000044)
    win1v: HalfWord,   // WIN1V - Window 1 Vertical Dimensions (0x4000046)
    winin: HalfWord,   // WININ - Inside of Window 0 and 1 (0x4000048)
    winout: HalfWord,  // WINOUT - Outside of Windows & Inside of Window OBJ (0x400004A)
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
            win0h: 0,      // WIN0H - Window 0 Horizontal Dimensions
            win1h: 0,      // WIN1H - Window 1 Horizontal Dimensions
            win0v: 0,      // WIN0V - Window 0 Vertical Dimensions
            win1v: 0,      // WIN1V - Window 1 Vertical Dimensions
            winin: 0,      // WININ - Inside of Window 0 and 1
            winout: 0,     // WINOUT - Outside of Windows & Inside of Window OBJ
        }
    }

    pub fn run(&mut self, cycles: usize) -> (bool, bool) {
        self.cycles += cycles;
        let mut vblank_entered = false;

        loop {
            if self.cycles < CYCLES_PER_LINE {
                return (false, vblank_entered);
            }
            self.cycles -= CYCLES_PER_LINE;
            let prev_lines = self.lines;
            self.lines += 1;

            // Debug: Print line progression
            if self.lines % 50 == 0 || self.lines >= 158 {
                println!("🔧 LCD line: {} -> {}", prev_lines, self.lines);
            }

            // Check for VBlank entry (line 160)
            if prev_lines == 159 && self.lines == 160 {
                vblank_entered = true;
                println!("🔥 VBlank entered at line 160!");
            }

            if self.lines >= LINES_PER_FRAME {
                println!("🔧 Frame completed: lines {} -> 0", self.lines);
                self.lines -= LINES_PER_FRAME;
                return (true, vblank_entered);
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
            0x0020 => self.bg2pa, // BG2PA - BG2 Rotation/Scaling Parameter A (dx)
            0x0022 => self.bg2pb, // BG2PB - BG2 Rotation/Scaling Parameter B (dmx)
            0x0024 => self.bg2pc, // BG2PC - BG2 Rotation/Scaling Parameter C (dy)
            0x0026 => self.bg2pd, // BG2PD - BG2 Rotation/Scaling Parameter D (dmy)
            0x0030 => self.bg3pa, // BG3PA - BG3 Rotation/Scaling Parameter A (dx)
            0x0032 => self.bg3pb, // BG3PB - BG3 Rotation/Scaling Parameter B (dmx)
            0x0034 => self.bg3pc, // BG3PC - BG3 Rotation/Scaling Parameter C (dy)
            0x0036 => self.bg3pd, // BG3PD - BG3 Rotation/Scaling Parameter D (dmy)
            0x0040 => self.win0h, // WIN0H - Window 0 Horizontal Dimensions
            0x0042 => self.win1h, // WIN1H - Window 1 Horizontal Dimensions
            0x0044 => self.win0v, // WIN0V - Window 0 Vertical Dimensions
            0x0046 => self.win1v, // WIN1V - Window 1 Vertical Dimensions
            0x0048 => self.winin, // WININ - Inside of Window 0 and 1
            0x004A => self.winout, // WINOUT - Outside of Windows & Inside of Window OBJ
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
            0x0020 => self.bg2pa as Word, // BG2PA - BG2 Rotation/Scaling Parameter A (dx)
            0x0022 => self.bg2pb as Word, // BG2PB - BG2 Rotation/Scaling Parameter B (dmx)
            0x0024 => self.bg2pc as Word, // BG2PC - BG2 Rotation/Scaling Parameter C (dy)
            0x0026 => self.bg2pd as Word, // BG2PD - BG2 Rotation/Scaling Parameter D (dmy)
            0x0028 => self.bg2x,         // BG2X - BG2 Reference Point X-Coordinate
            0x002C => self.bg2y,         // BG2Y - BG2 Reference Point Y-Coordinate
            0x0030 => self.bg3pa as Word, // BG3PA - BG3 Rotation/Scaling Parameter A (dx)
            0x0032 => self.bg3pb as Word, // BG3PB - BG3 Rotation/Scaling Parameter B (dmx)
            0x0034 => self.bg3pc as Word, // BG3PC - BG3 Rotation/Scaling Parameter C (dy)
            0x0036 => self.bg3pd as Word, // BG3PD - BG3 Rotation/Scaling Parameter D (dmy)
            0x0038 => self.bg3x,         // BG3X - BG3 Reference Point X-Coordinate
            0x003C => self.bg3y,         // BG3Y - BG3 Reference Point Y-Coordinate
            0x0040 => self.win0h as Word, // WIN0H - Window 0 Horizontal Dimensions
            0x0042 => self.win1h as Word, // WIN1H - Window 1 Horizontal Dimensions
            0x0044 => self.win0v as Word, // WIN0V - Window 0 Vertical Dimensions
            0x0046 => self.win1v as Word, // WIN1V - Window 1 Vertical Dimensions
            0x0048 => self.winin as Word, // WININ - Inside of Window 0 and 1
            0x004A => self.winout as Word, // WINOUT - Outside of Windows & Inside of Window OBJ
            _ => todo!(),
        }
    }

    pub fn write_halfword(&mut self, addr: Word, data: HalfWord) {
        match addr {
            0x0000 => self.dispcnt.write(data),
            0x0004 => {
                // DISPSTAT - General LCD Status (Read/Write)
                self.dispstat.write(data);
                println!("🔧 DISPSTAT write: 0x{:04x} -> masked: 0x{:04x} (VBlank IRQ: {}, HBlank IRQ: {}, VCounter IRQ: {})", 
                    data, self.dispstat.0 & 0x0038,
                    (data & 0x0008) != 0, 
                    (data & 0x0010) != 0,
                    (data & 0x0020) != 0
                );
            }
            0x0008 => self.bg0cnt.write(data),
            0x000A => self.bg1cnt.write(data),
            0x000C => self.bg2cnt.write(data),
            0x000E => self.bg3cnt.write(data),
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
                let x1 = (data >> 8) & 0xFF;  // Bit 8-15: X1, Leftmost coordinate
                let x2 = data & 0xFF;         // Bit 0-7: X2, Rightmost coordinate + 1
                // println!("WIN0H write: 0x{:04x} (X1:{}, X2:{})", data, x1, x2);
            }
            0x0042 => {
                // WIN1H - Window 1 Horizontal Dimensions (Write Only)
                self.win1h = data;
                let x1 = (data >> 8) & 0xFF;  // Bit 8-15: X1, Leftmost coordinate
                let x2 = data & 0xFF;         // Bit 0-7: X2, Rightmost coordinate + 1
                // println!("WIN1H write: 0x{:04x} (X1:{}, X2:{})", data, x1, x2);
            }
            0x0044 => {
                // WIN0V - Window 0 Vertical Dimensions (Write Only)
                self.win0v = data;
                let y1 = (data >> 8) & 0xFF;  // Bit 8-15: Y1, Top coordinate
                let y2 = data & 0xFF;         // Bit 0-7: Y2, Bottom coordinate + 1
                // println!("WIN0V write: 0x{:04x} (Y1:{}, Y2:{})", data, y1, y2);
            }
            0x0046 => {
                // WIN1V - Window 1 Vertical Dimensions (Write Only)
                self.win1v = data;
                let y1 = (data >> 8) & 0xFF;  // Bit 8-15: Y1, Top coordinate
                let y2 = data & 0xFF;         // Bit 0-7: Y2, Bottom coordinate + 1
                // println!("WIN1V write: 0x{:04x} (Y1:{}, Y2:{})", data, y1, y2);
            }
            0x0048 => {
                // WININ - Inside of Window 0 and 1 (Write Only)
                self.winin = data;
                // Bit 0-5: Win0 BG0-3, OBJ, Color Special Effect
                // Bit 8-13: Win1 BG0-3, OBJ, Color Special Effect
                // println!("WININ write: 0x{:04x}", data);
            }
            0x004A => {
                // WINOUT - Outside of Windows & Inside of Window OBJ (Write Only)
                self.winout = data;
                // Bit 0-5: Outside Window BG0-3, OBJ, Color Special Effect
                // Bit 8-13: Inside OBJ Window BG0-3, OBJ, Color Special Effect
                // println!("WINOUT write: 0x{:04x}", data);
            }
            0x004C => {
                // MOSAIC - Mosaic Size (Write Only)
                self.mosaic = data;
                let bg_h_size = data & 0x000F;           // Bit 0-3: BG Mosaic H-Size (minus 1)
                let bg_v_size = (data & 0x00F0) >> 4;   // Bit 4-7: BG Mosaic V-Size (minus 1)
                let obj_h_size = (data & 0x0F00) >> 8;  // Bit 8-11: OBJ Mosaic H-Size (minus 1)
                let obj_v_size = (data & 0xF000) >> 12; // Bit 12-15: OBJ Mosaic V-Size (minus 1)
                println!("MOSAIC register write: 0x{:04x} (BG H:{} V:{}, OBJ H:{} V:{})", 
                    data, bg_h_size + 1, bg_v_size + 1, obj_h_size + 1, obj_v_size + 1);
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
                let x1 = (data >> 8) & 0xFF;  // Bit 8-15: X1, Leftmost coordinate
                let x2 = data & 0xFF;         // Bit 0-7: X2, Rightmost coordinate + 1
            }
            0x0042 => {
                // WIN1H - Window 1 Horizontal Dimensions (Write Only)
                self.win1h = data;
                let x1 = (data >> 8) & 0xFF;  // Bit 8-15: X1, Leftmost coordinate
                let x2 = data & 0xFF;         // Bit 0-7: X2, Rightmost coordinate + 1
            }
            0x0044 => {
                // WIN0V - Window 0 Vertical Dimensions (Write Only)
                self.win0v = data;
                let y1 = (data >> 8) & 0xFF;  // Bit 8-15: Y1, Top coordinate
                let y2 = data & 0xFF;         // Bit 0-7: Y2, Bottom coordinate + 1
            }
            0x0046 => {
                // WIN1V - Window 1 Vertical Dimensions (Write Only)
                self.win1v = data;
                let y1 = (data >> 8) & 0xFF;  // Bit 8-15: Y1, Top coordinate
                let y2 = data & 0xFF;         // Bit 0-7: Y2, Bottom coordinate + 1
            }
            0x0048 => {
                // WININ - Inside of Window 0 and 1 (Write Only)
                self.winin = data;
            }
            0x004A => {
                // WINOUT - Outside of Windows & Inside of Window OBJ (Write Only)
                self.winout = data;
            }
            0x004C => {
                // MOSAIC - Mosaic Size (Write Only)
                self.mosaic = data;
                let bg_h_size = data & 0x000F;
                let bg_v_size = (data & 0x00F0) >> 4;
                let obj_h_size = (data & 0x0F00) >> 8;
                let obj_v_size = (data & 0xF000) >> 12;
                println!("MOSAIC word write: 0x{:04x} (BG H:{} V:{}, OBJ H:{} V:{})", 
                    data, bg_h_size + 1, bg_v_size + 1, obj_h_size + 1, obj_v_size + 1);
            }
            _ => {
                todo!("Unhandled LCD register word write: addr=0x{:04x}, data=0x{:08x}", addr, data);
            }
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

        // Render pixel by pixel with scroll support
        for screen_y in 0..160 {
            for screen_x in 0..240 {
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

                // Get color from palette
                let color = BGR::new(palette.read_halfword(palette_index as Word * 2));

                // Set pixel in output buffer
                let buf_index = ((screen_y * 240 + screen_x) * 4) as usize;
                buf[buf_index] = color.red();
                buf[buf_index + 1] = color.green();
                buf[buf_index + 2] = color.blue();
                buf[buf_index + 3] = 0xFF;
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
