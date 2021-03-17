use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::*;
use crate::cpu::registers::{
    psr::{Mode, PSR},
    BankGpr, BankSpsr,
};
use crate::types::*;

// 31    28 27  25 24  23  22  21  20 19    16 15                      0
// ---------------------------------------------------------------------
// | cond | 1 0 0 | P | U | S | W | L |  Rn  |      Register LIst      |
// ---------------------------------------------------------------------
// P = 0: Post index 1: Pre index
// U = 0: Decrement 1: Increment
// S = Restore force user bit. S specifies if banked register access should occur when in privileged modes [or if R15 and 26 bit and user mode, if the PSR should be written while PC is updated]
// W = 1: Auto Index
// L = 0: Store / 1: Load
pub fn exec_arm_ldm<T>(
    bus: &mut T,
    dec: BlockDataTransfer,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
    spsr: &mut PSR,
    bank_gpr: &mut BankGpr,
    bank_spsr: &mut BankSpsr,
) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    if 134219024 <= gpr[15] && 134218980 >= gpr[15] {
        dbg!("Before ldm", &gpr);
    }
    let mut cycle: Cycle = 0;
    let mut is_n_cycle = true;
    // let mut is_first_entry = true;
    // let mut is_rn_skipped = false;

    // dbg!("before LDM", &gpr);
    let mut register_list = dec.get_register_list();
    // dbg!(register_list);

    // let mut address = 0;
    let mut immediate = 0;
    let mut offset = 0;
    if dec.get_U() {
        if dec.get_P() {
            immediate = 4;
        }
        // for let m = 0x01, i = 0; i < 16; m <<= 1, ++i) {
        for i in 0..0x10 {
            let m = 0x01 << i;
            if register_list & m != 0 {
                if dec.get_W() && i == dec.get_Rn() && offset == 0 {
                    register_list &= !m;
                    immediate += 4;
                }
                offset += 4;
            }
        }
    } else {
        if !dec.get_P() {
            immediate = 4;
        }
        for i in 0..0x10 {
            let m = 0x01 << i;
            if register_list & m != 0 {
                if dec.get_W() && i == dec.get_Rn() && offset == 0 {
                    register_list &= !m;
                    immediate += 4;
                }
                immediate -= 4;
                offset -= 4;
            }
        }
    }

    // dbg!(register_list);

    let base: i64 = gpr[dec.get_Rn() as usize] as i64;
    let mut address = base.wrapping_add(immediate) as Word;
    let current_mode = cpsr.get_mode();

    if dec.get_W() {
        let v = gpr[dec.get_Rn() as usize] as i64 + offset as i64;
        gpr[dec.get_Rn() as usize] = v as u32;
    }
    // The lowest Register in Rlist (R0 if its in the list) will be loaded/stored to/from the lowest memory address.
    // Internally, the rlist register are always processed with INCREASING addresses
    // (ie. for DECREASING addressing modes, the CPU does first calculate the lowest address,
    // and does then process rlist with increasing addresses; this detail can be important when accessing memory mapped I/O ports).

    if dec.get_S() {
        cpsr.switch_mode(Mode::System, gpr, spsr, bank_gpr, bank_spsr);
    }

    // let offset: i64 = if dec.get_U() { 4 } else { -4 };
    for i in 0..0x10 {
        // let reg = if !dec.get_U() { 0x0F - i } else { i } as usize;
        if register_list & (1 << i) != 0 {
            //if dec.get_P() {
            //    base = base.wrapping_add(offset);
            //}
            //let addr = base as Word;
            //if !(dec.get_W() && (i == dec.get_Rn() as usize) && is_first_entry) {
            let access_type = if is_n_cycle {
                is_n_cycle = false;
                AccessType::NonSeq(AccessWidth::Word)
            } else {
                AccessType::Seq(AccessWidth::Word)
            };
            cycle += bus.compute_cycle(address, access_type);
            let data = bus.read_word(address & 0xFFFF_FFFC);
            // dbg!("LDM", address, data);
            //
            gpr[i] = data;
            address = address.wrapping_add(4); //} else {
                                               //    is_rn_skipped = true;
                                               //}
                                               //
                                               //is_first_entry = false;
                                               //
                                               //if !dec.get_P() {
                                               //    base = base.wrapping_add(offset);
                                               //}
        }
    }

    if dec.get_S() {
        cpsr.switch_mode(current_mode, gpr, spsr, bank_gpr, bank_spsr);
    }

    // Consume 1I cycle.
    let cycle = cycle + 1;

    if 134219024 <= gpr[15] && 134218980 >= gpr[15] {
        dbg!("After ldm", &gpr);
    }

    // dbg!("after LDM", &gpr);
    // If PC is loaded
    if register_list & 0x8000 != 0 {
        Ok((cycle, PipelineStatus::Flush))
    } else {
        // consume merged I-S cycle
        let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
        Ok((cycle, PipelineStatus::Continue))
    }
}

pub fn exec_arm_stm<T>(bus: &mut T, dec: BlockDataTransfer, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    dbg!("before STM", &gpr);
    let mut base: i64 = gpr[dec.get_Rn() as usize] as i64;
    let mut cycle: Cycle = 0;
    let mut is_n_cycle = true;
    let mut is_first_entry = true;
    let mut is_rn_skipped = false;

    if dec.get_S() {
        unimplemented!();
    }

    let mut register_list = dec.get_register_list();
    let mut immediate = 0;
    let mut offset = 0;
    let mut overwrap = false;
    if dec.get_U() {
        if dec.get_P() {
            immediate = 4;
        }
        // for let m = 0x01, i = 0; i < 16; m <<= 1, ++i) {
        for i in 0..0x10 {
            let m = 0x01 << i;
            if register_list & m != 0 {
                if dec.get_W() && i == dec.get_Rn() && offset == 0 {
                    register_list &= !m;
                    immediate += 4;
                    overwrap = true;
                }
                offset += 4;
            }
        }
    } else {
        if !dec.get_P() {
            immediate = 4;
        }
        for i in 0..0x10 {
            let m = 0x01 << i;
            if register_list & m != 0 {
                if dec.get_W() && i == dec.get_Rn() && offset == 0 {
                    register_list &= !m;
                    immediate += 4;
                    overwrap = true;
                }
                immediate -= 4;
                offset -= 4;
            }
        }
    }

    let base: i64 = gpr[dec.get_Rn() as usize] as i64;
    let mut address = base.wrapping_add(immediate) as Word;
    // let current_mode = cpsr.get_mode();

    if dec.get_W() {
        let v = gpr[dec.get_Rn() as usize] as i64 + offset as i64;
        dbg!(overwrap);
        if overwrap {
            dbg!((gpr[dec.get_Rn() as usize] as i64 + immediate - 4), gpr[dec.get_Rn() as usize], immediate);
            bus.write_word(
                (gpr[dec.get_Rn() as usize] as i64 + immediate - 4) as Word,
                gpr[dec.get_Rn() as usize] as Word,
            );
        }
        gpr[dec.get_Rn() as usize] = v as u32;
    }

    for i in 0..0x10 {
        // let reg = if !dec.get_U() { 0x0F - i } else { i } as usize;
        //    for i in 0..0x10 {
        if register_list & (1 << i) != 0 {
            let access_type = if is_n_cycle {
                is_n_cycle = false;
                AccessType::NonSeq(AccessWidth::Word)
            } else {
                AccessType::Seq(AccessWidth::Word)
            };
            cycle += bus.compute_cycle(address, access_type);
            bus.write_word(address, gpr[i] as Word);
            address = address.wrapping_add(4);
        }
    }

    if dec.get_S() {
        unimplemented!();
    }

    dbg!("after STM", &gpr);

    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::NonSeq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}
