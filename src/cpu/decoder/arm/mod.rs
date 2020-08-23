mod field;

pub use field::*;

use super::super::constants::COND_FIELD;
use super::super::types::{Shift, Word};

#[derive(Debug, PartialEq, Clone)]
pub enum InstructionType {
    Undefined,
    // ProgramStatusRegister,
    Multiple,
    Memory,
    ExtraMemory,
    DataProcessing,
    Branch,
    // MultiLoadAndStore,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Instruction {
    // DataProcessing
    AND(DataProcessing),
    EOR(DataProcessing),
    SUB(DataProcessing),
    RSB(DataProcessing),
    ADD(DataProcessing),
    ADC(DataProcessing),
    SBC(DataProcessing),
    RSC(DataProcessing),
    TST(DataProcessing),
    TEQ(DataProcessing),
    CMP(DataProcessing),
    CMN(DataProcessing),
    ORR(DataProcessing),
    MOV(DataProcessing),
    LSL(DataProcessing),
    LSR(DataProcessing),
    ASR(DataProcessing),
    RRX(DataProcessing),
    ROR(DataProcessing),
    BIC(DataProcessing),
    MVN(DataProcessing),
    MUL(Multiple),
    MLA(Multiple),
    UMULL(Multiple),
    UMLAL(Multiple),
    SMULL(Multiple),
    SMLAL(Multiple),
    STR(Memory),
    LDR(Memory),
    LDRB(Memory),
    STRB(Memory),
    STRH(ExtraMemory),
    LDRH(ExtraMemory),
    LDRSB(ExtraMemory),
    LDRSH(ExtraMemory),
    B(Branch),
    BL(Branch),
    // LDM,
    // STM,
    Undefined,
    // // SWI,
    // MSR,
    // MRS,
    // NOP,
} //

#[derive(Debug, PartialEq)]
pub enum Condition {
    EQ,
    NE,
    CS_HS,
    CC_LO,
    MI,
    PL,
    VS,
    VC,
    HI,
    LS,
    GE,
    LT,
    GT,
    LE,
    AL,
}

#[derive(Debug, PartialEq, Clone)]
pub enum IndexMode {
    PostIndex,
    Unsupported,
    Offset,
    PreIndex,
}

#[derive(Debug)]
pub struct BaseDecoder {
    // pub cond: Condition,
    pub opcode: Instruction,
    // pub category: InstructionType,
    pub raw: Word,
}

#[derive(Debug)]
pub struct MultipleDecoder(BaseDecoder);
#[derive(Debug)]
pub struct ExtraMemoryDecoder(BaseDecoder);

#[derive(Debug)]
pub struct ProgramStatusRegisterDecoder(BaseDecoder);

// #[derive(Debug)]
// pub struct MultiLoadAndStoreDecoder(BaseDecoder);

// pub const RAW_NOP: Word = 0b0000_00_0_1101_0_0000_0000_00000000_0000;

pub fn is_load(raw: Word) -> bool {
    raw & 0x0010_0000 != 0
}

#[allow(non_snake_case)]
fn get_I(raw: Word) -> Word {
    (raw & 0x0200_0000) >> 25
}

fn get_sh(raw: Word) -> Word {
    (raw & 0b11_0_0000) >> 5
}

fn get_instr(raw: Word) -> Word {
    (raw & 0b1111_1111_0000) >> 4
}

#[allow(non_snake_case)]
fn get_S(raw: Word) -> Word {
    (raw & 0x0010_0000) >> 20
}

pub trait Raw {
    fn raw(&self) -> u32;
    fn op(&self) -> Instruction;
}

pub trait Decoder: Raw {
    fn opcode(&self) -> Instruction {
        self.op()
    }

    #[allow(non_snake_case)]
    fn get_Rn(&self) -> usize {
        (self.raw() as usize >> 16) & 0b1111
    }

    #[allow(non_snake_case)]
    fn get_Rd(&self) -> usize {
        (self.raw() as usize >> 12) & 0b1111
    }

    #[allow(non_snake_case)]
    fn get_Ra(&self) -> usize {
        panic!("Ra field is not supported in default decoder");
    }

    fn get_src2(&self) -> usize {
        (self.raw() as usize) & 0x0fff
    }

    fn get_imm24(&self) -> i32 {
        self.raw() as i32 & 0xff_ffff
    }

    #[allow(non_snake_case)]
    fn has_I(&self) -> bool {
        self.raw() & 0x0200_0000 != 0
    }

    fn is_pre_indexed(&self) -> bool {
        (self.raw() & (1 << 24)) != 0
    }

    fn is_write_back(&self) -> bool {
        (self.raw() & (1 << 21)) != 0
    }

    // Program status register
    #[allow(non_snake_case)]
    fn has_R(&self) -> bool {
        (self.raw() & (1 << 22)) != 0
    }

    #[allow(non_snake_case)]
    fn get_SBO(&self) -> usize {
        (self.raw() as usize >> 12) & 0b1111
    }

    fn is_load(&self) -> bool {
        is_load(self.raw())
    }

    #[allow(non_snake_case)]
    fn get_memory_index_mode(&self) -> IndexMode {
        let P = (self.raw() & (1 << 24)) != 0;
        let W = (self.raw() & (1 << 21)) != 0;
        match (P, W) {
            (false, false) => IndexMode::PostIndex,
            (false, true) => IndexMode::Unsupported,
            (true, false) => IndexMode::Offset,
            (true, true) => IndexMode::PreIndex,
        }
    }

    #[allow(non_snake_case)]
    fn get_Rm(&self) -> usize {
        self.raw() as usize & 0b1111
    }

    fn get_sh(&self) -> Shift {
        match (self.raw() & 0b11_0_0000) >> 5 {
            0b00 => Shift::LSL,
            0b01 => Shift::LSR,
            0b10 => Shift::ASR,
            0b11 => Shift::ROR,
            _ => unreachable!(),
        }
    }

    #[allow(non_snake_case)]
    fn get_Rs(&self) -> u32 {
        (self.raw() & 0b1111_0_00_0_0000) >> 8
    }

    fn get_shamt5(&self) -> u32 {
        (self.raw() & 0b11111_00_0_0000) >> 7
    }

    fn get_imm8(&self) -> u32 {
        self.raw() & 0xFF
    }

    fn get_rot(&self) -> u32 {
        (self.raw() & 0x0000_0F_00) >> 8
    }

    // pub fn has_B(&self) -> bool {
    //     self.raw & 0x0040_0000 != 0
    // }

    // Bit: 23
    fn is_plus_offset(&self) -> bool {
        self.raw() & 0x0080_0000 != 0
    }

    fn is_reg_offset(&self) -> bool {
        self.raw() & 0x0000_0010 != 0
    }

    fn is_minus_offset(&self) -> bool {
        !self.is_plus_offset()
    }

    // fn is_branch_with_link(&self) -> bool {
    //     self.raw & 0x0100_0000 != 0
    // }
}

impl Decoder for BaseDecoder {}

// impl Decoder for MultiLoadAndStoreDecoder {}

impl Decoder for MultipleDecoder {
    #[allow(non_snake_case)]
    fn get_Ra(&self) -> usize {
        (self.raw() as usize >> 12) & 0b1111
    }

    #[allow(non_snake_case)]
    fn get_Rd(&self) -> usize {
        (self.raw() as usize >> 16) & 0b1111
    }

    #[allow(non_snake_case)]
    fn get_Rn(&self) -> usize {
        self.raw() as usize & 0b1111
    }

    fn get_Rm(&self) -> usize {
        (self.raw() >> 8) as usize & 0b1111
    }
}

impl Decoder for ExtraMemoryDecoder {
    #[allow(non_snake_case)]
    fn has_I(&self) -> bool {
        self.raw() & 0x0040_0000 != 0
    }

    fn get_imm8(&self) -> u32 {
        (self.raw() & 0xF00).wrapping_shr(4) + (self.raw() & 0xF)
    }
}

// impl MultiLoadAndStoreDecoder {
//     #[allow(non_snake_case)]
//     fn get_register_list(&self) -> usize {
//         (self.raw() & 0xFFFF) as usize
//     }
// }

impl Raw for BaseDecoder {
    fn raw(&self) -> u32 {
        self.raw
    }

    fn op(&self) -> Instruction {
        self.opcode.clone()
    }
}

impl Raw for MultipleDecoder {
    fn raw(&self) -> u32 {
        self.0.raw
    }

    fn op(&self) -> Instruction {
        self.0.opcode.clone()
    }
}

impl Raw for ExtraMemoryDecoder {
    fn raw(&self) -> u32 {
        self.0.raw
    }

    fn op(&self) -> Instruction {
        self.0.opcode.clone()
    }
}

// impl Raw for MultiLoadAndStoreDecoder {
//     fn raw(&self) -> u32 {
//         self.0.raw
//     }
//
//     fn op(&self) -> Instruction {
//         self.0.opcode.clone()
//     }
// }

fn decode_multiple(raw: Word) -> Instruction {
    let cmd = (raw & 0x01E0_0000) >> 21;
    match cmd {
        0b0000 => Instruction::MUL(Multiple(raw)),
        0b0001 => Instruction::MLA(Multiple(raw)),
        0b0100 => Instruction::UMULL(Multiple(raw)),
        0b0101 => Instruction::UMLAL(Multiple(raw)),
        0b0110 => Instruction::SMULL(Multiple(raw)),
        0b0111 => Instruction::SMLAL(Multiple(raw)),
        _ => unimplemented!(),
    }
}

// fn decode_program_status_register(raw: Word) -> Instruction {
//     match raw {
//         v if (v & 0x00b0_f000) == 0x0020_f000 => Instruction::MSR,
//         _ => Instruction::MRS,
//     }
// }

fn decode_memory(raw: Word) -> Instruction {
    match raw {
        v if (v & 0x0050_0000) == 0x0050_0000 => Instruction::LDRB(Memory(raw)),
        v if (v & 0x0010_0000) == 0x0010_0000 => Instruction::LDR(Memory(raw)),
        v if (v & 0x0040_0000) == 0x0040_0000 => Instruction::STRB(Memory(raw)),
        _ => Instruction::STR(Memory(raw)),
    }
}

fn decode_extra_memory(raw: Word) -> Instruction {
    let op2 = (raw >> 5) & 0b11;
    let l = is_load(raw);
    match op2 {
        0b01 if !l => Instruction::STRH(ExtraMemory(raw)),
        0b01 if l => Instruction::LDRH(ExtraMemory(raw)),
        0b10 if l => Instruction::LDRSB(ExtraMemory(raw)),
        0b11 if l => Instruction::LDRSH(ExtraMemory(raw)),
        _ => panic!("undefined instruction detected"),
    }
}

fn decode_data_processing(raw: Word) -> Instruction {
    let cmd = (raw & 0x01E0_0000) >> 21;
    let S = get_S(raw) != 0;
    let I = get_I(raw) != 0;
    let sh = get_sh(raw);
    let instr = get_instr(raw);
    //debug!("data processing cmd = {:x}", cmd);
    match cmd {
        0b0000 => Instruction::AND(DataProcessing(raw)),
        0b0001 => Instruction::EOR(DataProcessing(raw)),
        0b0010 => Instruction::SUB(DataProcessing(raw)),
        0b0011 => Instruction::RSB(DataProcessing(raw)),
        0b0100 => Instruction::ADD(DataProcessing(raw)),
        0b0101 => Instruction::ADC(DataProcessing(raw)),
        0b0110 => Instruction::SBC(DataProcessing(raw)),
        0b0111 => Instruction::RSC(DataProcessing(raw)),
        0b1000 if S => Instruction::TST(DataProcessing(raw)),
        0b1001 if S => Instruction::TEQ(DataProcessing(raw)),
        0b1010 if S => Instruction::CMP(DataProcessing(raw)),
        0b1011 if S => Instruction::CMN(DataProcessing(raw)),
        0b1100 => Instruction::ORR(DataProcessing(raw)),
        0b1101 if I || instr == 0 => Instruction::MOV(DataProcessing(raw)),
        0b1101 if !I && sh == 0b00 => Instruction::LSL(DataProcessing(raw)),
        0b1101 if !I && sh == 0b01 => Instruction::LSR(DataProcessing(raw)),
        0b1101 if !I && sh == 0b10 => Instruction::ASR(DataProcessing(raw)),
        0b1101 if !I && sh == 0b11 && (instr & 0xF90) == 0 => Instruction::RRX(DataProcessing(raw)),
        0b1101 if !I && sh == 0b11 && instr != 0 => Instruction::ROR(DataProcessing(raw)),
        0b1110 => Instruction::BIC(DataProcessing(raw)),
        0b1111 => Instruction::MVN(DataProcessing(raw)),
        _ => panic!("unsupported instruction"),
    }
}

// fn decode_multi_load_and_store(raw: Word) -> Instruction {
//     if is_load(raw) {
//         Instruction::LDM
//     } else {
//         Instruction::STM
//     }
// }

fn decode_branch(raw: Word) -> Instruction {
    let with_link = raw & 0x0100_0000 != 0;
    if with_link {
        Instruction::BL(Branch(raw))
    } else {
        Instruction::B(Branch(raw))
    }
}

pub fn decode(raw: Word) -> Instruction {
    // let cond = raw & COND_FIELD;
    // let cond = match cond {
    //     COND_AL => Condition::AL,
    //     _ => panic!("Unknowm condition {}", cond),
    // };

    let instruction_type = match raw {
        // v if (v & 0x0180_0000) == 0x0100_0000 && (v & 0x0010_0000) == 0x0 => {
        //     InstructionType::ProgramStatusRegister
        // }
        v if (v & 0x0E00_0000) == 0x0A00_0000 => InstructionType::Branch,
        v if (v & 0x0FC0_00F0) == 0x0000_0090 => InstructionType::Multiple,
        v if (v & 0x0F80_00F0) == 0x0080_0090 => InstructionType::Multiple,
        v if (v & 0x0E00_0010) == 0x0600_0010 => InstructionType::Undefined,
        v if (v & 0x0E40_0F90) == 0x0000_0090 => InstructionType::ExtraMemory,
        v if (v & 0x0E40_0090) == 0x0040_0090 => InstructionType::ExtraMemory,
        v if (v & 0x0C00_0000) == 0x0400_0000 => InstructionType::Memory,
        v if (v & 0x0C00_0000) == 0x0000_0000 => InstructionType::DataProcessing,
        // v if (v & 0x0E00_0000) == 0x0800_0000 => InstructionType::MultiLoadAndStore, // LDM and STM,
        // v if (v & 0x0F00_0000) == 0x0F00_0000 => InstructionType::SWI,
        _ => panic!("Unsupported instruction"),
    };

    match instruction_type {
        InstructionType::Undefined => Instruction::Undefined,
        //  InstructionType::ProgramStatusRegister => decode_program_status_register(raw),
        InstructionType::Multiple => decode_multiple(raw),
        InstructionType::Memory => decode_memory(raw),
        InstructionType::ExtraMemory => decode_extra_memory(raw),
        InstructionType::DataProcessing => decode_data_processing(raw),
        InstructionType::Branch => decode_branch(raw),
        //  InstructionType::MultiLoadAndStore => decode_multi_load_and_store(raw),
        // v if (v & 0x0F00_0000) == 0x0F00_0000 => Instruction::SWI,
        _ => panic!("unsupported instruction"),
    }

    // debug!("opcode = {:?}", opcode);
    // let dec = BaseDecoder { raw, cond, opcode };
    // match category {
    //     InstructionType::Multiple => Box::new(MultipleDecoder(dec)),
    //     InstructionType::ExtraMemory => Box::new(ExtraMemoryDecoder(dec)),
    //     _ => Box::new(dec),
    // }
}
