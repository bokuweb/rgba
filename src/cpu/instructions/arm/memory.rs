use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::{shift::*, ExecuteResult};
use crate::types::*;

fn exec_memory_store<T, F>(bus: &mut T, gpr: &mut [u32; 16], dec: Memory, store: F) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
    F: FnOnce(&mut T, &mut [u32; 16], u32),
{
    let mut base = gpr[dec.get_Rn() as usize];
    // INFO: Treat as imm12 if not I.
    let offset = if !dec.get_I() {
        dec.get_imm()
    } else {
        let rm = dec.get_Rm() as usize;
        let sh = dec.get_sh().into();
        let shamt5 = dec.get_shamt5();
        shift(sh, gpr[rm], shamt5)
    };
    let offset_base = if dec.get_U() {
        (base + offset) as Word
    } else {
        (base - offset) as Word
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

fn exec_memory_load<T, F>(bus: &T, gpr: &mut [u32; 16], dec: Memory, load: F) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
    F: FnOnce(&mut [u32; 16], u32),
{
    let mut base = gpr[dec.get_Rn() as usize];
    // INFO: Treat as imm12 if not I.
    let offset = if !dec.get_I() {
        dec.get_imm()
    } else {
        let rm = dec.get_Rm() as usize;
        let sh = dec.get_sh().into();
        let shamt5 = dec.get_shamt5();
        shift(sh, gpr[rm], shamt5)
    };
    let offset_base = if dec.get_U() {
        (base + offset) as Word
    } else {
        (base - offset) as Word
    };
    if dec.get_P() {
        base = offset_base;
    }

    load(gpr, base);

    // 1N + 1I cycle
    let cycle = bus.compute_cycle(base, AccessType::NonSeq(AccessWidth::Word)) + 1;

    if !dec.get_P() {
        gpr[dec.get_Rn() as usize] = offset_base;
    } else if dec.get_W() {
        gpr[dec.get_Rn() as usize] = base;
    }
    if dec.get_Rd() as usize == PC && dec.get_L() {
        Ok((cycle, PipelineStatus::Flush))
    } else {
        // Add merged I + S cycle.
        let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
        Ok((cycle, PipelineStatus::Continue))
    }
}

#[allow(non_snake_case)]
pub fn exec_arm_ldr<T>(bus: &mut T, dec: Memory, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    let res = exec_memory_load(bus, gpr, dec, |gpr, base| {
        gpr[rd] = bus.read_word(base);
    });
    dbg!("LDR", gpr);
    res
}

#[allow(non_snake_case)]
pub fn exec_arm_ldrb<T>(bus: &mut T, dec: Memory, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_memory_load(bus, gpr, dec, |gpr, base| {
        gpr[rd] = bus.read_byte(base) as Word;
    })
}

pub fn exec_arm_str<T>(bus: &mut T, dec: Memory, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_memory_store(bus, gpr, dec, |bus, gpr, base| {
        bus.write_word(base, gpr[rd]);
    })
}

pub fn exec_arm_strb<T>(bus: &mut T, dec: Memory, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_memory_store(bus, gpr, dec, |bus, gpr, base| {
        bus.write_byte(base, gpr[rd] as Byte);
    })
}
