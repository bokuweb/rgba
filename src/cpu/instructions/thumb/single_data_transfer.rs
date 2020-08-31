use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::thumb::*;
use crate::cpu::types::*;

pub fn exec_ldr3<T>(
    bus: &mut T,
    dec: SingleDataTransfer,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd10_8() as usize;
    let offset = dec.get_off8() as u32;
    let pc = gpr[PC] - (PC_OFFSET * 2);
    let addr = (pc & 0xFFFF_FFFC) + offset;
    let data = bus.read_word(addr);
    // TODO: calc cycle
    gpr[rd] = data;
    Ok(PipelineStatus::Continue)
}
