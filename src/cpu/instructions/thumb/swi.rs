use crate::cpu::bus::accessor::BusAccessor;
use crate::cpu::decoder::thumb::ThumbSoftwareInterrupt;
use crate::cpu::instructions::{ExecuteResult, PipelineStatus};
use crate::cpu::registers::psr::PSR;
use crate::cpu::bios::Bios;
use crate::types::Word;

pub fn exec_thumb_swi<T: BusAccessor>(
    bus: &mut T,
    dec: ThumbSoftwareInterrupt,
    gpr: &mut [Word; 16],
    _cpsr: &mut PSR,
    _spsr: &mut PSR,
) -> ExecuteResult {
    // 8bit 即値を BIOS ディスパッチへ渡す（optimized SWI path）
    let swi_number = dec.get_immediate() as u32;
    Bios::execute_swi(bus, swi_number, gpr);
    (1, PipelineStatus::Continue)
}
