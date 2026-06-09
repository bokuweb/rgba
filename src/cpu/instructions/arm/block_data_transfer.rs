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
    let mut cycle: Cycle = 0;
    let mut is_n_cycle = true;
    let register_list = dec.get_register_list();
    let mut immediate = 0;
    let mut offset = 0;
    if dec.get_U() {
        if dec.get_P() {
            immediate = 4;
        }
        for i in 0..0x10 {
            let m = 0x01 << i;
            if register_list & m != 0 {
                // t516: LDM writeback base first must still load Rn; do not drop Rn from rlist
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
                // t516: LDM writeback base first must still load Rn; do not drop Rn from rlist
                immediate -= 4;
                offset -= 4;
            }
        }
    }

    let base: i64 = gpr[dec.get_Rn() as usize] as i64;
    let mut address = base.wrapping_add(immediate) as Word;
    let current_mode = cpsr.get_mode();

    // t513: Load empty rlist
    //  - LDM* with an empty register list loads PC from a single address determined by addressing mode
    //    and performs writeback by +/- 0x40 bytes (as if 16 registers were transferred).
    //  - IA: load from [Rn],    Rn += 0x40
    //    IB: load from [Rn+4],  Rn += 0x40
    //    DA: load from [Rn-0x3C], Rn -= 0x40
    //    DB: load from [Rn-0x40], Rn -= 0x40
    //  - After loading PC, the pipeline must be flushed.
    if register_list == 0 {
        let rn_val = gpr[dec.get_Rn() as usize];
        let (pc_addr, wb): (Word, i64) = match (dec.get_U(), dec.get_P()) {
            (true, false) => (rn_val, 0x40),                      // IA
            (true, true) => (rn_val.wrapping_add(4), 0x40),       // IB
            (false, false) => (rn_val.wrapping_sub(0x3C), -0x40), // DA
            (false, true) => (rn_val.wrapping_sub(0x40), -0x40),  // DB
        };

        let access_type = if is_n_cycle {
            AccessType::NonSeq(AccessWidth::Word)
        } else {
            AccessType::Seq(AccessWidth::Word)
        };
        cycle += bus.compute_cycle(pc_addr, access_type);
        let data = bus.read_word(pc_addr & 0xFFFF_FFFC);
        gpr[PC] = data;

        if dec.get_W() {
            gpr[dec.get_Rn() as usize] = (rn_val as i64 + wb) as u32;
        }

        // Consume 1I cycle and flush pipeline due to PC load
        let cycle = cycle + 1;
        return Ok((cycle, PipelineStatus::Flush));
    }

    if dec.get_W() {
        let v = gpr[dec.get_Rn() as usize] as i64 + offset as i64;
        gpr[dec.get_Rn() as usize] = v as u32;
    }
    // The lowest Register in Rlist (R0 if its in the list) will be loaded/stored to/from the lowest memory address.
    // Internally, the rlist register are always processed with INCREASING addresses
    // (ie. for DECREASING addressing modes, the CPU does first calculate the lowest address,
    // and does then process rlist with increasing addresses; this detail can be important when accessing memory mapped I/O ports).

    // LDM^: in a privileged mode with S=1, L=1, and R15 absent, the targets are the User/System bank.
    // Temporarily switch to User mode for the load, then restore the original mode (follows tgba's impl).
    let do_user_load = dec.get_S() && current_mode != Mode::User;
    if do_user_load {
        cpsr.switch_mode(Mode::User, gpr, spsr, bank_gpr, bank_spsr);
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

    if do_user_load {
        // Snapshot right after the load into the User bank (FIQ: r8-r14, other privileged: r13-r14)
        let mut user_snapshot: [Option<u32>; 16] = [None; 16];
        for i in 0..16 {
            let is_banked_in_fiq = (8..=14).contains(&i);
            let is_banked_nonfiq = i == SP || i == LR;
            if is_banked_in_fiq || is_banked_nonfiq {
                user_snapshot[i] = Some(gpr[i]);
            }
        }

        // Restore the current mode (from here on, the visible values are the current mode's bank)
        cpsr.switch_mode(current_mode, gpr, spsr, bank_gpr, bank_spsr);

        // For one instruction only, overlay user|curr on the target registers
        let mut mask: u16 = 0;
        let mut overlays: [(usize, u32); 16] = [(0, 0); 16];
        let mut overlay_count: usize = 0;
        let is_fiq = matches!(current_mode, Mode::FIQ);
        for i in 0..16 {
            // Is this register banked?
            let is_banked_here = if is_fiq { (8..=14).contains(&i) } else { i == SP || i == LR };
            if !is_banked_here {
                continue;
            }
            // Is it in this LDM^'s register list?
            if (register_list & (1 << i)) == 0 {
                continue;
            }
            if let Some(user_val) = user_snapshot[i] {
                let cur_val = gpr[i];
                let overlay = user_val | cur_val;
                mask |= 1 << i;
                overlays[overlay_count] = (i, overlay);
                overlay_count += 1;
            }
        }
        if mask != 0 {
            bank_gpr.glitch_arm(mask, &overlays[..overlay_count], gpr);
        }
    }

    // Consume 1I cycle.
    let cycle = cycle + 1;

    // If PC is loaded
    if register_list & 0x8000 != 0 {
        Ok((cycle, PipelineStatus::Flush))
    } else {
        // consume merged I-S cycle
        let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
        Ok((cycle, PipelineStatus::Continue))
    }
}

pub fn exec_arm_stm<T>(
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
    let _base: i64 = gpr[dec.get_Rn() as usize] as i64;
    let mut cycle: Cycle = 0;
    let mut is_n_cycle = true;

    // t511, t512, etc.: S-bit handling (user register access)
    //  - For STM/LDM with S=1 in a privileged mode, the transferred registers reference the User-mode bank.
    //  - Here the base calculation/writeback before the store is done in the current mode; just before the store
    //    we temporarily switch to System(=User) to fetch the values,
    //    then restore the original mode afterward.

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

    // t515: Store empty rlist (STM* with {}): store PC+4 and writeback +/-0x40
    //  - IA (U=1,P=0): store [Rn],        Rn += 0x40
    //    IB (U=1,P=1): store [Rn+4],      Rn += 0x40
    //    DA (U=0,P=0): store [Rn-0x3C],   Rn -= 0x40
    //    DB (U=0,P=1): store [Rn-0x40],   Rn -= 0x40
    //  - The stored PC value is PC+4 (here gpr[PC] is +8, so +4 corresponds to effective PC+12).
    //  - Ref: fixtures/gba-tests/arm/block_transfer.asm t515
    if register_list == 0 {
        let rn_val = gpr[dec.get_Rn() as usize];
        let (store_addr, wb): (Word, i64) = match (dec.get_U(), dec.get_P()) {
            (true, false) => (rn_val, 0x40),                      // IA
            (true, true) => (rn_val.wrapping_add(4), 0x40),       // IB
            (false, false) => (rn_val.wrapping_sub(0x3C), -0x40), // DA
            (false, true) => (rn_val.wrapping_sub(0x40), -0x40),  // DB
        };

        let access_type = if is_n_cycle {
            AccessType::NonSeq(AccessWidth::Word)
        } else {
            AccessType::Seq(AccessWidth::Word)
        };
        cycle += bus.compute_cycle(store_addr, access_type);
        let value = gpr[PC].wrapping_add(4);
        bus.write_word(store_addr & 0xFFFF_FFFC, value as Word);
        if dec.get_W() {
            gpr[dec.get_Rn() as usize] = (rn_val as i64 + wb) as u32;
        }
        let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::NonSeq(AccessWidth::Word));
        return Ok((cycle, PipelineStatus::Continue));
    }

    if dec.get_W() {
        let v = gpr[dec.get_Rn() as usize] as i64 + offset as i64;
        if overwrap {
            bus.write_word((gpr[dec.get_Rn() as usize] as i64 + immediate - 4) as Word, gpr[dec.get_Rn() as usize] as Word);
        }
        gpr[dec.get_Rn() as usize] = v as u32;
    }

    let current_mode = cpsr.get_mode();
    if dec.get_S() {
        cpsr.switch_mode(Mode::System, gpr, spsr, bank_gpr, bank_spsr);
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
            // t508: Memory alignment (block transfer)
            //  - On ARM7TDMI a word transfer ignores bits[1:0] when unaligned and writes to the 4-byte boundary.
            //  - The ldm path already aligns reads with &0xFFFF_FFFC, and stm must align the same way.
            //  - Ref: fixtures/gba-tests/arm/block_transfer.asm t508
            // t510: Store PC + 4 in block store
            //  - An STM whose register list includes PC stores PC+4 (here gpr[PC] is always +8, so +4 means effective PC+12).
            //  - Ref: fixtures/gba-tests/arm/block_transfer.asm t510
            let value = if i == PC { gpr[PC].wrapping_add(4) } else { gpr[i] };
            bus.write_word(address & 0xFFFF_FFFC, value as Word);
            address = address.wrapping_add(4);
        }
    }

    if dec.get_S() {
        // Restore the original mode
        cpsr.switch_mode(current_mode, gpr, spsr, bank_gpr, bank_spsr);
    }

    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::NonSeq(AccessWidth::Word));
    Ok((cycle, PipelineStatus::Continue))
}
