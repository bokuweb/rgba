use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::decoder::thumb::*;
use crate::cpu::instructions::shift::*;
use crate::cpu::registers::psr::PSR;
use crate::cpu::types::*;

pub fn exec_thumb_lsl<T>(
    bus: &mut T,
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let sh = dec.get_sh() as u32;

    if sh == 0 {
        gpr[rd] = gpr[rn];
    } else {
        cpsr.set_C(is_carry_over(Shift::LSL, gpr[rn], sh));
        gpr[rd] = lsl(gpr[rn], sh);
    }
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    Ok(PipelineStatus::Continue)
}
