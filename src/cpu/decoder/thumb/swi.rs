use crate::types::HalfWord;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ThumbSoftwareInterrupt(HalfWord);

impl ThumbSoftwareInterrupt {
    pub const fn new(raw: HalfWord) -> Self { Self(raw) }
    pub const fn get_immediate(&self) -> u8 { (self.0 & 0x00FF) as u8 }
}

pub const fn decode_thumb_swi(raw: HalfWord) -> ThumbSoftwareInterrupt {
    ThumbSoftwareInterrupt::new(raw)
}


