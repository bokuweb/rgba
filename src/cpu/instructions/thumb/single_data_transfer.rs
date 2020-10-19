use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::thumb::*;
use crate::cpu::instructions::ExecuteResult;
use crate::types::*;

pub fn exec_thumb_ldr3<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd10_8() as usize;
    let offset = dec.get_off8() as u32;
    let pc = gpr[PC];
    let addr = (pc & 0xFFFF_FFFC) + offset.wrapping_shl(2);
    let data = bus.read_word(addr);

    // 1N + 1I cycle
    let cycle = bus.compute_cycle(addr, AccessType::NonSeq(AccessWidth::Word)) + 1;

    gpr[rd] = data;

    dbg!("ldr");

    // Add merged I + S cycle.
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_str1<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let offset = dec.get_off5() as u32;

    let addr = gpr[rn] + offset;
    bus.write_word(addr, gpr[rd]);

    let access_type = AccessType::NonSeq(AccessWidth::Word);
    let store_cycle = bus.compute_cycle(addr, access_type);
    let access_type = AccessType::NonSeq(AccessWidth::HalfWord);
    let fetch_cycle = bus.compute_cycle(gpr[PC], access_type);
    // Store consume 2N cycle
    (store_cycle + fetch_cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_strh<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let offset = dec.get_off5() as u32;

    let addr = gpr[rn] + offset;
    bus.write_halfword(addr, gpr[rd] as u16);

    let access_type = AccessType::NonSeq(AccessWidth::HalfWord);
    let store_cycle = bus.compute_cycle(addr, access_type);
    let fetch_cycle = bus.compute_cycle(gpr[PC], access_type);
    dbg!("strh");
    // Store consume 2N cycle
    (store_cycle + fetch_cycle, PipelineStatus::Continue)
}
