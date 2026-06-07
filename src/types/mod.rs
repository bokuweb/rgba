pub type Cycle = usize;

pub type Byte = u8;
pub type HalfWord = u16;
pub type Word = u32;

#[derive(Debug, Copy, Clone)]
pub enum AccessType {
    NonSeq(AccessWidth),
    Seq(AccessWidth),
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum AccessWidth {
    Byte,
    HalfWord,
    Word,
}
