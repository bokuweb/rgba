// Conditions
pub(crate) const COND_FIELD: u32 = 0xF0000000;
pub(crate) const COND_EQ: u32 = 0x00000000;
pub(crate) const COND_NE: u32 = 0x10000000;
pub(crate) const COND_CS_HS: u32 = 0x20000000;
pub(crate) const COND_CC_LO: u32 = 0x30000000;
pub(crate) const COND_MI: u32 = 0x40000000;
pub(crate) const COND_PL: u32 = 0x50000000;
pub(crate) const COND_VS: u32 = 0x60000000;
pub(crate) const COND_VC: u32 = 0x70000000;
pub(crate) const COND_HI: u32 = 0x80000000;
pub(crate) const COND_LS: u32 = 0x90000000;
pub(crate) const COND_GE: u32 = 0xA0000000;
pub(crate) const COND_LT: u32 = 0xB0000000;
pub(crate) const COND_GT: u32 = 0xC0000000;
pub(crate) const COND_LE: u32 = 0xD0000000;
pub(crate) const COND_AL: u32 = 0xE0000000;

// Op
pub(crate) const OP_FIELD: u32 = 0x0C000000;
pub(crate) const OP_MEM: u32 = 0x04000000;
pub(crate) const OP_DATA: u32 = 0x00000000;

// Funct
pub(crate) const FUNCT_FIELD: u32 = 0x03F00000;
pub(crate) const FUNCT_I: u32 = 0x02000000;
pub(crate) const FUNCT_P: u32 = 0x01000000;
pub(crate) const FUNCT_U: u32 = 0x00800000;
pub(crate) const FUNCT_B: u32 = 0x00400000;
pub(crate) const FUNCT_W: u32 = 0x00200000;
pub(crate) const FUNCT_L: u32 = 0x00100000;

// Base
pub(crate) const RN: u32 = 0x000F0000;

// Dist
pub(crate) const RD: u32 = 0x0000F000;


pub(crate) const SP: usize = 13;
pub(crate) const LR: usize = 14;
pub(crate) const PC: usize = 15;

pub const PC_OFFSET: u32 = 2;
