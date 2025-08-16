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

        // dec.get_rotate().rotate_right();
        ror(
            dec.get_imm(),
            dec.get_rotate().checked_shl(1).unwrap_or_default(),
            cpsr.get_C(),
            true,
        )
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
        if mask & 0xF000_0000 != 0 {
            cpsr.set_N(value & 0x8000_0000 != 0);
            cpsr.set_Z(value & 0x4000_0000 != 0);
            cpsr.set_C(value & 0x2000_0000 != 0);
            cpsr.set_V(value & 0x1000_0000 != 0);
        }

        let current_mode = cpsr.get_mode();
        if current_mode != Mode::User && mask & 0x0000_00CF != 0 {
            cpsr.set_I(value & 0x0000_0080 != 0);
            cpsr.set_F(value & 0x0000_0040 != 0);

            let new_mode = Mode::from((value & 0x0000_000F) | 0x0000_0010);

            cpsr.switch_mode(new_mode, gpr, spsr, bank_gpr, bank_spsr);
        }
    }

    let cycle = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}
