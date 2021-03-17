use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::{shift::*, ExecuteResult};
use crate::cpu::registers::psr::PSR;
use crate::types::*;

fn exec_memory_store<T, F>(bus: &mut T, gpr: &mut [u32; 16], dec: Memory, cpsr: &PSR, store: F) -> Result<ExecuteResult, ()>
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
        shift(sh, gpr[rm], shamt5, cpsr.get_C(), false)
    };
    let offset_base = if dec.get_U() {
        (base.wrapping_add(offset) & 0x0FFF_FFFF) as Word
    } else {
        (base.wrapping_sub(offset) & 0x0FFF_FFFF) as Word
    };
    if dec.get_P() {
        base = offset_base;
    }

    // dbg!("00000000");

    store(bus, gpr, base);
    //b// dbg!("00000000");
    let access_type = AccessType::NonSeq(AccessWidth::Word);
    let store_cycle = bus.compute_cycle(base, access_type);

    // dbg!("ppp!!!8787878");
    // dbg!(dec.get_P());

    if !dec.get_P() {
        // dbg!("ppp!!!", offset_base);
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
    // if gpr[15] == g {
    //     info!("");
    //     // panic!();
    // }
    let mut base = gpr[dec.get_Rn() as usize];
    // INFO: Treat as imm12 if not I.
    let offset = if !dec.get_I() {
        dec.get_imm()
    } else {
        let rm = dec.get_Rm() as usize;
        let sh = dec.get_sh().into();
        let shamt5 = dec.get_shamt5();
        shift(sh, gpr[rm], shamt5, cpsr.get_C(), false)
    };
    let offset_base = if dec.get_U() {
        (base.wrapping_add(offset) & 0x0FFF_FFFF) as Word
    } else {
        (base.wrapping_sub(offset) & 0x0FFF_FFFF) as Word
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
    // if gpr[15] >= 134222536 {
    //     dbg!("before LDR", &gpr);
    // }
    let rd = dec.get_Rd() as usize;
    let res = exec_memory_load(bus, gpr, dec, cpsr, |gpr, base| {
        let data = bus.read_word(base & 0xFFFF_FFFC);
        if gpr[15] == 134225876 {
            dbg!(base & 0xFFFF_FFFC, data);
        }
        // The loaded data is rotated right by one, two or three bytes according to bits [1:0] of the address.
        // https://www.keil.com/support/man/docs/armasm/armasm_dom1359731171041.htm
        let rotate = base & 0x03;
        gpr[rd] = data.rotate_right(rotate.wrapping_shl(3));
    });
    // debug!("after LDR {:?}", &gpr);
    // if gpr[15] >= 134222536 {
    //     dbg!("after LDR", &gpr);
    // }

    res
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
    dbg!("Before STR", &gpr);
    let res = exec_memory_store(bus, gpr, dec, cpsr, |bus, gpr, base| {
        dbg!(format!("{:X}", base));
        bus.write_word(base, gpr[rd]);
    });
    res
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
