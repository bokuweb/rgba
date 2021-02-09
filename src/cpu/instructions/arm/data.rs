use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::shift::{is_carry_over, ror, shift};
use crate::cpu::instructions::ExecuteResult;
use crate::cpu::registers::psr::PSR;
use crate::types::*;

pub fn exec_data_processing<T, F>(
    bus: &T,
    gpr: &mut [Word; 16],
    dec: DataProcessing,
    cpsr: &mut PSR,
    data_process: &mut F,
) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
    F: FnMut(&mut [Word; 16], Word, bool, &mut PSR),
{
    let mut cycle = 0;
    let rm = dec.get_Rm() as usize;

    let (value, carry) = if dec.get_I() {
        if dec.get_rotate() == 0 {
            (dec.get_imm(), cpsr.get_C())
        } else {
            let shift_value = dec.get_rotate() * 2;
            (
                ror(dec.get_imm(), shift_value, cpsr.get_C(), true),
                is_carry_over(dec.get_sh().into(), dec.get_imm(), shift_value, cpsr.get_C(), true),
            )
        }
    } else if dec.get_bit4() {
        let rm = gpr[rm] + if rm == PC { 4 } else { 0 };
        // if shifted by register, consume 1I cycle.
        cycle += 1;
        // only lower 8bit used.
        let rs = dec.get_Rs() as usize;
        let rs = gpr[rs] + if rs == PC { 4 } else { 0 };
        let shift_value = rs & 0xFF;
        // dbg!("shift", rm, shift_value);
        (
            shift(dec.get_sh().into(), rm, shift_value, cpsr.get_C(), true),
            is_carry_over(dec.get_sh().into(), rm, shift_value, cpsr.get_C(), true),
        )
    } else {
        let shift_value = dec.get_shamt5();
        let rm = gpr[rm];
        (
            shift(dec.get_sh().into(), rm, shift_value, cpsr.get_C(), dec.get_bit4()),
            is_carry_over(dec.get_sh().into(), rm, shift_value, cpsr.get_C(), dec.get_bit4()),
        )
    };

    data_process(gpr, value, carry, cpsr);

    if dec.get_Rd() == PC as u32 {
        // waiting for pipeline filled is executed by caller.
        Ok((cycle, PipelineStatus::Flush))
    } else {
        let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
        Ok((s + cycle, PipelineStatus::Continue))
    }
}

pub fn exec_arm_mov<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, c, cpsr| {
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(value as u32);
                cpsr.set_Z_from(value as u32);
                cpsr.set_C(c);
            }
        }
        gpr[rd] = value;
    })
}

pub fn exec_arm_and<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, c, cpsr| {
        // dbg!("and0");
        let d = gpr[rn] & value;
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d as u32);
                cpsr.set_Z_from(d as u32);
                cpsr.set_C(c);
            }
        }
        gpr[rd] = d;
        // dbg!("and1", value);
    })
}

pub fn exec_arm_eor<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, carry, cpsr| {
        let d = gpr[rn] ^ value;
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                // dbg!("EOR", carry, rd);
                cpsr.set_N_from(d);
                cpsr.set_Z_from(d);
                cpsr.set_C(carry);
            }
        }
        gpr[rd] = d;
    })
}

pub fn exec_arm_sub<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        let d = gpr[rn].wrapping_sub(value);
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d as u32);
                cpsr.set_Z_from(d as u32);
                cpsr.set_C(gpr[rn] >= value);
                let (_, v) = (d as i32).overflowing_sub(value as i32);
                cpsr.set_V(v);
            }
        }
        gpr[rd] = d;
    })
}

pub fn exec_arm_rsb<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        if s {
            unimplemented!()
        }
        gpr[rd] = value.wrapping_sub(gpr[rn]);
    })
}

pub fn exec_arm_add<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let result = exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        let d = gpr[rn] as u64 + value as u64;
        // dbg!(d, gpr[rn], value);
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d as u32);
                cpsr.set_Z_from(d as u32);
                cpsr.set_C_from(d);
                cpsr.set_V_from(gpr[rd], d as u32);
            }
        }
        gpr[rd] = d as u32;
    });
    result
}

pub fn exec_arm_adc<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let c = cpsr.get_C();
    let result = exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        let d = gpr[rn] as u64 + value as u64 + if c { 1 } else { 0 };
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d as u32);
                cpsr.set_Z_from(d as u32);
                cpsr.set_C_from(d);
                cpsr.set_V_from(gpr[rd], d as u32);
            }
        }
        gpr[rd] = d as u32;
    });
    result
}

pub fn exec_arm_sbc<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let c = cpsr.get_C();
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        let c = u32::from(!c);
        let d = gpr[rn].wrapping_sub(value).wrapping_sub(c);
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d as u32);
                cpsr.set_Z_from(d as u32);
                cpsr.set_C(gpr[rn] >= value + c);
                cpsr.set_V_from(gpr[rd], d as u32);
            }
        }
        gpr[rd] = d
    })
}

pub fn exec_arm_rsc<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let c = cpsr.get_C();
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        let c = if c { 0 } else { 1 };
        let d = value.wrapping_sub(gpr[rn]).wrapping_sub(c);
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d as u32);
                cpsr.set_Z_from(d as u32);
                cpsr.set_C(gpr[rn] >= d);
                cpsr.set_V_from(gpr[rd], d as u32);
            }
        }
        gpr[rd] = d;
    })
}

pub fn exec_arm_tst<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, carry, cpsr| {
        let tst = gpr[rn] & value;
        cpsr.set_N(tst >> 31 != 0);
        cpsr.set_Z(tst == 0);
        cpsr.set_C(carry);
    })
}

pub fn exec_arm_teq<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, carry, cpsr| {
        let teq = gpr[rn] ^ value;
        cpsr.set_N(teq >> 31 != 0);
        cpsr.set_Z(teq == 0);
        cpsr.set_C(carry);
    })
}

pub fn exec_arm_cmp<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        let rn = gpr[rn];
        let cmp = rn.wrapping_sub(value);
        dbg!(cmp);
        cpsr.set_N(cmp >> 31 != 0);
        cpsr.set_Z(cmp == 0);
        // let (_, v) = (rn as i32).overflowing_sub(value as i32);
        let v = rn >> 31 != value >> 31 && rn >> 31 != cmp >> 31;
        cpsr.set_V(v);
        // NOTE: Should we consider to shifted carry?
        cpsr.set_C(rn >= value);
    })
}

pub fn exec_arm_cmn<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        let rn = gpr[rn];
        let cmn = (rn as u64).wrapping_add(value as u64);
        cpsr.set_N((cmn as i32) < 0);
        cpsr.set_Z(cmn == 0);
        let (_, v) = (rn as i32).overflowing_add(value as i32);
        cpsr.set_V(v);
        cpsr.set_C(cmn & (1 << 32) != 0);
    })
}

pub fn exec_arm_orr<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, carry, cpsr| {
        let d = gpr[rn] | value;
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d);
                cpsr.set_Z_from(d);
                cpsr.set_C(carry);
            }
        }
        // dbg!(d, rn, value, &gpr);
        gpr[rd] = d;
    })
}

pub fn exec_arm_shift<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, carry, cpsr| {
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(value);
                cpsr.set_Z_from(value);
                cpsr.set_C(carry);
            }
        }
        gpr[rd] = value;
    })
}

pub fn exec_arm_bic<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, carry, cpsr| {
        let d = gpr[rn] & !value;
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d);
                cpsr.set_Z_from(d);
                cpsr.set_C(carry);
            }
        }
        gpr[rd] = d;
    })
}

pub fn exec_arm_mvn<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        if s {
            unimplemented!()
        }
        gpr[rd] = !value
    })
}

pub fn exec_arm_rrx<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        if s {
            unimplemented!()
        }
        gpr[rd] = value;
        dbg!(&gpr);
    })
}
