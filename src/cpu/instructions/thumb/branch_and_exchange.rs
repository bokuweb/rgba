use crate::cpu::constants::*;
use crate::cpu::decoder::thumb::*;
use crate::cpu::instructions::*;
use crate::cpu::registers::psr::{CpuState, PSR};
use crate::cpu::types::*;

pub fn exec_thumb_bx(
    dec: Branch,
    cpsr: &mut PSR,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()> {
    // TODO: Add cycle
    dbg!(format!("{:x}", dec.0), dec.get_Rm());
    let addr = gpr[dec.get_Rm() as usize];
    if addr & 0x01 == 0x01 {
        // Switch cpu mode to execute thumb instructions.
        cpsr.set_cpu_state(CpuState::Thumb);
        info!("switch to thumb");
    } else {
        cpsr.set_cpu_state(CpuState::ARM);
        info!("switch to arm");
    }
    let m = if dec.get_Rm() == PC as u16 {
        addr & 0x0000_0002
    } else {
        0
    };
    gpr[PC] = addr & (0xFFFF_FFFE - m);
    dbg!(&gpr);
    Ok(PipelineStatus::Flush)
}
