use crate::cpu::types::HalfWord;

mod branch;
mod data_processing;
mod single_data_transfer;

pub use branch::*;
pub use data_processing::*;
pub use single_data_transfer::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Instruction {
    LDR1(SingleDataTransfer),
    LDR2(SingleDataTransfer),
    LDR3(SingleDataTransfer),
    LDR4(SingleDataTransfer),
    ADD3(DataProcessing),
    LSL(DataProcessing),
    LSR(DataProcessing),
    ASR(DataProcessing),
    MOV1(DataProcessing),
    B(Branch),
    BL(Branch),
}

pub fn decode(raw: HalfWord) -> Instruction {
    match raw {
        v if ((v & 0xF800) == 0x4800) => Instruction::LDR3(SingleDataTransfer(v)),
        v if ((v & 0xFC00) == 0x1800) => Instruction::ADD3(DataProcessing(v)),
        v if ((v & 0xFC00) == 0x1C00) => todo!("ADD1?"),
        v if ((v & 0xE000) == 0x0000) => {
            let dec = DataProcessing(v);
            match dec.get_op() {
                0b00 => Instruction::LSL(dec),
                0b01 => Instruction::LSR(dec),
                0b10 => Instruction::ASR(dec),
                _ => unreachable!("unknown thumb data processing instruction {}", dec.get_op()),
            }
        }
        v if ((v & 0xE000) == 0x2000) => {
            let dec = DataProcessing(v);
            match dec.get_op() {
                0b00 => Instruction::MOV1(dec),
                0b01 => todo!("CMP1"),
                0b10 => todo!("ADD2"),
                0b11 => todo!("SUB2"),
                _ => unreachable!("unknown thumb data processing instruction {}", dec.get_op()),
            }
        }
        v if ((v & 0x7F00) == 0x5F00) => todo!("SWI"),
        v if ((v & 0xF000) == 0xF000) => Instruction::BL(Branch(v)),
        v if ((v & 0x7000) == 0x5000) => Instruction::B(Branch(v)),
        _ => panic!("Unsupported instruction"),
    }
}
