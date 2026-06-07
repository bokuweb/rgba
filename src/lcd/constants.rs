use crate::types::*;

// Visible     240 dots,  57.221 us,    960 cycles - 78% of h-time
// H-Blanking   68 dots,  16.212 us,    272 cycles - 22% of h-time
// Total       308 dots,  73.433 us,   1232 cycles - ca. 13.620 kHz
pub const CYCLES_PER_LINE: usize = 1232;
pub const HBLANK_LENGTH: usize = 226;
// Visible (*) 160 lines, 11.749 ms, 197120 cycles - 70% of v-time
// V-Blanking   68 lines,  4.994 ms,  83776 cycles - 30% of v-time
// Total       228 lines, 16.743 ms, 280896 cycles - ca. 59.737 Hz
pub const CYCLES_PER_FRAME: usize = 280_896;
pub const LINES_PER_FRAME: usize = 228;

pub const DISPLAY_TILE_WIDTH: Word = 30;
pub const DISPLAY_TILE_HEIGHT: Word = 20;

pub const VIRTUAL_DISPLAY_TILE_WIDTH: Word = 32;
pub const VIRTUAL_DISPLAY_TILE_HEIGHT: Word = 32;
