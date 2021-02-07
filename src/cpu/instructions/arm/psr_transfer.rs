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
    dbg!("Before msr", &gpr);

    if gpr[PC] == 134221712 {
        dbg!("hello");
    }
    let value = if dec.get_I() {
        dbg!(&dec, dec.get_imm(), dec.get_rotate(), dec.get_C());
        ror(dec.get_imm(), dec.get_rotate().wrapping_shl(1), cpsr.get_C(), false)
    } else {
        // dbg!(&gpr, dec.get_Rm());
        gpr[dec.get_Rm() as usize]
    };
    dbg!(value);

    let mut mask = 0;
    if dec.get_F() {
        mask |= 0xff << 24;
    }
    // if dec.get_S() {
    //     mask |= 0xff << 16;
    // }
    // if dec.get_X() {
    //     mask |= 0xff << 8;
    // }
    if dec.get_C() {
        mask |= 0xff;
    }

    //let current_mode = cpsr.get_mode();
    //match current_mode {
    //    Mode::User => {
    //        dbg!("user");
    //        cpsr.set_flags(value);
    //    }
    //    _ => {
    if dec.get_Pd() {
        dbg!("get_pd");
        spsr.set(spsr.get() & !mask | value & mask)
    // spsr.set(value);
    } else {
        if mask & 0x8000_0000 != 0 {
            dbg!(value & 0x2000_0000 != 0);
            dbg!(value & 0x8000_0000 != 0);
            dbg!(format!("0x{:x}", value));
            cpsr.set_N(value & 0x8000_0000 != 0);
            cpsr.set_Z(value & 0x4000_0000 != 0);
            cpsr.set_C(value & 0x2000_0000 != 0);
            cpsr.set_V(value & 0x1000_0000 != 0);
        }

        let current_mode = cpsr.get_mode();
        if current_mode != Mode::User && mask & 0x0000_00CF != 0 {
            cpsr.set_I(value & 0x0000_0080 != 0);
            cpsr.set_F(value & 0x0000_0040 != 0);

            // TODO: move to PSR?
            //       switch mode
            // let current_value = cpsr.get();
            let current_mode = cpsr.get_mode();
            let new_mode = Mode::from((value & 0x0000_000F) | 0x0000_0010);
            // let new_mode = cpsr.get_mode();
            if current_mode != new_mode {
                // TODO: support FIQ
                if new_mode == Mode::FIQ {
                    dbg!(&gpr, new_mode);
                    unimplemented!("Support FIQ, Copy banked registers to current gpr")
                }

                if current_mode == Mode::FIQ {
                    dbg!(&gpr, current_mode);
                    unimplemented!("Support FIQ, Copy current gpr to banked registers")
                }

                if current_mode != Mode::System && current_mode != Mode::User {
                    bank_gpr.write(current_mode, SP, gpr[SP]);
                    bank_gpr.write(current_mode, LR, gpr[LR]);
                    bank_spsr.write(current_mode, *spsr);
                }

                if new_mode != Mode::System && new_mode != Mode::User {
                    gpr[SP] = bank_gpr.read(new_mode, SP);
                    gpr[LR] = bank_gpr.read(new_mode, LR);
                    *spsr = bank_spsr.read(new_mode);
                }
            }
            cpsr.set_mode(new_mode);
        }
    }
    //        }
    //}

    let cycle = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}
