use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::thumb::*;
use crate::cpu::instructions::ExecuteResult;
use crate::cpu::registers::psr::PSR;
use crate::cpu::types::*;
use crate::types::*;

/// Format 16
pub fn exec_thumb_b(dec: Branch, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let mut offset = dec.get_offset8() as i8;

    let cond: Cond = dec.get_cond().into();
    if cpsr.condition_ok(cond) {
        gpr[PC] = (gpr[PC] as i64 + (offset as i64).wrapping_shl(1)) as u32;
        return (0, PipelineStatus::Flush);
    }
    // Consume 1S if condition false
    (1, PipelineStatus::Continue)
}

/// Format 18
pub fn exec_thumb_b2(dec: Branch, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    // TODO: Add cycle
    let offset = dec.get_offset11();
    let offset = if offset & 0x0400 != 0 {
        (offset as u32 | 0xFFFF_FC00 as u32) as i32
    } else {
        offset as i32
    }
    .wrapping_shl(1);
    let pc = gpr[PC] as i64 + offset as i64;
    gpr[PC] = pc as u32;
    // Consume: 2S+1N
    (0, PipelineStatus::Flush)
}

/// Format 19
/// Consume 3S+1N (first 1S, second 2S+1N)
pub fn exec_thumb_bl1<T: BusAccessor>(bus: &T, dec: Branch, gpr: &mut [Word; 16]) -> ExecuteResult {
    let offset = dec.get_offset11();
    // dbg!("BL", &gpr);
    let access_type = AccessType::NonSeq(AccessWidth::HalfWord);
    let s_cycle = bus.compute_cycle(gpr[PC], access_type);
    if dec.get_H11() {
        let pc = gpr[PC];
        gpr[PC] = (gpr[LR] as i64 + ((offset as i64).wrapping_shl(1))) as u32;
        gpr[LR] = pc - 1;
        // dbg!(&gpr, "bl1!~!!!!!!!!!!!!");
        (s_cycle, PipelineStatus::Flush)
    } else {
        let offset = if offset & 0x0400 != 0 {
            ((offset as u32 | 0xFFFF_FC00 as u32) as i32).wrapping_shl(12)
        } else {
            (offset as i32).wrapping_shl(12)
        };
        gpr[LR] = (gpr[PC] as i64 + offset as i64) as u32;
        // dbg!(&gpr, "bl1!~!!!!!!!!!!!!");
        (s_cycle, PipelineStatus::Continue)
    }
}
