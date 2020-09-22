use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::thumb::*;
use crate::types::*;

pub fn exec_thumb_stmia<T>(bus: &mut T, dec: BlockDataTransfer, gpr: &mut [Word; 16]) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let mut base = gpr[dec.get_Rn() as usize];
    let register_list = dec.get_register_list();
    for i in 0..0x8 {
        if register_list & (1 << i) != 0 {
            bus.write_word(base, gpr[i as usize] as Word);
            base = base.wrapping_add(4);
        }
    }
    gpr[dec.get_Rn() as usize] = base;
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_ldmia<T>(bus: &T, dec: BlockDataTransfer, gpr: &mut [Word; 16]) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn();
    let mut base = gpr[rn as usize];
    let register_list = dec.get_register_list();
    for i in 0..0x8 {
        if register_list & (1 << i) != 0 {
            let d = bus.read_word(base);
            gpr[i] = d;
            base = base.wrapping_add(4);
        }
    }
    if (1 << rn) & register_list == 0 {
        gpr[rn as usize] = base;
    }
    dbg!("LDMIA", &gpr);
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_push<T>(bus: &mut T, dec: BlockDataTransfer, gpr: &mut [Word; 16]) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let mut addr = gpr[SP] - 4;
    let register_list = dec.get_register_list();
    dbg!("push", register_list);

    // TODO: wait
    if dec.get_R() {
        bus.write_word(addr, gpr[LR]);
        addr = addr - 4;
    }

    for i in 0..0x8 {
        let i = 7 - i;
        if register_list & (1 << i) != 0 {
            bus.write_word(addr, gpr[i]);
            addr -= 4;
        }
    }
    // TODO: wait
    gpr[SP] = addr + 4;
    dbg!(&gpr);
    Ok(PipelineStatus::Continue)
}

pub fn exec_thumb_pop<T>(bus: &mut T, dec: BlockDataTransfer, gpr: &mut [Word; 16]) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    dbg!("pop");
    // TODO: wait
    let mut addr = gpr[SP];
    let register_list = dec.get_register_list();

    for i in 0..0x8 {
        if register_list & (1 << i) != 0 {
            // TODO: wait
            let data = bus.read_word(addr);
            gpr[i] = data;
            addr += 4;
        }
    }

    if dec.get_R() {
        let data = bus.read_word(addr);
        gpr[PC] = data & 0xFFFF_FFFE;
        addr += 4;
    }
    // TODO: wait
    gpr[SP] = addr;
    dbg!(&gpr);
    if dec.get_R() {
        Ok(PipelineStatus::Flush)
    } else {
        Ok(PipelineStatus::Continue)
    }
}
