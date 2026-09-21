use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::{arm, thumb};
use crate::cpu::instructions::arm::{
    block_data_transfer::*, branch::*, branch_and_exchange::*, data::*, extra_memory::*, memory::*, multiple::*, psr_transfer::*, single_data_swap::*, swi::*, undefined::*,
};

use crate::cpu::instructions::thumb::*;

use crate::cpu::instructions::PipelineStatus;
use crate::cpu::registers::{BankGpr, BankSpsr, CpuState, PSR};
use crate::cpu::types::*;
use crate::types::*;

pub const INITIAL_PIPELINE_WAIT: u8 = 2;

enum CpuMode {
    System,
    Supervisor,
    FIQ,
}

pub struct ARM {
    pub gpr: [u32; 16],
    pub bank_gpr: BankGpr,
    pub bank_spsr: BankSpsr,
    pub cpsr: PSR,
    pub spsr: PSR,

    pipeline_wait: u8,
    mode: CpuMode,
    irq_disable: bool,
    fiq_disable: bool,
    optimise_swi: bool,
    irq_pending: bool,
    // Three-stage prefetch buffers (current/next1/next2)
    arm_pipe_curr: Option<(Word, Word)>,       // (addr, instr)
    arm_pipe_next1: Option<(Word, Word)>,
    arm_pipe_next2: Option<(Word, Word)>,
    thumb_pipe_curr: Option<(Word, HalfWord)>, // (addr, instr)
    thumb_pipe_next1: Option<(Word, HalfWord)>,
    thumb_pipe_next2: Option<(Word, HalfWord)>,
}

impl ARM {
    pub fn new() -> Self {
        Self {
            pipeline_wait: INITIAL_PIPELINE_WAIT,
            gpr: [0; 16],
            bank_gpr: BankGpr::default(),
            cpsr: PSR::default(),
            spsr: PSR::default(),
            bank_spsr: BankSpsr::default(),
            mode: CpuMode::System,
            irq_disable: false,
            fiq_disable: false,
            optimise_swi: false,
            irq_pending: false,
            arm_pipe_curr: None,
            arm_pipe_next1: None,
            arm_pipe_next2: None,
            thumb_pipe_curr: None,
            thumb_pipe_next1: None,
            thumb_pipe_next2: None,
        }
    }

    pub fn reset(&mut self) {
        self.gpr[PC] = 0x0000_0000;

        self.cpsr = PSR::default();

        self.mode = CpuMode::Supervisor;
        self.irq_disable = true;
        self.fiq_disable = true;

        // Initialize banked SPs for privileged modes to IWRAM per common GBA conventions
        // SVC (Supervisor) stack
        self.bank_gpr.write(crate::cpu::registers::psr::Mode::Supervisor, SP, 0x0300_7FE0);
        // IRQ stack
        self.bank_gpr.write(crate::cpu::registers::psr::Mode::IRQ, SP, 0x0300_7FA0);
        // FIQ stack (rarely used on GBA; provide a sane default)
        self.bank_gpr.write(crate::cpu::registers::psr::Mode::FIQ, SP, 0x0300_7F00);
        // Set System/User stack (current visible SP in System mode)
        self.gpr[SP] = 0x0300_7F00;
    }

    const fn flush_pipeline(&mut self) {
        self.pipeline_wait = INITIAL_PIPELINE_WAIT;
        self.arm_pipe_curr = None;
        self.arm_pipe_next1 = None;
        self.arm_pipe_next2 = None;
        self.thumb_pipe_curr = None;
        self.thumb_pipe_next1 = None;
        self.thumb_pipe_next2 = None;
    }

    fn increment_pc(&mut self) {
        let next = if self.cpsr.get_cpu_state() == CpuState::ARM { 4 } else { 2 };
        let pc = if self.cpsr.get_cpu_state() == CpuState::ARM {
            self.gpr[PC] & 0xFFFF_FFFC
        } else {
            self.gpr[PC] & 0xFFFF_FFFE
        };
        self.gpr[PC] = pc.wrapping_add(next);
    }

    fn get_inst_addr(&self) -> Word {
        // During pipeline refill after a flush/reset, PC may point to the branch target
        // (not yet the usual "visible PC"). In that case, use the aligned PC directly
        // as the fetch address for the first instruction to prefill buffers correctly.
        if self.pipeline_wait > 0 {
            if self.cpsr.get_cpu_state() == CpuState::ARM {
                self.gpr[PC] & 0xFFFF_FFFC
            } else {
                self.gpr[PC] & 0xFFFF_FFFE
            }
        } else {
            if self.cpsr.get_cpu_state() == CpuState::ARM {
                self.gpr[PC].saturating_sub((PC_OFFSET * 4) as Word)
            } else {
                self.gpr[PC].saturating_sub((PC_OFFSET * 2) as Word)
            }
        }
    }

    fn get_prefetch_width(&self) -> AccessWidth {
        if self.cpsr.get_cpu_state() == CpuState::ARM {
            AccessWidth::Word
        } else {
            AccessWidth::HalfWord
        }
    }

    pub const fn get_gpr(&self, n: usize) -> Word {
        self.gpr[n]
    }

    pub const fn get_cpsr(&self) -> PSR {
        self.cpsr
    }

    pub const fn set_gpr(&mut self, n: usize, data: u32) {
        self.gpr[n] = data;
    }

    /// Drive the (level-sensitive) IRQ input. The caller re-evaluates
    /// `IE & IF & IME` every step and passes the result; the CPU takes the
    /// exception only while the line is still high once IRQs are enabled.
    /// A latched request would fire a spurious IRQ after a handler that
    /// acknowledges IF (and possibly narrows IE) re-enables interrupts.
    pub const fn set_irq_line(&mut self, asserted: bool) {
        self.irq_pending = asserted;
    }

    #[cfg(test)]
    pub const fn request_irq(&mut self) {
        self.irq_pending = true;
    }

    fn handle_irq<T>(&mut self, _bus: &mut T) -> Cycle
    where
        T: BusAccessor,
    {
        let current_cpsr = self.cpsr;

        // Save current PC (return address) to LR
        // For IRQ, return address should be current PC (instruction being interrupted)
        let return_addr = if current_cpsr.get_cpu_state() == crate::cpu::registers::psr::CpuState::Thumb {
            self.gpr[PC] // Thumb IRQ return is handled by BIOS as LR-4; use visible PC here.
        } else {
            self.gpr[PC].wrapping_sub(4) // ARM mode: adjust for 4-byte instruction pipeline
        };

        // Switch to IRQ mode with full banked register/SPSR handling
        self.cpsr.switch_mode(
            crate::cpu::registers::psr::Mode::IRQ,
            &mut self.gpr,
            &mut self.spsr,
            &mut self.bank_gpr,
            &mut self.bank_spsr,
        );
        // Save current CPSR to SPSR_irq
        self.spsr = current_cpsr;
        // Save return address to banked LR_irq
        self.gpr[LR] = return_addr;

        self.cpsr.set_I(true); // Disable IRQ
        self.cpsr.set_T(false); // Switch to ARM mode (IRQ handlers are always ARM)

        // Jump to IRQ vector (0x18)
        self.gpr[PC] = 0x18;
        self.flush_pipeline();

        // Clear pending IRQ
        self.irq_pending = false;

        // Return cycle count for IRQ handling
        2 // Approximate cycle cost for IRQ handling
    }

    // wait_pipeline_filled consumes 1N + 2S cycle to fill pipeline and fetch next instruction.
    fn wait_pipeline_filled<T>(&mut self, bus: &mut T) -> Cycle
    where
        T: BusAccessor,
    {
        let mut cycle = 0;
        let width = self.get_prefetch_width();
        // Capture base instruction address before PC advances in the loop
        let base_addr = self.get_inst_addr();
        // Fill three-stage prefetch buffer with actual instruction values
        match self.cpsr.get_cpu_state() {
            CpuState::ARM => {
                let i0 = bus.read_word(base_addr & 0xFFFF_FFFC);
                let a1 = (base_addr.wrapping_add(4)) & 0xFFFF_FFFC;
                let a2 = (base_addr.wrapping_add(8)) & 0xFFFF_FFFC;
                let i1 = bus.read_word(a1);
                let i2 = bus.read_word(a2);
                self.arm_pipe_curr = Some((base_addr & 0xFFFF_FFFC, i0));
                self.arm_pipe_next1 = Some((a1, i1));
                self.arm_pipe_next2 = Some((a2, i2));
            }
            CpuState::Thumb => {
                let i0 = bus.read_halfword(base_addr & 0xFFFF_FFFE);
                let a1 = (base_addr.wrapping_add(2)) & 0xFFFF_FFFE;
                let a2 = (base_addr.wrapping_add(4)) & 0xFFFF_FFFE;
                let i1 = bus.read_halfword(a1);
                let i2 = bus.read_halfword(a2);
                self.thumb_pipe_curr = Some((base_addr & 0xFFFF_FFFE, i0));
                self.thumb_pipe_next1 = Some((a1, i1));
                self.thumb_pipe_next2 = Some((a2, i2));
            }
        }
        while self.pipeline_wait > 0 {
            cycle += if self.pipeline_wait == INITIAL_PIPELINE_WAIT {
                // consume 1N cycle
                bus.compute_cycle(self.gpr[PC], AccessType::NonSeq(width))
            } else {
                // consume 1S cycle
                bus.compute_cycle(self.gpr[PC], AccessType::Seq(width))
            };
            self.pipeline_wait -= 1;
            self.increment_pc();
        }
        // consume 1S cycle for next cycle prefetch
        let next = bus.compute_cycle(self.gpr[PC], AccessType::Seq(width));
        next + cycle
    }

    pub fn step<T>(&mut self, bus: &mut T, started: bool) -> Result<Cycle, ()>
    where
        T: BusAccessor,
    {
        if bus.is_cpu_halted() {
            if self.irq_pending && !self.cpsr.get_I() {
                bus.set_cpu_halted(false);
                let irq_cycle = self.handle_irq(bus);
                return Ok(irq_cycle);
            }
            if bus.has_pending_interrupt_flags() {
                bus.set_cpu_halted(false);
            }
            return Ok(1);
        }

        // Check for pending IRQ before executing instruction
        if self.irq_pending && !self.cpsr.get_I() {
            bus.set_cpu_halted(false);
            let irq_cycle = self.handle_irq(bus);
            return Ok(irq_cycle);
        }

        let cycle = if self.pipeline_wait > 0 { self.wait_pipeline_filled(bus) } else { 0 };

        // Check for pending IRQ before executing normal instructions
        if self.irq_pending && !self.cpsr.get_I() {
            bus.set_cpu_halted(false);
            let irq_cycle = self.handle_irq(bus);
            return Ok(cycle + irq_cycle);
        }

        let instruction_width = if self.cpsr.get_cpu_state() == CpuState::ARM { 4 } else { 2 };
        bus.set_open_bus_context(self.gpr[PC], instruction_width);
        // let log = format!("{:?}", self.gpr);
        // dbg!(&self.gpr);
        if self.gpr[15] == 134_225_604 {
            // panic!("aa")
        }

        match self.cpsr.get_cpu_state() {
            CpuState::ARM => {
                // dbg!(&self.gpr);

                let fetched = self.get_arm_executable(bus);
                let cond: Cond = fetched.wrapping_shr(28).into();
                let condition_result = self.cpsr.condition_ok(cond);
                if !condition_result {
                    let s = bus.compute_cycle(self.gpr[PC], AccessType::Seq(AccessWidth::Word));
                    self.increment_pc();
                    return Ok(s + cycle);
                }
                let instruction = arm::decode(fetched);
                let cycle = cycle + self.execute_arm(instruction, bus)?;

                Ok(cycle)
            }
            CpuState::Thumb => {
                let fetched = self.get_thumb_executable(bus);
                // dbg!(&self.gpr);

                // if self.gpr[15] == 134219062 {
                //    dbg!("hello");
                // }
                let instruction = thumb::decode(fetched);
                let cycle = cycle + self.execute_thumb(instruction, bus, started);
                Ok(cycle)
            }
        }
    }

    fn get_arm_executable<T>(&mut self, bus: &mut T) -> Word
    where
        T: BusAccessor,
    {
        let addr = self.get_inst_addr() & 0xFFFF_FFFC;
        if let Some((a, v)) = self.arm_pipe_curr {
            if a == addr {
                return v;
            }
        }
        // Fallback if buffer is not initialized/mismatched
        let v = bus.read_word(addr);
        self.arm_pipe_curr = Some((addr, v));
        // Preload next1/next2 slots for robustness
        let next_addr1 = addr.wrapping_add(4) & 0xFFFF_FFFC;
        let next_addr2 = addr.wrapping_add(8) & 0xFFFF_FFFC;
        let nv1 = bus.read_word(next_addr1);
        let nv2 = bus.read_word(next_addr2);
        self.arm_pipe_next1 = Some((next_addr1, nv1));
        self.arm_pipe_next2 = Some((next_addr2, nv2));
        v
    }

    fn get_thumb_executable<T>(&mut self, bus: &mut T) -> HalfWord
    where
        T: BusAccessor,
    {
        let addr = self.get_inst_addr() & 0xFFFF_FFFE;
        if let Some((a, v)) = self.thumb_pipe_curr {
            if a == addr {
                return v;
            }
        }
        // Fallback if buffer is not initialized/mismatched
        let v = bus.read_halfword(addr);
        self.thumb_pipe_curr = Some((addr, v));
        // Preload next1/next2 slots for robustness
        let next_addr1 = addr.wrapping_add(2) & 0xFFFF_FFFE;
        let next_addr2 = addr.wrapping_add(4) & 0xFFFF_FFFE;
        let nv1 = bus.read_halfword(next_addr1);
        let nv2 = bus.read_halfword(next_addr2);
        self.thumb_pipe_next1 = Some((next_addr1, nv1));
        self.thumb_pipe_next2 = Some((next_addr2, nv2));
        v
    }

    fn execute_arm<T>(&mut self, instruction: arm::Instruction, bus: &mut T) -> Result<Cycle, ()>
    where
        T: BusAccessor,
    {
        // Extract condition code from instruction and check if it should execute
        use crate::cpu::types::Cond;
        let cond = match &instruction {
            arm::Instruction::AND(dec) => dec.get_cond().into(),
            arm::Instruction::EOR(dec) => dec.get_cond().into(),
            arm::Instruction::SUB(dec) => dec.get_cond().into(),
            arm::Instruction::RSB(dec) => dec.get_cond().into(),
            arm::Instruction::ADD(dec) => dec.get_cond().into(),
            arm::Instruction::ADC(dec) => dec.get_cond().into(),
            arm::Instruction::SBC(dec) => dec.get_cond().into(),
            arm::Instruction::RSC(dec) => dec.get_cond().into(),
            arm::Instruction::TST(dec) => dec.get_cond().into(),
            arm::Instruction::TEQ(dec) => dec.get_cond().into(),
            arm::Instruction::CMP(dec) => dec.get_cond().into(),
            arm::Instruction::CMN(dec) => dec.get_cond().into(),
            arm::Instruction::ORR(dec) => dec.get_cond().into(),
            arm::Instruction::MOV(dec) => dec.get_cond().into(),
            arm::Instruction::LSL(dec) => dec.get_cond().into(),
            arm::Instruction::LSR(dec) => dec.get_cond().into(),
            arm::Instruction::ASR(dec) => dec.get_cond().into(),
            arm::Instruction::RRX(dec) => dec.get_cond().into(),
            arm::Instruction::ROR(dec) => dec.get_cond().into(),
            arm::Instruction::BIC(dec) => dec.get_cond().into(),
            arm::Instruction::MVN(dec) => dec.get_cond().into(),
            arm::Instruction::MUL(dec) => dec.get_cond().into(),
            arm::Instruction::MLA(dec) => dec.get_cond().into(),
            arm::Instruction::UMULL(dec) => dec.get_cond().into(),
            arm::Instruction::UMLAL(dec) => dec.get_cond().into(),
            arm::Instruction::SMULL(dec) => dec.get_cond().into(),
            arm::Instruction::SMLAL(dec) => dec.get_cond().into(),
            arm::Instruction::LDR(dec) => dec.get_cond().into(),
            arm::Instruction::STR(dec) => dec.get_cond().into(),
            arm::Instruction::LDRB(dec) => dec.get_cond().into(),
            arm::Instruction::STRB(dec) => dec.get_cond().into(),
            arm::Instruction::STRH(dec) => dec.get_cond().into(),
            arm::Instruction::LDRH(dec) => dec.get_cond().into(),
            arm::Instruction::LDRSB(dec) => dec.get_cond().into(),
            arm::Instruction::LDRSH(dec) => dec.get_cond().into(),
            arm::Instruction::B(dec) => dec.get_cond().into(),
            arm::Instruction::BL(dec) => dec.get_cond().into(),
            arm::Instruction::BX(dec) => dec.get_cond().into(),
            arm::Instruction::LDM(dec) => dec.get_cond().into(),
            arm::Instruction::STM(dec) => dec.get_cond().into(),
            arm::Instruction::MRS(dec) => dec.get_cond().into(),
            arm::Instruction::MSR(dec) => dec.get_cond().into(),
            arm::Instruction::SWP(dec) => dec.get_cond().into(),
            arm::Instruction::SWPB(dec) => dec.get_cond().into(),
            arm::Instruction::SWI(dec) => ((dec.raw >> 28) & 0xF).into(), // Extract condition from raw field
            arm::Instruction::Undefined => Cond::AL, // Undefined instructions are always executed
        };

        let cond_ok = self.cpsr.condition_ok(cond);

        // If condition is not met, instruction does not execute (takes 1 cycle)
        if !cond_ok {
            return Ok(1);
        }

        let (cycle, pipeline_status) = {
            match instruction {
                        arm::Instruction::Undefined => {
                            // Undefined instruction exception (e.g., coprocessor op on ARM7TDMI)
                            exec_arm_undefined(bus, &mut self.gpr, &mut self.cpsr, &mut self.spsr)?
                        }
                arm::Instruction::AND(dec) => exec_arm_and(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::EOR(dec) => exec_arm_eor(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::SUB(dec) => exec_arm_sub(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::RSB(dec) => exec_arm_rsb(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::ADD(dec) => exec_arm_add(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::ADC(dec) => exec_arm_adc(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::SBC(dec) => exec_arm_sbc(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::RSC(dec) => exec_arm_rsc(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::TST(dec) => exec_arm_tst(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::TEQ(dec) => exec_arm_teq(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::CMP(dec) => exec_arm_cmp(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::CMN(dec) => exec_arm_cmn(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::ORR(dec) => exec_arm_orr(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::MOV(dec) => exec_arm_mov(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::LSL(dec) => exec_arm_shift(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::LSR(dec) => exec_arm_shift(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::ASR(dec) => exec_arm_shift(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::RRX(dec) => exec_arm_rrx(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::ROR(dec) => exec_arm_shift(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::BIC(dec) => exec_arm_bic(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::MVN(dec) => exec_arm_mvn(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::MUL(dec) => exec_arm_mul(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::MLA(dec) => exec_arm_mla(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::UMULL(dec) => {
                    // When the LDM^ glitch is active, exclude the multiply operands (Rm, Rs) from the overlay
                    if self.bank_gpr.is_glitch_active_or_armed() {
                        self.bank_gpr.refine_remove_indices_from_overlay(&[dec.get_Rm() as usize, dec.get_Rs() as usize], &mut self.gpr);
                    }
                    let rd_idx = dec.get_Rd() as usize;
                    let rn_idx = dec.get_Rn() as usize;
                    let r = exec_arm_umull(bus, dec, &mut self.gpr, &mut self.cpsr)?;
                    // Right after execution, drop the destinations (RdHi, RdLo) from the overlay if present.
                    if self.bank_gpr.is_glitch_active_or_armed() {
                        self.bank_gpr.refine_remove_indices_from_overlay(&[rd_idx, rn_idx], &mut self.gpr);
                    }
                    r
                }
                arm::Instruction::UMLAL(dec) => {
                    if self.bank_gpr.is_glitch_active_or_armed() {
                        self.bank_gpr.refine_remove_indices_from_overlay(&[dec.get_Rm() as usize, dec.get_Rs() as usize], &mut self.gpr);
                    }
                    let rd_idx = dec.get_Rd() as usize;
                    let rn_idx = dec.get_Rn() as usize;
                    let r = exec_arm_umlal(bus, dec, &mut self.gpr, &mut self.cpsr)?;
                    // Right after execution, drop the destinations (RdHi, RdLo) from the overlay if present.
                    if self.bank_gpr.is_glitch_active_or_armed() {
                        self.bank_gpr.refine_remove_indices_from_overlay(&[rd_idx, rn_idx], &mut self.gpr);
                    }
                    r
                }
                arm::Instruction::SMULL(dec) => {
                    if self.bank_gpr.is_glitch_active_or_armed() {
                        self.bank_gpr.refine_remove_indices_from_overlay(&[dec.get_Rm() as usize, dec.get_Rs() as usize], &mut self.gpr);
                    }
                    let rd_idx = dec.get_Rd() as usize;
                    let rn_idx = dec.get_Rn() as usize;
                    let r = exec_arm_smull(bus, dec, &mut self.gpr, &mut self.cpsr)?;
                    // Right after execution, drop the destinations (RdHi, RdLo) from the overlay if present.
                    if self.bank_gpr.is_glitch_active_or_armed() {
                        self.bank_gpr.refine_remove_indices_from_overlay(&[rd_idx, rn_idx], &mut self.gpr);
                    }
                    r
                }
                arm::Instruction::SMLAL(dec) => {
                    if self.bank_gpr.is_glitch_active_or_armed() {
                        self.bank_gpr.refine_remove_indices_from_overlay(&[dec.get_Rm() as usize, dec.get_Rs() as usize], &mut self.gpr);
                    }
                    let rd_idx = dec.get_Rd() as usize;
                    let rn_idx = dec.get_Rn() as usize;
                    let r = exec_arm_smlal(bus, dec, &mut self.gpr, &mut self.cpsr)?;
                    // Right after execution, drop the destinations (RdHi, RdLo) from the overlay if present.
                    if self.bank_gpr.is_glitch_active_or_armed() {
                        self.bank_gpr.refine_remove_indices_from_overlay(&[rd_idx, rn_idx], &mut self.gpr);
                    }
                    r
                }
                arm::Instruction::LDR(dec) => exec_arm_ldr(bus, dec, &mut self.gpr, &self.cpsr)?,
                arm::Instruction::STR(dec) => exec_arm_str(bus, dec, &mut self.gpr, &self.cpsr)?,
                arm::Instruction::LDRB(dec) => exec_arm_ldrb(bus, dec, &mut self.gpr, &self.cpsr)?,
                arm::Instruction::STRB(dec) => exec_arm_strb(bus, dec, &mut self.gpr, &self.cpsr)?,
                arm::Instruction::STRH(dec) => exec_arm_strh(bus, dec, &mut self.gpr)?,
                arm::Instruction::LDRH(dec) => exec_arm_ldrh(bus, dec, &mut self.gpr)?,
                arm::Instruction::LDRSB(dec) => exec_arm_ldrsb(bus, dec, &mut self.gpr)?,
                arm::Instruction::LDRSH(dec) => exec_arm_ldrsh(bus, dec, &mut self.gpr)?,
                arm::Instruction::B(dec) => exec_arm_b(dec, &mut self.gpr)?,
                arm::Instruction::BL(dec) => exec_arm_bl(dec, &mut self.gpr)?,
                arm::Instruction::BX(dec) => exec_arm_bx(dec, &mut self.cpsr, &mut self.gpr)?,
                arm::Instruction::LDM(dec) => exec_arm_ldm(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::STM(dec) => exec_arm_stm(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::MRS(dec) => exec_arm_mrs(bus, dec, &mut self.gpr, &self.cpsr, &self.spsr)?,
                arm::Instruction::MSR(dec) => exec_arm_msr(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::SWP(dec) => exec_arm_swp(bus, dec, &mut self.gpr)?,
                arm::Instruction::SWPB(dec) => exec_arm_swpb(bus, dec, &mut self.gpr)?,
                arm::Instruction::SWI(dec) => exec_arm_swi(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr)?,
                // Unknown ARM encodings decode to `Instruction::Undefined`
                // (handled above as the undefined-instruction exception), so the
                // match is exhaustive and needs no catch-all.
            }
        };
        match pipeline_status {
            PipelineStatus::Continue => {
                self.increment_pc();
                // After retiring this instruction, if there is an Armed glitch (from LDM^ just executed),
                // apply it now so that it affects the next instruction (Armed -> Active).
                self.bank_gpr.advance_glitch_window(&mut self.gpr);
                // Advance prefetch buffer and fetch next2
                match self.cpsr.get_cpu_state() {
                    CpuState::ARM => {
                        let curr_addr = self.get_inst_addr() & 0xFFFF_FFFC; // now points to former next1
                        let next2_addr = curr_addr.wrapping_add(8) & 0xFFFF_FFFC;
                        // Shift: curr <- next1, next1 <- next2, fetch next2
                        self.arm_pipe_curr = self.arm_pipe_next1.take();
                        self.arm_pipe_next1 = self.arm_pipe_next2.take();
                        let nv2 = bus.read_word(next2_addr);
                        self.arm_pipe_next2 = Some((next2_addr, nv2));
                    }
                    CpuState::Thumb => {
                        let curr_addr = self.get_inst_addr() & 0xFFFF_FFFE; // former next1
                        let next2_addr = curr_addr.wrapping_add(4) & 0xFFFF_FFFE;
                        self.thumb_pipe_curr = self.thumb_pipe_next1.take();
                        self.thumb_pipe_next1 = self.thumb_pipe_next2.take();
                        let nv2 = bus.read_halfword(next2_addr);
                        self.thumb_pipe_next2 = Some((next2_addr, nv2));
                    }
                }
                Ok(cycle)
            }
            PipelineStatus::Flush => {
                self.flush_pipeline();
                // On pipeline flush, also advance glitch window so timing remains one-instruction long.
                self.bank_gpr.advance_glitch_window(&mut self.gpr);
                Ok(cycle + self.wait_pipeline_filled(bus))
            }
        }
    }

    // The trailing `_` arm is a deliberate catch-all for undefined/unhandled
    // THUMB encodings (treated as NOP), so keep it as a wildcard.
    #[allow(clippy::match_wildcard_for_single_variants)]
    fn execute_thumb<T>(&mut self, instruction: thumb::Instruction, bus: &mut T, started: bool) -> Cycle
    where
        T: BusAccessor,
    {
        let (cycle, pipeline_status) = {
            match instruction {
                thumb::Instruction::LDR1(dec) => exec_thumb_ldr_imm_offset(bus, dec, &mut self.gpr, started),
                thumb::Instruction::LDRRegOffset(dec) => exec_thumb_ldr_reg_offset(bus, dec, &mut self.gpr),
                thumb::Instruction::LDR3(dec) => exec_thumb_ldr3(bus, dec, &mut self.gpr),
                thumb::Instruction::LDR4(dec) => exec_thumb_load_sp_relative(bus, dec, &mut self.gpr),
                thumb::Instruction::STR1(dec) => exec_thumb_str1(bus, dec, &mut self.gpr),
                thumb::Instruction::STR3(dec) => exec_thumb_str3(bus, dec, &mut self.gpr),
                thumb::Instruction::LDRB(dec) => exec_thumb_ldrb_imm_offset(bus, dec, &mut self.gpr),
                thumb::Instruction::LDRBRegOffset(dec) => exec_thumb_ldrb_reg_offset(bus, dec, &mut self.gpr),
                thumb::Instruction::LDRH(dec) => exec_thumb_ldrh(bus, dec, &mut self.gpr),
                thumb::Instruction::LDRHThumb8(dec) => exec_thumb8_ldrh(bus, dec, &mut self.gpr),
                thumb::Instruction::LDSBThumb8(dec) => exec_thumb8_ldsb(bus, dec, &mut self.gpr),
                thumb::Instruction::LDSHThumb8(dec) => exec_thumb8_ldsh(bus, dec, &mut self.gpr),
                thumb::Instruction::STRH(dec) => exec_thumb_strh(bus, dec, &mut self.gpr),
                thumb::Instruction::StrbImmOffset(dec) => exec_thumb_strb_imm_offset(bus, dec, &mut self.gpr),
                thumb::Instruction::STRRegOffset(dec) => exec_thumb_str_reg_offset(bus, dec, &mut self.gpr),
                thumb::Instruction::STRBRegOffset(dec) => exec_thumb_strb_reg_offset(bus, dec, &mut self.gpr),
                thumb::Instruction::STRHRegOffset(dec) => exec_thumb_strh_reg_offset(bus, dec, &mut self.gpr),
                thumb::Instruction::ADD1(dec) => exec_thumb_add1(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::ADD2(dec) => exec_thumb_add2(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::ADD3(dec) => exec_thumb_add3(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::ADDHiRegister(dec) => exec_thumb_add_hi_register(bus, dec, &mut self.gpr, &mut self.cpsr),
                // THUMB.12 6 nad 5
                thumb::Instruction::ADD6(dec) => exec_thumb_add_relative_address(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::ADD7(dec) => exec_thumb_add7(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::CMPThumb5(dec) => exec_thumb5_cmp(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::MOV3(dec) => exec_thumb_mov3(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::SUB1(dec) => exec_thumb_sub1(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::SUB3(dec) => exec_thumb_sub3(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::AND(dec) => exec_thumb_and(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::EOR(dec) => exec_thumb_eor(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::LSLThumb4(dec) => exec_thumb4_lsl(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::LSR2(dec) => exec_thumb_lsr2(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::ASRThumb1(dec) => exec_thumb1_asr(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::ASRThumb4(dec) => exec_thumb4_asr(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::ADCThumb4(dec) => exec_thumb4_adc(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::SBC(dec) => exec_thumb_sbc(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::RORThumb4(dec) => exec_thumb4_ror(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::TST(dec) => exec_thumb_tst(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::NEG(dec) => exec_thumb_neg(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::CMP1(dec) => exec_thumb_cmp1(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::CMP2(dec) => exec_thumb_cmp2(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::CMNThumb4(dec) => exec_thumb4_cmn(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::ORR(dec) => exec_thumb_orr(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::MUL(dec) => exec_thumb_mul(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::BIC(dec) => exec_thumb_bic(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::MVN(dec) => exec_thumb_mvn(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::LSLThumb1(dec) => exec_thumb1_lsl(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::LSRThumb1(dec) => exec_thumb1_lsr(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::MOV1(dec) => exec_thumb_mov1(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::SUB2(dec) => exec_thumb_sub2(bus, dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::B(dec) => exec_thumb_b(dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::B2(dec) => exec_thumb_b2(dec, &mut self.gpr, &mut self.cpsr),
                thumb::Instruction::BL(dec) => exec_thumb_bl1(bus, dec, &mut self.gpr),
                thumb::Instruction::BX(dec) => exec_thumb_bx(bus, dec, &mut self.cpsr, &mut self.gpr),
                thumb::Instruction::STMIA(dec) => exec_thumb_stmia(bus, dec, &mut self.gpr),
                thumb::Instruction::LDMIA(dec) => exec_thumb_ldmia(bus, dec, &mut self.gpr, started),
                thumb::Instruction::PUSH(dec) => exec_thumb_push(bus, dec, &mut self.gpr),
                thumb::Instruction::POP(dec) => exec_thumb_pop(bus, dec, &mut self.gpr),
                thumb::Instruction::SWI(dec) => {
                    // Cause: Thumb SWI was unimplemented, so BIOS calls failed and tests broke.
                    // This branch delegates to the Thumb SWI implementation.
                    let (c, ps) = exec_thumb_swi(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr);
                    (c, ps)
                }
                // Unknown/undefined Thumb encoding (decoded as `Undefined`) and
                // any unhandled variant: treat as a NOP rather than aborting.
                _ => (1, PipelineStatus::Continue),
            }
        };

        if self.gpr[15] % 2 == 1 {
            // // dbg!("aaaa!!!", &b, &self.gpr);
        }
        match pipeline_status {
            PipelineStatus::Continue => {
                self.increment_pc();
                // Advance prefetch buffer and fetch next2
                match self.cpsr.get_cpu_state() {
                    CpuState::ARM => {
                        let curr_addr = self.get_inst_addr() & 0xFFFF_FFFC;
                        let next2_addr = curr_addr.wrapping_add(8) & 0xFFFF_FFFC;
                        self.arm_pipe_curr = self.arm_pipe_next1.take();
                        self.arm_pipe_next1 = self.arm_pipe_next2.take();
                        let nv2 = bus.read_word(next2_addr);
                        self.arm_pipe_next2 = Some((next2_addr, nv2));
                    }
                    CpuState::Thumb => {
                        let curr_addr = self.get_inst_addr() & 0xFFFF_FFFE;
                        let next2_addr = curr_addr.wrapping_add(4) & 0xFFFF_FFFE;
                        self.thumb_pipe_curr = self.thumb_pipe_next1.take();
                        self.thumb_pipe_next1 = self.thumb_pipe_next2.take();
                        let nv2 = bus.read_halfword(next2_addr);
                        self.thumb_pipe_next2 = Some((next2_addr, nv2));
                    }
                }
                cycle
            }
            PipelineStatus::Flush => {
                self.flush_pipeline();
                cycle + self.wait_pipeline_filled(bus)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    trait CpuTest {
        fn run_immediately<T>(&mut self, bus: &mut T)
        where
            T: BusAccessor;
    }

    struct MockBus {
        pub mem: Vec<u8>,
    }

    impl MockBus {
        pub fn new() -> Self {
            Self { mem: vec![0; 1024] }
        }

        pub fn set(&mut self, addr: Word, data: Word) {
            let a = addr as usize;
            self.mem[a..a + 4].copy_from_slice(&data.to_le_bytes());
        }

        pub fn get_mem(&self, addr: usize) -> u32 {
            u32::from_le_bytes([self.mem[addr], self.mem[addr + 1], self.mem[addr + 2], self.mem[addr + 3]])
        }
    }

    impl BusAccessor for MockBus {
        fn read_byte(&self, addr: Word) -> Byte {
            self.mem[addr as usize]
        }

        fn read_halfword(&self, addr: Word) -> HalfWord {
            let a = addr as usize;
            u16::from_le_bytes([self.mem[a], self.mem[a + 1]])
        }

        fn read_word(&self, addr: Word) -> Word {
            let a = addr as usize;
            u32::from_le_bytes([self.mem[a], self.mem[a + 1], self.mem[a + 2], self.mem[a + 3]])
        }

        fn write_byte(&mut self, addr: Word, data: Byte) {
            self.mem[addr as usize] = data;
        }

        fn write_halfword(&mut self, addr: Word, data: HalfWord) {
            let a = addr as usize;
            self.mem[a..a + 2].copy_from_slice(&data.to_le_bytes());
        }

        fn write_word(&mut self, addr: Word, data: Word) {
            let a = addr as usize;
            self.mem[a..a + 4].copy_from_slice(&data.to_le_bytes());
        }

        fn compute_cycle(&self, addr: Word, access_type: AccessType) -> Cycle {
            let _ = (addr, access_type);
            1
        }
    }

    impl CpuTest for ARM {
        fn run_immediately<T>(&mut self, bus: &mut T)
        where
            T: BusAccessor,
        {
            for _ in 0..=INITIAL_PIPELINE_WAIT {
                let _ = self.step(bus, false);
            }
        }
    }

    fn setup() {
        use std::sync::Once;
        static INIT: Once = Once::new();
        INIT.call_once(|| {});
        // INIT.call_once(|| env_logger::init());
    }

    #[test]
    fn irq_exception_uses_banked_irq_sp_lr() {
        setup();
        let mut bus = MockBus::new();
        let mut arm = ARM::new();
        arm.reset();
        arm.set_gpr(PC, 0x0800_0008);
        arm.set_gpr(SP, 0x0300_7F00);
        arm.set_gpr(LR, 0xDEAD_BEEF);
        arm.request_irq();

        let _ = arm.step(&mut bus, false).unwrap();

        assert_eq!(arm.cpsr.get_mode(), crate::cpu::registers::psr::Mode::IRQ);
        assert_eq!(arm.get_gpr(PC), 0x0000_0018);
        assert_eq!(arm.get_gpr(SP), 0x0300_7FA0);
        assert_eq!(arm.get_gpr(LR), 0x0800_0004);
        assert_eq!(arm.spsr.get_mode(), crate::cpu::registers::psr::Mode::System);

        arm.cpsr.switch_mode(
            crate::cpu::registers::psr::Mode::System,
            &mut arm.gpr,
            &mut arm.spsr,
            &mut arm.bank_gpr,
            &mut arm.bank_spsr,
        );
        assert_eq!(arm.get_gpr(SP), 0x0300_7F00);
        assert_eq!(arm.get_gpr(LR), 0xDEAD_BEEF);
    }

    #[test]
    fn irq_line_is_level_sensitive_not_latched() {
        // Take one IRQ, then (as a handler would) acknowledge it so the line
        // drops before IRQs are re-enabled: no second exception may fire.
        setup();
        let mut bus = MockBus::new();
        let mut arm = ARM::new();
        arm.reset();
        arm.set_gpr(PC, 0x0800_0008);
        arm.set_irq_line(true);
        let _ = arm.step(&mut bus, false).unwrap();
        assert_eq!(arm.get_gpr(PC), 0x0000_0018);
        assert!(arm.cpsr.get_I());

        // Handler acknowledges IF: the level drops while I is still set.
        arm.set_irq_line(false);
        // Handler re-enables IRQs (e.g. `msr cpsr_c, #0x1f` for nesting).
        arm.cpsr.set_I(false);
        let pc_before = arm.get_gpr(PC);
        let _ = arm.step(&mut bus, false).unwrap();
        assert_ne!(arm.get_gpr(PC), 0x0000_0018, "stale IRQ must not fire");
        assert_eq!(arm.cpsr.get_mode(), crate::cpu::registers::psr::Mode::IRQ);
        assert!(arm.get_gpr(PC) > pc_before);
    }

    /// Run a Thumb snippet from address 0x100 with the given initial r0..r3
    /// and return the CPU afterwards. Each instruction is stepped exactly once.
    fn run_thumb_seq(code: &[u16], regs: [u32; 4]) -> ARM {
        let mut bus = MockBus::new();
        for (i, insn) in code.iter().enumerate() {
            let a = 0x100 + i * 2;
            bus.mem[a..a + 2].copy_from_slice(&insn.to_le_bytes());
        }
        let mut arm = ARM::new();
        arm.reset();
        arm.cpsr.set_cpu_state(CpuState::Thumb);
        arm.set_gpr(PC, 0x100);
        arm.flush_pipeline();
        for (i, r) in regs.iter().enumerate() {
            arm.set_gpr(i, *r);
        }
        for _ in 0..code.len() {
            arm.step(&mut bus, false).unwrap();
        }
        arm
    }

    #[test]
    fn thumb_add_imm3_sets_carry_on_wrap() {
        // adds r0, r0, #1 with r0 = 0xFFFF_FFFF -> 0, C=1, Z=1. Compilers emit
        // `bcc loop` after this to count a negative index up to zero.
        let arm = run_thumb_seq(&[0x1C40], [0xFFFF_FFFF, 0, 0, 0]);
        assert_eq!(arm.get_gpr(0), 0);
        assert!(arm.cpsr.get_C());
        assert!(arm.cpsr.get_Z());
        let arm = run_thumb_seq(&[0x1C40], [5, 0, 0, 0]);
        assert_eq!(arm.get_gpr(0), 6);
        assert!(!arm.cpsr.get_C());
    }

    #[test]
    fn thumb_neg_carry_only_when_operand_is_zero() {
        // neg r0, r1 == rsbs r0, r1, #0: C = NOT borrow(0 - r1).
        let arm = run_thumb_seq(&[0x4248], [0, 5, 0, 0]);
        assert_eq!(arm.get_gpr(0), (-5i32) as u32);
        assert!(!arm.cpsr.get_C());
        assert!(arm.cpsr.get_N());
        let arm = run_thumb_seq(&[0x4248], [0, 0, 0, 0]);
        assert_eq!(arm.get_gpr(0), 0);
        assert!(arm.cpsr.get_C());
        assert!(arm.cpsr.get_Z());
    }

    #[test]
    fn thumb_sbc_carry_is_not_borrow() {
        // cmp r2, r2 (C=1) ; sbc r0, r1
        let arm = run_thumb_seq(&[0x4292, 0x4188], [32, 16, 0, 0]);
        assert_eq!(arm.get_gpr(0), 16);
        assert!(arm.cpsr.get_C(), "32 - 16 - 0 does not borrow");
        // cmp r2, r3 with r2 < r3 (C=0) ; sbc r0, r1 -> 5 - 16 - 1 borrows
        let arm = run_thumb_seq(&[0x429A, 0x4188], [5, 16, 1, 2]);
        assert_eq!(arm.get_gpr(0), 5u32.wrapping_sub(17));
        assert!(!arm.cpsr.get_C());
        assert!(arm.cpsr.get_N());
    }

    #[test]
    // step
    fn increment_pc_by_tick() {
        setup();
        let mut bus = MockBus::new();
        let mut arm = ARM::new();
        let _ = arm.step(&mut bus, false);
        assert_eq!(arm.get_gpr(PC), 0x0000_000C);
    }

    // --- ARM instruction cycle-count validation (vs GBATEK) ---
    //
    // `MockBus` reports every memory access as 1 cycle, so an instruction's
    // measured steady-state cost here equals its GBATEK N/S/I cycle count with
    // each of N, S and I counted as 1. These tests pin the cycle *formula* of
    // each instruction class so a future change to wait-state / prefetch timing
    // (the cycle *values*) can't silently corrupt the underlying structure.

    /// Run `insn` repeatedly from PC=0 in flat 1-cycle memory and return the
    /// steady-state per-instruction cycle count (after the pipeline has filled).
    fn steady(insn: u32) -> Cycle {
        steady_with(insn, |_| {})
    }

    /// As [`steady`], but `setup` is applied to the CPU before each step so
    /// operand-dependent timing (e.g. MUL) can be measured with fixed operands.
    fn steady_with(insn: u32, setup: impl Fn(&mut ARM)) -> Cycle {
        let mut bus = MockBus::new();
        for i in 0..16 {
            bus.set((i * 4) as u32, insn);
        }
        let mut arm = ARM::new();
        arm.reset();
        arm.set_gpr(PC, 0);
        let mut last = 0;
        for _ in 0..12 {
            setup(&mut arm);
            last = arm.step(&mut bus, false).unwrap();
        }
        last
    }

    #[test]
    fn arm_cycle_counts_match_gbatek() {
        // Data processing: 1S; with a register-specified shift: +1I.
        assert_eq!(steady(0xE1A0_0000), 1, "MOV r0,r0 (DP) = 1S");
        assert_eq!(steady(0xE080_0000), 1, "ADD r0,r0,r0 (DP reg) = 1S");
        assert_eq!(steady(0xE080_0110), 2, "ADD r0,r0,r0 LSL r1 (DP reg-shift) = 1S+1I");

        // Single data transfer.
        assert_eq!(steady(0xE592_1000), 3, "LDR r1,[r2] = 1S+1N+1I");
        assert_eq!(steady(0xE582_1000), 2, "STR r1,[r2] = 2N");

        // Block data transfer: LDM = nS+1N+1I, STM = (n-1)S+2N.
        assert_eq!(steady(0xE890_0002), 3, "LDM {{r1}} = 1S+1N+1I");
        assert_eq!(steady(0xE880_0002), 2, "STM {{r1}} = 2N");
        assert_eq!(steady(0xE890_000E), 5, "LDM {{r1-r3}} = 3S+1N+1I");
        assert_eq!(steady(0xE880_000E), 4, "STM {{r1-r3}} = 2S+2N");

        // Taken branch: 2S+1N.
        assert_eq!(steady(0xEAFF_FFFE), 3, "B . (taken) = 2S+1N");
    }

    #[test]
    fn arm_mul_cycle_count_is_operand_dependent() {
        // MUL r3,r1,r2 = 1S + mI, where m (1..=4) depends on the multiplier Rs
        // (=r2): m grows as more of the top bits of Rs are neither all-0 nor
        // all-1. GBATEK: m=1 if bits[31:8] are all 0/1, m=2 if bits[31:16],
        // m=3 if bits[31:24], else m=4.
        const MUL_R3_R1_R2: u32 = 0xE003_0291;
        let with_rs = |rs: u32| steady_with(MUL_R3_R1_R2, move |arm| {
            arm.set_gpr(1, 0x1234_5678);
            arm.set_gpr(2, rs);
        });
        assert_eq!(with_rs(0x0000_0001), 2, "m=1 -> 1S+1I");
        assert_eq!(with_rs(0x0000_0100), 3, "m=2 -> 1S+2I");
        assert_eq!(with_rs(0x0001_0000), 4, "m=3 -> 1S+3I");
        assert_eq!(with_rs(0x0100_0000), 5, "m=4 -> 1S+4I");
    }

    #[test]
    // mov r0, #1
    fn mov_r0_imm1() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE3A0_0001);
        let mut arm = ARM::new();
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(0), 0x0000_0001);
    }

    #[test]
    // and r3, r1, r2
    // r3 <- r1 & r2
    fn and_r3_r1_r2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE001_3002);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0xAA55_55AA);
        arm.set_gpr(2, 0xA050_1122);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(3), 0xA050_1122);
    }

    #[test]
    // eor r3, r1, r2
    // r3 <- r1 ^ r2
    fn eor_r3_r1_r2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE021_3002);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0xAA55_55AA);
        arm.set_gpr(2, 0xA050_1122);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(3), 0x0A05_4488);
    }

    #[test]
    // sub r3, r1, r2
    // r3 <- r1 - r2
    fn sub_r3_r1_r2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE041_3002);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0xAA55_5588);
        arm.set_gpr(2, 0xA050_1122);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(3), 0x0A05_4466);
    }

    #[test]
    // rsb r3, r1, r2
    // r3 <- r2 - r1
    fn rsb_r3_r1_r2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE061_3002);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0x1234_5678);
        arm.set_gpr(2, 0x2345_6789);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(3), 0x1111_1111);
    }

    #[test]
    // add r3, r1, r2
    // r3 <- r1 + r2
    fn add_r3_r1_r2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE081_3002);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0x1234_5678);
        arm.set_gpr(2, 0x2345_6789);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(3), 0x3579_BE01);
    }

    #[test]
    // adc r3, r1, r2
    // r3 <- r1 + r2 + C
    fn adc_r3_r1_r2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE0A1_3002);
        let mut arm = ARM::new();
        arm.cpsr.set_C(true);
        arm.set_gpr(1, 0x1234_5678);
        arm.set_gpr(2, 0x2345_6789);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(3), 0x3579_BE02);
    }

    #[test]
    // sbc r3, r1, r2
    // r3 <- r1 - r2 - !C
    fn sbc_r3_r1_r2_with_set_c() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE0C1_3002);
        let mut arm = ARM::new();
        arm.cpsr.set_C(true);
        arm.set_gpr(1, 0x2345_6789);
        arm.set_gpr(2, 0x1234_5678);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(3), 0x1111_1111);
    }

    #[test]
    // sbc r3, r1, r2
    // r3 <- r1 - r2 - !C
    fn sbc_r3_r1_r2_with_cleared_c() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE0C1_3002);
        let mut arm = ARM::new();
        arm.cpsr.set_C(false);
        arm.set_gpr(1, 0x2345_6789);
        arm.set_gpr(2, 0x1234_5678);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(3), 0x1111_1110);
    }

    #[test]
    // rsc r3, r1, r2
    // r3 <- r2 - r1 -!C
    fn rsc_r3_r1_r2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE061_3002);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0x1234_5678);
        arm.set_gpr(2, 0x2345_6789);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(3), 0x1111_1111);
    }

    #[test]
    // tst r0, r1
    fn tst_r0_r1() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE110_0001);
        let mut arm = ARM::new();
        arm.set_gpr(0, 0x8234_5678);
        arm.set_gpr(1, 0x8345_6789);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_cpsr().get_C(), false);
        assert_eq!(arm.get_cpsr().get_N(), true);
        assert_eq!(arm.get_cpsr().get_Z(), false);
    }

    #[test]
    // tst r1, r2, asr #4
    fn tst_r1_r2_asr_4_without_zero() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE111_0242);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0x8234_5678);
        arm.set_gpr(2, 0x80FF_0008);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_cpsr().get_C(), true);
        assert_eq!(arm.get_cpsr().get_N(), true);
        assert_eq!(arm.get_cpsr().get_Z(), false);
    }

    #[test]
    // tst r1, r2, asr #4
    fn tst_r1_r2_asr_4_with_zero() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE111_0242);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0x8234_5678);
        arm.set_gpr(2, 0x0000_0000);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_cpsr().get_C(), false);
        assert_eq!(arm.get_cpsr().get_N(), false);
        assert_eq!(arm.get_cpsr().get_Z(), true);
    }

    #[test]
    // teq r1, r2
    fn tst_r1_r2_equal() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE131_0002);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0x8234_5678);
        arm.set_gpr(2, 0x8234_5678);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_cpsr().get_C(), false);
        assert_eq!(arm.get_cpsr().get_N(), false);
        assert_eq!(arm.get_cpsr().get_Z(), true);
    }

    #[test]
    // teq r1, r2
    fn tst_r1_r2_not_equal() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE131_0002);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0x8234_5678);
        arm.set_gpr(2, 0x0234_5678);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_cpsr().get_C(), false);
        assert_eq!(arm.get_cpsr().get_N(), true);
        assert_eq!(arm.get_cpsr().get_Z(), false);
    }

    #[test]
    // cmp r1, r2
    fn cmp_r1_r2_carry() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE151_0002);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0x0000_0002);
        arm.set_gpr(2, 0x0000_0001);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_cpsr().get_C(), true);
        assert_eq!(arm.get_cpsr().get_N(), false);
        assert_eq!(arm.get_cpsr().get_Z(), false);
        assert_eq!(arm.get_cpsr().get_V(), false);
    }

    #[test]
    // cmp r1, r2
    fn cmp_r1_r2_without_carry() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE151_0002);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0x0000_0001);
        arm.set_gpr(2, 0x0000_0002);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_cpsr().get_C(), false);
        assert_eq!(arm.get_cpsr().get_N(), true);
        assert_eq!(arm.get_cpsr().get_Z(), false);
        assert_eq!(arm.get_cpsr().get_V(), false);
    }

    #[test]
    // cmp r1, r2
    fn cmp_r1_r2_with_overflow() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE151_0002);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0x8000_0000);
        arm.set_gpr(2, 0x0000_0001);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_cpsr().get_C(), true);
        assert_eq!(arm.get_cpsr().get_N(), false);
        assert_eq!(arm.get_cpsr().get_Z(), false);
        assert_eq!(arm.get_cpsr().get_V(), true);
    }

    #[test]
    // cmn r1, r2
    fn cmn_r1_r2_with_overflow() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE171_0001);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0x7FFF_FFFF);
        arm.set_gpr(2, 0x0000_0001);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_cpsr().get_C(), false);
        assert_eq!(arm.get_cpsr().get_N(), true);
        assert_eq!(arm.get_cpsr().get_Z(), false);
        assert_eq!(arm.get_cpsr().get_V(), true);
    }

    #[test]
    // orr r1, r2, r3
    fn orr_r1_r2_r3() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE182_1003);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0xAA55_55AA);
        arm.set_gpr(3, 0x5500_AA00);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0xFF55_FFAA);
    }

    #[test]
    // lsl r1, r2, #16
    fn lsl_r1_r2_16() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE1A0_1802);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0x0000_AA55);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0xAA55_0000);
    }

    #[test]
    // lsr r1, r2, #16
    fn lsr_r1_r2_16() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE1A0_1822);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0x00AA_AA55);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0x0000_00AA);
    }

    #[test]
    // asr r1, r2, #16
    fn asr_r1_r2_16() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE1A0_1842);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0x80AA_AA55);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0xFFFF_80AA);
    }

    #[test]
    // rrx r2, r1
    fn rrx_r2_r1() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE1A0_2061);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0x0000_007b);
        arm.cpsr.set_C(false);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(2), 0x0000_003d);
    }

    #[test]
    // ror r1, r2, #16
    fn ror_r1_r2_16() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE1A0_1862);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0x00AA_AA55);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0xAA55_00AA);
    }

    #[test]
    // bic r1, r2, r3
    fn bic_r1_r2_r3() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE1C2_1003);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0x00AA_AA55);
        arm.set_gpr(3, 0x00AA_AAAA);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0x0000_0055);
    }

    #[test]
    // mvn r1, r2
    fn mvn_r1_r2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE1E0_1002);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0x00AA_AA55);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0xFF55_55AA);
    }

    #[test]
    // mul r1, r2, r3
    fn mul_r1_r2_r3() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE001_0392);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0xF000_0000);
        arm.set_gpr(3, 2);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0xE000_0000);
    }

    #[test]
    // mla r1, r2, r3, r4
    fn mla_r1_r2_r3_r4() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE021_4392);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0xF000_0000);
        arm.set_gpr(3, 2);
        arm.set_gpr(4, 0xAA55);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0xE000_AA55);
    }

    #[test]
    // umull r1, r2, r3, r4
    fn umull_r1_r2_r3_r4() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE082_1493);
        let mut arm = ARM::new();
        arm.set_gpr(3, 0x7000_0001);
        arm.set_gpr(4, 0x0070_0000);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0x0070_0000);
        assert_eq!(arm.get_gpr(2), 0x0031_0000);
    }

    #[test]
    // umlal r1, r2, r3, r4
    fn umlal_r1_r2_r3_r4() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE0A2_1493);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0x0000_0001);
        arm.set_gpr(2, 0x0000_0002);
        arm.set_gpr(3, 0x7000_0001);
        arm.set_gpr(4, 0x0070_0000);
        arm.run_immediately(&mut bus);
        // assert_eq!(arm.get_gpr(1), 0x0070_0001);
        assert_eq!(arm.get_gpr(2), 0x0031_0002);
    }

    #[test]
    // smull r1, r2, r3, r4
    fn smull_r1_r2_r3_r4() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE0C2_1493);
        let mut arm = ARM::new();
        arm.set_gpr(3, 0xFFFF_FFFE);
        arm.set_gpr(4, 0x7FFF_FFFF);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0x0000_0002);
        assert_eq!(arm.get_gpr(2), 0xFFFF_FFFF);
    }

    #[test]
    // smlal r1, r2, r3, r4
    fn smlal_r1_r2_r3_r4() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE0E2_1493);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0xFFFF_FFFF);
        arm.set_gpr(2, 0xFFFF_FFFF);
        arm.set_gpr(3, 0xFFFF_FFFE);
        arm.set_gpr(4, 0x7FFF_FFFF);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0x0000_0001);
        assert_eq!(arm.get_gpr(2), 0xFFFF_FFFF);
    }

    #[test]
    // ldr pc, =0x8000_0000
    fn ldr_pc_eq0x8000_0000() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE51F_F004);
        bus.set(0x4, 0x0000_0010);
        let mut arm = ARM::new();
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(PC), 0x0000_0020);
    }

    #[test]
    // LDR offset addressing
    // ldrb r1, [r0]
    fn ldrb_r1_r0() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE5D0_1000);
        bus.set(0x100, 0xAAAA_5555);
        let mut arm = ARM::new();
        arm.set_gpr(0, 0x100);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0x55);
        assert_eq!(arm.get_gpr(0), 0x0000_0100);
    }

    #[test]
    // LDR post index addressing
    // ldr	r0, [r1], #4
    fn ldrb_r0_r1_4() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE491_0004);
        bus.set(0x100, 0xAAAA_5555);
        let mut arm = ARM::new();
        arm.set_gpr(1, 0x100);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(0), 0xAAAA_5555);
        assert_eq!(arm.get_gpr(1), 0x0104);
    }

    #[test]
    // ldr r8, [r9, r2, lsl #2]
    // R8 <- mem[r9 + (r2 << 2)]
    fn ldr_r8_r9_r2_lsl_2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE799_8102);
        bus.set(0x140, 0xAA55_55AA);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0x10);
        arm.set_gpr(9, 0x100);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(8), 0xAA55_55AA);
    }

    #[test]
    // str r4, [r3]
    fn str_r4_r3() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE583_4000);
        let mut arm = ARM::new();
        arm.set_gpr(3, 0x200);
        arm.set_gpr(4, 0xAA55_55AA);
        arm.run_immediately(&mut bus);
        assert_eq!(bus.get_mem(0x200), 0xAA55_55AA);
    }

    #[test]
    // strb r4, [r3]
    fn strb_r4_r3() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE5C3_4000);
        let mut arm = ARM::new();
        arm.set_gpr(3, 0x200);
        arm.set_gpr(4, 0x1155_55AA);
        arm.run_immediately(&mut bus);
        assert_eq!(bus.get_mem(0x200), 0x0000_00AA);
    }

    #[test]
    // strh r1, [r2]
    fn strh_r1_r2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE1C2_10B0);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0x200);
        arm.set_gpr(1, 0x1155_55AA);
        arm.run_immediately(&mut bus);
        assert_eq!(bus.get_mem(0x200), 0x0000_55AA);
    }

    #[test]
    // strh r1, [r2, 0xff]
    fn strh_r1_r2_0xff() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE1C2_1FBF);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0x200);
        arm.set_gpr(1, 0x1155_55AA);
        arm.run_immediately(&mut bus);
        // STRH force-aligns the address (0x2FF -> 0x2FE) on ARM7TDMI.
        assert_eq!(bus.get_mem(0x2FE), 0x0000_55AA);
    }

    #[test]
    // mrs r1, spsr  (in System mode, which has no banked SPSR)
    fn mrs_spsr_in_system_mode_reads_cpsr() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE14F_1000); // mrs r1, spsr
        let mut arm = ARM::new();
        arm.cpsr.set_mode(crate::cpu::registers::psr::Mode::System);
        // Put a distinct value in the (nonexistent) SPSR to prove it is not read.
        arm.spsr.set(0x0000_00F0);
        arm.run_immediately(&mut bus);
        // In User/System mode MRS spsr returns the CPSR, not the SPSR sentinel.
        assert_eq!(arm.get_gpr(1), arm.cpsr.get());
        assert_ne!(arm.get_gpr(1), 0x0000_00F0);
    }

    #[test]
    // ldrh r1, [r2]
    fn ldrh_r1_r2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE1D2_10B0);
        bus.set(0x200, 0xA5A5_5A5A);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0x200);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0x0000_5A5A);
    }

    #[test]
    // ldrsb r1, [r2]
    fn ldrsb_r1_r2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE1D2_10D0);
        bus.set(0x200, 0xA5A5_5AFF);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0x200);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0xFFFF_FFFF);
    }

    #[test]
    // ldrsh r1, [r2]
    fn ldrsh_r1_r2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, 0xE1D2_10D0);
        bus.set(0x200, 0xA5A5_FFFE);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0x200);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(1), 0xFFFF_FFFE);
    }

    #[test]
    // b pc-2
    fn b_pc_sub_2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0000_0000, 0xEAFF_FFFE);
        let mut arm = ARM::new();
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(PC), 0x0000_0008);
    }

    #[test]
    // bl pc-2
    fn bl_pc_sub_2() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0000_0000, 0xEBFF_FFFE);
        let mut arm = ARM::new();
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(PC), 0x0000_0008);
        assert_eq!(arm.get_gpr(LR), 0x0000_0004);
    }

    #[test]
    // ldm r0!, {r4-r11}
    // Load 8 words from the source
    fn ldm_r0_r4_r11() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0000_0000, 0xE8B0_0FF0);
        for i in 0..0x10 {
            bus.set(0x100 + (i * 4), 0xA000_0000 + i);
        }
        let mut arm = ARM::new();
        arm.set_gpr(0, 0x100);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(PC), 0x0000_0014);
        assert_eq!(arm.get_gpr(0), 0x0000_0120);
        assert_eq!(arm.get_gpr(4), 0xA000_0000);
        assert_eq!(arm.get_gpr(5), 0xA000_0001);
        assert_eq!(arm.get_gpr(6), 0xA000_0002);
        assert_eq!(arm.get_gpr(7), 0xA000_0003);
        assert_eq!(arm.get_gpr(8), 0xA000_0004);
        assert_eq!(arm.get_gpr(9), 0xA000_0005);
        assert_eq!(arm.get_gpr(10), 0xA000_0006);
        assert_eq!(arm.get_gpr(11), 0xA000_0007);
        assert_eq!(arm.get_gpr(12), 0x0000_0000);
    }

    #[test]
    // stm r0!, {r4-r11}
    // Store 8 words from the source
    fn stm_r0_r4_r11() {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0000_0000, 0xE8A0_0FF0);
        let mut arm = ARM::new();
        arm.set_gpr(0, 0x100);
        for i in 0..8 {
            arm.set_gpr(4 + i, 0xA000_0000 + i as u32);
        }
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(PC), 0x0000_0014);
        assert_eq!(arm.get_gpr(0), 0x0000_0120);
        assert_eq!(bus.get_mem(0x0000_0100), 0xA000_0000);
        assert_eq!(bus.get_mem(0x0000_0104), 0xA000_0001);
        assert_eq!(bus.get_mem(0x0000_0108), 0xA000_0002);
        assert_eq!(bus.get_mem(0x0000_010C), 0xA000_0003);
        assert_eq!(bus.get_mem(0x0000_0110), 0xA000_0004);
        assert_eq!(bus.get_mem(0x0000_0114), 0xA000_0005);
        assert_eq!(bus.get_mem(0x0000_0118), 0xA000_0006);
        assert_eq!(bus.get_mem(0x0000_011c), 0xA000_0007);
    }

    // --- Thumb ALU flag regressions ---
    //
    // Run a single Thumb instruction (padded with a NOP `mov r8, r8`) from
    // address 0 with the CPU already in Thumb state.
    fn run_thumb(op: u16, init: impl FnOnce(&mut ARM)) -> ARM {
        setup();
        let mut bus = MockBus::new();
        bus.set(0x0, u32::from(op) | (0x46C0 << 16)); // op; nop
        bus.set(0x4, 0x46C0_46C0);
        let mut arm = ARM::new();
        arm.cpsr.set_cpu_state(crate::cpu::registers::psr::CpuState::Thumb);
        init(&mut arm);
        arm.run_immediately(&mut bus);
        arm
    }

    #[test]
    // neg r3, r0 (rsbs r3, r0, #0) with r0 = 3: 0 - 3 borrows, so C must be
    // clear. GCC relies on this for `x == K` idioms (`rsbs; adcs`), and
    // BPCore-Engine's Lua parser mis-parsed every script when C was set here.
    fn thumb_neg_positive_operand_clears_carry() {
        let arm = run_thumb(0x4243, |arm| {
            arm.cpsr.set_C(true);
            arm.set_gpr(0, 3);
        });
        assert_eq!(arm.get_gpr(3), 0xFFFF_FFFD);
        assert!(!arm.get_cpsr().get_C());
        assert!(arm.get_cpsr().get_N());
        assert!(!arm.get_cpsr().get_Z());
        assert!(!arm.get_cpsr().get_V());
    }

    #[test]
    // neg r3, r0 with r0 = 0: no borrow, C set, Z set.
    fn thumb_neg_zero_operand_sets_carry_and_zero() {
        let arm = run_thumb(0x4243, |arm| {
            arm.cpsr.set_C(false);
            arm.set_gpr(0, 0);
        });
        assert_eq!(arm.get_gpr(3), 0);
        assert!(arm.get_cpsr().get_C());
        assert!(arm.get_cpsr().get_Z());
        assert!(!arm.get_cpsr().get_N());
        assert!(!arm.get_cpsr().get_V());
    }

    #[test]
    // neg r3, r0 with r0 = 0x8000_0000: result wraps to itself, borrow (C=0)
    // and signed overflow (V=1). Must not panic in debug builds either.
    fn thumb_neg_int_min_sets_overflow() {
        let arm = run_thumb(0x4243, |arm| {
            arm.set_gpr(0, 0x8000_0000);
        });
        assert_eq!(arm.get_gpr(3), 0x8000_0000);
        assert!(!arm.get_cpsr().get_C());
        assert!(arm.get_cpsr().get_V());
        assert!(arm.get_cpsr().get_N());
    }

    #[test]
    // neg r3, r0 with r0 = -5: 0 - (-5) = 5, borrow as unsigned, so C=0.
    fn thumb_neg_negative_operand_clears_carry() {
        let arm = run_thumb(0x4243, |arm| {
            arm.set_gpr(0, 0xFFFF_FFFB);
        });
        assert_eq!(arm.get_gpr(3), 5);
        assert!(!arm.get_cpsr().get_C());
        assert!(!arm.get_cpsr().get_V());
    }

    #[test]
    // sbc r3, r0 (r3 = r3 - r0 - !C): 10 - 3 - 0 with C set -> 7, no borrow, C=1.
    fn thumb_sbc_no_borrow_sets_carry() {
        let arm = run_thumb(0x4183, |arm| {
            arm.cpsr.set_C(true);
            arm.set_gpr(3, 10);
            arm.set_gpr(0, 3);
        });
        assert_eq!(arm.get_gpr(3), 7);
        assert!(arm.get_cpsr().get_C());
    }

    #[test]
    // sbc r3, r0: 3 - 10 - 1 with C clear -> borrow, C=0.
    fn thumb_sbc_borrow_clears_carry() {
        let arm = run_thumb(0x4183, |arm| {
            arm.cpsr.set_C(false);
            arm.set_gpr(3, 3);
            arm.set_gpr(0, 10);
        });
        assert_eq!(arm.get_gpr(3), 0xFFFF_FFF8);
        assert!(!arm.get_cpsr().get_C());
    }
}
