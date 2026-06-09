use crate::cpu::bus::accessor::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::{ExecuteResult, PipelineStatus};
use crate::cpu::instructions::helpers::read_ldr_data;
use crate::types::*;

pub fn exec_arm_swp<T>(bus: &mut T, dec: SingleDataSwap, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;
    let addr = gpr[rn];
    let cycle = bus.compute_cycle(addr, AccessType::NonSeq(AccessWidth::Word));
    let cycle = cycle + bus.compute_cycle(addr, AccessType::NonSeq(AccessWidth::Word));
    // t452: Misaligned swap
    //  - Read: like LDR, when misaligned apply ROR(8*addr[1:0]) (uses read_ldr_data)
    //  - Write: like STR, ignore bits[1:0] and round down to the 4-byte boundary
    let read_val = read_ldr_data(bus, addr);
    let eff_addr = addr & 0xFFFF_FFFC;
    bus.write_word(eff_addr, gpr[rm] as Word);
    gpr[rd] = read_val;
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
