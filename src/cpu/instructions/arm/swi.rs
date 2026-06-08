use crate::cpu::bus::accessor::BusAccessor;
use crate::cpu::decoder::arm::SoftwareInterrupt;
use crate::cpu::registers::psr::PSR;
use crate::types::Word;
use crate::cpu::instructions::{ExecuteResult, PipelineStatus};
use crate::cpu::bios::Bios;

const PC: usize = 15;

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
    // If a BIOS IntrWait/VBlankIntrWait armed a wait (CPU halted with a mask),
    // rewind to this SWI so it re-executes — and re-checks the awaited flag —
    // each time an interrupt wakes the CPU, instead of falling through to the
    // next instruction. (ARM visible PC = instruction address + 8.)
    if bus.is_cpu_halted() && bus.intr_wait_mask().is_some() {
        gpr[PC] = gpr[PC].wrapping_sub(8);
        return Ok((1, PipelineStatus::Flush));
    }
    Ok((1, PipelineStatus::Continue))
}
