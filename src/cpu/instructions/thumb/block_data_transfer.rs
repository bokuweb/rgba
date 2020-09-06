use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::decoder::thumb::*;
use crate::cpu::types::*;

pub fn exec_thumb_stmia<T>(
    bus: &mut T,
    dec: BlockDataTransfer,
    gpr: &mut [Word; 16],
) -> Result<PipelineStatus, ()>
where
    T: BusAccessor,
{
    let mut base = gpr[dec.get_Rn() as usize];
    let register_list = dec.get_register_list();
    dbg!(register_list);
    for i in 0..0x8 {
        if register_list & (1 << i) != 0 {
            dbg!(base, gpr[i as usize]);
            bus.write_word(base, gpr[i as usize] as Word);
            base = base.wrapping_add(4);
        }
    }
    gpr[dec.get_Rn() as usize] = base;
    dbg!(&gpr);
    Ok(PipelineStatus::Continue)
}
