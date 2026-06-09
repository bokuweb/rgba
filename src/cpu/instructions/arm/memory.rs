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
        // t362 support: special shift RRX
        //  - ROR #0 is interpreted as RRX, which feeds CPSR.C into bit 31 and shifts right by 1.
        //  - The test expects RRX(0) = 0x8000_0000 with C=1, Rm=0.
        //  - Use shift(...) here to evaluate the shift (including RRX) and apply it to the offset.
        shift(sh, gpr[rm], shamt5, cpsr.get_C(), false)
    } else {
        dec.get_imm()
    };
    // t362 aims to generate 0x8000_0000 via RRX and reflect it in Rn through pre-indexing/writeback.
    // The old 28-bit mask is removed; use the wrapping-arithmetic result directly (GBA address wrap is handled on the bus side).
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
        //  - Per the ARMv4 (ARM7TDMI) spec, when a word store targets an unaligned address,
        //    the low 2 bits of the address are ignored and the write goes to the 4-byte-aligned address.
        //  - gba-tests/arm/single_transfer.asm t354 ("ARM 7: Misaligned store") verifies this behavior.
        //    This impl matches that requirement by rounding to the 4-byte boundary in the STR execution path.
        let eff_addr = base & 0xFFFF_FFFC;
        // t356: Store PC + 4
        //  - An STR with Rd==PC stores "PC+4" by spec.
        //    Here gpr[PC] always represents "current instruction address + 8", so we write gpr[PC]+4 (= effective PC+12).
        //  - Ref: fixtures/gba-tests/arm/single_transfer.asm t356 ("ARM 7: Store PC + 4")
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
