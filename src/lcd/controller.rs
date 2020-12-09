use crate::memory::ram::Ram;
use crate::memory::readable::*;
use crate::types::*;

use super::*;

// Visible     240 dots,  57.221 us,    960 cycles - 78% of h-time
// H-Blanking   68 dots,  16.212 us,    272 cycles - 22% of h-time
// Total       308 dots,  73.433 us,   1232 cycles - ca. 13.620 kHz
const CYCLES_PER_LINE: usize = 1232;
// Visible (*) 160 lines, 11.749 ms, 197120 cycles - 70% of v-time
// V-Blanking   68 lines,  4.994 ms,  83776 cycles - 30% of v-time
// Total       228 lines, 16.743 ms, 280896 cycles - ca. 59.737 Hz
const CYCLES_PER_FRAME: usize = 280896;
const LINES_PER_FRAME: usize = 228;

const DISPLAY_TILE_WIDTH: Word = 30;
const DISPLAY_TILE_HEIGHT: Word = 20;

const VIRTUAL_DISPLAY_TILE_WIDTH: Word = 32;
const VIRTUAL_DISPLAY_TILE_HEIGHT: Word = 32;

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
            bg0cnt: BGCNT::new(),
            bg1cnt: BGCNT::new(),
            bg2cnt: BGCNT::new(),
            bg3cnt: BGCNT::new(),
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
            0x0008 => self.bg0cnt.write(data),
            0x000A => self.bg1cnt.write(data),
            0x000C => self.bg2cnt.write(data),
            0x000E => self.bg3cnt.write(data),
            _ => todo!(),
        }
    }

    pub fn write_word(&mut self, addr: Word, data: Word) {
        let data = data as HalfWord;
        match addr {
            0x0000 => self.dispcnt.write(data),
            0x0008 => self.bg0cnt.write(data),
            0x000A => self.bg1cnt.write(data),
            0x000C => self.bg2cnt.write(data),
            0x000E => self.bg3cnt.write(data),
            _ => todo!(),
        }
    }

    pub fn render(&self, vram: &Ram, palette: &Ram, oam: &Ram) -> Vec<u8> {
        match self.dispcnt.mode() {
            BgMode::Mode0 => self.render_with_mode0(vram, palette),
            BgMode::Mode3 => self.render_with_mode3(vram),
            BgMode::Mode4 => self.render_with_mode0(vram, palette),
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
}
