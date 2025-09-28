use crate::types::HalfWord;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ThumbSoftwareInterrupt(HalfWord);

impl ThumbSoftwareInterrupt {
    pub fn new(raw: HalfWord) -> Self { Self(raw) }
    pub fn get_immediate(&self) -> u8 { (self.0 & 0x00FF) as u8 }
}

pub fn decode_thumb_swi(raw: HalfWord) -> ThumbSoftwareInterrupt {
    ThumbSoftwareInterrupt::new(raw)
}


