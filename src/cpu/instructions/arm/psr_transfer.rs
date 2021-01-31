use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::{shift::ror, ExecuteResult};
use crate::cpu::registers::psr::{Mode, PSR};
use crate::types::*;

pub fn exec_arm_mrs<T>(bus: &T, dec: PsrTransfer, gpr: &mut [Word; 16], cpsr: &PSR, spsr: &PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    gpr[rd] = if dec.get_Pd() {
        spsr.get()
    } else {
        dbg!(
            cpsr.get_M(),
            cpsr.get_T(),
            cpsr.get_F(),
            cpsr.get_I(),
            cpsr.get_Z(),
            cpsr.get_C(),
            cpsr.get_V()
        );
        cpsr.get()
    };
    let cycle = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}

pub fn exec_arm_msr<T>(bus: &T, dec: PsrTransfer, gpr: &mut [Word; 16], cpsr: &mut PSR, spsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let value = if dec.get_I() {
        ror(dec.get_imm(), dec.get_rotate(), cpsr.get_C(), false)
    } else {
        // dbg!(&gpr, dec.get_Rm());
        gpr[dec.get_Rm() as usize]
    };

    let current_mode = cpsr.get_mode();
    match current_mode {
        Mode::User => {
            cpsr.set_flags(value);
        }
        _ => {
            if dec.get_Pd() {
                spsr.set(value);
            } else {
                let mut mask = 0;
                if dec.get_F() {
                    mask |= 0xff << 24;
                }
                if dec.get_S() {
                    mask |= 0xff << 16;
                }
                if dec.get_X() {
                    mask |= 0xff << 8;
                }
                if dec.get_C() {
                    mask |= 0xff;
                }
                info!("{:x}", mask);
                let current_value = cpsr.get();
                // dbg!(current_value);
                let current_mode = cpsr.get_mode();
                let new_value = (current_value & !mask) | (value & mask);
                // dbg!(new_value);
                cpsr.set(new_value);
                let new_mode = cpsr.get_mode();
                if current_mode != new_mode {
                    // dbg!(current_mode, new_mode);
                    // todo!("should change mode.");
                }
            }
        }
    }

    let cycle = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}
