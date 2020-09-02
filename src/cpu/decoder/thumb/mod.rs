use crate::cpu::types::HalfWord;

mod data_processing;
mod single_data_transfer;

pub use data_processing::*;
pub use single_data_transfer::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum InstructionType {
    SingleDataTransfer,
    DataProcessing,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Instruction {
    LDR1(SingleDataTransfer),
    LDR2(SingleDataTransfer),
    LDR3(SingleDataTransfer),
    LDR4(SingleDataTransfer),
    LSL(DataProcessing),
    LSR(DataProcessing),
    ASR(DataProcessing),
}

pub fn decode(raw: HalfWord) -> Instruction {
    match raw {
        v if ((v & 0xF800) == 0x4800) => Instruction::LDR3(SingleDataTransfer(v)),
        v if ((v & 0xFC00) == 0x1800) => todo!("ADD"),
        v if ((v & 0xFC00) == 0x1C00) => todo!("ADD"),
        v if ((v & 0xE000) == 0x0000) => {
            let dec = DataProcessing(v);
            match dec.get_op() {
                0b00 => Instruction::LSL(dec),
                0b01 => Instruction::LSR(dec),
                0b10 => Instruction::ASR(dec),
                _ => unreachable!("unknown thumb data processing instruction {}", dec.get_op()),
            }
        }
        _ => panic!("Unsupported instruction"),
    }
}
