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

pub fn exec_arm_mov<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    dbg!("arm mov");
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = value;
    })
}

pub fn exec_arm_and<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn] & value;
    })
}

pub fn exec_arm_eor<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn] ^ value;
    })
}

pub fn exec_arm_sub<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn].wrapping_sub(value);
    })
}

pub fn exec_arm_rsb<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = value.wrapping_sub(gpr[rn]);
    })
}

pub fn exec_arm_add<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn].wrapping_add(value);
    })
}

pub fn exec_arm_adc<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cspr: &PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn].wrapping_add(value).wrapping_add(if cspr.get_C() { 1 } else { 0 });
    })
}

pub fn exec_arm_sbc<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cspr: &PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn].wrapping_sub(value).wrapping_sub(if cspr.get_C() { 0 } else { 1 });
    })
}

pub fn exec_arm_rsc<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cspr: &PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn].wrapping_sub(value).wrapping_sub(if cspr.get_C() { 0 } else { 1 });
    })
}

pub fn exec_arm_tst<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cspr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, carry| {
        let tst = gpr[rn] & value;
        cspr.set_N(tst >> 31 != 0);
        cspr.set_Z(tst == 0);
        if carry {
            cspr.set_C(true);
        }
    })
}

pub fn exec_arm_teq<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cspr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, carry| {
        let teq = gpr[rn] ^ value;
        cspr.set_N(teq >> 31 != 0);
        cspr.set_Z(teq == 0);
        if carry {
            cspr.set_C(true);
        }
    })
}

pub fn exec_arm_cmp<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cspr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        let rn = gpr[rn];
        let cmp = rn.wrapping_sub(value);
        cspr.set_N(cmp >> 31 != 0);
        cspr.set_Z(cmp == 0);
        let (_, v) = (rn as i32).overflowing_sub(value as i32);
        cspr.set_V(v);
        // NOTE: Should we consider to shifted carry?
        cspr.set_C(rn >= value);
    })
}

pub fn exec_arm_cmn<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cspr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        let rn = gpr[rn];
        let cmn = (rn as u64).wrapping_add(value as u64);
        cspr.set_N((cmn as i32) < 0);
        cspr.set_Z(cmn == 0);
        let (_, v) = (rn as i32).overflowing_add(value as i32);
        cspr.set_V(v);
        cspr.set_C(cmn & (1 << 32) != 0);
    })
}

pub fn exec_arm_orr<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn] | value;
    })
}

pub fn exec_arm_shift<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = value;
    })
}

pub fn exec_arm_bic<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| gpr[rd] = gpr[rn] & !value)
}

pub fn exec_arm_mvn<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| gpr[rd] = !value)
}

pub fn exec_arm_rrx<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cspr: &PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_data_processing(bus, gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = value >> 1 | (if cspr.get_C() { 0x8000_0000 } else { 0 })
    })
}
