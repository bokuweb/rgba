use crate::cpu::bus::accessor::BusAccessor;
use crate::cpu::decoder::arm::SoftwareInterrupt;
use crate::cpu::registers::psr::{PSR, Mode};
use crate::types::Word;
use crate::cpu::instructions::{ExecuteResult, PipelineStatus};
use crate::cpu::bios::Bios;

/// Software Interrupt (SWI) instruction execution
/// This switches to Supervisor mode and jumps to the SWI vector
pub fn exec_arm_swi<T>(
    _bus: &T,
    dec: SoftwareInterrupt,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
    spsr: &mut PSR,
) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    // Save current CPSR to SPSR_svc
    *spsr = *cpsr;

    // Save return address in LR_svc (current PC + 4)
    let lr_svc = gpr[15]; // PC is already incremented by fetch stage

    // Switch to Supervisor mode and disable IRQs
    cpsr.set_mode(Mode::Supervisor);
    cpsr.set_I(true);       // Disable IRQ
    cpsr.set_T(false);      // ARM mode

    // Set LR_svc to return address
    gpr[14] = lr_svc;

    // Jump to SWI vector (0x08)
    gpr[15] = 0x00000008;

    // Get the immediate value for BIOS function dispatch
    let swi_number = dec.get_immediate();

    // Execute BIOS function
    // Note: In a real GBA, this would jump to BIOS ROM and execute there
    // We implement the functions directly here for emulation
    Bios::execute_swi(_bus, swi_number, gpr);

    Ok((1, PipelineStatus::Continue))
}