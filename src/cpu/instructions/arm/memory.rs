use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::shift::*;
use crate::types::*;

fn exec_memory_processing<F>(gpr: &mut [u32; 16], dec: Memory, load_or_store: F) -> Result<PipelineStatus, ()>
where
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
    load_or_store(gpr, base);
    if !dec.get_P() {
        gpr[dec.get_Rn() as usize] = offset_base;
    } else if dec.get_W() {
        gpr[dec.get_Rn() as usize] = base;
    }
    if dec.get_Rd() as usize == PC && dec.get_L() {
        Ok(PipelineStatus::Flush)
    } else {
        Ok(PipelineStatus::Continue)
    }
}

#[allow(non_snake_case)]
pub fn exec_ldr<T>(bus: &mut T, dec: Memory, gpr: &mut [Word; 16]) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_memory_processing(gpr, dec, |gpr, base| {
        gpr[rd] = bus.read_word(base);
    })
}

#[allow(non_snake_case)]
pub fn exec_ldrb<T>(bus: &mut T, dec: Memory, gpr: &mut [Word; 16]) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_memory_processing(gpr, dec, |gpr, base| {
        gpr[rd] = bus.read_byte(base) as Word;
    })
}

pub fn exec_str<T>(bus: &mut T, dec: Memory, gpr: &mut [Word; 16]) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_memory_processing(gpr, dec, |gpr, base| {
        bus.write_word(base, gpr[rd]);
    })
}

pub fn exec_strb<T>(bus: &mut T, dec: Memory, gpr: &mut [Word; 16]) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd() as usize;
    exec_memory_processing(gpr, dec, |gpr, base| {
        bus.write_byte(base, gpr[rd] as Byte);
    })
}
