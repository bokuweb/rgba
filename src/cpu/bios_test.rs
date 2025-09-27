#[cfg(test)]
mod tests {
    use super::bios::Bios;
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
        let bus = MockBus::new();
        let mut gpr = [0u32; 16];

        // Test: 100 / 10 = 10 remainder 0
        gpr[0] = 100;
        gpr[1] = 10;

        Bios::div(&bus, &mut gpr);

        assert_eq!(gpr[0], 10);  // quotient
        assert_eq!(gpr[1], 0);   // remainder
        assert_eq!(gpr[3], 10);  // absolute quotient
    }

    #[test]
    fn test_div_negative() {
        let bus = MockBus::new();
        let mut gpr = [0u32; 16];

        // Test: -123 / 10 = -12 remainder -3
        gpr[0] = (-123i32) as u32;
        gpr[1] = 10;

        Bios::div(&bus, &mut gpr);

        assert_eq!(gpr[0] as i32, -12); // quotient
        assert_eq!(gpr[1] as i32, -3);  // remainder
        assert_eq!(gpr[3], 12);         // absolute quotient
    }

    #[test]
    fn test_div_by_zero() {
        let bus = MockBus::new();
        let mut gpr = [0u32; 16];

        gpr[0] = 100;
        gpr[1] = 0; // Division by zero

        Bios::div(&bus, &mut gpr);

        // Should handle division by zero gracefully
        assert_eq!(gpr[0], 0x7FFFFFFF); // Max positive value
        assert_eq!(gpr[1], 100);        // Original numerator
        assert_eq!(gpr[3], 0x7FFFFFFF); // Max positive value
    }

    #[test]
    fn test_sqrt() {
        let bus = MockBus::new();
        let mut gpr = [0u32; 16];

        // Test sqrt(16) = 4
        gpr[0] = 16;
        Bios::sqrt(&bus, &mut gpr);
        assert_eq!(gpr[0], 4);

        // Test sqrt(0) = 0
        gpr[0] = 0;
        Bios::sqrt(&bus, &mut gpr);
        assert_eq!(gpr[0], 0);

        // Test sqrt(2) = 1 (integer result)
        gpr[0] = 2;
        Bios::sqrt(&bus, &mut gpr);
        assert_eq!(gpr[0], 1);
    }

    #[test]
    fn test_div_arm() {
        let bus = MockBus::new();
        let mut gpr = [0u32; 16];

        // DivArm swaps parameters: r1/r0 instead of r0/r1
        gpr[0] = 10;  // denominator for DivArm
        gpr[1] = 100; // numerator for DivArm

        Bios::div_arm(&bus, &mut gpr);

        assert_eq!(gpr[0], 10);  // 100/10 = 10
        assert_eq!(gpr[1], 0);   // remainder
        assert_eq!(gpr[3], 10);  // absolute quotient
    }

    #[test]
    fn test_cpu_set_copy() {
        let mut bus = MockBus::new();
        let mut gpr = [0u32; 16];

        // Set up source data
        bus.write_32(0x1000, 0x12345678);
        bus.write_32(0x1004, 0x9ABCDEF0);

        // Copy 2 words from 0x1000 to 0x2000
        gpr[0] = 0x1000; // source
        gpr[1] = 0x2000; // dest
        gpr[2] = 2 | 0x04000000; // count=2, word size

        Bios::cpu_set(&bus, &mut gpr);

        // Note: In a full implementation, we'd verify the writes occurred
        // For now, we just test that the function doesn't panic
    }

    #[test]
    fn test_arc_tan2_basic() {
        let bus = MockBus::new();
        let mut gpr = [0u32; 16];

        // Test atan2(1, 1) should be PI/4 ≈ 45 degrees
        gpr[0] = 16384; // x = 1.0 in 14-bit fixed point
        gpr[1] = 16384; // y = 1.0 in 14-bit fixed point

        Bios::arc_tan2(&bus, &mut gpr);

        // Result should be approximately 0x4000 (1/4 of full circle)
        assert!((gpr[0] as i32 - 0x4000).abs() < 0x100);
    }
}