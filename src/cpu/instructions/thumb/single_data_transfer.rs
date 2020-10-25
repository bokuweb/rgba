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

    dbg!("ldr3", &gpr);

    // Add merged I + S cycle.
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
    (cycle, PipelineStatus::Continue)
}

// THUMB.10
pub fn exec_thumb_ldrh<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let offset = dec.get_off5() as u32;

    let addr = gpr[rn] + offset;
    let data = bus.read_halfword(addr);

    // 1N + 1I cycle
    let cycle = bus.compute_cycle(addr, AccessType::NonSeq(AccessWidth::Word)) + 1;

    gpr[rd] = data as u32;

    // Add merged I + S cycle.
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
    dbg!("ldrh", &gpr);
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

// THUMB.11: store_sp_relative
pub fn exec_thumb_str3<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd10_8() as usize;
    let offset = dec.get_off8() as u32;
    let data = gpr[rd as usize];
    let addr = gpr[SP] + offset;
    bus.write_word(addr, data);
    let access_type = AccessType::NonSeq(AccessWidth::Word);
    let store_cycle = bus.compute_cycle(addr, access_type);
    let access_type = AccessType::NonSeq(AccessWidth::HalfWord);
    let fetch_cycle = bus.compute_cycle(gpr[PC], access_type);
    dbg!("after str3", &gpr);
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

pub fn exec_thumb_strb<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let offset = dec.get_off5() as u32;

    let addr = gpr[rn] + offset;
    bus.write_byte(addr, gpr[rd] as u8);

    let access_type = AccessType::NonSeq(AccessWidth::Byte);
    let store_cycle = bus.compute_cycle(addr, access_type);
    let fetch_cycle = bus.compute_cycle(gpr[PC], access_type);
    dbg!("strB");
    // Store consume 2N cycle
    (store_cycle + fetch_cycle, PipelineStatus::Continue)
}
