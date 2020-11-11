use crate::types::*;

// Visible     240 dots,  57.221 us,    960 cycles - 78% of h-time
// H-Blanking   68 dots,  16.212 us,    272 cycles - 22% of h-time
// Total       308 dots,  73.433 us,   1232 cycles - ca. 13.620 kHz
const CYCLES_PER_LINE: usize = 1232;
// Visible (*) 160 lines, 11.749 ms, 197120 cycles - 70% of v-time
// V-Blanking   68 lines,  4.994 ms,  83776 cycles - 30% of v-time
// Total       228 lines, 16.743 ms, 280896 cycles - ca. 59.737 Hz
const CYCLES_PER_FRAME: usize = 280896;
const LINES_PER_FRAME: usize = 228;

pub struct LCDController {
    cycles: usize,
    lines: usize,
}

impl LCDController {
    pub fn new() -> LCDController {
        LCDController { cycles: 0, lines: 0 }
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

    pub fn read(&self) -> HalfWord {
        self.lines as HalfWord
    }
}
