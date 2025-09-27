use crate::types::Word;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoftwareInterrupt {
    pub raw: Word,
}

impl SoftwareInterrupt {
    pub fn new(raw: Word) -> Self {
        Self { raw }
    }

    /// Get the immediate value (comment field) from SWI instruction
    /// In ARM mode, only the upper 8 bits of the 24-bit comment field are used
    pub fn get_immediate(&self) -> u32 {
        (self.raw >> 16) & 0xFF
    }

    /// Get the full 24-bit comment field
    pub fn get_comment(&self) -> u32 {
        self.raw & 0x00FFFFFF
    }
}

pub fn decode_swi(raw: Word) -> SoftwareInterrupt {
    SoftwareInterrupt::new(raw)
}