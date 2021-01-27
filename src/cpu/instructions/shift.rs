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
                Shift::LSL => current,                  // Not affected
                Shift::LSR => value & 0x8000_0000 != 0, // C becomes bit 31 of Rm
                Shift::ASR => value & 0x8000_0000 != 0, // C becomes bit 31 of Rm
                Shift::ROR => value & 0x0000_0001 != 0,
            }
        } else {
            current
        }
    } else {
        match shift_type {
            Shift::LSL => value & (1 << (32 - shift)) != 0,
            Shift::ASR => {
                if shift == 0 {
                    current
                } else if shift < 32 {
                    value & (1 << (shift - 1)) != 0
                } else if value & 0x8000_0000 != 0 {
                    true
                } else {
                    false
                }
            }
            _ => value & (1 << (shift - 1)) != 0,
        }
    }
}

pub fn lsl(value: u32, shift: u32) -> u32 {
    if shift == 0 {
        // Op2 = Rm
        return value;
    }
    value.wrapping_shl(shift)
}

pub fn lsr(value: u32, shift: u32, shift_by_reg: bool) -> u32 {
    if shift == 0 {
        if !shift_by_reg {
            return 0; // Op2 becomes 0
        } else {
            return value;
        }
    }
    value.wrapping_shr(shift)
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
        value.wrapping_shr(shift)
    } else if (shift < 32) {
        value.wrapping_shr(shift) | ((0xFFFF_FFFF as u32).wrapping_shl(32 - shift))
    } else {
        0
    }
}

pub fn ror(value: u32, shift: u32, c: bool, shift_by_reg: bool) -> u32 {
    if shift == 0 {
        if !shift_by_reg {
            if c {
                return value.wrapping_shr(1) | 0x8000_0000; // Op2 bit 31 set to old C
            } else {
                return value.wrapping_shr(1);
            }
        } else {
            return value;
        }
    }
    value.wrapping_shr(shift) | (value.wrapping_shl(32 - shift))
}

#[test]
fn test_ror() {
    assert_eq!(ror(0xA5A5_5A5A, 4, false, false), 0xAA5A_55A5);
}

#[test]
fn test_asr() {
    assert_eq!(ror(0xA5A5_5A5A, 4, false, false), 0xAA5A_55A5);
}

#[test]
fn test_carry_lsl() {
    assert_eq!(is_carry_over(Shift::LSL, 0x8000_0000, 1, true, false), true);
}

#[test]
fn test_without_carry_lsl() {
    assert_eq!(is_carry_over(Shift::LSL, 0x8000_0000, 2, false, false), false);
}

#[test]
fn test_carry_ror() {
    assert_eq!(is_carry_over(Shift::ROR, 0x0000_0001, 1, true, false), true);
}

#[test]
fn test_without_carry_ror() {
    assert_eq!(is_carry_over(Shift::ROR, 0x0000_0001, 2, false, false), false);
}
