use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::{helpers::*, ExecuteResult};
use crate::cpu::registers::psr::PSR;
use crate::types::*;

pub fn exec_arm_mul<T>(bus: &mut T, dec: Multiple, gpr: &mut [Word; 16], _cspr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    if s {
        unimplemented!()
    }
    let rd = dec.get_Rd() as usize;
    let rm = dec.get_Rm() as usize;
    let rs = dec.get_Rs() as usize;
    gpr[rd] = ((gpr[rm] as i128) * gpr[rs] as i128) as u32;
    // MUL consume (m)I + S
    let cycle = compute_multiple_cycle(gpr[rs]) + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}

pub fn exec_arm_mla<T>(bus: &mut T, dec: Multiple, gpr: &mut [Word; 16], _cspr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    if s {
        unimplemented!()
    }
    let rd = dec.get_Rd() as usize;
    let rm = dec.get_Rm() as usize;
    let rs = dec.get_Rs() as usize;
    let rn = dec.get_Rn() as usize;
    gpr[rd] = (((gpr[rm] as u64) * gpr[rs] as u64) + gpr[rn] as u64) as u32;
    // MLA consume I + (m)I + S
    let cycle = 1 + compute_multiple_cycle(gpr[rs]) + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}

pub fn exec_arm_umull<T>(bus: &mut T, dec: Multiple, gpr: &mut [Word; 16], _cspr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    if s {
        unimplemented!()
    }
    let rd = dec.get_Rd() as usize;
    let rm = dec.get_Rm() as usize;
    let rs = dec.get_Rs() as usize;
    let rn = dec.get_Rn() as usize;
    let mul = (gpr[rm] as u64) * gpr[rs] as u64;
    gpr[rn] = mul as u32;
    gpr[rd] = (mul >> 32) as u32;
    // MULL consume (m)I + I + S
    let cycle = compute_multiple_cycle(gpr[rs]) + 1 + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}

pub fn exec_arm_umlal<T>(bus: &mut T, dec: Multiple, gpr: &mut [Word; 16], _cspr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    if s {
        unimplemented!()
    }
    let rd = dec.get_Rd() as usize;
    let rm = dec.get_Rm() as usize;
    let rs = dec.get_Rs() as usize;
    let rn = dec.get_Rn() as usize;
    let mul = (gpr[rm] as u64) * gpr[rs] as u64;
    let base = ((gpr[rd] as u64) << 32) + (gpr[rn] as u64);
    let result = mul + base;
    gpr[rn] = result as u32;
    gpr[rd] = (result >> 32) as u32;
    // MLAL consume I + (m)I + I + S
    let cycle = 1 + compute_multiple_cycle(gpr[rs]) + 1 + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}

pub fn exec_arm_smull<T>(bus: &mut T, dec: Multiple, gpr: &mut [Word; 16], _cspr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    if s {
        unimplemented!()
    }
    let rd = dec.get_Rd() as usize;
    let rm = dec.get_Rm() as usize;
    let rs = dec.get_Rs() as usize;
    let rn = dec.get_Rn() as usize;
    let mul = (gpr[rm] as i32 as i64) * gpr[rs] as i32 as i64;
    gpr[rn] = mul as u32;
    gpr[rd] = (mul >> 32) as u32;
    // MULL consume (m)I + I + S
    let cycle = compute_multiple_cycle(gpr[rs]) + 1 + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}

pub fn exec_arm_smlal<T>(bus: &mut T, dec: Multiple, gpr: &mut [Word; 16], _cspr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    if s {
        unimplemented!()
    }
    let rd = dec.get_Rd() as usize;
    let rm = dec.get_Rm() as usize;
    let rs = dec.get_Rs() as usize;
    let rn = dec.get_Rn() as usize;
    let mul = (gpr[rm] as i32 as i64) * gpr[rs] as i32 as i64;
    let base = (((gpr[rd] as u64) << 32) + (gpr[rn] as u64)) as i64;
    let result = mul + base;
    gpr[rn] = result as u32;
    gpr[rd] = (result >> 32) as u32;
    // MLAL consume I + (m)I + I + S
    let cycle = 1 + compute_multiple_cycle(gpr[rs]) + 1 + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}
