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
    if dec.get_bit0() {
        // Switch cpu mode to execute thumb instructions.
        let addr = dec.0 & !0x1;
        cpsr.set_cpu_state(CpuState::Thumb);
        gpr[PC] = addr;
    } else {
        // clear Rm[1:0]
        let addr = dec.0 & !0x3;
        cpsr.set_cpu_state(CpuState::ARM);
        gpr[PC] = addr;
    }
    Ok(PipelineStatus::Flush)
}
