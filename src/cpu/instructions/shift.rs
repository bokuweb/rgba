use crate::cpu::types::*;

pub fn shift(shift_type: Shift, value: u32, shift: u32, c: bool, shift_by_reg: bool) -> u32 {
    // レジスタ指定のときは下位8bitのみ有効
    let amount = if shift_by_reg { shift & 0xFF } else { shift };

    match shift_type {
        Shift::LSL => lsl(value, amount),
        Shift::LSR => lsr(value, amount, shift_by_reg),
        Shift::ASR => asr(value, amount, shift_by_reg),
        Shift::ROR => ror(value, amount, c, shift_by_reg),
    }
}

pub const fn is_carry_over(shift_type: Shift, value: u32, shift: u32, current: bool, shift_by_reg: bool) -> bool {
    // レジスタ指定のときは下位8bitのみ有効
    let shift = if shift_by_reg { shift & 0xFF } else { shift };
    
    if shift == 0 {
        if shift_by_reg {
            current
        } else {
            match shift_type {
                Shift::LSL => current,                   // LSL#0: Not affected
                Shift::LSR => (value & 0x8000_0000) != 0, // LSR#0 -> LSR#32: C becomes bit 31 of Rm
                Shift::ASR => (value & 0x8000_0000) != 0, // ASR#0 -> ASR#32: C becomes bit 31 of Rm
                Shift::ROR => (value & 0x0000_0001) != 0, // ROR#0 -> RRX: C becomes bit 0 of Rm
            }
        }
    } else {
        match shift_type {
            Shift::LSL => {
                if shift > 32 {
                    false
                } else if shift == 32 {
                    (value & 0x0000_0001) != 0 // bit 0 goes to carry for LSL#32
                } else {
                    (value & (1 << (32 - shift))) != 0
                }
            }
            Shift::LSR => {
                if shift > 32 {
                    false
                } else if shift == 32 {
                    (value & 0x8000_0000) != 0 // bit 31 goes to carry for LSR#32
                } else {
                    (value & (1 << (shift - 1))) != 0
                }
            }
            Shift::ASR => {
                if shift >= 32 {
                    (value & 0x8000_0000) != 0 // ASR#32+: C becomes bit 31 of Rm
                } else {
                    (value & (1 << (shift - 1))) != 0
                }
            }
            Shift::ROR => {
                let effective_shift = shift % 32;
                if shift_by_reg {
                    if effective_shift == 0 {
                        // Rs % 32 == 0 かつ shift != 0 → C = Rm[31]
                        (value & 0x8000_0000) != 0
                    } else {
                        (value & (1 << (effective_shift - 1))) != 0
                    }
                } else {
                    // 即値 ROR：shift==0 は上の大枠の分岐で RRX として処理済み
                    if effective_shift == 0 {
                        // 実際にはここに来ない（ROR #32 はエンコードされない想定）
                        current
                    } else {
                        (value & (1 << (effective_shift - 1))) != 0
                    }
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
        if shift_by_reg {
            return value;
        }
        return 0; // Op2 becomes 0
    }
    value.checked_shr(shift).unwrap_or_default()
}

pub fn asr(value: u32, shift: u32, shift_by_reg: bool) -> u32 {
    // Op2 is filled gy bit 31 of rm
    if shift == 0 {
        if shift_by_reg {
            return value;
        } else if value & 0x8000_0000 == 0 {
            return 0;
        }
        return 0xFFFF_FFFF;
    }
    if value & (1 << 31) == 0 {
        value.checked_shr(shift).unwrap_or_default()
    } else if shift < 32 {
        value.checked_shr(shift).unwrap_or_default() | (0xFFFF_FFFF_u32.checked_shl(32 - shift).unwrap_or_default())
    } else {
        // Negative value shifted by >= 32: the sign bit fills the whole word.
        0xFFFF_FFFF
    }
}

pub const fn ror(value: u32, shift: u32, c: bool, shift_by_reg: bool) -> u32 {
    if shift == 0 {
        if shift_by_reg {
            value
        } else {
            // ROR#0 interpreted as RRX (rotate right with extend)
            (value >> 1) | if c { 0x8000_0000 } else { 0 }
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
        assert_eq!(lsl(0x0000_0001, 1), 0x0000_0002);
        assert_eq!(lsl(0x0000_0001, 4), 0x0000_0010);
        assert_eq!(lsl(0x8000_0000, 1), 0x0000_0000); // overflow
    }

    #[test]
    fn test_lsl_zero_shift() {
        assert_eq!(lsl(0xA5A5_A5A5, 0), 0xA5A5_A5A5); // no shift
    }

    #[test]
    fn test_lsr_basic() {
        assert_eq!(lsr(0x0000_0002, 1, false), 0x0000_0001);
        assert_eq!(lsr(0x0000_0010, 4, false), 0x0000_0001);
        assert_eq!(lsr(0x8000_0000, 1, false), 0x4000_0000);
    }

    #[test]
    fn test_lsr_zero_shift() {
        // LSR#0 interpreted as LSR#32 -> result becomes 0
        assert_eq!(lsr(0xA5A5_A5A5, 0, false), 0x0000_0000);
        // LSR by register with 0 shift -> no shift
        assert_eq!(lsr(0xA5A5_A5A5, 0, true), 0xA5A5_A5A5);
    }

    #[test]
    fn test_asr_basic() {
        assert_eq!(asr(0x0000_0002, 1, false), 0x0000_0001);
        assert_eq!(asr(0x8000_0000, 1, false), 0xC000_0000); // sign extend
        assert_eq!(asr(0x4000_0000, 1, false), 0x2000_0000); // positive
    }

    #[test]
    fn test_asr_zero_shift() {
        // ASR#0 interpreted as ASR#32 -> filled with bit 31
        assert_eq!(asr(0x8000_0000, 0, false), 0xFFFF_FFFF); // negative
        assert_eq!(asr(0x4000_0000, 0, false), 0x0000_0000); // positive
        // ASR by register with 0 shift -> no shift
        assert_eq!(asr(0x8000_0000, 0, true), 0x8000_0000);
    }

    #[test]
    fn test_ror_basic() {
        assert_eq!(ror(0xA5A5_A5A5, 4, false, false), 0x5A5A_5A5A);
        assert_eq!(ror(0x1234_5678, 8, false, false), 0x7812_3456);
    }

    #[test]
    fn test_ror_zero_shift_rrx() {
        // ROR#0 interpreted as RRX (rotate right with extend)
        assert_eq!(ror(0x0000_0001, 0, false, false), 0x0000_0000); // C=0, bit 0 -> C
        assert_eq!(ror(0x0000_0001, 0, true, false), 0x8000_0000);  // C=1, old C -> bit 31
        assert_eq!(ror(0x0000_0002, 0, false, false), 0x0000_0001);
        // ROR by register with 0 shift -> no shift
        assert_eq!(ror(0x0000_0001, 0, false, true), 0x0000_0001);
    }

    // Carry flag tests
    #[test]
    fn test_lsl_carry() {
        // LSL carry tests
        assert_eq!(is_carry_over(Shift::LSL, 0x8000_0000, 1, false, false), true);
        assert_eq!(is_carry_over(Shift::LSL, 0x4000_0000, 1, false, false), false);
        assert_eq!(is_carry_over(Shift::LSL, 0x4000_0000, 2, false, false), true);
        assert_eq!(is_carry_over(Shift::LSL, 0x2000_0000, 2, false, false), false);
        // LSL#0 doesn't affect carry
        assert_eq!(is_carry_over(Shift::LSL, 0x8000_0000, 0, true, false), true);
        assert_eq!(is_carry_over(Shift::LSL, 0x8000_0000, 0, false, false), false);
    }

    #[test]
    fn test_lsr_carry() {
        // LSR carry tests
        assert_eq!(is_carry_over(Shift::LSR, 0x0000_0001, 1, false, false), true);
        assert_eq!(is_carry_over(Shift::LSR, 0x0000_0002, 1, false, false), false);
        assert_eq!(is_carry_over(Shift::LSR, 0x0000_0003, 2, false, false), true);
        // LSR#0 interpreted as LSR#32 -> C becomes bit 31
        assert_eq!(is_carry_over(Shift::LSR, 0x8000_0000, 0, false, false), true);
        assert_eq!(is_carry_over(Shift::LSR, 0x4000_0000, 0, false, false), false);
        // LSR by register with 0 shift -> doesn't affect carry
        assert_eq!(is_carry_over(Shift::LSR, 0x8000_0000, 0, true, true), true);
        assert_eq!(is_carry_over(Shift::LSR, 0x8000_0000, 0, false, true), false);
    }

    #[test]
    fn test_asr_carry() {
        // ASR carry tests
        assert_eq!(is_carry_over(Shift::ASR, 0x0000_0001, 1, false, false), true);
        assert_eq!(is_carry_over(Shift::ASR, 0x0000_0002, 1, false, false), false);
        assert_eq!(is_carry_over(Shift::ASR, 0x0000_0003, 2, false, false), true);
        // ASR#0 interpreted as ASR#32 -> C becomes bit 31
        assert_eq!(is_carry_over(Shift::ASR, 0x8000_0000, 0, false, false), true);
        assert_eq!(is_carry_over(Shift::ASR, 0x4000_0000, 0, false, false), false);
    }

    #[test]
    fn test_ror_carry() {
        // ROR carry tests
        assert_eq!(is_carry_over(Shift::ROR, 0x0000_0001, 1, false, false), true);
        assert_eq!(is_carry_over(Shift::ROR, 0x0000_0002, 1, false, false), false);
        assert_eq!(is_carry_over(Shift::ROR, 0x0000_0003, 2, false, false), true);
        // ROR#0 interpreted as RRX -> C becomes bit 0
        assert_eq!(is_carry_over(Shift::ROR, 0x0000_0001, 0, false, false), true);
        assert_eq!(is_carry_over(Shift::ROR, 0x0000_0002, 0, false, false), false);
    }

    #[test]
    fn test_shift_large_amounts() {
        // Test shifts >= 32
        assert_eq!(lsl(0xFFFF_FFFF, 32), 0x0000_0000);
        assert_eq!(lsr(0xFFFF_FFFF, 32, false), 0x0000_0000);
        assert_eq!(asr(0x8000_0000, 32, false), 0xFFFF_FFFF);
        assert_eq!(asr(0x4000_0000, 32, false), 0x0000_0000);
        
        // Carry for large shifts
        assert_eq!(is_carry_over(Shift::LSL, 0x0000_0001, 32, false, false), true);  // bit 0 -> carry
        assert_eq!(is_carry_over(Shift::LSL, 0x0000_0001, 33, false, false), false); // > 32 -> false
    }

    #[test]
    fn test_register_shift_normalization() {
        // 下位8bitのみ有効：0x100 は実質 0
        assert_eq!(shift(Shift::LSL, 0x1234_5678, 0x100, false, true), 0x1234_5678);
        assert_eq!(is_carry_over(Shift::LSL, 0x8000_0000, 0x100, false, true), false); // 変化なし

        // ROR(by reg), Rs=32 → 結果は同じ・Cはbit31
        assert_eq!(shift(Shift::ROR, 0x8000_0001, 32, false, true), 0x8000_0001);
        assert!(is_carry_over(Shift::ROR, 0x8000_0001, 32, false, true));

        // Rs=0x100（≒0）→ 変化なし・Cも変化なし
        assert_eq!(shift(Shift::ROR, 0xDEAD_BEEF, 0x100, true, true), 0xDEAD_BEEF);
        assert_eq!(is_carry_over(Shift::ROR, 0xDEAD_BEEF, 0x100, true, true), true);
        
        // LSR by register with upper bits set should be normalized
        assert_eq!(shift(Shift::LSR, 0xFFFF_FFFF, 0x101, false, true), 0x7FFF_FFFF); // 0x101 & 0xFF = 1
        assert!(is_carry_over(Shift::LSR, 0xFFFF_FFFF, 0x101, false, true));
        
        // ASR by register with upper bits set should be normalized  
        assert_eq!(shift(Shift::ASR, 0x8000_0000, 0x201, false, true), 0xC000_0000); // 0x201 & 0xFF = 1
        assert!(!is_carry_over(Shift::ASR, 0x8000_0000, 0x201, false, true)); // bit 0 is 0
    }
}
