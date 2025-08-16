use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::{shift::ror, ExecuteResult};
use crate::cpu::registers::{
    psr::{Mode, PSR},
    BankGpr, BankSpsr,
};
use crate::types::*;

pub fn exec_arm_mrs<T>(bus: &T, dec: PsrTransfer, gpr: &mut [Word; 16], cpsr: &PSR, spsr: &PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    gpr[rd] = if dec.get_Pd() { spsr.get() } else { cpsr.get() };
    let cycle = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}

pub fn exec_arm_msr<T>(
    bus: &T,
    dec: PsrTransfer,
    gpr: &mut [Word; 16],

    cpsr: &mut PSR,
    spsr: &mut PSR,
    bank_gpr: &mut BankGpr,
    bank_spsr: &mut BankSpsr,
) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let value = if dec.get_I() {
        // Rotate right by 2*rotate (ARM data processing immediate semantics)
        let rotate = (dec.get_rotate() * 2) & 0x1F;
        ror(dec.get_imm(), rotate, cpsr.get_C(), true)
    } else {

        gpr[dec.get_Rm() as usize]
    };

    let mut mask = 0;
    if dec.get_F() {
        mask |= 0xff << 24;
    }
    if dec.get_C() {
        mask |= 0xff;
    }

    if dec.get_Pd() {
        spsr.set(spsr.get() & !mask | value & mask)
    } else {
        // Track whether we changed execution state or mode -> requires pipeline flush
        let mut should_flush = false;

        // Update flags (N Z C V)
        if mask & 0xF000_0000 != 0 {
            cpsr.set_N(value & 0x8000_0000 != 0);
            cpsr.set_Z(value & 0x4000_0000 != 0);
            cpsr.set_C(value & 0x2000_0000 != 0);
            cpsr.set_V(value & 0x1000_0000 != 0);
        }

        let current_mode = cpsr.get_mode();
        if current_mode != Mode::User && mask & 0x0000_00CF != 0 {
            // Control field write: I, F, T, and Mode
            let old_t = cpsr.get_T();
            let new_t = (value & 0x0000_0020) != 0;
            if new_t != old_t {
                cpsr.set_T(new_t);
                should_flush = true;
            }

            cpsr.set_I(value & 0x0000_0080 != 0);
            cpsr.set_F(value & 0x0000_0040 != 0);

            let old_mode = current_mode;
            let new_mode = Mode::from((value & 0x0000_001F) | 0x0000_0010);
            if new_mode != old_mode {
                cpsr.switch_mode(new_mode, gpr, spsr, bank_gpr, bank_spsr);
                should_flush = true;
            }
        }

        let cycle = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
        if should_flush {
            return Ok((cycle, PipelineStatus::Flush));
        } else {
            return Ok((cycle, PipelineStatus::Continue));
        }
    }

    let cycle = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}
