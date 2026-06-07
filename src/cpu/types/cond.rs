#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Cond {
    EQ = 0b0000,
    NE = 0b0001,
    CS = 0b0010,
    CC = 0b0011,
    MI = 0b0100,
    PL = 0b0101,
    VS = 0b0110,
    VC = 0b0111,
    HI = 0b1000,
    LS = 0b1001,
    GE = 0b1010,
    LT = 0b1011,
    GT = 0b1100,
    LE = 0b1101,
    AL = 0b1110,
    NV = 0b1111,
}

impl From<u32> for Cond {
    fn from(val: u32) -> Self {
        match val {
            0b0000 => Self::EQ,
            0b0001 => Self::NE,
            0b0010 => Self::CS,
            0b0011 => Self::CC,
            0b0100 => Self::MI,
            0b0101 => Self::PL,
            0b0110 => Self::VS,
            0b0111 => Self::VC,
            0b1000 => Self::HI,
            0b1001 => Self::LS,
            0b1010 => Self::GE,
            0b1011 => Self::LT,
            0b1100 => Self::GT,
            0b1101 => Self::LE,
            0b1110 => Self::AL,
            0b1111 => Self::NV,
            _ => panic!("illegal condition detected."),
        }
    }
}

impl From<u16> for Cond {
    fn from(val: u16) -> Self {
        match val {
            0b0000 => Self::EQ,
            0b0001 => Self::NE,
            0b0010 => Self::CS,
            0b0011 => Self::CC,
            0b0100 => Self::MI,
            0b0101 => Self::PL,
            0b0110 => Self::VS,
            0b0111 => Self::VC,
            0b1000 => Self::HI,
            0b1001 => Self::LS,
            0b1010 => Self::GE,
            0b1011 => Self::LT,
            0b1100 => Self::GT,
            0b1101 => Self::LE,
            0b1110 => Self::AL,
            0b1111 => Self::NV,
            _ => panic!("illegal condition detected."),
        }
    }
}
