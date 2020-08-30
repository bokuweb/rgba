use crate::cpu::types::HalfWord;

mod single_data_transfer;

pub use single_data_transfer::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum InstructionType {
    SingleDataTransfer,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Instruction {
    LDR1(SingleDataTransfer),
    LDR2(SingleDataTransfer),
    LDR3(SingleDataTransfer),
    LDR4(SingleDataTransfer),
}

pub fn decode(raw: HalfWord) -> Instruction {
    match raw {
        v if ((v & 0xF800) == 0x4800) => Instruction::LDR3(SingleDataTransfer(v)),
        _ => panic!("Unsupported instruction"),
    }
}
