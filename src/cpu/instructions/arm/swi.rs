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

    // Save return address in LR_svc (PC - 4 for correct return)
    // PC points to the instruction after SWI, so we need PC - 4 to return to after SWI
    let lr_svc = gpr[15] - 4;

    // Switch to Supervisor mode and disable IRQs
    cpsr.set_mode(Mode::Supervisor);
    cpsr.set_I(true);       // Disable IRQ
    cpsr.set_T(false);      // ARM mode

    // Set LR_svc to return address
    gpr[14] = lr_svc;

    // Get the immediate value for BIOS function dispatch
    let swi_number = dec.get_immediate();

    // Execute BIOS function directly (emulated BIOS)
    // Note: In a real GBA, this would jump to BIOS ROM and execute there
    // We implement the functions directly here for emulation efficiency
    println!("SWI: Before BIOS execution - PC=0x{:08X}, CPSR=0x{:08X}, SPSR=0x{:08X}", gpr[15], cpsr.get(), spsr.get());
    Bios::execute_swi(_bus, swi_number, gpr);
    println!("SWI: After BIOS execution - PC=0x{:08X}, LR=0x{:08X}", gpr[15], gpr[14]);

    // For emulated BIOS: restore mode and return to caller directly
    // ARM semantics: LR_svc should contain the address to return to (PC after SWI)
    // We saved LR_svc as (PC_at_SWI + 4). To resume correctly, set PC to LR_svc.
    *cpsr = *spsr;  // Restore original CPSR (mode, flags, etc.)
    gpr[15] = lr_svc;  // Return to instruction after SWI (no additional +4)
    println!("SWI: After restoration - PC=0x{:08X}, CPSR=0x{:08X}, returning to 0x{:08X}", gpr[15], cpsr.get(), lr_svc);

    Ok((1, PipelineStatus::Flush))  // Flush pipeline due to PC change
}