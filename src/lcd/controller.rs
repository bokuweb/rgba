use crate::memory::ram::Ram;
use crate::memory::readable::*;
use crate::types::*;

use super::constants::*;
use super::*;
use crate::gba::interrupt::{InterruptController, InterruptType};

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
        }
    }

    pub fn run(&mut self, cycles: usize, interrupt_controller: &mut InterruptController) -> bool {
        self.cycles += cycles;

        loop {
            if self.cycles < CYCLES_PER_LINE {
                return false;
            }
            self.cycles -= CYCLES_PER_LINE;
            let old_lines = self.lines;
            self.lines += 1;

            // Check for VBlank start (line 160)
            if old_lines == 159 && self.lines == 160 {
                // VBlank started - request VBlank interrupt if enabled
                if self.dispstat.vblank_irq_enable() {
                    interrupt_controller.request_interrupt(InterruptType::VBlank);
                }
            }

            // Check for HBlank interrupt
            if self.dispstat.hblank_irq_enable() {
                interrupt_controller.request_interrupt(InterruptType::HBlank);
            }

            // Check for VCounter match interrupt
            if self.dispstat.vcounter_irq_enable() && self.dispstat.vcount_setting() == self.lines as u16 {
                interrupt_controller.request_interrupt(InterruptType::VCounter);
            }

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
            _ => {
                // Silently return 0 for unimplemented registers
                0
            }
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
            _ => {
                // Silently return 0 for unimplemented registers  
                0
            }
        }
    }

    pub fn write_halfword(&mut self, addr: Word, data: HalfWord) {
        match addr {
            0x0000 => self.dispcnt.write(data),
            0x0004 => self.dispstat.write(data),
            0x0008 => self.bg0cnt.write(data),
            0x000A => self.bg1cnt.write(data),
            0x000C => self.bg2cnt.write(data),
            0x000E => self.bg3cnt.write(data),
            // BG scroll offsets - implement as dummy registers
            0x0010 => {}, // BG0HOFS - BG0 horizontal offset
            0x0012 => {}, // BG0VOFS - BG0 vertical offset  
            0x0014 => {}, // BG1HOFS - BG1 horizontal offset
            0x0016 => {}, // BG1VOFS - BG1 vertical offset
            0x0018 => {}, // BG2HOFS - BG2 horizontal offset
            0x001A => {}, // BG2VOFS - BG2 vertical offset
            0x001C => {}, // BG3HOFS - BG3 horizontal offset
            0x001E => {}, // BG3VOFS - BG3 vertical offset
            // BG rotation/scaling parameters - implement as dummy registers
            0x0020 => {}, // BG2PA - BG2 rotation/scaling parameter A
            0x0022 => {}, // BG2PB - BG2 rotation/scaling parameter B
            0x0024 => {}, // BG2PC - BG2 rotation/scaling parameter C
            0x0026 => {}, // BG2PD - BG2 rotation/scaling parameter D
            0x0030 => {}, // BG3PA - BG3 rotation/scaling parameter A
            0x0032 => {}, // BG3PB - BG3 rotation/scaling parameter B
            0x0034 => {}, // BG3PC - BG3 rotation/scaling parameter C
            0x0036 => {}, // BG3PD - BG3 rotation/scaling parameter D
            // Window control registers - implement as dummy registers
            0x0040 => {}, // WIN0H - Window 0 horizontal dimensions
            0x0042 => {}, // WIN1H - Window 1 horizontal dimensions
            0x0044 => {}, // WIN0V - Window 0 vertical dimensions
            0x0046 => {}, // WIN1V - Window 1 vertical dimensions
            0x0048 => {}, // WININ - Control of inside of window(s)
            0x004A => {}, // WINOUT - Control of outside of windows & inside of OBJ window
            0x004C => {}, // MOSAIC - Mosaic size
            0x0050 => {}, // BLDCNT - Color special effects selection
            0x0052 => {}, // BLDALPHA - Alpha blending coefficients
            0x0054 => {}, // BLDY - Brightness (fade-in/out) coefficient
            _ => {
                // Silently ignore other unimplemented registers to prevent spam
            }
        }
    }

    pub fn write_word(&mut self, addr: Word, data: Word) {
        match addr {
            0x0000 => self.dispcnt.write(data as HalfWord),
            0x0004 => self.dispstat.write(data as HalfWord),
            0x0008 => self.bg0cnt.write(data as HalfWord),
            0x000A => self.bg1cnt.write(data as HalfWord),
            0x000C => self.bg2cnt.write(data as HalfWord),
            0x000E => self.bg3cnt.write(data as HalfWord),
            // BG reference point coordinates - implement as dummy registers
            0x0028 => {}, // BG2X - BG2 reference point X coordinate
            0x002C => {}, // BG2Y - BG2 reference point Y coordinate  
            0x0038 => {}, // BG3X - BG3 reference point X coordinate
            0x003C => {}, // BG3Y - BG3 reference point Y coordinate
            _ => {
                // Silently ignore other unimplemented registers to prevent spam
            }
        }
    }

    pub fn render(&self, vram: &Ram, palette: &Ram, oam: &Ram) -> Vec<u8> {
        println!("Render called - DISPCNT: 0x{:04x}, Mode: {:?}, Forced blank: {}", 
                 self.dispcnt.read(), self.dispcnt.mode(), self.dispcnt.forced_vlank());
        
        // Check if forced blank is enabled
        if self.dispcnt.forced_vlank() {
            println!("Forced blank enabled - returning black screen");
            // Return blank screen (black)
            return vec![0; 240 * 160 * 4];
        }
        
        match self.dispcnt.mode() {
            BgMode::Mode0 => {
                println!("Rendering with Mode 0");
                self.render_with_mode0(vram, palette)
            },
            BgMode::Mode3 => {
                println!("Rendering with Mode 3");
                self.render_with_mode3(vram)
            },
            BgMode::Mode4 => {
                println!("Rendering with Mode 4");
                self.render_with_mode4(vram, palette)
            },
            _ => {
                println!("Unsupported mode: {:?} - returning black screen", self.dispcnt.mode());
                // For unsupported modes, return blank screen
                vec![0; 240 * 160 * 4]
            }
        }
    }

    fn render_with_mode0(&self, vram: &Ram, palette: &Ram) -> Vec<u8> {
        let mut buf = vec![0; 240 * 160 * 4];
        let tile_offset = self.bg0cnt.bg_tile_offset();
        let map_offset = self.bg0cnt.bg_map_offset();
        
        println!("Mode0 render: tile_offset=0x{:08x}, map_offset=0x{:08x}", tile_offset, map_offset);
        
        // Check first few tiles and palette entries
        let first_tile_index = vram.read_halfword(map_offset) as Word;
        let second_tile_index = vram.read_halfword(map_offset + 2) as Word;
        let third_tile_index = vram.read_halfword(map_offset + 4) as Word;
        
        // Check more palette entries
        let mut palette_summary = String::new();
        for i in 0..8 {
            let color = palette.read_halfword(i * 2);
            if color != 0 {
                palette_summary.push_str(&format!(" P{}: 0x{:04x}", i, color));
            }
        }
        
        println!("Tiles: [0x{:04x}, 0x{:04x}, 0x{:04x}], Non-zero palette entries:{}", 
                 first_tile_index, second_tile_index, third_tile_index, palette_summary);

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
        
        // Debug: Check if we have any non-black pixels from actual rendering
        let mut non_black_pixels = 0;
        for i in (0..buf.len()).step_by(4) {
            if buf[i] != 0 || buf[i+1] != 0 || buf[i+2] != 0 {
                non_black_pixels += 1;
            }
        }
        println!("Non-black pixels from actual rendering: {}", non_black_pixels);
        
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
