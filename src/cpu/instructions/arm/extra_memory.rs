use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::ExecuteResult;
use crate::types::*;

fn exec_ex_memory_load<F, T>(bus: &T, gpr: &mut [u32; 16], dec: ExtraMemory, load: F) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
    F: FnOnce(&mut [u32; 16], u32),
{
    let mut base = gpr[dec.get_Rn() as usize];

    let offset = if dec.get_I() { dec.get_imm8() } else { gpr[dec.get_Rm() as usize] };
    let offset_base = if dec.get_U() {
        base.wrapping_add(offset)
    } else {
        base.wrapping_sub(offset)
    };
    if dec.get_P() {
        base = offset_base;
    }
    if !dec.get_P() {
        gpr[dec.get_Rn() as usize] = offset_base;
    } else if dec.get_W() {
        gpr[dec.get_Rn() as usize] = base;
    }

    load(gpr, base);
    let access_type = AccessType::NonSeq(AccessWidth::Word);
    let load_cycle = bus.compute_cycle(base, access_type);

    // load cycle + 1I cycle
    let cycle = load_cycle + 1;

    if dec.get_Rd() as usize == PC {
        Ok((cycle, PipelineStatus::Flush))
    } else {
        // Next prefetch cycle is merged  I-S cycle.
        // See ARM DDI 0084F 6-14
        let access_type = AccessType::Seq(AccessWidth::Word);
        let cycle = cycle + bus.compute_cycle(gpr[PC], access_type);
        Ok((cycle, PipelineStatus::Continue))
    }
}

pub fn exec_arm_strh<T>(bus: &mut T, dec: ExtraMemory, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let access_type = AccessType::NonSeq(AccessWidth::Word);

    let mut base = gpr[dec.get_Rn() as usize];
    let offset = if dec.get_I() { dec.get_imm8() } else { gpr[dec.get_Rm() as usize] };
    let offset_base = if dec.get_U() {
        base.wrapping_add(offset)
    } else {
        base.wrapping_sub(offset)
    };
    if dec.get_P() {
        base = offset_base;
    }
    // t407: Aligned store halfword
    //  - STRH は未アラインドアドレス時に bit0 を無視し、2バイト境界へ丸めて格納する（ARM7TDMI仕様）。
    //  - gba-tests/arm/halfword_transfer.asm t407 がこの挙動を検証。
    //  - 16bit 幅で書き込む必要があるため、write_halfword を使用する。
    let eff_addr = base & 0xFFFF_FFFE;
    bus.write_halfword(eff_addr, (gpr[rd] & 0xFFFF) as HalfWord);
    let store_cycle = bus.compute_cycle(base, access_type);
    if !dec.get_P() {
        gpr[dec.get_Rn() as usize] = offset_base;
    } else if dec.get_W() {
        gpr[dec.get_Rn() as usize] = base;
    }
    let fetch_cycle = bus.compute_cycle(gpr[PC], access_type);
    // Store consume 2N cycle
    Ok((store_cycle + fetch_cycle, PipelineStatus::Continue))
}

#[allow(non_snake_case)]
pub fn exec_arm_ldrh<T>(bus: &T, dec: ExtraMemory, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_ex_memory_load(bus, gpr, dec, |gpr, base| {
        // t408: Misaligned load halfword (rotated)
        //  - LDRH が奇数アドレスの場合、読み出し値は 32bit として 8bit ROR されたものになる（ARM7TDMI仕様）。
        //  - 参照: fixtures/gba-tests/arm/halfword_transfer.asm t408
        if (base & 1) != 0 {
            let raw = bus.read_halfword(base & 0xFFFF_FFFE) as u32;
            gpr[rd] = raw.rotate_right(8);
        } else {
            gpr[rd] = bus.read_halfword(base) as u32;
        }
    })
}

#[allow(non_snake_case)]
pub fn exec_arm_ldrsb<T>(bus: &T, dec: ExtraMemory, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_ex_memory_load(bus, gpr, dec, |gpr, base| {
        gpr[rd] = bus.read_byte(base) as i8 as i32 as u32;
    })
}

#[allow(non_snake_case)]
pub fn exec_arm_ldrsh<T>(bus: &T, dec: ExtraMemory, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_ex_memory_load(bus, gpr, dec, |gpr, base| {
        // Misaligned signed halfword load semantics (used by t409): rotate by 8 if odd, then sign-extend
        if (base & 1) != 0 {
            let raw = bus.read_halfword(base & 0xFFFF_FFFE);
            let rotated = (((raw as u32) >> 8) | (((raw as u32) & 0xFF) << 8)) & 0xFFFF;
            gpr[rd] = (rotated as i16 as i32) as u32;
        } else {
            let raw = bus.read_halfword(base) as u32;
            gpr[rd] = (raw as i16 as i32) as u32;
        }
    })
}
