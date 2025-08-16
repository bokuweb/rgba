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
    // Edge flags for DMA timing
    vblank_edge_triggered: bool,
    hblank_edges: u32,
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
            vblank_edge_triggered: false,
            hblank_edges: 0,
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
                // Always request VBlank interrupt to help ROM progress (even if IRQ disabled)
                interrupt_controller.request_interrupt(InterruptType::VBlank);
                println!("VBlank interrupt requested (line 159->160)");
                // Record VBlank rising edge for DMA timing
                self.vblank_edge_triggered = true;
            }

            // HBlank edge occurs at end of each visible scanline
            if old_lines < 160 {
                // Count HBlank edges for DMA timing regardless of IRQ enable
                self.hblank_edges = self.hblank_edges.saturating_add(1);
                // If HBlank IRQ is enabled, request it
                if self.dispstat.hblank_irq_enable() {
                    interrupt_controller.request_interrupt(InterruptType::HBlank);
                }
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

    pub fn take_vblank_edge(&mut self) -> bool {
        let e = self.vblank_edge_triggered;
        self.vblank_edge_triggered = false;
        e
    }

    pub fn take_hblank_edges(&mut self) -> u32 {
        let n = self.hblank_edges;
        self.hblank_edges = 0;
        n
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

        // レイヤの定義（優先度低→高の順に描画し、パレットインデックス0は透過扱い）
        let layers = [
            (self.bg3cnt, self.dispcnt.screen_display_bg3(), "BG3"),
            (self.bg2cnt, self.dispcnt.screen_display_bg2(), "BG2"),
            (self.bg1cnt, self.dispcnt.screen_display_bg1(), "BG1"),
            (self.bg0cnt, self.dispcnt.screen_display_bg0(), "BG0"),
        ];

        for (bgcnt, enabled, name) in layers {
            if !enabled { continue; }

            let tile_base = bgcnt.bg_tile_offset();
            let map_base = bgcnt.bg_map_offset();
            let is_8bpp = bgcnt.colors_palettes();
            println!(
                "Mode0 render layer {}: tile_base=0x{:08x}, map_base=0x{:08x}, bpp={}",
                name,
                tile_base,
                map_base,
                if is_8bpp { "8bpp" } else { "4bpp" }
            );

            for tile_y in 0..DISPLAY_TILE_HEIGHT {
                for tile_x in 0..DISPLAY_TILE_WIDTH {
                    let screen_entry_addr = map_base
                        + (tile_y as Word) * (VIRTUAL_DISPLAY_TILE_HEIGHT * 2)
                        + (tile_x as Word) * 2;
                    let entry = vram.read_halfword(screen_entry_addr);

                    let tile_number: Word = (entry & 0x03FF) as Word;
                    let hflip = (entry & (1 << 10)) != 0;
                    let vflip = (entry & (1 << 11)) != 0;
                    let palette_bank: Word = ((entry >> 12) & 0xF) as Word;

                    let dst_line_base = (tile_y * 240 * 8 + tile_x * 8) as usize;
                    for y in 0..8 {
                        let ty = if vflip { 7 - y } else { y } as Word;
                        for x in 0..8 {
                            let tx = if hflip { 7 - x } else { x } as Word;
                            let dst_idx = (dst_line_base + (y * 240 + x)) * 4;

                            let palette_index: Word = if is_8bpp {
                                let addr = tile_base + tile_number * 64 + ty * 8 + tx;
                                vram.read_byte(addr) as Word
                            } else {
                                let byte_addr = tile_base + tile_number * 32 + ty * 4 + (tx / 2);
                                let b = vram.read_byte(byte_addr);
                                let nibble = if (tx & 1) == 0 { b & 0x0F } else { (b >> 4) & 0x0F };
                                (palette_bank * 16 + nibble as Word) as Word
                            };

                            if palette_index == 0 { continue; }
                            let color = BGR::new(palette.read_halfword(palette_index * 2));
                            buf[dst_idx] = color.red();
                            buf[dst_idx + 1] = color.green();
                            buf[dst_idx + 2] = color.blue();
                            buf[dst_idx + 3] = 0xFF;
                        }
                    }
                }
            }
        }

        let mut non_black_pixels = 0;
        for i in (0..buf.len()).step_by(4) {
            if buf[i] != 0 || buf[i + 1] != 0 || buf[i + 2] != 0 {
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
