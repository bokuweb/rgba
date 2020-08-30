use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::*;
use crate::cpu::registers::psr::{CpuState, PSR};
use crate::cpu::types::*;

pub fn exec_bx(
    dec: BranchAndExchange,
    cpsr: &mut PSR,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()> {
    // TODO: Add cycle
    let addr = gpr[dec.get_Rm() as usize];
    if addr & 0x01 == 0x01 {
        // Switch cpu mode to execute thumb instructions.
        cpsr.set_cpu_state(CpuState::Thumb);
        debug!("Switch cpu state to thumb");
        gpr[PC] = addr & !0x1;
    } else {
        // clear Rm[1:0]
        cpsr.set_cpu_state(CpuState::ARM);
        gpr[PC] = addr & !0x3;
    }
    Ok(PipelineStatus::Flush)
}
