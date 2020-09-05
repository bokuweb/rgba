use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::decoder::thumb::*;
use crate::cpu::instructions::shift::*;
use crate::cpu::registers::psr::PSR;
use crate::cpu::types::*;

pub fn exec_thumb_add1(
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
) -> Result<PipelineStatus, ()> {
    let imm = dec.get_imm3() as u32;
    let d = ((gpr[dec.get_Rn() as usize]) + imm) as u64;

    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C_from(d);
    cpsr.set_V_from(gpr[dec.get_Rd2_0() as usize], d as u32);
    gpr[dec.get_Rd2_0() as usize] = d as u32;
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_add3(
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
) -> Result<PipelineStatus, ()> {
    let d = (gpr[dec.get_Rn() as usize]) as u64 + (gpr[dec.get_Rm8_6() as usize]) as u64;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C_from(d);
    cpsr.set_V_from(gpr[dec.get_Rd2_0() as usize], d as u32);
    gpr[dec.get_Rd2_0() as usize] = d as u32;
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_and(
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
) -> Result<PipelineStatus, ()> {
    let d = gpr[dec.get_Rd2_0() as usize] & gpr[dec.get_Rm5_3() as usize];
    gpr[dec.get_Rd2_0() as usize] = d;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_eor(
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
) -> Result<PipelineStatus, ()> {
    let d = gpr[dec.get_Rd2_0() as usize] ^ gpr[dec.get_Rm5_3() as usize];
    gpr[dec.get_Rd2_0() as usize] = d;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_lsl2(
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
) -> Result<PipelineStatus, ()> {
    let rd = dec.get_Rd2_0() as usize;
    let sh = dec.get_Rs() as u32;

    if sh == 0 {
        return Ok(PipelineStatus::Continue);
    }
    cpsr.set_C(is_carry_over(Shift::LSL, gpr[rd], sh));
    gpr[rd] = lsl(gpr[rd], sh);
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_lsr2(
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
) -> Result<PipelineStatus, ()> {
    let rd = dec.get_Rd2_0() as usize;
    let sh = dec.get_Rs() as u32;

    if sh == 0 {
        return Ok(PipelineStatus::Continue);
    }
    cpsr.set_C(is_carry_over(Shift::LSR, gpr[rd], sh));
    gpr[rd] = lsr(gpr[rd], sh);
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_sbc(
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
) -> Result<PipelineStatus, ()> {
    let rd = dec.get_Rd2_0() as usize;
    let rm = dec.get_Rm5_3() as usize;

    let c = if cpsr.get_C() { 0 } else { 1 };
    let m = gpr[rm] as u64 + c;
    let d = gpr[rd] as u64 - m;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C_from(d);
    cpsr.set_V_from(gpr[rd], d as u32);
    gpr[rd] = d as u32;
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_bic(
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
) -> Result<PipelineStatus, ()> {
    let d = gpr[dec.get_Rd2_0() as usize] & !gpr[dec.get_Rm5_3() as usize];
    gpr[dec.get_Rd2_0() as usize] = d;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_lsl<T>(
    bus: &mut T,
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let sh = dec.get_sh() as u32;

    if sh == 0 {
        gpr[rd] = gpr[rn];
    } else {
        cpsr.set_C(is_carry_over(Shift::LSL, gpr[rn], sh));
        gpr[rd] = lsl(gpr[rn], sh);
    }
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_mov1(
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
) -> Result<PipelineStatus, ()> {
    let imm = dec.get_imm8() as u32;
    gpr[dec.get_Rd10_8() as usize] = imm;
    cpsr.set_N_from(imm);
    cpsr.set_Z_from(imm);
    Ok(PipelineStatus::Continue)
}
