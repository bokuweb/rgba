use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::*;
use crate::cpu::registers::psr::{CpuState, PSR};
use crate::types::*;

pub fn exec_arm_bx(dec: BranchAndExchange, cpsr: &mut PSR, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()> {
    let addr = gpr[dec.get_Rm() as usize];
    if addr & 0x01 == 0x01 {
        // Switch cpu mode to execute thumb instructions.
        cpsr.set_cpu_state(CpuState::Thumb);
        dbg!("Switch cpu state to thumb");
        gpr[PC] = addr & !0x1;
        // dbg!(gpr[PC]);
    } else {
        // clear Rm[1:0]
        cpsr.set_cpu_state(CpuState::ARM);
        gpr[PC] = addr & !0x3;
    }
    // bx does not consume extra cycle.
    Ok((0, PipelineStatus::Flush))
}
