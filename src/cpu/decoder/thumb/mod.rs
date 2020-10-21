use crate::types::HalfWord;

mod block_data_transfer;
mod branch;
mod data_processing;
mod single_data_transfer;

pub use block_data_transfer::*;
pub use branch::*;
pub use data_processing::*;
pub use single_data_transfer::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Instruction {
    LDR1(SingleDataTransfer),
    LDRB(SingleDataTransfer),
    LDRH(SingleDataTransfer),
    LDR2(SingleDataTransfer),
    LDR3(SingleDataTransfer),
    LDR4(SingleDataTransfer),
    STR1(SingleDataTransfer),
    STRH(SingleDataTransfer),
    STRB(SingleDataTransfer),
    ADD1(DataProcessing),
    ADD2(DataProcessing),
    ADD3(DataProcessing),
    ADD4(DataProcessing),
    ADD7(DataProcessing),
    CMP1(DataProcessing),
    CMP3(DataProcessing),
    SUB1(DataProcessing),
    SUB3(DataProcessing),
    MOV3(DataProcessing),
    AND(DataProcessing),
    EOR(DataProcessing),
    LSL2(DataProcessing),
    LSR2(DataProcessing),
    ASR2(DataProcessing),
    SBC(DataProcessing),
    ROR(DataProcessing),
    TST(DataProcessing),
    NEG(DataProcessing),
    CMP2(DataProcessing),
    CMN(DataProcessing),
    ORR(DataProcessing),
    MUL(DataProcessing),
    BIC(DataProcessing),
    MVN(DataProcessing),
    LSL1(DataProcessing),
    LSR1(DataProcessing),
    ASR1(DataProcessing),
    MOV1(DataProcessing),
    SUB2(DataProcessing),
    B(Branch),
    B2(Branch),
    BL(Branch),
    BX(Branch),
    LDMIA(BlockDataTransfer),
    STMIA(BlockDataTransfer),
    POP(BlockDataTransfer),
    PUSH(BlockDataTransfer),
}

pub fn decode(raw: HalfWord) -> Instruction {
    match raw {
        // THUMB.13
        v if (v & 0xFF00) == 0xB000 => {
            let dec = DataProcessing(v);
            Instruction::ADD7(dec)
        }
        // THUMB.8
        v if ((v & 0xE000) == 0x6000) => {
            let dec = SingleDataTransfer(v);
            if dec.get_L() {
                if dec.get_B() {
                    Instruction::LDRB(dec)
                } else {
                    Instruction::LDR1(dec)
                }
            } else {
                if dec.get_B() {
                    Instruction::STRB(dec)
                } else {
                    Instruction::STR1(dec)
                }
            }
        }
        // THUMB.10
        v if ((v & 0xF000) == 0x8000) => {
            let dec = SingleDataTransfer(v);
            if dec.get_L() {
                Instruction::LDRH(dec)
            } else {
                Instruction::STRH(dec)
            }
        }
        v if ((v & 0xF800) == 0x4800) => Instruction::LDR3(SingleDataTransfer(v)),
        // THUMB 2: add/subtract
        v if ((v & 0xFE00) == 0x1A00) => Instruction::SUB3(DataProcessing(v)),
        v if ((v & 0xFE00) == 0x1800) => Instruction::ADD3(DataProcessing(v)),
        v if ((v & 0xFE00) == 0x1C00) => Instruction::ADD1(DataProcessing(v)),
        v if ((v & 0xFE00) == 0x1E00) => Instruction::SUB1(DataProcessing(v)),
        // THUMB 4: ALU operations
        v if ((v & 0xFC00) == 0x4000) => {
            let dec = DataProcessing(v);
            match dec.get_op9_6() {
                0b0000 => Instruction::AND(dec),
                0b0001 => Instruction::EOR(dec),
                0b0010 => Instruction::LSL2(dec),
                0b0011 => Instruction::LSR2(dec),
                0b0100 => Instruction::ASR2(dec),
                0b0110 => Instruction::SBC(dec),
                0b0111 => Instruction::ROR(dec),
                0b1000 => Instruction::TST(dec),
                0b1001 => Instruction::NEG(dec),
                0b1010 => Instruction::CMP2(dec),
                0b1011 => Instruction::CMN(dec),
                0b1100 => Instruction::ORR(dec),
                0b1101 => Instruction::MUL(dec),
                0b1110 => Instruction::BIC(dec),
                0b1111 => Instruction::MVN(dec),
                _ => unreachable!("unknown thumb data processing instruction {}", dec.get_op9_6()),
            }
        }
        v if ((v & 0xE000) == 0x0000) => {
            let dec = DataProcessing(v);
            match dec.get_op12_11() {
                0b00 => Instruction::LSL1(dec),
                0b01 => Instruction::LSR1(dec),
                0b10 => Instruction::ASR1(dec),
                _ => unreachable!("unknown thumb data processing instruction {}", dec.get_op12_11()),
            }
        }
        v if ((v & 0xE000) == 0x2000) => {
            let dec = DataProcessing(v);
            match dec.get_op12_11() {
                0b00 => Instruction::MOV1(dec),
                0b01 => Instruction::CMP1(dec),
                0b10 => Instruction::ADD2(dec),
                0b11 => Instruction::SUB2(dec),
                _ => unreachable!("unknown thumb data processing instruction {}", dec.get_op12_11()),
            }
        }
        v if ((v & 0xFF00) == 0x4700) => Instruction::BX(Branch(v)),
        v if ((v & 0xFC00) == 0x4400) => {
            let dec = DataProcessing(v);
            match dec.get_op9_8() {
                0b00 => Instruction::ADD4(dec),
                0b01 => Instruction::CMP3(dec),
                0b10 => Instruction::MOV3(dec),
                // 0b11 => Instruction::BX(dec),
                _ => unreachable!(),
            }
        }
        v if ((v & 0x7F00) == 0x5F00) => todo!("SWI"),
        v if ((v & 0xF000) == 0xF000) => Instruction::BL(Branch(v)),
        v if ((v & 0x7000) == 0x5000) => Instruction::B(Branch(v)),
        v if ((v & 0xF800) == 0xE000) => Instruction::B2(Branch(v)),
        v if ((v & 0xF000) == 0xC000) => {
            let dec = BlockDataTransfer(v);
            if dec.get_L() {
                Instruction::LDMIA(dec)
            } else {
                Instruction::STMIA(dec)
            }
        }
        v if ((v & 0xF000) == 0xB000) => {
            let dec = BlockDataTransfer(v);
            if dec.get_L() {
                Instruction::POP(dec)
            } else {
                Instruction::PUSH(dec)
            }
        }
        _ => panic!(format!("Unsupported instruction {:x}", raw)),
    }
}
