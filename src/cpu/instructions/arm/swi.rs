use crate::cpu::bus::accessor::BusAccessor;
use crate::cpu::decoder::arm::SoftwareInterrupt;
use crate::cpu::registers::psr::PSR;
use crate::types::Word;
use crate::cpu::instructions::{ExecuteResult, PipelineStatus};
use crate::cpu::bios::Bios;

/// Software Interrupt (SWI) instruction execution
/// This switches to Supervisor mode and jumps to the SWI vector
pub fn exec_arm_swi<T>(
    bus: &mut T,
    dec: SoftwareInterrupt,
    gpr: &mut [Word; 16],
    _cpsr: &mut PSR,
    _spsr: &mut PSR,
) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    // Optimized SWI path: dispatch BIOS call directly.
    let swi_number = dec.get_immediate();
    Bios::execute_swi(bus, swi_number, gpr);
    Ok((1, PipelineStatus::Continue))
}
