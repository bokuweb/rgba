mod cond;

pub use cond::*;

// pub type Byte = u8;
// pub type HalfWord = u16;
// pub type Word = u32;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Shift {
    LSL,
    LSR,
    ASR,
    ROR,
}

impl Into<Shift> for u32 {
    fn into(self) -> Shift {
        match self {
            0b00 => Shift::LSL,
            0b01 => Shift::LSR,
            0b10 => Shift::ASR,
            0b11 => Shift::ROR,
            _ => panic!("shift value should be 0b00~0b11. {} is illegal value", self),
        }
    }
}
