use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::thumb::*;
use crate::cpu::instructions::{shift::*, ExecuteResult};
use crate::cpu::registers::psr::PSR;
use crate::cpu::types::*;
use crate::types::*;

pub fn exec_thumb_add1<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let imm = dec.get_imm3() as u32;
    let d = ((gpr[dec.get_Rn5_3() as usize]) + imm) as u64;

    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C_from(d);
    cpsr.set_V_from(gpr[dec.get_Rd2_0() as usize], d as u32);
    gpr[dec.get_Rd2_0() as usize] = d as u32;
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_add2<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rn_rd = dec.get_Rd10_8() as usize;
    let d = gpr[rn_rd] as u64 + dec.get_imm8() as u64;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C_from(d);
    cpsr.set_V_from(gpr[rn_rd], d as u32);
    gpr[rn_rd] = d as u32;
    dbg!("ADD2", &gpr, cpsr.get_C(), cpsr.get_V(), cpsr.get_N(), cpsr.get_Z());
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_add3<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let d = (gpr[dec.get_Rn5_3() as usize]) as u64 + (gpr[dec.get_Rm8_6() as usize]) as u64;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C_from(d);
    cpsr.set_V_from(gpr[dec.get_Rd2_0() as usize], d as u32);
    gpr[dec.get_Rd2_0() as usize] = d as u32;
    dbg!("ADd3", &gpr, cpsr.get_C(), cpsr.get_V(), cpsr.get_N(), cpsr.get_Z());
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_sub2<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let imm = dec.get_imm8() as u32;
    let rn = dec.get_Rn10_8() as u32;
    let d = gpr[rn as usize] as i64 - imm as i64;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C(gpr[rn as usize] >= imm);
    cpsr.set_V_from(gpr[dec.get_Rd10_8() as usize], d as u32);
    gpr[dec.get_Rd10_8() as usize] = d as u32;
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_sub3<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    dbg!(format!("{:x}", dec.0));
    let rn = dec.get_Rn5_3() as usize;
    let rm = dec.get_Rm8_6() as usize;
    let rd = dec.get_Rd2_0() as usize;
    let d = ((gpr[rn]) as i64 - (gpr[rm]) as i64) as u64;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C(gpr[rn] >= gpr[rm]);
    cpsr.set_V_from(gpr[rd], d as u32);
    gpr[rd] = d as u32;
    dbg!("SUB3", &gpr, cpsr.get_C(), cpsr.get_V(), cpsr.get_N(), cpsr.get_Z());
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_and<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let d = gpr[dec.get_Rd2_0() as usize] & gpr[dec.get_Rm5_3() as usize];
    gpr[dec.get_Rd2_0() as usize] = d;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_eor<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let d = gpr[dec.get_Rd2_0() as usize] ^ gpr[dec.get_Rm5_3() as usize];
    gpr[dec.get_Rd2_0() as usize] = d;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_asr1<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn5_3() as usize;
    let sh = dec.get_sh() as u32;
    if sh == 0 {
        gpr[rd] = gpr[rn];
    } else {
        cpsr.set_C(is_carry_over(Shift::ASR, gpr[rn], sh));
        gpr[rd] = asr(gpr[rn], sh);
    }
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    dbg!("ASR1", &gpr, cpsr.get_C(), cpsr.get_N(), cpsr.get_Z(), cpsr.get_V());
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_lsl2<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rd = dec.get_Rd2_0() as usize;
    let sh = dec.get_Rs() as u32;

    if sh == 0 {
        return (1, PipelineStatus::Continue);
    }
    cpsr.set_C(is_carry_over(Shift::LSL, gpr[rd], sh));
    gpr[rd] = lsl(gpr[rd], sh);
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    // Consume S + 1 cycle
    (s + 1, PipelineStatus::Continue)
}

pub fn exec_thumb_lsr2<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rd = dec.get_Rd2_0() as usize;
    let sh = dec.get_Rs() as u32;

    if sh == 0 {
        return (1, PipelineStatus::Continue);
    }
    cpsr.set_C(is_carry_over(Shift::LSR, gpr[rd], sh));
    gpr[rd] = lsr(gpr[rd], sh);
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    // Consume S + 1 cycle
    (s + 1, PipelineStatus::Continue)
}

pub fn exec_thumb_sbc<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
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
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_ror<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rd = dec.get_Rd2_0() as usize;
    let sh = dec.get_Rs() as u32;
    if sh == 0 {
        return (1, PipelineStatus::Continue);
    }
    cpsr.set_C(is_carry_over(Shift::ROR, gpr[rd], sh));
    gpr[rd] = ror(gpr[rd], sh);
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    // Consume S + 1 cycle
    (s + 1, PipelineStatus::Continue)
}

pub fn exec_thumb_tst<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let v = gpr[dec.get_Rd2_0() as usize] & gpr[dec.get_Rm5_3() as usize];
    cpsr.set_N_from(v);
    cpsr.set_Z_from(v);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_neg<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let s = gpr[dec.get_Rm5_3() as usize] as i32;
    let d = -s;
    cpsr.set_Z_from(d as u32);
    cpsr.set_N_from(d as u32);
    cpsr.set_C(d <= 0);
    cpsr.set_V((0 as i32).overflowing_sub(s as i32).1);
    gpr[dec.get_Rd2_0() as usize] = d as u32;
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_cmp1<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let d = gpr[dec.get_Rd10_8() as usize] as i64 - dec.get_imm8() as i64;
    cpsr.set_Z_from(d as u32);
    cpsr.set_N_from(d as u32);
    cpsr.set_C(d >= 0);
    cpsr.set_V_from(gpr[dec.get_Rd10_8() as usize], d as u32);
    dbg!("CMP1", &gpr, cpsr.get_C(), cpsr.get_V(), cpsr.get_N(), cpsr.get_Z());
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_cmp2<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let d = gpr[dec.get_Rd2_0() as usize] as i64 - gpr[dec.get_Rm5_3() as usize] as i64;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C(d >= 0);
    cpsr.set_V_from(gpr[dec.get_Rd2_0() as usize], d as u32);
    dbg!("CMP2", &gpr, cpsr.get_C(), cpsr.get_V(), cpsr.get_N(), cpsr.get_Z());
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_cmn<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    todo!("cmn");
    (0, PipelineStatus::Continue)
}

pub fn exec_thumb_orr<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    todo!("orr");
    (0, PipelineStatus::Continue)
}

pub fn exec_thumb_mul<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    todo!("mul");
    (0, PipelineStatus::Continue)
}

pub fn exec_thumb_mvn<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    todo!("mvn");
    // let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (0, PipelineStatus::Continue)
}

pub fn exec_thumb_bic<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let d = gpr[dec.get_Rd2_0() as usize] & !gpr[dec.get_Rm5_3() as usize];
    gpr[dec.get_Rd2_0() as usize] = d;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    dbg!("bic", &gpr);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_lsl1<T>(bus: &mut T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn5_3() as usize;
    let sh = dec.get_sh() as u32;

    if sh == 0 {
        gpr[rd] = gpr[rn];
    } else {
        cpsr.set_C(is_carry_over(Shift::LSL, gpr[rn], sh));
        gpr[rd] = lsl(gpr[rn], sh);
    }
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_lsr1<T>(bus: &mut T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn5_3() as usize;
    let sh = dec.get_sh() as u32;

    if sh == 0 {
        gpr[rd] = gpr[rn];
    } else {
        cpsr.set_C(is_carry_over(Shift::LSR, gpr[rn], sh));
        gpr[rd] = lsr(gpr[rn], sh);
    }
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    dbg!("LSR1", &gpr, cpsr.get_C(), cpsr.get_V(), cpsr.get_N(), cpsr.get_Z());
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_mov1<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let imm = dec.get_imm8() as u32;
    gpr[dec.get_Rd10_8() as usize] = imm;
    cpsr.set_N_from(imm);
    cpsr.set_Z_from(imm);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

// Format 5
pub fn exec_thumb_mov3<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rm = dec.get_Rm5_3() as usize;
    let rd = dec.get_Rd2_0() as usize;

    gpr[rd] = gpr[rm];
    dbg!("MOV3", &gpr, cpsr.get_C(), cpsr.get_V(), cpsr.get_N(), cpsr.get_Z());

    if rd == PC {
        (0, PipelineStatus::Flush)
    } else {
        let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
        (s, PipelineStatus::Continue)
    }
}
