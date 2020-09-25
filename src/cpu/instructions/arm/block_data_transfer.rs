use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
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
fn exec_block_data_transfer<F, T>(gpr: &mut [u32; 16], dec: BlockDataTransfer, bus: &mut T, load_or_store: F) -> Result<ExecuteResult, ()>
where
    F: Fn(&mut [u32; 16], u32, u32, &mut T),
    T: BusAccessor,
{
    let mut base: i64 = gpr[dec.get_Rn() as usize] as i64;
    let register_list = dec.get_register_list();
    let offset: i64 = if dec.get_U() { 4 } else { -4 };
    for i in 0..0x10 {
        if register_list & (1 << i) != 0 {
            if dec.get_P() {
                base = base.wrapping_add(offset);
            }
            load_or_store(gpr, base as u32, i, bus);
            if !dec.get_P() {
                base = base.wrapping_add(offset);
            }
        }
    }
    // TODO: Handle S flag.
    if dec.get_W() {
        gpr[dec.get_Rn() as usize] = base as u32;
    }

    // If PC is loaded
    if register_list & 0x8000 != 0 && dec.get_L() {
        Ok((0, PipelineStatus::Flush))
    } else {
        Ok((0, PipelineStatus::Continue))
    }
}

pub fn exec_arm_ldm<T>(bus: &mut T, dec: BlockDataTransfer, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    exec_block_data_transfer(gpr, dec, bus, |gpr, base, i, bus| {
        let data = bus.read_word(base);
        gpr[i as usize] = data;
    })
}

pub fn exec_arm_stm<T>(bus: &mut T, dec: BlockDataTransfer, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    exec_block_data_transfer(gpr, dec, bus, |gpr, base, i, bus| {
        bus.write_word(base, gpr[i as usize] as Word);
    })
}
