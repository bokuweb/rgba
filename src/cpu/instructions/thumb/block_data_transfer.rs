use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::thumb::*;
use crate::cpu::instructions::*;
use crate::types::*;

pub fn exec_thumb_stmia<T>(bus: &mut T, dec: BlockDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let mut base = gpr[dec.get_Rn() as usize];
    let register_list = dec.get_register_list();

    let mut cycle: Cycle = 0;
    let mut is_n_cycle = true;

    for i in 0..0x8 {
        if register_list & (1 << i) != 0 {
            let access_type = if is_n_cycle {
                is_n_cycle = false;
                AccessType::NonSeq(AccessWidth::Word)
            } else {
                AccessType::Seq(AccessWidth::Word)
            };
            cycle += bus.compute_cycle(base, access_type);

            bus.write_word(base, gpr[i as usize] as Word);
            base = base.wrapping_add(4);
        }
    }
    gpr[dec.get_Rn() as usize] = base;
    // consume 1N cycle to prefetch next cycle
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::NonSeq(AccessWidth::HalfWord));
    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_ldmia<T>(bus: &T, dec: BlockDataTransfer, gpr: &mut [Word; 16], started: bool) -> ExecuteResult
where
    T: BusAccessor,
{
    if started {
        dbg!("before ldmia", &gpr);
    }
    let rn = dec.get_Rn();
    let mut base = gpr[rn as usize];
    let register_list = dec.get_register_list();

    let mut cycle: Cycle = 0;
    let mut is_n_cycle = true;

    for i in 0..0x8 {
        if register_list & (1 << i) != 0 {
            let d = bus.read_word(base & 0xFFFF_FFFC);
            // dbg!(base, d);
            gpr[i] = d;
            let access_type = if is_n_cycle {
                is_n_cycle = false;
                AccessType::NonSeq(AccessWidth::Word)
            } else {
                AccessType::Seq(AccessWidth::Word)
            };
            cycle += bus.compute_cycle(base, access_type);

            base = base.wrapping_add(4);
        }
    }
    if (1 << rn) & register_list == 0 {
        gpr[rn as usize] = base;
    }

    if started {
        dbg!("after ldmia", &gpr);
    }

    // Consume 1I cycle.
    let cycle = cycle + 1;
    // consume merged I-S cycle
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));

    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_push<T>(bus: &mut T, dec: BlockDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let mut addr = gpr[SP] - 4;
    let register_list = dec.get_register_list();

    let mut cycle: Cycle = 0;
    let mut is_n_cycle = true;

    if dec.get_R() {
        bus.write_word(addr, gpr[LR]);
        addr = addr - 4;
    }

    for i in 0..0x8 {
        let i = 7 - i;
        if register_list & (1 << i) != 0 {
            bus.write_word(addr & 0xFFFF_FFFC, gpr[i]);

            let access_type = if is_n_cycle {
                is_n_cycle = false;
                AccessType::NonSeq(AccessWidth::Word)
            } else {
                AccessType::Seq(AccessWidth::Word)
            };
            cycle += bus.compute_cycle(addr, access_type);

            addr -= 4;
        }
    }

    gpr[SP] = addr + 4;
    // Consume 1N cycle to next prefetch
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::NonSeq(AccessWidth::HalfWord));
    // dbg!(&gpr, "after push");
    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_pop<T>(bus: &mut T, dec: BlockDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let mut cycle: Cycle = 0;
    let mut is_n_cycle = true;

    let mut addr = gpr[SP];
    let register_list = dec.get_register_list();

    // dbg!("befpre pop", &gpr);

    for i in 0..0x8 {
        if register_list & (1 << i) != 0 {
            let data = bus.read_word(addr & 0xFFFF_FFFC);
            let access_type = if is_n_cycle {
                is_n_cycle = false;
                AccessType::NonSeq(AccessWidth::Word)
            } else {
                AccessType::Seq(AccessWidth::Word)
            };
            cycle += bus.compute_cycle(addr, access_type);

            gpr[i] = data;
            addr += 4;
        }
    }

    if dec.get_R() {
        let data = bus.read_word(addr);
        gpr[PC] = data & 0xFFFF_FFFE;
        addr += 4;
    }

    gpr[SP] = addr;
    // dbg!("after pop", &gpr);

    // Consume 1I cycle.
    let cycle = cycle + 1;

    if dec.get_R() {
        (cycle, PipelineStatus::Flush)
    } else {
        // consume merged I-S cycle
        let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
        (cycle, PipelineStatus::Continue)
    }
}
