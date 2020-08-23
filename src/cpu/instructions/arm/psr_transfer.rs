use super::super::PipelineStatus;

use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::arm::shift::ror;
use crate::cpu::registers::psr::{Mode, PSR};
use crate::cpu::types::*;

pub fn exec_mrs(
    dec: PsrTransfer,
    gpr: &mut [Word; 16],
    cpsr: PSR,
    spsr: PSR,
) -> Result<PipelineStatus, ()> {
    let rd = dec.get_Rd() as usize;
    gpr[rd] = if dec.get_Pd() { spsr.get() } else { cpsr.get() };
    // TODO: Add 1S cycle.
    Ok(PipelineStatus::Continue)
}

pub fn exec_msr(
    dec: PsrTransfer,
    gpr: &mut [Word; 16],
    cpsr: PSR,
    spsr: PSR,
) -> Result<PipelineStatus, ()> {
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

    let value = if dec.get_I() {
        ror(dec.get_imm(), dec.get_rotate())
    } else {
        gpr[dec.get_Rm() as usize]
    };

    let current_mode = cpsr.get_mode();
    match current_mode {
        Mode::User => {}
        _ => {}
    }

    /*
        match self.cpsr.mode() {
            CpuMode::User => {
                if is_spsr {
                    panic!("User mode can't access SPSR")
                }
                self.cpsr.set_flag_bits(value);
            }
            _ => {
                if is_spsr {
                    self.spsr.set(value);
                } else {
                    let old_mode = self.cpsr.mode();
                    let new_psr = RegPSR::new((self.cpsr.get() & !mask) | (value & mask));
                    let new_mode = new_psr.mode();
                    if old_mode != new_mode {
                        self.change_mode(old_mode, new_mode);
                    }
                    self.cpsr = new_psr;
                }
            }
        }
    */

    // TODO: Add 1S cycle.
    Ok(PipelineStatus::Continue)
}
