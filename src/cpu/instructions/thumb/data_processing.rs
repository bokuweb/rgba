use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::thumb::*;
use crate::cpu::instructions::{helpers::compute_multiple_cycle, shift::*, ExecuteResult};
use crate::cpu::registers::psr::PSR;
use crate::cpu::types::*;
use crate::types::*;

pub fn exec_thumb_add1<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let imm = dec.get_imm3() as u32;
    let d = ((gpr[dec.get_Rn5_3() as usize]) + imm) as u64;

    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C_from(d);
    let (_, v) = (gpr[dec.get_Rn5_3() as usize] as i32).overflowing_add(imm as i32);
    cpsr.set_V(v);
    // cpsr.set_V_from(gpr[dec.get_Rd2_0() as usize], d as u32);
    gpr[dec.get_Rd2_0() as usize] = d as u32;
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_add2<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rn_rd = dec.get_Rd10_8() as usize;
    let imm = dec.get_imm8() as u32;
    let d = gpr[rn_rd] as u64 + imm as u64;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C_from(d);
    let v = gpr[rn_rd] >> 31 == 0 && (gpr[rn_rd] as u32 ^ d as u32) >> 31 != 0 && (imm ^ d as u32) >> 31 != 0;
    // cpsr.set_V_from(gpr[rn_rd], d as u32);
    cpsr.set_V(v);
    gpr[rn_rd] = d as u32;
    // dbg!("ADD2", &gpr, cpsr.get_C(), cpsr.get_V(), cpsr.get_N(), cpsr.get_Z());
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_add3<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let d = (gpr[dec.get_Rn5_3() as usize]) as u64 + (gpr[dec.get_Rm8_6() as usize]) as u64;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C_from(d);
    let (_, v) = (gpr[dec.get_Rn5_3() as usize] as i32).overflowing_add(gpr[dec.get_Rm8_6() as usize] as i32);
    cpsr.set_V(v);
    gpr[dec.get_Rd2_0() as usize] = d as u32;
    // dbg!("ADd3", &gpr, cpsr.get_C(), cpsr.get_V(), cpsr.get_N(), cpsr.get_Z());
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_add_hi_register<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rs = usize::from(dec.get_Rs6_3());
    let rd = usize::from(dec.get_Rd_7_2_0());
    gpr[rd] = gpr[rd].wrapping_add(gpr[rs]);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    if rd == PC {
        (0, PipelineStatus::Flush)
    } else {
        (s, PipelineStatus::Continue)
    }
}

// THUMB.12: get relative address
pub fn exec_thumb_add_relative_address<T: BusAccessor>(
    bus: &T,
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
) -> ExecuteResult {
    let imm = dec.get_imm8();
    let imm = (imm as u32).wrapping_shl(2);
    let rd = dec.get_Rd10_8();
    let data = if dec.get_bit11() { gpr[SP] } else { gpr[PC] & 0xFFFF_FFFC };
    let data = data + imm as u32;
    gpr[rd as usize] = data;
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_add7<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let imm = dec.get_imm7() as i8;
    let imm = if dec.get_A() { -imm } else { imm };
    // dbg!(imm);
    let imm = (imm as i32).wrapping_shl(2);
    // dbg!(imm);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    gpr[SP] = (gpr[SP] as i64 + imm as i64) as u32;
    // dbg!(&gpr, imm);
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_sub1<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let imm = dec.get_imm3() as u32;
    let rn = dec.get_Rn5_3() as usize;
    let d = (gpr[rn] as i64) - imm as i64;

    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C(gpr[rn] >= imm);
    let (_, v) = (gpr[rn] as i32).overflowing_sub(imm as i32);
    cpsr.set_V(v);
    // cpsr.set_V_from(gpr[dec.get_Rd2_0() as usize], d as u32);
    gpr[dec.get_Rd2_0() as usize] = d as u32;
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
    cpsr.set_V((gpr[rn as usize] as i32).overflowing_sub(imm as i32).1);
    // let v = gpr[rn_rd] >> 31 == 0 && (gpr[rn_rd] as u32 ^ d as u32) >> 31 != 0 && (imm ^ d as u32) >> 31 != 0;
    // cpsr.set_V_from(gpr[rn_rd], d as u32);
    gpr[dec.get_Rd10_8() as usize] = d as u32;
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_sub3<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    // dbg!(format!("{:x}", dec.0));
    let rn = dec.get_Rn5_3() as usize;
    let rm = dec.get_Rm8_6() as usize;
    let rd = dec.get_Rd2_0() as usize;
    let d = ((gpr[rn]) as i64 - (gpr[rm]) as i64) as u64;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C(gpr[rn] >= gpr[rm]);
    let (_, v) = (gpr[rn] as i32).overflowing_sub(gpr[rm] as i32);
    cpsr.set_V(v);
    // cpsr.set_V_from(gpr[rd], d as u32);
    gpr[rd] = d as u32;
    // dbg!("SUB3", &gpr, cpsr.get_C(), cpsr.get_V(), cpsr.get_N(), cpsr.get_Z());
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

pub fn exec_thumb1_asr<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn5_3() as usize;
    let sh = dec.get_sh() as u32;
    if sh == 0 {
        gpr[rd] = gpr[rn];
        let c = gpr[rn] >> 31 != 0;
        cpsr.set_C(c);
        if c {
            gpr[rd] = 0xFFFF_FFFF;
        } else {
            gpr[rd] = 0x0;
        }
    } else {
        cpsr.set_C(is_carry_over(Shift::ASR, gpr[rn], sh, cpsr.get_C(), true));
        gpr[rd] = asr(gpr[rn], sh, true);
    }
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    // dbg!("ASR1", &gpr, cpsr.get_C(), cpsr.get_N(), cpsr.get_Z(), cpsr.get_V());
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb4_lsl<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rd = dec.get_Rd2_0() as usize;
    let rs = dec.get_Rs() as usize;
    let sh = gpr[rs] & 0xFF;
    if sh != 0 {
        if sh < 32 {
            cpsr.set_C(gpr[rd] & (1 << (32 - sh)) != 0);
            gpr[rd] = gpr[rd].wrapping_shl(sh);
        } else {
            if sh > 32 {
                cpsr.set_C(false);
            } else {
                cpsr.set_C(gpr[rd] & 0x01 != 0);
            }
            gpr[rd] = 0;
        }
        // return (1, PipelineStatus::Continue);
    }
    // cpsr.set_C(is_carry_over(Shift::LSL, gpr[rd], sh, cpsr.get_C(), true));
    // gpr[rd] = lsl(gpr[rd], sh);
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    let s = if sh != 0 {
        bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word))
    } else {
        0
    };
    // Consume S + 1 cycle
    (s + 1, PipelineStatus::Continue)
}

pub fn exec_thumb4_asr<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rd = dec.get_Rd2_0() as usize;
    let rs = dec.get_Rs() as usize;
    let sh = gpr[rs] & 0xFF;
    if sh != 0 {
        if sh < 32 {
            cpsr.set_C(gpr[rd] & (1 << (sh - 1)) != 0);
            gpr[rd] = gpr[rd].wrapping_shr(sh)
                | if gpr[rd] & 0x8000_0000 != 0 {
                    ((0xFFFF_FFFF as u32).wrapping_shl(32 - sh))
                } else {
                    0
                };
        } else {
            cpsr.set_C(gpr[rd].wrapping_shr(31) != 0);
            if cpsr.get_C() {
                gpr[rd] = 0xFFFF_FFFF;
            } else {
                gpr[rd] = 0;
            }
        }
    }
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    let s = if sh != 0 {
        bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word))
    } else {
        0
    };
    // Consume S + 1 cycle
    (s + 1, PipelineStatus::Continue)
}

pub fn exec_thumb_lsr2<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rd = dec.get_Rd2_0() as usize;
    let rs = dec.get_Rs() as usize;
    let sh = gpr[rs] & 0xFF;

    if sh != 0 {
        if sh < 32 {
            cpsr.set_C(gpr[rd] & (1 << (rs - 1)) != 0);
            gpr[rd] = gpr[rd].wrapping_shr(sh);
        } else {
            if sh > 32 {
                cpsr.set_C(false);
            } else {
                cpsr.set_C(gpr[rd].wrapping_shr(31) != 0);
            }
            gpr[rd] = 0;
        }
    }
    // cpsr.set_C(is_carry_over(Shift::LSR, gpr[rd], sh, cpsr.get_C(), true));
    // gpr[rd] = lsr(gpr[rd], sh, true);
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
    let d = (gpr[rd] as u64).wrapping_sub(m);
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C_from(d);
    let (_, v) = (gpr[rd] as i32).overflowing_sub(m as i32);
    cpsr.set_V(v);
    // cpsr.set_V_from(gpr[rd], d as u32);
    gpr[rd] = d as u32;
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb4_ror<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rd = dec.get_Rd2_0() as usize;
    let sh = gpr[dec.get_Rs() as usize] & 0xFF;
    if sh != 0 {
        let r = sh & 0x1F;
        if r > 0 {
            cpsr.set_C((gpr[rd] & (1 << (r - 1))) != 0);
            gpr[rd] = gpr[rd].rotate_right(r);
        } else {
            cpsr.set_C(gpr[rd] >> 31 != 0)
        }
    }
    // cpsr.set_C(is_carry_over(Shift::ROR, gpr[rd], sh, cpsr.get_C(), true));
    // gpr[rd] = ror(gpr[rd], sh, cpsr.get_C(), true);
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
    let (_, v) = (gpr[dec.get_Rn10_8() as usize] as i32).overflowing_sub(dec.get_imm8() as i32);
    cpsr.set_V(v);
    // cpsr.set_V_from(gpr[dec.get_Rd10_8() as usize], d as u32);
    // dbg!("CMP1", &gpr, cpsr.get_C(), cpsr.get_V(), cpsr.get_N(), cpsr.get_Z());
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_cmp2<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let d = gpr[dec.get_Rd2_0() as usize] as i64 - gpr[dec.get_Rm5_3() as usize] as i64;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_C(d >= 0);
    let (_, v) = (gpr[dec.get_Rd2_0() as usize] as i32).overflowing_sub(dec.get_Rm5_3() as i32);
    cpsr.set_V(v);
    // cpsr.set_V_from(gpr[dec.get_Rd2_0() as usize], d as u32);
    // dbg!("CMP2", &gpr, cpsr.get_C(), cpsr.get_V(), cpsr.get_N(), cpsr.get_Z());
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb5_cmp<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rd = gpr[dec.get_Rd_7_2_0() as usize];
    let rs = gpr[dec.get_Rs6_3() as usize];
    let d = rd as i64 - rs as i64;
    cpsr.set_Z_from(d as u32);
    cpsr.set_N_from(d as u32);
    cpsr.set_C(d >= 0);
    let (_, v) = (rd as i32).overflowing_sub(rs as i32);
    cpsr.set_V(v);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb4_cmn<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rd = gpr[dec.get_Rd2_0() as usize];
    let rs = gpr[dec.get_Rs() as usize];
    let d = (rd as u64).wrapping_add(rs as u64);
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    let (_, v) = (rd as i32).overflowing_add(rs as i32);
    cpsr.set_V(v);
    cpsr.set_C(d & (1 << 32) != 0);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb4_adc<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rd = gpr[dec.get_Rd2_0() as usize];
    let rs = gpr[dec.get_Rs() as usize];
    let c: u32 = cpsr.get_C().into();
    let (_, v) = (rd as i32).overflowing_add(rs as i32 + c as i32);
    let d = (rd as u64).wrapping_add(rs as u64 + c as u64);
    gpr[dec.get_Rd2_0() as usize] = d as u32;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    cpsr.set_V(v);
    cpsr.set_C(d & (1 << 32) != 0);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_orr<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rd = dec.get_Rd2_0() as usize;
    let rm = dec.get_Rm5_3() as usize;
    let d = gpr[rd] | gpr[rm];
    gpr[rd] = d;
    cpsr.set_N_from(d);
    cpsr.set_Z_from(d);
    // Consume 1S
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    // dbg!("orr", &gpr);
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_mul<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rd = dec.get_Rd2_0() as usize;
    let rm = dec.get_Rm5_3() as usize;
    let d = gpr[rd] as i128 * gpr[rm] as i128;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    gpr[rd] = d as u32;
    // MUL consume (m)I + S
    let cycle = compute_multiple_cycle(gpr[rm]) + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    // dbg!("MUL", &gpr);
    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_mvn<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let d = !gpr[dec.get_Rm5_3() as usize];
    gpr[dec.get_Rd2_0() as usize] = d;
    cpsr.set_N_from(d);
    cpsr.set_Z_from(d);
    // Consume 1S
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    // dbg!("mvn", &gpr);
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_bic<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let d = gpr[dec.get_Rd2_0() as usize] & !gpr[dec.get_Rm5_3() as usize];
    gpr[dec.get_Rd2_0() as usize] = d;
    cpsr.set_N_from(d as u32);
    cpsr.set_Z_from(d as u32);
    // dbg!("bic", &gpr);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb1_lsl<T>(bus: &mut T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn5_3() as usize;
    let sh = dec.get_sh() as u32;

    if sh == 0 {
        gpr[rd] = gpr[rn];
    } else {
        cpsr.set_C((gpr[rn] & (1 << (32 - sh))) != 0);
        gpr[rd] = gpr[rn].wrapping_shl(sh);
    }
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb1_lsr<T>(bus: &mut T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn5_3() as usize;
    let sh = dec.get_sh() as u32;

    if sh == 0 {
        cpsr.set_C(gpr[rn] >> 31 != 0);
        gpr[rd] = 0x0;
    } else {
        cpsr.set_C(is_carry_over(Shift::LSR, gpr[rn], sh, cpsr.get_C(), true));
        gpr[rd] = lsr(gpr[rn], sh, true);
    }
    cpsr.set_N_from(gpr[rd]);
    cpsr.set_Z_from(gpr[rd]);
    // dbg!("LSR1", &gpr, cpsr.get_C(), cpsr.get_V(), cpsr.get_N(), cpsr.get_Z());
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    (s, PipelineStatus::Continue)
}

pub fn exec_thumb_mov1<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let imm = dec.get_imm8() as u32;
    gpr[dec.get_Rd10_8() as usize] = imm;
    cpsr.set_N_from(imm);
    cpsr.set_Z_from(imm);
    let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
    // dbg!("mov1", &gpr);
    (s, PipelineStatus::Continue)
}

// Format 5 Hi register operations
pub fn exec_thumb_mov3<T: BusAccessor>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> ExecuteResult {
    let rs = dec.get_Rs6_3() as usize;
    let rd = usize::from(if dec.get_msbd() { dec.get_Rd2_0() | 0x8 } else { dec.get_Rd2_0() });
    // let rd = dec.get_Rd_7_2_0() as usize;

    gpr[rd] = gpr[rs];
    // dbg!("MOV3", &gpr, cpsr.get_C(), cpsr.get_V(), cpsr.get_N(), cpsr.get_Z());

    if rd == PC {
        gpr[PC] = gpr[PC] & 0xFFFF_FFFE;
        (0, PipelineStatus::Flush)
    } else {
        let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
        (s, PipelineStatus::Continue)
    }
}
