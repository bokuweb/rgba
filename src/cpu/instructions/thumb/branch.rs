use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::thumb::*;
use crate::cpu::registers::psr::PSR;
use crate::cpu::types::*;

pub fn exec_thumb_b(
    dec: Branch,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
) -> Result<PipelineStatus, ()> {
    // TODO: Add cycle
    let offset = dec.get_offset8() as i8;

    let cond: Cond = dec.get_cond().into();
    if cpsr.condition_ok(cond) {
        gpr[PC] = (gpr[PC] as i64 + (offset as i64).wrapping_shl(1)) as u32;
        return Ok(PipelineStatus::Flush);
    }
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_bl(dec: Branch, gpr: &mut [Word; 16]) -> Result<PipelineStatus, ()> {
    // TODO: Add cycle
    let offset = dec.get_offset11();

    if dec.get_H11() {
        let pc = gpr[PC];
        gpr[PC] = (gpr[LR] as i64 + ((offset as i64).wrapping_shl(1))) as u32;
        gpr[LR] = pc - 1;
        return Ok(PipelineStatus::Flush);
    } else {
        let offset = if offset & 0x0400 != 0 {
            ((offset as u32 | 0xFFFF_FC00 as u32) as i32).wrapping_shl(12)
        } else {
            (offset as i32).wrapping_shl(12)
        };
        gpr[LR] = (gpr[PC] as i64 + offset as i64) as u32;
        return Ok(PipelineStatus::Continue);
    };
}
