use crate::types::Word;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SoftwareInterrupt {
    pub raw: Word,
}

impl SoftwareInterrupt {
    pub const fn new(raw: Word) -> Self {
        Self { raw }
    }

    /// Get the immediate value (comment field) from SWI instruction
    /// In ARM mode, only the upper 8 bits of the 24-bit comment field are used
    pub const fn get_immediate(&self) -> u32 {
        (self.raw >> 16) & 0xFF
    }

    /// Get the full 24-bit comment field
    pub const fn get_comment(&self) -> u32 {
        self.raw & 0x00FF_FFFF
    }
}

pub const fn decode_swi(raw: Word) -> SoftwareInterrupt {
    SoftwareInterrupt::new(raw)
}