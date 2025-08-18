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
        }
    }

    pub fn run(&mut self, cycles: usize) -> bool {
        self.cycles += cycles;

        loop {
            if self.cycles < CYCLES_PER_LINE {
                return false;
            }
            self.cycles -= CYCLES_PER_LINE;
            self.lines += 1;

            if self.lines >= LINES_PER_FRAME {
                self.lines -= LINES_PER_FRAME;
                return true;
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

        // TODO: We need to consider about scroll?
        for tile_y in 0..DISPLAY_TILE_HEIGHT {
            for tile_x in 0..DISPLAY_TILE_WIDTH {
                let addr = tile_y as Word * (VIRTUAL_DISPLAY_TILE_HEIGHT * 2) + (tile_x * 2) + map_offset;
                let tile_index = vram.read_halfword(addr) as Word;

                let base = (tile_y * 240 * 8 + tile_x * 8) * 4;

                for y in 0..8 {
                    for x in 0..8 {
                        let base = (base + (y * 240 + x) * 4) as usize;
                        let palette_index = vram.read_byte(tile_offset + tile_index * 64 + y * 8 + x);
                        let color = BGR::new(palette.read_halfword(palette_index as Word * 2));
                        buf[base] = color.red();
                        buf[base + 1] = color.green();
                        buf[base + 2] = color.blue();
                        buf[base + 3] = 0xFF;
                    }
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
