use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::thumb::*;
use crate::types::*;

pub fn exec_thumb_ldr3<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd10_8() as usize;
    let offset = dec.get_off8() as u32;
    let pc = gpr[PC];
    let addr = (pc & 0xFFFF_FFFC) + offset.wrapping_shl(2);
    let data = bus.read_word(addr);
    // TODO: calc cycle
    gpr[rd] = data;
    dbg!(&gpr);
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_str1<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let offset = dec.get_off5() as u32;

    let addr = gpr[rn] + offset;
    bus.write_word(addr, gpr[rd]);
    // TODO: Add wait
    // dbg!(&gpr);
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_strh<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let offset = dec.get_off5() as u32;

    let addr = gpr[rn] + offset;
    bus.write_halfword(addr, gpr[rd] as u16);
    // TODO: Add wait
    Ok(PipelineStatus::Continue)
}
