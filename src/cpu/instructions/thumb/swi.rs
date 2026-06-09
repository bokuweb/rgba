use crate::cpu::bus::accessor::BusAccessor;
use crate::cpu::decoder::thumb::ThumbSoftwareInterrupt;
use crate::cpu::instructions::{ExecuteResult, PipelineStatus};
use crate::cpu::registers::psr::PSR;
use crate::cpu::bios::Bios;
use crate::types::Word;

const PC: usize = 15;

pub fn exec_thumb_swi<T: BusAccessor>(
    bus: &mut T,
    dec: ThumbSoftwareInterrupt,
    gpr: &mut [Word; 16],
    _cpsr: &mut PSR,
    _spsr: &mut PSR,
) -> ExecuteResult {
    // Pass the 8-bit immediate to the BIOS dispatch (optimized SWI path)
    let swi_number = dec.get_immediate() as u32;
    Bios::execute_swi(bus, swi_number, gpr);
    // See exec_arm_swi: re-execute this SWI while a BIOS IntrWait is armed so the
    // awaited interrupt flag is re-checked on every wake. (Thumb visible PC =
    // instruction address + 4.)
    if bus.is_cpu_halted() && bus.intr_wait_mask().is_some() {
        gpr[PC] = gpr[PC].wrapping_sub(4);
        return (1, PipelineStatus::Flush);
    }
    (1, PipelineStatus::Continue)
}
