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
