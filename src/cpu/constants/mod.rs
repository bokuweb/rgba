// Conditions
pub const COND_FIELD: u32 = 0xF000_0000;
pub const COND_EQ: u32 = 0x0000_0000;
pub const COND_NE: u32 = 0x1000_0000;
pub const COND_CS_HS: u32 = 0x2000_0000;
pub const COND_CC_LO: u32 = 0x3000_0000;
pub const COND_MI: u32 = 0x4000_0000;
pub const COND_PL: u32 = 0x5000_0000;
pub const COND_VS: u32 = 0x6000_0000;
pub const COND_VC: u32 = 0x7000_0000;
pub const COND_HI: u32 = 0x8000_0000;
pub const COND_LS: u32 = 0x9000_0000;
pub const COND_GE: u32 = 0xA000_0000;
pub const COND_LT: u32 = 0xB000_0000;
pub const COND_GT: u32 = 0xC000_0000;
pub const COND_LE: u32 = 0xD000_0000;
pub const COND_AL: u32 = 0xE000_0000;

// Op
pub const OP_FIELD: u32 = 0x0C00_0000;
pub const OP_MEM: u32 = 0x0400_0000;
pub const OP_DATA: u32 = 0x0000_0000;

// Funct
pub const FUNCT_FIELD: u32 = 0x03F0_0000;
pub const FUNCT_I: u32 = 0x0200_0000;
pub const FUNCT_P: u32 = 0x0100_0000;
pub const FUNCT_U: u32 = 0x0080_0000;
pub const FUNCT_B: u32 = 0x0040_0000;
pub const FUNCT_W: u32 = 0x0020_0000;
pub const FUNCT_L: u32 = 0x0010_0000;

// Base
pub const RN: u32 = 0x000F_0000;

// Dist
pub const RD: u32 = 0x0000_F000;


pub const SP: usize = 13;
pub const LR: usize = 14;
pub const PC: usize = 15;

pub const PC_OFFSET: u32 = 2;
