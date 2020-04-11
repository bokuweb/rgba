use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::types::*;

fn exec_ex_memory_processing<F>(
    gpr: &mut [u32; 16],
    dec: Box<dyn Decoder>,
    load_or_store: F,
) -> Result<PipelineStatus, ()>
where
    F: FnOnce(&mut [u32; 16], u32),
{
    let mut base = gpr[dec.get_Rn()];
    let offset = if dec.has_I() {
        dec.get_imm8()
    } else {
        gpr[dec.get_Rm() as usize]
    };
    let offset_base = if dec.is_plus_offset() {
        (base + offset) as Word
    } else {
        (base - offset) as Word
    };
    if dec.is_pre_indexed() {
        base = offset_base;
    }
    load_or_store(gpr, base);
    if !dec.is_pre_indexed() {
        gpr[dec.get_Rn()] = offset_base;
    } else if dec.is_write_back() {
        gpr[dec.get_Rn()] = base;
    }
    if dec.get_Rd() == PC {
        Ok(PipelineStatus::Flush)
    } else {
        Ok(PipelineStatus::Continue)
    }
}

pub fn exec_strh<T>(
    bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    exec_ex_memory_processing(gpr, dec, |gpr, base| {
        bus.write_word(base, gpr[rd] & 0xFFFF);
    })
}

#[allow(non_snake_case)]
pub fn exec_ldrh<T>(
    bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    exec_ex_memory_processing(gpr, dec, |gpr, base| {
        gpr[rd] = bus.read_word(base) & 0xFFFF;
    })
}

#[allow(non_snake_case)]
pub fn exec_ldrsb<T>(
    bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    exec_ex_memory_processing(gpr, dec, |gpr, base| {
        gpr[rd] = bus.read_byte(base) as i8 as i32 as u32;
    })
}

#[allow(non_snake_case)]
pub fn exec_ldrsh<T>(
    bus: &mut T,
    dec: Box<dyn Decoder>,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rd = dec.get_Rd();
    exec_ex_memory_processing(gpr, dec, |gpr, base| {
        gpr[rd] = (bus.read_word(base) & 0xFFFF) as i16 as i32 as u32;
    })
}
