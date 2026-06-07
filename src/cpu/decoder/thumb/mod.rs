use crate::types::HalfWord;

mod block_data_transfer;
mod branch;
mod data_processing;
mod single_data_transfer;
mod swi;

pub use block_data_transfer::*;
pub use branch::*;
pub use data_processing::*;
pub use single_data_transfer::*;
pub use swi::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Instruction {
    LDR1(SingleDataTransfer),
    LDRB(SingleDataTransfer),
    LDRH(SingleDataTransfer),
    LDRHThumb8(SingleDataTransfer),
    LDSBThumb8(SingleDataTransfer),
    LDSHThumb8(SingleDataTransfer),
    LDRBRegOffset(SingleDataTransfer),
    LDRRegOffset(SingleDataTransfer),
    LDR3(SingleDataTransfer),
    LDR4(SingleDataTransfer),
    STR1(SingleDataTransfer),
    STR3(SingleDataTransfer),
    STRH(SingleDataTransfer),
    STRB_IMM_OFFET(SingleDataTransfer),
    STRRegOffset(SingleDataTransfer),
    STRBRegOffset(SingleDataTransfer),
    STRHRegOffset(SingleDataTransfer),
    ADD1(DataProcessing),
    ADD2(DataProcessing),
    ADD3(DataProcessing),
    ADDHiRegister(DataProcessing),
    // ADD5(DataProcessing),
    ADD6(DataProcessing), // 6 and 5
    ADD7(DataProcessing),
    CMP1(DataProcessing),
    CMPThumb5(DataProcessing),
    SUB1(DataProcessing),
    SUB3(DataProcessing),
    MOV3(DataProcessing),
    AND(DataProcessing),
    EOR(DataProcessing),
    LSLThumb4(DataProcessing),
    LSR2(DataProcessing),
    ASRThumb4(DataProcessing),
    ADCThumb4(DataProcessing),
    SBC(DataProcessing),
    RORThumb4(DataProcessing),
    TST(DataProcessing),
    NEG(DataProcessing),
    CMP2(DataProcessing),
    CMNThumb4(DataProcessing),
    ORR(DataProcessing),
    MUL(DataProcessing),
    BIC(DataProcessing),
    MVN(DataProcessing),
    LSLThumb1(DataProcessing),
    LSRThumb1(DataProcessing),
    ASRThumb1(DataProcessing),
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
    SWI(ThumbSoftwareInterrupt),
    /// Unknown/undefined Thumb encoding (treated as a no-op by the executor).
    Undefined,
}

pub fn decode(raw: HalfWord) -> Instruction {
    match raw {
        // THUMB.13
        v if (v & 0xFF00) == 0xB000 => {
            let dec = DataProcessing(v);
            Instruction::ADD7(dec)
        }
        // THUMB.7 / 8
        v if ((v & 0xF000) == 0x5000) => {
            let dec = SingleDataTransfer(v);
            if dec.get_S() {
                match dec.get_op11_10() {
                    0b00 => Instruction::STRHRegOffset(dec),
                    0b01 => Instruction::LDSBThumb8(dec),
                    0b10 => Instruction::LDRHThumb8(dec),
                    0b11 => Instruction::LDSHThumb8(dec),
                    _ => unreachable!("unknown thumb data transfer instruction {}", dec.get_op11_10()),
                }
            } else {
                match dec.get_op11_10() {
                    0b00 => Instruction::STRRegOffset(dec),
                    0b01 => Instruction::STRBRegOffset(dec),
                    0b10 => Instruction::LDRRegOffset(dec),
                    0b11 => Instruction::LDRBRegOffset(dec),
                    _ => unreachable!("unknown thumb data transfer instruction {}", dec.get_op11_10()),
                }
            }
        }
        // THUMB.9
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
                    Instruction::STRB_IMM_OFFET(dec)
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
        // THUMB.6
        v if ((v & 0xF800) == 0x4800) => Instruction::LDR3(SingleDataTransfer(v)),
        // THUMB.11 load/store SP-relative
        v if ((v & 0xF000) == 0x9000) => {
            let dec = SingleDataTransfer(v);
            if dec.get_L() {
                Instruction::LDR4(dec)
            } else {
                Instruction::STR3(dec)
            }
        }
        // THUMB.12
        v if ((v & 0xF000) == 0xA000) => Instruction::ADD6(DataProcessing(v)),
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
                0b0010 => Instruction::LSLThumb4(dec),
                0b0011 => Instruction::LSR2(dec),
                0b0100 => Instruction::ASRThumb4(dec),
                0b0101 => Instruction::ADCThumb4(dec),
                0b0110 => Instruction::SBC(dec),
                0b0111 => Instruction::RORThumb4(dec),
                0b1000 => Instruction::TST(dec),
                0b1001 => Instruction::NEG(dec),
                0b1010 => Instruction::CMP2(dec),
                0b1011 => Instruction::CMNThumb4(dec),
                0b1100 => Instruction::ORR(dec),
                0b1101 => Instruction::MUL(dec),
                0b1110 => Instruction::BIC(dec),
                0b1111 => Instruction::MVN(dec),
                _ => unreachable!("unknown thumb data processing instruction {}", dec.get_op9_6()),
            }
        }
        // THUMB.1: move shifted register
        v if ((v & 0xE000) == 0x0000) => {
            let dec = DataProcessing(v);
            match dec.get_op12_11() {
                0b00 => Instruction::LSLThumb1(dec),
                0b01 => Instruction::LSRThumb1(dec),
                0b10 => Instruction::ASRThumb1(dec),
                _ => unreachable!("unknown thumb data processing instruction {}", dec.get_op12_11()),
            }
        }
        // THUMB.3
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
            // THUMB.5: Hi register operations
            match dec.get_op9_8() {
                0b00 => Instruction::ADDHiRegister(dec),
                0b01 => Instruction::CMPThumb5(dec),
                0b10 => Instruction::MOV3(dec),
                // 0b11 => Instruction::BX(dec),
                _ => unreachable!(),
            }
        }
        // THUMB.17 SWI
        // 原因: 以前は todo!("SWI") で未実装のため、Thumb SWI 呼び出しが失敗していた。
        //       jsmolka/gba-tests の Thumb テストでも SWI は 0xDF00 形式で使用されるため、
        //       ここでデコードを実装し実行系へ渡す。
        v if ((v & 0xFF00) == 0xDF00) => Instruction::SWI(decode_thumb_swi(v)),
        // THUMB.19
        v if ((v & 0xF000) == 0xF000) => Instruction::BL(Branch(v)),
        // THUMB.16 conditional branch
        // 原因: 以前は todo!("conditional branch") で未実装だったため、条件分岐系テストが落ちていた。
        //       cond は bits[11:8]、オフセットは sign-extend した 8bit << 1 を PC に加算する仕様。
        //       仕様は gba-tests/thumb の分岐テスト（beq/bne/...）に準拠。
        v if ((v & 0xF000) == 0xD000) => Instruction::B(Branch(v)),
        // THUMB.18 unconditional branch
        v if ((v & 0xF800) == 0xE000) => Instruction::B2(Branch(v)),
        // THUMB.15
        v if ((v & 0xF000) == 0xC000) => {
            let dec = BlockDataTransfer(v);
            if dec.get_L() {
                Instruction::LDMIA(dec)
            } else {
                Instruction::STMIA(dec)
            }
        }
        //  THUMB.14
        v if ((v & 0xF000) == 0xB000) => {
            let dec = BlockDataTransfer(v);
            if dec.get_L() {
                Instruction::POP(dec)
            } else {
                Instruction::PUSH(dec)
            }
        }
        // Unknown Thumb encoding: decode as Undefined (no-op) instead of
        // aborting the emulator.
        _ => Instruction::Undefined,
    }
}
