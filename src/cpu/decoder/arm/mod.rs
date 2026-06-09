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

use crate::types::Word;

#[derive(Debug, PartialEq, Eq, Clone)]
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

#[derive(Debug, PartialEq, Eq, Clone)]
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IndexMode {
    PostIndex,
    Unsupported,
    Offset,
    PreIndex,
}

const fn decode_multiple(raw: Word) -> Instruction {
    let cmd = (raw & 0x01E0_0000) >> 21;
    match cmd {
        0b0000 => Instruction::MUL(Multiple(raw)),
        0b0001 => Instruction::MLA(Multiple(raw)),
        0b0100 => Instruction::UMULL(Multiple(raw)),
        0b0101 => Instruction::UMLAL(Multiple(raw)),
        0b0110 => Instruction::SMULL(Multiple(raw)),
        0b0111 => Instruction::SMLAL(Multiple(raw)),
        // Unknown multiply encoding: fall through to the undefined-instruction
        // exception instead of aborting the emulator.
        _ => Instruction::Undefined,
    }
}

const fn decode_psr_transfer(raw: Word) -> Instruction {
    match raw {
        v if (v & 0x00b0_f000) == 0x0020_f000 => Instruction::MSR(PsrTransfer(raw)),
        _ => Instruction::MRS(PsrTransfer(raw)),
    }
}

fn decode_single_data_swap(raw: Word) -> Instruction {
    let dec = SingleDataSwap(raw);
    if dec.get_B() { Instruction::SWPB(dec) } else { Instruction::SWP(dec) }
}

const fn decode_memory(raw: Word) -> Instruction {
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
        _ => Instruction::Undefined,
    }
}

// `S` and `I` are the ARM data-processing instruction's set-flags / immediate
// bits; the upper-case names mirror the encoding tables.
#[allow(non_snake_case)]
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
        _ => Instruction::Undefined,
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

const fn decode_branch(raw: Word) -> Instruction {
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
        v if ((v & 0x0fff_fff0) == 0x012f_ff10) => InstructionType::BranchAndExchange,
        v if (v & 0x0F80_0FF0) == 0x0100_0090 => InstructionType::SingleDataSwap,
        v if (v & 0x0E00_0000) == 0x0A00_0000 => InstructionType::Branch,
        v if (v & 0x0E00_0000) == 0x0800_0000 => InstructionType::BlockDataTransfer, // LDM and STM,
        // MRS/MSR live in the data-processing encoding space (bits 27:26 == 00)
        // with the "10xx" opcode slot (bits 24:23 == 10) and S=0 (bit 20 == 0).
        // Requiring bits 27:26 == 00 is essential: without it, pre-indexed
        // decrementing single-register stores like `STR Rd,[Rn,#-imm]!`
        // (e.g. `PUSH {lr}` = 0xe52de004, bits 27:26 == 01) also satisfy the
        // 24:23/20 test and get mis-decoded as MRS — corrupting the register
        // with CPSR and skipping the store/writeback.
        v if (v & 0x0C00_0000) == 0x0
            && (v & 0x0180_0000) == 0x0100_0000
            && (v & 0x0010_0000) == 0x0 =>
        {
            InstructionType::PsrTransfer
        }
        v if (v & 0x0FC0_00F0) == 0x0000_0090 => InstructionType::Multiple,
        v if (v & 0x0F80_00F0) == 0x0080_0090 => InstructionType::Multiple,
        v if (v & 0x0E00_0010) == 0x0600_0010 => InstructionType::Undefined,
        v if (v & 0x0E40_0F90) == 0x0000_0090 => InstructionType::ExtraMemory,
        v if (v & 0x0E40_0090) == 0x0040_0090 => InstructionType::ExtraMemory,
        // Coprocessor register transfer/data operations (e.g., MCR/MRC/CDP): undefined on ARM7TDMI
        v if (v & 0x0F00_0000) == 0x0E00_0000 => InstructionType::Undefined,
        // Coprocessor memory operations (LDC/STC): undefined on ARM7TDMI
        v if (v & 0x0E00_0000) == 0x0C00_0000 => InstructionType::Undefined,
        v if (v & 0x0C00_0000) == 0x0400_0000 => InstructionType::Memory,
        v if (v & 0x0C00_0000) == 0x0000_0000 => InstructionType::DataProcessing,
        // Unclassifiable encoding: treat as undefined rather than aborting.
        _ => InstructionType::Undefined,
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
    }

    // debug!("opcode = {:?}", opcode);
    // let dec = BaseDecoder { raw, cond, opcode };
    // match category {
    //     InstructionType::Multiple => Box::new(MultipleDecoder(dec)),
    //     InstructionType::ExtraMemory => Box::new(ExtraMemoryDecoder(dec)),
    //     _ => Box::new(dec),
    // }
}

#[cfg(test)]
mod harden_tests {
    use super::*;

    #[test]
    fn coprocessor_and_undefined_encodings_decode_to_undefined() {
        // These previously aborted the emulator via `panic!`/`unimplemented!`.
        // On ARM7TDMI they are undefined, so they must decode to `Undefined`
        // (which the executor turns into the undefined-instruction exception).
        let cases = [
            0xEE00_0000u32, // CDP   (coprocessor data operation)
            0xEE10_0010u32, // MRC   (coprocessor register read)
            0xEC00_0000u32, // LDC   (coprocessor load)
            0x0600_0010u32, // undefined-instruction encoding slot
        ];
        for raw in cases {
            assert_eq!(decode(raw), Instruction::Undefined, "raw=0x{raw:08X}");
        }
    }

    #[test]
    fn predecrement_single_register_store_is_str_not_psr_transfer() {
        // `STR Rd,[Rn,#-imm]!` (P=1, U=0, W=1, L=0) shares the bits-24:23/20
        // pattern of MRS/MSR but lives in the load/store class (bits 27:26 == 01).
        // It must decode as STR, not get mis-classified as a PSR transfer
        // (which would write CPSR into Rd and skip the store + base writeback).
        let cases = [
            0xe52de004u32, // STR  lr, [sp, #-4]!   (`PUSH {lr}`)
            0xe50b0008u32, // STR  r0, [r11, #-8]   (P=1, U=0, no writeback)
            0xe54d1001u32, // STRB r1, [sp, #-1]
        ];
        for raw in cases {
            assert!(
                matches!(decode(raw), Instruction::STR(_) | Instruction::STRB(_)),
                "raw=0x{raw:08X} decoded as {:?}",
                decode(raw)
            );
        }
        // Genuine MRS/MSR must still decode as PSR transfers.
        assert!(matches!(decode(0xe10f0000), Instruction::MRS(_))); // MRS r0, cpsr
        assert!(matches!(decode(0xe129f000), Instruction::MSR(_))); // MSR cpsr, r0
        assert!(matches!(decode(0xe329f0ff), Instruction::MSR(_))); // MSR cpsr_f, #imm
    }
}
