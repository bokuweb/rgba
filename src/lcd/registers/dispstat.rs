use super::constants::*;
use crate::types::*;

bitfield! {
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct DISPSTAT(u16);
    pub vcount_setting, _: 15, 8;
    pub vcounter_irq_enable, _: 5;
    pub hblank_irq_enable, _: 4;
    pub vblank_irq_enable, _: 3;
}

impl DISPSTAT {
    pub fn new() -> Self {
        DISPSTAT::default()
    }

    pub fn write(&mut self, data: HalfWord) {
        self.0 = data & 0xFFF8;
    }

    pub fn read(&self, cycles: usize, lines: usize) -> HalfWord {
        let mut v = self.0;

        if DISPSTAT::is_vblank(lines) {
            v |= 0x0001;
        }

        if DISPSTAT::is_hblank(cycles) {
            v |= 0x0002;
        }

        if self.vcount_setting() == lines as u16 {
            v |= 0x0004;
        }
        v
    }

    fn is_vblank(lines: usize) -> bool {
        // The VBlank flag is set in lines 160..=226, but NOT in the last line
        // (227), per GBATek / hardware behaviour.
        lines >= 160 && lines <= 226
    }

    fn is_hblank(cycles: usize) -> bool {
        cycles % CYCLES_PER_LINE >= CYCLES_PER_LINE - HBLANK_LENGTH
    }
}

impl Default for DISPSTAT {
    fn default() -> Self {
        DISPSTAT(0x0000)
    }
}
