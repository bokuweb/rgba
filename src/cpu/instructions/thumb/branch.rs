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
    // THUMB.16: conditional branch (0xD000..=0xD7FF except 0xD000 for BEQ etc.)
    // Bits[11:8] are condition code, Bits[7:0] is signed 8-bit offset << 1
    let cond: Cond = dec.get_cond().into();
    let offset8 = dec.get_offset8();
    let signed = if (offset8 & 0x80) != 0 { (offset8 | 0xFF00) as i16 } else { offset8 as i16 };
    if cpsr.condition_ok(cond) {
        let delta = (signed as i32).wrapping_shl(1) as i64;
        gpr[PC] = (gpr[PC] as i64 + delta) as u32;
        return (0, PipelineStatus::Flush);
    }
    // not taken: 1S
    (1, PipelineStatus::Continue)
}

/// Format 18
pub fn exec_thumb_b2(dec: Branch, gpr: &mut [Word; 16], _cpsr: &mut PSR) -> ExecuteResult {
    // THUMB.18: unconditional branch, 11-bit signed offset << 1
    let offset11 = dec.get_offset11();
    let signed = if (offset11 & 0x0400) != 0 {
        ((offset11 as u32) | 0xFFFF_F800) as i32
    } else {
        offset11 as i32
    };
    let delta = signed.wrapping_shl(1) as i64;
    gpr[PC] = (gpr[PC] as i64 + delta) as u32;
    // Consume: 2S+1N (simplified model: return 0 here and Flush)
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
            ((offset as u32 | 0xFFFF_FC00_u32) as i32).wrapping_shl(12)
        } else {
            (offset as i32).wrapping_shl(12)
        };
        gpr[LR] = (gpr[PC] as i64 + offset as i64) as u32;
        // dbg!(&gpr, "bl1!~!!!!!!!!!!!!");
        (s_cycle, PipelineStatus::Continue)
    }
}
