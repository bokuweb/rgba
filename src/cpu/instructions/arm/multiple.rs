use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::registers::psr::PSR;
use crate::cpu::types::*;

pub fn exec_multiple<F>(
    gpr: &mut [Word; 16],
    dec: Multiple,
    multiple: &mut F,
) -> Result<PipelineStatus, ()>
where
    F: FnMut(&mut [Word; 16]),
{
    multiple(gpr);
    if dec.get_Rd() == PC as u32 {
        Ok(PipelineStatus::Flush)
    } else {
        Ok(PipelineStatus::Continue)
    }
}

pub fn exec_mul<T>(
    __bus: &mut T,
    dec: Multiple,
    gpr: &mut [Word; 16],
    _cspr: &PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;
    exec_multiple(gpr, dec, &mut |gpr| {
        gpr[rd] = ((gpr[rn] as u64) * gpr[rm] as u64) as u32;
    })
}

pub fn exec_mla<T>(
    __bus: &mut T,
    dec: Multiple,
    gpr: &mut [Word; 16],
    _cspr: &PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;
    let ra = dec.get_Ra() as usize;
    exec_multiple(gpr, dec, &mut |gpr| {
        gpr[rd] = (((gpr[rn] as u64) * gpr[rm] as u64) + gpr[ra] as u64) as u32;
    })
}

pub fn exec_umull<T>(
    __bus: &mut T,
    dec: Multiple,
    gpr: &mut [Word; 16],
    _cspr: &PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;
    let ra = dec.get_Ra() as usize;
    exec_multiple(gpr, dec, &mut |gpr| {
        let mul = (gpr[rn] as u64) * gpr[rm] as u64;
        gpr[ra] = mul as u32;
        gpr[rd] = (mul >> 32) as u32;
    })
}

pub fn exec_umlal<T>(
    _bus: &mut T,
    dec: Multiple,
    gpr: &mut [Word; 16],
    _cspr: &PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;
    let ra = dec.get_Ra() as usize;
    exec_multiple(gpr, dec, &mut |gpr| {
        let mul = (gpr[rn] as u64) * gpr[rm] as u64;
        let base = ((gpr[rd] as u64) << 32) + (gpr[ra] as u64);
        let result = mul + base;
        gpr[ra] = result as u32;
        gpr[rd] = (result >> 32) as u32;
    })
}

pub fn exec_smull<T>(
    _bus: &mut T,
    dec: Multiple,
    gpr: &mut [Word; 16],
    _cspr: &PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;
    let ra = dec.get_Ra() as usize;
    exec_multiple(gpr, dec, &mut |gpr| {
        let mul = (gpr[rn] as i32 as i64) * gpr[rm] as i32 as i64;
        gpr[ra] = mul as u32;
        gpr[rd] = (mul >> 32) as u32;
    })
}

pub fn exec_smlal<T>(
    _bus: &mut T,
    dec: Multiple,
    gpr: &mut [Word; 16],
    _cspr: &PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;
    let ra = dec.get_Ra() as usize;
    exec_multiple(gpr, dec, &mut |gpr| {
        let mul = (gpr[rn] as i32 as i64) * gpr[rm] as i32 as i64;
        let base = (((gpr[rd] as u64) << 32) + (gpr[ra] as u64)) as i64;
        let result = mul + base;
        gpr[ra] = result as u32;
        gpr[rd] = (result >> 32) as u32;
    })
}
