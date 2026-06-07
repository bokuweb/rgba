use super::constants::*;
use crate::types::*;

bitfield! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct DISPSTAT(u16);
    pub vcount_setting, _: 15, 8;
    pub vcounter_irq_enable, _: 5;
    pub hblank_irq_enable, _: 4;
    pub vblank_irq_enable, _: 3;
}

impl DISPSTAT {
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn write(&mut self, data: HalfWord) {
        self.0 = data & 0xFFF8;
    }

    pub fn read(&self, cycles: usize, lines: usize) -> HalfWord {
        let mut v = self.0;

        if Self::is_vblank(lines) {
            v |= 0x0001;
        }

        if Self::is_hblank(cycles) {
            v |= 0x0002;
        }

        if self.vcount_setting() == lines as u16 {
            v |= 0x0004;
        }
        v
    }

    const fn is_vblank(lines: usize) -> bool {
        // The VBlank flag is set in lines 160..=226, but NOT in the last line
        // (227), per GBATek / hardware behaviour.
        lines >= 160 && lines <= 226
    }

    const fn is_hblank(cycles: usize) -> bool {
        cycles % CYCLES_PER_LINE >= CYCLES_PER_LINE - HBLANK_LENGTH
    }
}

// `Default` cannot be derived through the `bitfield!` macro, so keep it manual.
#[allow(clippy::derivable_impls)]
impl Default for DISPSTAT {
    fn default() -> Self {
        Self(0x0000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VBLANK: u16 = 0x0001;
    const HBLANK: u16 = 0x0002;
    const VCOUNT: u16 = 0x0004;

    #[test]
    fn vblank_flag_set_for_lines_160_to_226_only() {
        let d = DISPSTAT::default();
        // Visible lines: no VBlank flag.
        for line in 0..160 {
            assert_eq!(d.read(0, line) & VBLANK, 0, "line {line} should not be in VBlank");
        }
        // VBlank period 160..=226: flag set.
        for line in 160..=226 {
            assert_eq!(d.read(0, line) & VBLANK, VBLANK, "line {line} should be in VBlank");
        }
        // Last line (227): VBlank flag cleared (regression test for the
        // agb_checker LCD vblank_status behaviour).
        assert_eq!(d.read(0, 227) & VBLANK, 0, "line 227 must clear the VBlank flag");
    }

    #[test]
    fn hblank_flag_tracks_position_within_line() {
        let d = DISPSTAT::default();
        // Start of a line (HDraw): no HBlank flag.
        assert_eq!(d.read(0, 0) & HBLANK, 0);
        // Just before HBlank begins.
        assert_eq!(d.read(CYCLES_PER_LINE - HBLANK_LENGTH - 1, 0) & HBLANK, 0);
        // Inside the HBlank region.
        assert_eq!(d.read(CYCLES_PER_LINE - HBLANK_LENGTH, 0) & HBLANK, HBLANK);
        assert_eq!(d.read(CYCLES_PER_LINE - 1, 0) & HBLANK, HBLANK);
    }

    #[test]
    fn vcount_match_flag_reflects_setting() {
        let mut d = DISPSTAT::default();
        d.write(100 << 8); // VCount setting = 100
        assert_eq!(d.read(0, 100) & VCOUNT, VCOUNT, "match line should set VCount flag");
        assert_eq!(d.read(0, 99) & VCOUNT, 0, "non-match line should clear VCount flag");
        assert_eq!(d.read(0, 101) & VCOUNT, 0, "non-match line should clear VCount flag");
    }

    #[test]
    fn write_masks_out_read_only_status_bits() {
        let mut d = DISPSTAT::default();
        // Lower 3 bits (VBlank/HBlank/VCount status) are read-only; writes are masked.
        d.write(0xFFFF);
        assert_eq!(d.0 & 0x0007, 0, "status bits must not be writable");
        // IRQ-enable and VCount-setting bits are retained.
        assert_eq!(d.0 & 0xFFF8, 0xFFF8);
    }
}
