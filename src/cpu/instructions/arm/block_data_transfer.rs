use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::*;
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

pub fn exec_arm_ldm<T>(bus: &mut T, dec: BlockDataTransfer, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let mut base: i64 = gpr[dec.get_Rn() as usize] as i64;
    let mut cycle: Cycle = 0;
    let mut is_n_cycle = true;

    dbg!("before LDM", &gpr);
    let register_list = dec.get_register_list();
    let offset: i64 = if dec.get_U() { 4 } else { -4 };
    for i in 0..0x10 {
        let reg = if !dec.get_U() { 0x0F - i } else { i } as usize;
        if register_list & (1 << reg) != 0 {
            if dec.get_P() {
                base = base.wrapping_add(offset);
            }
            let addr = base as Word;
            let access_type = if is_n_cycle {
                is_n_cycle = false;
                AccessType::NonSeq(AccessWidth::Word)
            } else {
                AccessType::Seq(AccessWidth::Word)
            };
            cycle += bus.compute_cycle(addr, access_type);
            let data = bus.read_word(addr & 0xFFFF_FFFC);
            dbg!("LDM", addr, data);

            gpr[reg] = data;
            if !dec.get_P() {
                base = base.wrapping_add(offset);
            }
        }
    }
    if dec.get_W() && register_list & (1 << dec.get_Rn()) == 0 {
        gpr[dec.get_Rn() as usize] = base as u32;
    }

    if dec.get_S() {
        unimplemented!();
    }

    // Consume 1I cycle.
    let cycle = cycle + 1;

    dbg!("after LDM", &gpr);
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

    let register_list = dec.get_register_list();
    let offset: i64 = if dec.get_U() { 4 } else { -4 };

    for i in 0..0x10 {
        let reg = if !dec.get_U() { 0x0F - i } else { i } as usize;
        //    for i in 0..0x10 {
        if register_list & (1 << reg) != 0 {
            if dec.get_P() {
                base = base.wrapping_add(offset);
            }
            let addr = base as Word;
            let access_type = if is_n_cycle {
                is_n_cycle = false;
                AccessType::NonSeq(AccessWidth::Word)
            } else {
                AccessType::Seq(AccessWidth::Word)
            };
            cycle += bus.compute_cycle(addr, access_type);
            bus.write_word(addr, gpr[reg] as Word);
            if !dec.get_P() {
                base = base.wrapping_add(offset);
            }
        }
    }
    if dec.get_W() {
        gpr[dec.get_Rn() as usize] = base as u32;
    }

    if dec.get_S() {
        unimplemented!();
    }

    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::NonSeq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}
