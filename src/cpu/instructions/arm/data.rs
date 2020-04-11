use super::super::PipelineStatus;

use super::shift::{is_carry_over, ror, shift};
use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::registers::psr::PSR;
use crate::cpu::types::*;

pub fn exec_data_processing<F>(
    gpr: &mut [Word; 16],
    dec: Box<dyn Decoder>,
    data_process: &mut F,
) -> Result<PipelineStatus, ()>
where
    F: FnMut(&mut [Word; 16], Word, Option<bool>),
{
    let (value, carry) = if dec.has_I() {
        let shift_value = dec.get_rot() * 2;
        (
            ror(dec.get_imm8(), shift_value),
            is_carry_over(dec.get_sh(), dec.get_imm8(), shift_value),
        )
    } else {
        let rm = dec.get_Rm() as usize;
        let shift_value = if dec.is_reg_offset() {
            dec.get_Rs()
        } else {
            dec.get_shamt5()
        };
        (
            shift(dec.get_sh(), gpr[rm], shift_value),
            is_carry_over(dec.get_sh(), gpr[rm], shift_value),
        )
    };
    data_process(gpr, value, carry);
    if dec.get_Rd() == PC {
        Ok(PipelineStatus::Flush)
    } else {
        Ok(PipelineStatus::Continue)
    }
}

pub fn exec_mov<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = value;
    })
}

pub fn exec_and<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    let rn = dec.get_Rn();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn] & value;
    })
}

pub fn exec_eor<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    let rn = dec.get_Rn();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn] ^ value;
    })
}

pub fn exec_sub<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    let rn = dec.get_Rn();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn].wrapping_sub(value);
    })
}

pub fn exec_rsb<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    let rn = dec.get_Rn();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = value.wrapping_sub(gpr[rn]);
    })
}

pub fn exec_add<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    let rn = dec.get_Rn();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn].wrapping_add(value);
    })
}

pub fn exec_adc<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
    cspr: &PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    let rn = dec.get_Rn();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn]
            .wrapping_add(value)
            .wrapping_add(if cspr.get_C() { 1 } else { 0 });
    })
}

pub fn exec_sbc<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
    cspr: &PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    let rn = dec.get_Rn();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn]
            .wrapping_sub(value)
            .wrapping_sub(if cspr.get_C() { 0 } else { 1 });
    })
}

pub fn exec_rsc<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
    cspr: &PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    let rn = dec.get_Rn();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn]
            .wrapping_sub(value)
            .wrapping_sub(if cspr.get_C() { 0 } else { 1 });
    })
}

pub fn exec_tst<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
    cspr: &mut PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn();
    exec_data_processing(gpr, dec, &mut |gpr, value, carry| {
        let tst = gpr[rn] & value;
        cspr.set_N(tst >> 31 != 0);
        cspr.set_Z(tst == 0);
        if let Some(c) = carry {
            cspr.set_C(c);
        }
    })
}

pub fn exec_teq<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
    cspr: &mut PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn();
    exec_data_processing(gpr, dec, &mut |gpr, value, carry| {
        let teq = gpr[rn] ^ value;
        cspr.set_N(teq >> 31 != 0);
        cspr.set_Z(teq == 0);
        if let Some(c) = carry {
            cspr.set_C(c);
        }
    })
}

pub fn exec_cmp<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
    cspr: &mut PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| {
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

pub fn exec_cmn<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
    cspr: &mut PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| {
        let rn = gpr[rn];
        let cmn = (rn as u64).wrapping_add(value as u64);
        cspr.set_N((cmn as i32) < 0);
        cspr.set_Z(cmn == 0);
        let (_, v) = (rn as i32).overflowing_add(value as i32);
        cspr.set_V(v);
        cspr.set_C(cmn & (1 << 32) != 0);
    })
}

pub fn exec_orr<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    let rn = dec.get_Rn();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = gpr[rn] | value;
    })
}

pub fn exec_shift<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = value;
    })
}

pub fn exec_bic<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    let rn = dec.get_Rn();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| gpr[rd] = gpr[rn] & !value)
}

pub fn exec_mvn<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| gpr[rd] = !value)
}

pub fn exec_rrx<T>(
    _bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
    cspr: &PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    exec_data_processing(gpr, dec, &mut |gpr, value, _| {
        gpr[rd] = value >> 1 | (if cspr.get_C() { 0x8000_0000 } else { 0 })
    })
}
