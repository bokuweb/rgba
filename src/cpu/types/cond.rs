#[derive(Debug, PartialEq, Clone, Copy)]
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
}

impl Into<Cond> for u32 {
    fn into(self) -> Cond {
        match self {
            0b0000 => Cond::EQ,
            0b0001 => Cond::NE,
            0b0010 => Cond::CS,
            0b0011 => Cond::CC,
            0b0100 => Cond::MI,
            0b0101 => Cond::PL,
            0b0110 => Cond::VS,
            0b0111 => Cond::VC,
            0b1000 => Cond::HI,
            0b1001 => Cond::LS,
            0b1010 => Cond::GE,
            0b1011 => Cond::LT,
            0b1100 => Cond::GT,
            0b1101 => Cond::LE,
            0b1110 => Cond::AL,
            _ => panic!("illegal condition detected."),
        }
    }
}

impl Into<Cond> for u16 {
    fn into(self) -> Cond {
        match self {
            0b0000 => Cond::EQ,
            0b0001 => Cond::NE,
            0b0010 => Cond::CS,
            0b0011 => Cond::CC,
            0b0100 => Cond::MI,
            0b0101 => Cond::PL,
            0b0110 => Cond::VS,
            0b0111 => Cond::VC,
            0b1000 => Cond::HI,
            0b1001 => Cond::LS,
            0b1010 => Cond::GE,
            0b1011 => Cond::LT,
            0b1100 => Cond::GT,
            0b1101 => Cond::LE,
            0b1110 => Cond::AL,
            _ => panic!("illegal condition detected."),
        }
    }
}
