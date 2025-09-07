use crate::cpu::types::*;

pub fn shift(shift_type: Shift, value: u32, shift: u32, c: bool, shift_by_reg: bool) -> u32 {
    match shift_type {
        Shift::LSL => lsl(value, shift),
        Shift::LSR => lsr(value, shift, shift_by_reg),
        Shift::ASR => asr(value, shift, shift_by_reg),
        Shift::ROR => ror(value, shift, c, shift_by_reg),
    }
}

pub fn is_carry_over(shift_type: Shift, value: u32, shift: u32, current: bool, shift_by_reg: bool) -> bool {
    if shift == 0 {
        if !shift_by_reg {
            match shift_type {
                Shift::LSL => current,                   // LSL#0: Not affected
                Shift::LSR => (value & 0x80000000) != 0, // LSR#0 -> LSR#32: C becomes bit 31 of Rm
                Shift::ASR => (value & 0x80000000) != 0, // ASR#0 -> ASR#32: C becomes bit 31 of Rm
                Shift::ROR => (value & 0x00000001) != 0, // ROR#0 -> RRX: C becomes bit 0 of Rm
            }
        } else {
            current
        }
    } else {
        match shift_type {
            Shift::LSL => {
                if shift > 32 {
                    false
                } else if shift == 32 {
                    (value & 0x00000001) != 0 // bit 0 goes to carry for LSL#32
                } else {
                    (value & (1 << (32 - shift))) != 0
                }
            }
            Shift::LSR => {
                if shift > 32 {
                    false
                } else if shift == 32 {
                    (value & 0x80000000) != 0 // bit 31 goes to carry for LSR#32
                } else {
                    (value & (1 << (shift - 1))) != 0
                }
            }
            Shift::ASR => {
                if shift >= 32 {
                    (value & 0x80000000) != 0 // ASR#32+: C becomes bit 31 of Rm
                } else {
                    (value & (1 << (shift - 1))) != 0
                }
            }
            Shift::ROR => {
                let effective_shift = shift % 32;
                if effective_shift == 0 {
                    current // ROR by multiple of 32 doesn't change carry
                } else {
                    (value & (1 << (effective_shift - 1))) != 0
                }
            }
        }
    }
}

pub fn lsl(value: u32, shift: u32) -> u32 {
    if shift == 0 {
        // Op2 = Rm
        return value;
    }
    value.checked_shl(shift).unwrap_or_default()
}

pub fn lsr(value: u32, shift: u32, shift_by_reg: bool) -> u32 {
    if shift == 0 {
        if !shift_by_reg {
            return 0; // Op2 becomes 0
        } else {
            return value;
        }
    }
    value.checked_shr(shift).unwrap_or_default()
}

pub fn asr(value: u32, shift: u32, shift_by_reg: bool) -> u32 {
    // Op2 is filled gy bit 31 of rm
    if shift == 0 {
        if !shift_by_reg {
            if value & 0x8000_0000 == 0 {
                return 0;
            } else {
                return 0xFFFF_FFFF;
            }
        } else {
            return value;
        }
    }
    if value & (1 << 31) == 0 {
        value.checked_shr(shift).unwrap_or_default()
    } else if shift < 32 {
        value.checked_shr(shift).unwrap_or_default() | ((0xFFFF_FFFF as u32).checked_shl(32 - shift).unwrap_or_default())
    } else if value & 0x8000_0000 == 0 {
        return 0;
    } else {
        return 0xFFFF_FFFF;
    }
}

pub fn ror(value: u32, shift: u32, c: bool, shift_by_reg: bool) -> u32 {
    if shift == 0 {
        if !shift_by_reg {
            // ROR#0 interpreted as RRX (rotate right with extend)
            let new_value = (value >> 1) | if c { 0x80000000 } else { 0 };
            new_value
        } else {
            return value;
        }
    } else {
        value.rotate_right(shift % 32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsl_basic() {
        assert_eq!(lsl(0x00000001, 1), 0x00000002);
        assert_eq!(lsl(0x00000001, 4), 0x00000010);
        assert_eq!(lsl(0x80000000, 1), 0x00000000); // overflow
    }

    #[test]
    fn test_lsl_zero_shift() {
        assert_eq!(lsl(0xA5A5A5A5, 0), 0xA5A5A5A5); // no shift
    }

    #[test]
    fn test_lsr_basic() {
        assert_eq!(lsr(0x00000002, 1, false), 0x00000001);
        assert_eq!(lsr(0x00000010, 4, false), 0x00000001);
        assert_eq!(lsr(0x80000000, 1, false), 0x40000000);
    }

    #[test]
    fn test_lsr_zero_shift() {
        // LSR#0 interpreted as LSR#32 -> result becomes 0
        assert_eq!(lsr(0xA5A5A5A5, 0, false), 0x00000000);
        // LSR by register with 0 shift -> no shift
        assert_eq!(lsr(0xA5A5A5A5, 0, true), 0xA5A5A5A5);
    }

    #[test]
    fn test_asr_basic() {
        assert_eq!(asr(0x00000002, 1, false), 0x00000001);
        assert_eq!(asr(0x80000000, 1, false), 0xC0000000); // sign extend
        assert_eq!(asr(0x40000000, 1, false), 0x20000000); // positive
    }

    #[test]
    fn test_asr_zero_shift() {
        // ASR#0 interpreted as ASR#32 -> filled with bit 31
        assert_eq!(asr(0x80000000, 0, false), 0xFFFFFFFF); // negative
        assert_eq!(asr(0x40000000, 0, false), 0x00000000); // positive
        // ASR by register with 0 shift -> no shift
        assert_eq!(asr(0x80000000, 0, true), 0x80000000);
    }

    #[test]
    fn test_ror_basic() {
        assert_eq!(ror(0xA5A5A5A5, 4, false, false), 0x5A5A5A5A);
        assert_eq!(ror(0x12345678, 8, false, false), 0x78123456);
    }

    #[test]
    fn test_ror_zero_shift_rrx() {
        // ROR#0 interpreted as RRX (rotate right with extend)
        assert_eq!(ror(0x00000001, 0, false, false), 0x00000000); // C=0, bit 0 -> C
        assert_eq!(ror(0x00000001, 0, true, false), 0x80000000);  // C=1, old C -> bit 31
        assert_eq!(ror(0x00000002, 0, false, false), 0x00000001);
        // ROR by register with 0 shift -> no shift
        assert_eq!(ror(0x00000001, 0, false, true), 0x00000001);
    }

    // Carry flag tests
    #[test]
    fn test_lsl_carry() {
        // LSL carry tests
        assert_eq!(is_carry_over(Shift::LSL, 0x80000000, 1, false, false), true);
        assert_eq!(is_carry_over(Shift::LSL, 0x40000000, 1, false, false), false);
        assert_eq!(is_carry_over(Shift::LSL, 0x40000000, 2, false, false), true);
        assert_eq!(is_carry_over(Shift::LSL, 0x20000000, 2, false, false), false);
        // LSL#0 doesn't affect carry
        assert_eq!(is_carry_over(Shift::LSL, 0x80000000, 0, true, false), true);
        assert_eq!(is_carry_over(Shift::LSL, 0x80000000, 0, false, false), false);
    }

    #[test]
    fn test_lsr_carry() {
        // LSR carry tests
        assert_eq!(is_carry_over(Shift::LSR, 0x00000001, 1, false, false), true);
        assert_eq!(is_carry_over(Shift::LSR, 0x00000002, 1, false, false), false);
        assert_eq!(is_carry_over(Shift::LSR, 0x00000003, 2, false, false), true);
        // LSR#0 interpreted as LSR#32 -> C becomes bit 31
        assert_eq!(is_carry_over(Shift::LSR, 0x80000000, 0, false, false), true);
        assert_eq!(is_carry_over(Shift::LSR, 0x40000000, 0, false, false), false);
        // LSR by register with 0 shift -> doesn't affect carry
        assert_eq!(is_carry_over(Shift::LSR, 0x80000000, 0, true, true), true);
        assert_eq!(is_carry_over(Shift::LSR, 0x80000000, 0, false, true), false);
    }

    #[test]
    fn test_asr_carry() {
        // ASR carry tests
        assert_eq!(is_carry_over(Shift::ASR, 0x00000001, 1, false, false), true);
        assert_eq!(is_carry_over(Shift::ASR, 0x00000002, 1, false, false), false);
        assert_eq!(is_carry_over(Shift::ASR, 0x00000003, 2, false, false), true);
        // ASR#0 interpreted as ASR#32 -> C becomes bit 31
        assert_eq!(is_carry_over(Shift::ASR, 0x80000000, 0, false, false), true);
        assert_eq!(is_carry_over(Shift::ASR, 0x40000000, 0, false, false), false);
    }

    #[test]
    fn test_ror_carry() {
        // ROR carry tests
        assert_eq!(is_carry_over(Shift::ROR, 0x00000001, 1, false, false), true);
        assert_eq!(is_carry_over(Shift::ROR, 0x00000002, 1, false, false), false);
        assert_eq!(is_carry_over(Shift::ROR, 0x00000003, 2, false, false), true);
        // ROR#0 interpreted as RRX -> C becomes bit 0
        assert_eq!(is_carry_over(Shift::ROR, 0x00000001, 0, false, false), true);
        assert_eq!(is_carry_over(Shift::ROR, 0x00000002, 0, false, false), false);
    }

    #[test]
    fn test_shift_large_amounts() {
        // Test shifts >= 32
        assert_eq!(lsl(0xFFFFFFFF, 32), 0x00000000);
        assert_eq!(lsr(0xFFFFFFFF, 32, false), 0x00000000);
        assert_eq!(asr(0x80000000, 32, false), 0xFFFFFFFF);
        assert_eq!(asr(0x40000000, 32, false), 0x00000000);
        
        // Carry for large shifts
        assert_eq!(is_carry_over(Shift::LSL, 0x00000001, 32, false, false), true);  // bit 0 -> carry
        assert_eq!(is_carry_over(Shift::LSL, 0x00000001, 33, false, false), false); // > 32 -> false
    }
}
