use crate::cpu::types::*;

pub fn shift(shift_type: Shift, value: u32, shift: u32) -> u32 {
    match shift_type {
        Shift::LSL => lsl(value, shift),
        Shift::LSR => lsr(value, shift),
        Shift::ASR => asr(value, shift),
        Shift::ROR => ror(value, shift),
    }
}

pub fn is_carry_over(shift_type: Shift, value: u32, shift: u32) -> bool {
    if shift == 0 {
        false
    } else {
        match shift_type {
            Shift::LSL => value & (1 << (32 - shift)) != 0,
            _ => value & (1 << (shift - 1)) != 0,
        }
    }
}

pub fn lsl(value: u32, shift: u32) -> u32 {
    if shift == 0 {
        return value;
    }
    value.wrapping_shl(shift)
}

pub fn lsr(value: u32, shift: u32) -> u32 {
    if shift == 0 {
        return value;
    }
    // dbg!(shift);
    value.wrapping_shr(shift)
}

pub fn asr(value: u32, shift: u32) -> u32 {
    if shift == 0 {
        return value;
    }
    if value & (1 << 31) == 0 {
        value.wrapping_shr(shift)
    } else {
        value.wrapping_shr(shift) | ((0xFFFF_FFFF as u32).wrapping_shl(32 - shift))
    }
}

pub fn ror(value: u32, shift: u32) -> u32 {
    if shift == 0 {
        return value;
    }
    value.wrapping_shr(shift) | (value.wrapping_shl(32 - shift))
}

#[test]
fn test_ror() {
    assert_eq!(ror(0xA5A5_5A5A, 4), 0xAA5A_55A5);
}

#[test]
fn test_asr() {
    assert_eq!(ror(0xA5A5_5A5A, 4), 0xAA5A_55A5);
}

#[test]
fn test_carry_lsl() {
    assert_eq!(is_carry_over(Shift::LSL, 0x8000_0000, 1), true);
}

#[test]
fn test_without_carry_lsl() {
    assert_eq!(is_carry_over(Shift::LSL, 0x8000_0000, 2), false);
}

#[test]
fn test_carry_ror() {
    assert_eq!(is_carry_over(Shift::ROR, 0x0000_0001, 1), true);
}

#[test]
fn test_without_carry_ror() {
    assert_eq!(is_carry_over(Shift::ROR, 0x0000_0001, 2), false);
}
