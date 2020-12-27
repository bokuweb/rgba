use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::shift::{is_carry_over, ror, shift};
use crate::cpu::instructions::ExecuteResult;
use crate::cpu::registers::psr::PSR;
use crate::types::*;

pub fn exec_data_processing<T, F>(bus: &T, gpr: &mut [Word; 16], dec: DataProcessing, data_process: &mut F) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
    F: FnMut(&mut [Word; 16], Word, bool),
{
    let mut cycle = 0;
    let (value, carry) = if dec.get_I() {
        let shift_value = dec.get_rotate() * 2;
        (
            ror(dec.get_imm(), shift_value),
            is_carry_over(dec.get_sh().into(), dec.get_imm(), shift_value),
        )
    } else {
        let rm = dec.get_Rm() as usize;
        let shift_value = if dec.get_bit4() {
            // if shifted by register, consume 1I cycle.
            cycle += 1;
            dec.get_Rs()
        } else {
            dec.get_shamt5()
        };
        (
            shift(dec.get_sh().into(), gpr[rm], shift_value),
            is_carry_over(dec.get_sh().into(), gpr[rm], shift_value),
        )
    };

    data_process(gpr, value, carry);

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
    // dbg!("arm mov");
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        if s {
            unimplemented!()
        }
        gpr[rd] = value;
        // dbg!(&gpr, value, rd);
    })
}

pub fn exec_arm_and<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, c| {
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
    })
}

pub fn exec_arm_eor<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        if s {
            unimplemented!()
        }
        gpr[rd] = gpr[rn] ^ value;
    })
}

pub fn exec_arm_sub<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        let d = gpr[rn].wrapping_sub(value);
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d as u32);
                cpsr.set_Z_from(d as u32);
                cpsr.set_C(gpr[rn] >= value);
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
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
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
    let result = exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        let d = gpr[rn] as u64 + value as u64;
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
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        if s {
            unimplemented!()
        }
        gpr[rd] = gpr[rn].wrapping_add(value).wrapping_add(if c { 1 } else { 0 });
    })
}

pub fn exec_arm_sbc<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let c = cpsr.get_C();
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        if s {
            unimplemented!()
        }
        gpr[rd] = gpr[rn].wrapping_sub(value).wrapping_sub(if c { 0 } else { 1 });
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
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        if s {
            unimplemented!()
        }
        gpr[rd] = gpr[rn].wrapping_sub(value).wrapping_sub(if c { 0 } else { 1 });
    })
}

pub fn exec_arm_tst<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, carry| {
        let tst = gpr[rn] & value;
        cpsr.set_N(tst >> 31 != 0);
        cpsr.set_Z(tst == 0);
        if carry {
            cpsr.set_C(true);
        }
    })
}

pub fn exec_arm_teq<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, carry| {
        let teq = gpr[rn] ^ value;
        cpsr.set_N(teq >> 31 != 0);
        cpsr.set_Z(teq == 0);
        if carry {
            cpsr.set_C(true);
        }
    })
}

pub fn exec_arm_cmp<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        let rn = gpr[rn];
        let cmp = rn.wrapping_sub(value);
        cpsr.set_N(cmp >> 31 != 0);
        cpsr.set_Z(cmp == 0);
        let (_, v) = (rn as i32).overflowing_sub(value as i32);
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
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
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
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        if s {
            unimplemented!()
        }
        gpr[rd] = gpr[rn] | value;
    })
}

pub fn exec_arm_shift<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        if s {
            unimplemented!()
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
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        if s {
            unimplemented!()
        }
        gpr[rd] = gpr[rn] & !value;
    })
}

pub fn exec_arm_mvn<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
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
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        if s {
            unimplemented!()
        }
        gpr[rd] = value >> 1 | (if cpsr.get_C() { 0x8000_0000 } else { 0 })
    })
}
