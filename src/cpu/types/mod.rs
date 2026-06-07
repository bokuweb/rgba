mod cond;

pub use cond::*;

// pub type Byte = u8;
// pub type HalfWord = u16;
// pub type Word = u32;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Shift {
    LSL,
    LSR,
    ASR,
    ROR,
}

impl From<u32> for Shift {
    fn from(val: u32) -> Self {
        match val {
            0b00 => Self::LSL,
            0b01 => Self::LSR,
            0b10 => Self::ASR,
            0b11 => Self::ROR,
            _ => panic!("shift value should be 0b00~0b11. {} is illegal value", val),
        }
    }
}
