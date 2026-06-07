use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::thumb::*;
use crate::cpu::instructions::{helpers::read_ldr_data, ExecuteResult};
use crate::types::*;

pub fn exec_thumb_ldr_imm_offset<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16], _started: bool) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let offset = dec.get_off5() as u32;
    let addr = gpr[rn].wrapping_add(offset.wrapping_shl(2));
    let data = read_ldr_data(bus, addr);
    // 1N + 1I cycle
    let cycle = bus.compute_cycle(addr, AccessType::NonSeq(AccessWidth::Word)) + 1;

    gpr[rd] = data;

    // Add merged I + S cycle.
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_ldr_reg_offset<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    // dbg!("before ldr2", &gpr);
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;

    let addr = gpr[rn].wrapping_add(gpr[rm]);
    let data = read_ldr_data(bus, addr);

    // 1N + 1I cycle
    let cycle = bus.compute_cycle(addr, AccessType::NonSeq(AccessWidth::Word)) + 1;

    gpr[rd] = data;

    // Add merged I + S cycle.
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_ldrb_reg_offset<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;

    let addr = gpr[rn] + gpr[rm];

    let data = bus.read_byte(addr);

    // 1N + 1I cycle
    let cycle = bus.compute_cycle(addr, AccessType::NonSeq(AccessWidth::Word)) + 1;

    gpr[rd] = data as u32;

    // Add merged I + S cycle.
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_ldr3<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd10_8() as usize;
    let offset = dec.get_off8() as u32;
    let pc = gpr[PC];
    let addr = (pc & 0xFFFF_FFFC) + offset.wrapping_shl(2);
    let data = read_ldr_data(bus, addr);

    // 1N + 1I cycle
    let cycle = bus.compute_cycle(addr, AccessType::NonSeq(AccessWidth::Word)) + 1;

    gpr[rd] = data;

    // Add merged I + S cycle.
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
    (cycle, PipelineStatus::Continue)
}

// THUMB.11: load_sp_relative
pub fn exec_thumb_load_sp_relative<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{

    let rd = dec.get_Rd10_8() as usize;
    let offset = dec.get_off8() as u32;
    let addr = gpr[SP] + offset.wrapping_shl(2);
    let data = read_ldr_data(bus, addr);

    // 1N + 1I cycle
    let cycle = bus.compute_cycle(addr, AccessType::NonSeq(AccessWidth::Word)) + 1;

    gpr[rd] = data;

    // Add merged I + S cycle.
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb8_ldrh<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;

    let addr = gpr[rn].wrapping_add(gpr[rm]);
    // 原因: アドレスが奇数のとき、LDRH は32bitにゼロ拡張した後に8bit右ローテートした値を返す必要がある。
    let aligned = addr & !1;
    let raw = bus.read_halfword(aligned) as u32;
    let data32 = if (addr & 1) != 0 { raw.rotate_right(8) } else { raw };

    // 1N + 1I cycle
    let cycle = bus.compute_cycle(addr, AccessType::NonSeq(AccessWidth::Word)) + 1;

    gpr[rd] = data32;

    // Add merged I + S cycle.
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb8_ldsh<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;

    let addr = gpr[rn].wrapping_add(gpr[rm]);
    // 原因: アドレスが奇数のとき、LDRSH は符号付きバイトとして読み出し（LDRSB と同等）になる仕様。
    //       これまで常に半ワード読みを行っていたため、test 212 が失敗していた。
    let data: i32 = if (addr & 1) != 0 {
        bus.read_byte(addr) as i8 as i32
    } else {
        bus.read_halfword(addr) as i16 as i32
    };

    // 1N + 1I cycle
    let cycle = bus.compute_cycle(addr, AccessType::NonSeq(AccessWidth::Word)) + 1;

    gpr[rd] = data as u32;

    // Add merged I + S cycle.
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb8_ldsb<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;

    let addr = gpr[rn].wrapping_add(gpr[rm]);
    let data = bus.read_byte(addr) as i8;

    // 1N + 1I cycle
    let cycle = bus.compute_cycle(addr, AccessType::NonSeq(AccessWidth::Word)) + 1;

    gpr[rd] = data as u32;

    // Add merged I + S cycle.
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
    (cycle, PipelineStatus::Continue)
}

// THUMB.10
pub fn exec_thumb_ldrh<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let offset = dec.get_off5() as u32;

    let addr = gpr[rn] + offset.wrapping_shl(1);
    // Misaligned halfword: zero-extend to 32bit then ROR #8
    let aligned = addr & !1;
    let raw = bus.read_halfword(aligned) as u32;
    let data32 = if (addr & 1) != 0 { raw.rotate_right(8) } else { raw };

    // 1N + 1I cycle
    let cycle = bus.compute_cycle(addr, AccessType::NonSeq(AccessWidth::Word)) + 1;

    gpr[rd] = data32;

    // Add merged I + S cycle.
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
    // dbg!("ldrh", &gpr);
    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_str1<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let offset = dec.get_off5() as u32;

    let addr = gpr[rn] + offset.wrapping_shl(2);

    if addr == 0x0400_0006 {
        // dbg!("read 0x0400_0006", &gpr);
    }

    bus.write_word(addr, gpr[rd]);

    let access_type = AccessType::NonSeq(AccessWidth::Word);
    let store_cycle = bus.compute_cycle(addr, access_type);
    let access_type = AccessType::NonSeq(AccessWidth::HalfWord);
    let fetch_cycle = bus.compute_cycle(gpr[PC], access_type);
    // Store consume 2N cycle
    (store_cycle + fetch_cycle, PipelineStatus::Continue)
}

// THUMB.11: store_sp_relative
pub fn exec_thumb_str3<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd10_8() as usize;
    let offset = dec.get_off8() as u32;
    let data = gpr[rd];
    let addr = gpr[SP] + offset.wrapping_shl(2);

    bus.write_word(addr, data);
    let access_type = AccessType::NonSeq(AccessWidth::Word);
    let store_cycle = bus.compute_cycle(addr, access_type);
    let access_type = AccessType::NonSeq(AccessWidth::HalfWord);
    let fetch_cycle = bus.compute_cycle(gpr[PC], access_type);
    // dbg!("after str3", &gpr);
    // Store consume 2N cycle

    (store_cycle + fetch_cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_strh<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let offset = dec.get_off5() as u32;

    let addr = gpr[rn] + offset.wrapping_shl(1);
    bus.write_halfword(addr, gpr[rd] as u16);

    let access_type = AccessType::NonSeq(AccessWidth::HalfWord);
    let store_cycle = bus.compute_cycle(addr, access_type);
    let fetch_cycle = bus.compute_cycle(gpr[PC], access_type);
    // dbg!("strh");
    // Store consume 2N cycle
    (store_cycle + fetch_cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_ldrb_imm_offset<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let offset = dec.get_off5() as u32;

    let addr = gpr[rn] + offset;

    // 1N + 1I cycle
    let cycle = bus.compute_cycle(addr, AccessType::NonSeq(AccessWidth::Word)) + 1;

    gpr[rd] = bus.read_byte(addr) as u32;

    // Add merged I + S cycle.
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));

    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_strb_imm_offset<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let offset = dec.get_off5() as u32;

    let addr = gpr[rn] + offset;
    bus.write_byte(addr, gpr[rd] as u8);

    let access_type = AccessType::NonSeq(AccessWidth::Byte);
    let store_cycle = bus.compute_cycle(addr, access_type);
    let fetch_cycle = bus.compute_cycle(gpr[PC], access_type);
    // dbg!("strB");
    // Store consume 2N cycle
    (store_cycle + fetch_cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_strb_reg_offset<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;

    let addr = gpr[rn] + gpr[rm];
    bus.write_byte(addr, gpr[rd] as Byte);

    let access_type = AccessType::NonSeq(AccessWidth::Byte);
    let store_cycle = bus.compute_cycle(addr, access_type);
    let fetch_cycle = bus.compute_cycle(gpr[PC], access_type);
    // dbg!("strB");
    // Store consume 2N cycle
    (store_cycle + fetch_cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_strh_reg_offset<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;

    let addr = gpr[rn] + gpr[rm];
    bus.write_halfword(addr, gpr[rd] as HalfWord);

    let access_type = AccessType::NonSeq(AccessWidth::Byte);
    let store_cycle = bus.compute_cycle(addr, access_type);
    let fetch_cycle = bus.compute_cycle(gpr[PC], access_type);
    (store_cycle + fetch_cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_str_reg_offset<T>(bus: &mut T, dec: SingleDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rd = dec.get_Rd2_0() as usize;
    let rn = dec.get_Rn() as usize;
    let rm = dec.get_Rm() as usize;

    let addr = gpr[rn] + gpr[rm];
    bus.write_word(addr, gpr[rd] as Word);

    let access_type = AccessType::NonSeq(AccessWidth::Byte);
    let store_cycle = bus.compute_cycle(addr, access_type);
    let fetch_cycle = bus.compute_cycle(gpr[PC], access_type);
    (store_cycle + fetch_cycle, PipelineStatus::Continue)
}
