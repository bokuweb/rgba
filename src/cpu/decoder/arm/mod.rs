mod block_data_transfer;
mod branch;
mod branch_and_exchange;
mod data_processing;
mod extra_memory;
mod memory;
mod multiple;
mod psr_transfer;
mod single_data_swap;
mod swi;

pub use block_data_transfer::*;
pub use branch::*;
pub use branch_and_exchange::*;
pub use data_processing::*;
pub use extra_memory::*;
pub use memory::*;
pub use multiple::*;
pub use psr_transfer::*;
pub use single_data_swap::*;
pub use swi::*;

use crate::cpu::types::Cond;
use crate::types::Word;

#[derive(Debug, PartialEq, Clone)]
pub enum InstructionType {
    Undefined,
    PsrTransfer,
    Multiple,
    Memory,
    ExtraMemory,
    DataProcessing,
    Branch,
    BranchAndExchange,
    BlockDataTransfer,
    SingleDataSwap,
    Swi,
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
    BX(BranchAndExchange),
    LDM(BlockDataTransfer),
    STM(BlockDataTransfer),
    Undefined,
    SWI(SoftwareInterrupt),
    MSR(PsrTransfer),
    MRS(PsrTransfer),
    SWP(SingleDataSwap),
    SWPB(SingleDataSwap),
    // NOP,
}

// #[derive(Debug, PartialEq)]
// pub enum Condition {
//     EQ,
//     NE,
//     CS_HS,
//     CC_LO,
//     MI,
//     PL,
//     VS,
//     VC,
//     HI,
//     LS,
//     GE,
//     LT,
//     GT,
//     LE,
//     AL,
// }

#[derive(Debug, PartialEq, Clone)]
pub enum IndexMode {
    PostIndex,
    Unsupported,
    Offset,
    PreIndex,
}

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

fn decode_psr_transfer(raw: Word) -> Instruction {
    match raw {
        v if (v & 0x00b0_f000) == 0x0020_f000 => Instruction::MSR(PsrTransfer(raw)),
        _ => Instruction::MRS(PsrTransfer(raw)),
    }
}

fn decode_single_data_swap(raw: Word) -> Instruction {
    let dec = SingleDataSwap(raw);
    match dec.get_B() {
        true => Instruction::SWPB(dec),
        false => Instruction::SWP(dec),
    }
}

fn decode_memory(raw: Word) -> Instruction {
    match raw {
        v if (v & 0x0050_0000) == 0x0050_0000 => Instruction::LDRB(Memory(raw)),
        v if (v & 0x0010_0000) == 0x0010_0000 => Instruction::LDR(Memory(raw)),
        v if (v & 0x0040_0000) == 0x0040_0000 => Instruction::STRB(Memory(raw)),
        _ => Instruction::STR(Memory(raw)),
    }
}

fn decode_extra_memory(raw: Word) -> Instruction {
    let dec = ExtraMemory(raw);
    let op2 = dec.get_op2();
    let l = dec.get_L();
    match op2 {
        0b01 if !l => Instruction::STRH(dec),
        0b01 if l => Instruction::LDRH(dec),
        0b10 if l => Instruction::LDRSB(dec),
        0b11 if l => Instruction::LDRSH(dec),
        _ => panic!("undefined instruction detected"),
    }
}

fn decode_data_processing(raw: Word) -> Instruction {
    let dec = DataProcessing(raw);
    let cmd = dec.get_opcode();
    let S = dec.get_S();
    let I = dec.get_I();
    let sh = dec.get_sh();
    let instr = (raw & 0b1111_1111_0000) >> 4;
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

fn decode_block_data_transfer(raw: Word) -> Instruction {
    let dec = BlockDataTransfer(raw);
    if dec.get_L() {
        Instruction::LDM(dec)
    } else {
        Instruction::STM(dec)
    }
}

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
    // dbg!(raw);

    let instruction_type = match raw {
        // SWI must be detected before other overlapping classes
        v if (v & 0x0F00_0000) == 0x0F00_0000 => InstructionType::Swi,
        v if ((v & 0x0ffffff0) == 0x012fff10) => InstructionType::BranchAndExchange,
        v if (v & 0x0F80_0FF0) == 0x0100_0090 => InstructionType::SingleDataSwap,
        v if (v & 0x0E00_0000) == 0x0A00_0000 => InstructionType::Branch,
        v if (v & 0x0E00_0000) == 0x0800_0000 => InstructionType::BlockDataTransfer, // LDM and STM,
        v if (v & 0x0180_0000) == 0x0100_0000 && (v & 0x0010_0000) == 0x0 => InstructionType::PsrTransfer,
        v if (v & 0x0FC0_00F0) == 0x0000_0090 => InstructionType::Multiple,
        v if (v & 0x0F80_00F0) == 0x0080_0090 => InstructionType::Multiple,
        v if (v & 0x0E00_0010) == 0x0600_0010 => InstructionType::Undefined,
        v if (v & 0x0E40_0F90) == 0x0000_0090 => InstructionType::ExtraMemory,
        v if (v & 0x0E40_0090) == 0x0040_0090 => InstructionType::ExtraMemory,
        v if (v & 0x0C00_0000) == 0x0400_0000 => InstructionType::Memory,
        v if (v & 0x0C00_0000) == 0x0000_0000 => InstructionType::DataProcessing,
        _ => panic!("Unsupported instruction"),
    };

    match instruction_type {
        InstructionType::Undefined => Instruction::Undefined,
        InstructionType::PsrTransfer => decode_psr_transfer(raw),
        InstructionType::SingleDataSwap => decode_single_data_swap(raw),
        InstructionType::Multiple => decode_multiple(raw),
        InstructionType::Memory => decode_memory(raw),
        InstructionType::ExtraMemory => decode_extra_memory(raw),
        InstructionType::DataProcessing => decode_data_processing(raw),
        InstructionType::Branch => decode_branch(raw),
        InstructionType::BranchAndExchange => Instruction::BX(BranchAndExchange(raw)),
        InstructionType::BlockDataTransfer => decode_block_data_transfer(raw),
        InstructionType::Swi => Instruction::SWI(decode_swi(raw)),
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
