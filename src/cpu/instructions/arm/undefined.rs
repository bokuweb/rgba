use crate::cpu::bus::accessor::BusAccessor;
use crate::cpu::registers::psr::{Mode, PSR};
use crate::cpu::instructions::{ExecuteResult, PipelineStatus};
use crate::types::Word;

/// Handle ARM Undefined instruction exception (including coprocessor ops on ARM7TDMI)
/// ARM semantics on exception entry:
///  - LR_und = address of next instruction (PC at time of exception)
///  - SPSR_und = CPSR
///  - CPSR.M = Undefined mode, CPSR.I = 1 (disable IRQ), CPSR.T = 0 (ARM)
///  - PC = 0x00000004 (Undefined Instruction Vector)
pub fn exec_arm_undefined<T>(
    _bus: &T,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
    spsr: &mut PSR,
) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    // Save CPSR to SPSR_und
    *spsr = *cpsr;

    // LR_und gets address of next instruction (current visible PC already points to next fetch)
    let lr_und = gpr[15];

    // Enter Undefined mode, disable IRQ, switch to ARM state
    cpsr.set_mode(Mode::Undefined);
    cpsr.set_I(true);
    cpsr.set_T(false);

    // Set LR and branch to vector 0x04
    gpr[14] = lr_und;
    gpr[15] = 0x04;

    Ok((2, PipelineStatus::Flush))
}


