use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::*;
use crate::types::*;

pub fn exec_arm_bl(dec: Branch, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()> {
    gpr[LR] = gpr[PC] - 4;
    exec_arm_b(dec, gpr)
}

pub fn exec_arm_b(dec: Branch, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()> {
    let imm = dec.get_offset();
    let imm = (if imm & 0x0080_0000 != 0 {
        imm | 0xFF00_0000
    } else {
        imm
    }) as i32;
    gpr[PC] = (gpr[PC] as i32 + imm * 4) as Word;

    // dbg!("b");
    // b does not consume extra cycle.
    Ok((0, PipelineStatus::Flush))
}
