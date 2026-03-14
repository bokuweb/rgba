use crate::cpu::bus::accessor::BusAccessor;
use crate::types::Word;

/// BIOS function implementations
/// These implement the GBA BIOS System Call functions
pub struct Bios;

impl Bios {
    /// SWI 0x00 - SoftReset
    /// Clears 0x7E00-0x7FFF in IWRAM and sets up registers for reset
    pub fn soft_reset<T: BusAccessor>(
        _bus: &mut T,
        gpr: &mut [Word; 16],
        _iwram_flag: u8,
    ) {
        // Clear most of IWRAM (0x7E00-0x7FFF)
        // Set LR based on flag at 0x7FFA
        // For now, just set a default return address
        gpr[14] = 0x08000000; // Default to ROM
    }

    /// SWI 0x02 - Halt
    /// Halts the CPU until an interrupt occurs
    pub fn halt<T: BusAccessor>(_bus: &mut T, _gpr: &mut [Word; 16]) {
        _bus.set_cpu_halted(true);
    }

    /// SWI 0x04 - IntrWait
    /// Waits for specific interrupts
    pub fn intr_wait<T: BusAccessor>(
        bus: &mut T,
        gpr: &mut [Word; 16],
    ) {
        let discard_old = gpr[0] != 0;
        let interrupt_flags = (gpr[1] & 0xFFFF) as u16;

        // Match JS behavior: ensure IME is enabled while waiting.
        if (bus.read_halfword(0x0400_0208) & 0x0001) == 0 {
            bus.write_halfword(0x0400_0208, 0x0001);
        }

        // If caller does not request discarding old flags and target IF is already set, return immediately.
        let current_if = bus.read_halfword(0x0400_0202);
        if !discard_old && (current_if & interrupt_flags) != 0 {
            return;
        }

        // Clear latched interrupt flags before waiting.
        bus.write_halfword(0x0400_0202, 0xFFFF);
        bus.set_cpu_halted(true);
    }

    /// SWI 0x05 - VBlankIntrWait
    /// Waits for VBlank interrupt
    pub fn vblank_intr_wait<T: BusAccessor>(
        bus: &mut T,
        gpr: &mut [Word; 16],
    ) {
        // Set up for VBlank interrupt
        gpr[0] = 1; // Discard old interrupt
        gpr[1] = 1; // VBlank interrupt flag
        Self::intr_wait(bus, gpr);
    }

    /// SWI 0x06 - Div
    /// Signed division r0/r1
    pub fn div<T: BusAccessor>(_bus: &mut T, gpr: &mut [Word; 16]) {
        let numerator = gpr[0] as i32;
        let denominator = gpr[1] as i32;


        if denominator == 0 {
            // Division by zero typically causes endless loop in BIOS
            // For testing, we'll return max values
            gpr[0] = if numerator >= 0 { 0x7FFFFFFF } else { 0x80000000 };
            gpr[1] = numerator as u32;
            gpr[3] = 0x7FFFFFFF;
        } else {
            let result = numerator / denominator;
            let remainder = numerator % denominator;
            gpr[0] = result as u32;
            gpr[1] = remainder as u32;
            gpr[3] = result.unsigned_abs();
        }
    }

    /// SWI 0x07 - DivArm
    /// Division with swapped parameters (ARM library compatibility)
    pub fn div_arm<T: BusAccessor>(_bus: &mut T, gpr: &mut [Word; 16]) {
        // Swap parameters and call regular div
        let temp = gpr[0];
        gpr[0] = gpr[1];
        gpr[1] = temp;
        Self::div(_bus, gpr);
    }

    /// SWI 0x08 - Sqrt
    /// Calculate integer square root
    pub fn sqrt<T: BusAccessor>(_bus: &mut T, gpr: &mut [Word; 16]) {
        let value = gpr[0];

        let result = if value == 0 {
            0
        } else {
            // Integer square root
            (value as f64).sqrt() as u32
        };

        gpr[0] = result;
    }

    /// SWI 0x09 - ArcTan
    /// Calculate arctangent
    pub fn arc_tan<T: BusAccessor>(_bus: &mut T, gpr: &mut [Word; 16]) {
        let tan_value = gpr[0] as i16; // 16-bit signed fixed point

        // Convert from 14-bit decimal part to floating point
        let tan_float = (tan_value as f64) / 16384.0;
        let atan_result = tan_float.atan();

        // Convert back to fixed point range C000h-4000h for -PI/2 to PI/2
        let result = (atan_result / std::f64::consts::FRAC_PI_2 * 16384.0) as i32;
        gpr[0] = result as u32;
    }

    /// SWI 0x0A - ArcTan2
    /// Calculate arctangent with quadrant correction
    pub fn arc_tan2<T: BusAccessor>(_bus: &mut T, gpr: &mut [Word; 16]) {
        let x = gpr[0] as i16;
        let y = gpr[1] as i16;

        let x_float = (x as f64) / 16384.0;
        let y_float = (y as f64) / 16384.0;
        let atan2_result = y_float.atan2(x_float);

        // Convert to 0-FFFFh range for 0-2PI
        let normalized = if atan2_result < 0.0 {
            atan2_result + 2.0 * std::f64::consts::PI
        } else {
            atan2_result
        };

        let result = (normalized / (2.0 * std::f64::consts::PI) * 65536.0) as u32;
        gpr[0] = result & 0xFFFF;
    }

    /// SWI 0x0B - CpuSet
    /// Memory copy/fill function
    pub fn cpu_set<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16]) {
        let source = gpr[0];
        let dest = gpr[1];
        let control = gpr[2];

        let count = control & 0x000FFFFF;
        let fill_mode = (control & 0x01000000) != 0;
        let word_size = if (control & 0x04000000) != 0 { 4 } else { 2 };

        if fill_mode {
            // Fill mode - repeat first value
            let fill_value = if word_size == 4 {
                bus.read_word(source & !3)
            } else {
                bus.read_halfword(source & !1) as u32
            };

            for i in 0..count {
                let dst_addr = dest + i * word_size;
                if word_size == 4 {
                    bus.write_word(dst_addr & !3, fill_value);
                } else {
                    bus.write_halfword(dst_addr & !1, (fill_value & 0xFFFF) as u16);
                }
            }
        } else {
            // Copy mode
            for i in 0..count {
                let src_addr = source + i * word_size;
                let dst_addr = dest + i * word_size;

                if word_size == 4 {
                    let value = bus.read_word(src_addr & !3);
                    bus.write_word(dst_addr & !3, value);
                } else {
                    let value = bus.read_halfword(src_addr & !1);
                    bus.write_halfword(dst_addr & !1, value);
                }
            }
        }
    }

    /// SWI 0x0C - CpuFastSet
    /// Fast memory copy (32-bit aligned)
    pub fn cpu_fast_set<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16]) {
        let source = gpr[0] & !3; // Force word alignment
        let dest = gpr[1] & !3;   // Force word alignment
        let control = gpr[2];

        let count = control & 0x000FFFFF;
        let count = ((count + 7) >> 3) << 3; // Rounded up to multiples of 8 words
        let fill_mode = (control & 0x01000000) != 0;

        if fill_mode {
            let fill_value = bus.read_word(source);
            for i in 0..count {
                let dst_addr = dest + i * 4;
                bus.write_word(dst_addr, fill_value);
            }
        } else {
            for i in 0..count {
                let src_addr = source + i * 4;
                let dst_addr = dest + i * 4;
                let value = bus.read_word(src_addr);
                bus.write_word(dst_addr, value);
            }
        }
    }

    /// SWI 0x0E - BgAffineSet
    /// Generates affine transform matrices for BG rotation/scaling.
    pub fn bg_affine_set<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16]) {
        let mut offset = gpr[0];
        let mut destination = gpr[1];
        let mut count = gpr[2];

        while count > 0 {
            let ox = (bus.read_word(offset) as i32 as f64) / 256.0;
            let oy = (bus.read_word(offset + 4) as i32 as f64) / 256.0;
            let cx = bus.read_halfword(offset + 8) as i16 as f64;
            let cy = bus.read_halfword(offset + 10) as i16 as f64;
            let sx = (bus.read_halfword(offset + 12) as i16 as f64) / 256.0;
            let sy = (bus.read_halfword(offset + 14) as i16 as f64) / 256.0;
            let theta_u8 = ((bus.read_halfword(offset + 16) >> 8) & 0xFF) as f64;
            let theta = theta_u8 / 128.0 * std::f64::consts::PI;
            offset += 20;

            let mut a = theta.cos();
            let mut b = theta.sin();
            let mut c = theta.sin();
            let mut d = theta.cos();

            a *= sx;
            b *= -sx;
            c *= sy;
            d *= sy;

            let rx = ox - (a * cx + b * cy);
            let ry = oy - (c * cx + d * cy);

            bus.write_halfword(destination, ((a * 256.0) as i32) as u16);
            bus.write_halfword(destination + 2, ((b * 256.0) as i32) as u16);
            bus.write_halfword(destination + 4, ((c * 256.0) as i32) as u16);
            bus.write_halfword(destination + 6, ((d * 256.0) as i32) as u16);
            bus.write_word(destination + 8, ((rx * 256.0) as i32) as u32);
            bus.write_word(destination + 12, ((ry * 256.0) as i32) as u32);

            destination += 16;
            count -= 1;
        }
    }

    /// SWI 0x0F - ObjAffineSet
    /// Generates affine matrices for OBJ entries.
    pub fn obj_affine_set<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16]) {
        let mut offset = gpr[0];
        let mut destination = gpr[1];
        let mut count = gpr[2];
        let diff = gpr[3];

        while count > 0 {
            let sx = (bus.read_halfword(offset) as i16 as f64) / 256.0;
            let sy = (bus.read_halfword(offset + 2) as i16 as f64) / 256.0;
            let theta_u8 = ((bus.read_halfword(offset + 4) >> 8) & 0xFF) as f64;
            let theta = theta_u8 / 128.0 * std::f64::consts::PI;
            offset += 6;

            let mut a = theta.cos();
            let mut b = theta.sin();
            let mut c = theta.sin();
            let mut d = theta.cos();

            a *= sx;
            b *= -sx;
            c *= sy;
            d *= sy;

            bus.write_halfword(destination, ((a * 256.0) as i32) as u16);
            bus.write_halfword(destination + diff, ((b * 256.0) as i32) as u16);
            bus.write_halfword(destination + diff * 2, ((c * 256.0) as i32) as u16);
            bus.write_halfword(destination + diff * 3, ((d * 256.0) as i32) as u16);
            destination += diff * 4;
            count -= 1;
        }
    }

    /// Execute BIOS function based on SWI number
    pub fn execute_swi<T: BusAccessor>(
        bus: &mut T,
        swi_number: u32,
        gpr: &mut [Word; 16],
    ) {
        match swi_number {
            0x00 => Self::soft_reset(bus, gpr, 0),
            0x02 => Self::halt(bus, gpr),
            0x04 => Self::intr_wait(bus, gpr),
            0x05 => Self::vblank_intr_wait(bus, gpr),
            0x06 => Self::div(bus, gpr),
            0x07 => Self::div_arm(bus, gpr),
            0x08 => Self::sqrt(bus, gpr),
            0x09 => Self::arc_tan(bus, gpr),
            0x0A => Self::arc_tan2(bus, gpr),
            0x0B => Self::cpu_set(bus, gpr),
            0x0C => Self::cpu_fast_set(bus, gpr),
            0x0E => Self::bg_affine_set(bus, gpr),
            0x0F => Self::obj_affine_set(bus, gpr),
            _ => {
                unimplemented!("BIOS: Unimplemented SWI 0x{:02X}", swi_number);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Bios;
    use crate::cpu::bus::accessor::BusAccessor;
    use crate::types::*;
    use std::collections::HashMap;

    // Mock bus for testing
    struct MockBus {
        memory: HashMap<Word, Word>,
    }

    impl MockBus {
        fn new() -> Self {
            Self {
                memory: HashMap::new(),
            }
        }

        fn write_32(&mut self, addr: Word, value: Word) {
            self.memory.insert(addr & !3, value);
        }
    }

    impl BusAccessor for MockBus {
        fn compute_cycle(&self, _addr: Word, _access_type: crate::types::AccessType) -> crate::types::Cycle {
            1 // Mock implementation
        }

        fn read_byte(&self, addr: Word) -> Byte {
            let word_addr = addr & !3;
            let byte_offset = addr & 3;
            let word = self.memory.get(&word_addr).unwrap_or(&0);
            ((word >> (byte_offset * 8)) & 0xFF) as Byte
        }

        fn read_halfword(&self, addr: Word) -> HalfWord {
            let word_addr = addr & !3;
            let is_upper = (addr & 2) != 0;
            let word = self.memory.get(&word_addr).unwrap_or(&0);
            if is_upper {
                (word >> 16) as HalfWord
            } else {
                (word & 0xFFFF) as HalfWord
            }
        }

        fn read_word(&self, addr: Word) -> Word {
            *self.memory.get(&(addr & !3)).unwrap_or(&0)
        }

        fn write_byte(&mut self, _addr: Word, _value: Byte) {
            // Implement if needed for tests
        }

        fn write_halfword(&mut self, _addr: Word, _value: HalfWord) {
            // Implement if needed for tests
        }

        fn write_word(&mut self, _addr: Word, _value: Word) {
            // Implement if needed for tests
        }
    }

    #[test]
    fn test_div_basic() {
        let mut bus = MockBus::new();
        let mut gpr = [0u32; 16];

        // Test: 100 / 10 = 10 remainder 0
        gpr[0] = 100;
        gpr[1] = 10;

        Bios::div(&mut bus, &mut gpr);

        assert_eq!(gpr[0], 10);  // quotient
        assert_eq!(gpr[1], 0);   // remainder
        assert_eq!(gpr[3], 10);  // absolute quotient
    }

    #[test]
    fn test_div_negative() {
        let mut bus = MockBus::new();
        let mut gpr = [0u32; 16];

        // Test: -123 / 10 = -12 remainder -3
        gpr[0] = (-123i32) as u32;
        gpr[1] = 10;

        Bios::div(&mut bus, &mut gpr);

        assert_eq!(gpr[0] as i32, -12); // quotient
        assert_eq!(gpr[1] as i32, -3);  // remainder
        assert_eq!(gpr[3], 12);         // absolute quotient
    }

    #[test]
    fn test_sqrt() {
        let mut bus = MockBus::new();
        let mut gpr = [0u32; 16];

        // Test sqrt(16) = 4
        gpr[0] = 16;
        Bios::sqrt(&mut bus, &mut gpr);
        assert_eq!(gpr[0], 4);

        // Test sqrt(0) = 0
        gpr[0] = 0;
        Bios::sqrt(&mut bus, &mut gpr);
        assert_eq!(gpr[0], 0);
    }
}
