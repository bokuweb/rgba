use super::super::PipelineStatus;

use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::{shift::*, ExecuteResult};
use crate::cpu::registers::psr::PSR;
use crate::cpu::{bus::accessor::*, instructions::helpers::read_ldr_data};
use crate::types::*;

fn exec_memory_store<T, F>(bus: &mut T, gpr: &mut [u32; 16], dec: Memory, cpsr: &PSR, store: F) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
    F: FnOnce(&mut T, &mut [u32; 16], u32),
{
    let mut base = gpr[dec.get_Rn() as usize];
    // INFO: Treat as imm12 if not I.
    let offset = if dec.get_I() {
        let rm = dec.get_Rm() as usize;
        let sh = dec.get_sh().into();
        let shamt5 = dec.get_shamt5();
        // t362 対応: 特殊シフト RRX
        //  - ROR #0 は RRX として解釈され、CPSR.C をビット31に入れて右1シフトする。
        //  - テストでは C=1, Rm=0 により RRX(0) = 0x8000_0000 となることを期待。
        //  - ここで shift(...) を使って RRX を含むシフトを評価し、オフセットに反映する。
        shift(sh, gpr[rm], shamt5, cpsr.get_C(), false)
    } else {
        dec.get_imm()
    };
    // t362 の目的は RRX により 0x8000_0000 を生成し、プリインデックス/書き戻しで Rn に反映させること。
    // 以前の 28bit マスクは外し、wrap演算の結果をそのまま使う（GBAのアドレスラップはバス側で処理）。
    let offset_base = if dec.get_U() {
        base.wrapping_add(offset)
    } else {
        base.wrapping_sub(offset)
    };
    if dec.get_P() {
        base = offset_base;
    }

    store(bus, gpr, base);
    let access_type = AccessType::NonSeq(AccessWidth::Word);
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

fn exec_memory_load<T, F>(bus: &T, gpr: &mut [u32; 16], dec: Memory, cpsr: &PSR, load: F) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
    F: FnOnce(&mut [u32; 16], u32),
{
    let mut base = gpr[dec.get_Rn() as usize];
    // INFO: Treat as imm12 if not I.
    let offset = if dec.get_I() {
        let rm = dec.get_Rm() as usize;
        let sh = dec.get_sh().into();
        let shamt5 = dec.get_shamt5();
        shift(sh, gpr[rm], shamt5, cpsr.get_C(), false)
    } else {
        dec.get_imm()
    };
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

    // 1N + 1I cycle
    let cycle = bus.compute_cycle(base, AccessType::NonSeq(AccessWidth::Word)) + 1;

    if dec.get_Rd() as usize == PC && dec.get_L() {
        Ok((cycle, PipelineStatus::Flush))
    } else {
        // Add merged I + S cycle.
        let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
        Ok((cycle, PipelineStatus::Continue))
    }
}

#[allow(non_snake_case)]
pub fn exec_arm_ldr<T>(bus: &mut T, dec: Memory, gpr: &mut [Word; 16], cpsr: &PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    
    exec_memory_load(bus, gpr, dec, cpsr, |gpr, base| {
        let data = read_ldr_data(bus, base);
        gpr[rd] = data;
    })
}

#[allow(non_snake_case)]
pub fn exec_arm_ldrb<T>(bus: &mut T, dec: Memory, gpr: &mut [Word; 16], cpsr: &PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_memory_load(bus, gpr, dec, cpsr, |gpr, base| {
        gpr[rd] = bus.read_byte(base) as Word;
    })
}

pub fn exec_arm_str<T>(bus: &mut T, dec: Memory, gpr: &mut [Word; 16], cpsr: &PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    
    exec_memory_store(bus, gpr, dec, cpsr, |bus, gpr, base| {
        // t354: Misaligned store
        //  - ARMv4 (ARM7TDMI) 仕様では、ワードストアが未アラインドアドレスに対して行われた場合、
        //    アドレスの下位2bitは無視され、4バイト境界にアラインされたアドレスへ書き込む。
        //  - gba-tests/arm/single_transfer.asm の t354（"ARM 7: Misaligned store"）がこの挙動を検証。
        //    本実装はその要件に合わせ、STR の実行側で 4 バイト境界へ丸める。
        let eff_addr = base & 0xFFFF_FFFC;
        // t356: Store PC + 4
        //  - Rd==PC の STR は「PC+4」を格納する仕様。
        //    本エミュレータでは gpr[PC] は常に「現在命令アドレス+8」を表すため、実装上は gpr[PC]+4 (= 実効PC+12) を書き込む。
        //  - 参照: fixtures/gba-tests/arm/single_transfer.asm の t356 ("ARM 7: Store PC + 4")
        let value = if rd == PC { gpr[PC].wrapping_add(4) } else { gpr[rd] };
        bus.write_word(eff_addr, value);
    })
}

pub fn exec_arm_strb<T>(bus: &mut T, dec: Memory, gpr: &mut [Word; 16], cpsr: &PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_memory_store(bus, gpr, dec, cpsr, |bus, gpr, base| {
        bus.write_byte(base, gpr[rd] as Byte);
    })
}
