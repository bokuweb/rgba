use crate::cpu::bus::accessor::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::{ExecuteResult, PipelineStatus};
use crate::types::*;

pub fn exec_arm_swp<T>(bus: &mut T, dec: SingleDataSwap, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;
    let cycle = bus.compute_cycle(gpr[rn], AccessType::NonSeq(AccessWidth::Word));
    let cycle = cycle + bus.compute_cycle(gpr[rn], AccessType::NonSeq(AccessWidth::Word));
    let data = bus.read_word(gpr[rn]);
    bus.write_word(gpr[rn], gpr[rm] as Word);
    gpr[rd] = data;
    Ok((cycle + 1, PipelineStatus::Continue))
}

pub fn exec_arm_swpb<T>(bus: &mut T, dec: SingleDataSwap, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;
    let cycle = bus.compute_cycle(gpr[rn], AccessType::NonSeq(AccessWidth::Byte));
    let cycle = cycle + bus.compute_cycle(gpr[rn], AccessType::NonSeq(AccessWidth::Byte));
    let data = bus.read_byte(gpr[rn]);
    bus.write_byte(gpr[rn], gpr[rm] as Byte);
    gpr[rd] = data as Word;
    Ok((cycle + 1, PipelineStatus::Continue))
}
