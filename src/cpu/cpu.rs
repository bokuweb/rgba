use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::{arm, thumb};
use crate::cpu::instructions::arm::{
    block_data_transfer::*, branch::*, branch_and_exchange::*, data::*, extra_memory::*, memory::*, multiple::*, psr_transfer::*, single_data_swap::*,
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
}

impl ARM {
    pub fn new() -> ARM {
        ARM {
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
        }
    }

    pub fn reset(&mut self) {
        self.gpr[PC] = 0x00000000;

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

    fn flush_pipeline(&mut self) {
        self.pipeline_wait = INITIAL_PIPELINE_WAIT;
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
        if self.cpsr.get_cpu_state() == CpuState::ARM {
            self.gpr[PC] - (PC_OFFSET * 4) as Word
        } else {
            self.gpr[PC] - (PC_OFFSET * 2) as Word
        }
    }

    fn get_prefetch_width(&self) -> AccessWidth {
        if self.cpsr.get_cpu_state() == CpuState::ARM {
            AccessWidth::Word
        } else {
            AccessWidth::HalfWord
        }
    }

    pub fn get_gpr(&self, n: usize) -> Word {
        self.gpr[n]
    }

    pub fn get_cpsr(&self) -> PSR {
        self.cpsr
    }

    pub fn set_gpr(&mut self, n: usize, data: u32) {
        self.gpr[n] = data;
    }

    pub fn request_irq(&mut self) {
        self.irq_pending = true;
        println!("🔥 IRQ requested and pending flag set");
    }

    fn handle_irq<T>(&mut self, bus: &mut T) -> Cycle
    where
        T: BusAccessor,
    {
        println!("🔥 Handling IRQ - switching to IRQ mode");
        
        // Save current CPSR to SPSR_irq
        self.spsr = self.cpsr;
        
        // Save current PC (return address) to LR
        // For IRQ, return address should be current PC (instruction being interrupted)
        let return_addr = if self.cpsr.get_cpu_state() == crate::cpu::registers::psr::CpuState::Thumb {
            self.gpr[PC] - 2 // Thumb mode: adjust for 2-byte instruction pipeline
        } else {
            self.gpr[PC] - 4 // ARM mode: adjust for 4-byte instruction pipeline
        };
        self.gpr[LR] = return_addr;
        
        // Switch to IRQ mode and disable IRQ in CPSR
        self.cpsr.set_mode(crate::cpu::registers::psr::Mode::IRQ);
        self.cpsr.set_I(true); // Disable IRQ
        self.cpsr.set_T(false); // Switch to ARM mode (IRQ handlers are always ARM)
        
        // Jump to IRQ vector (0x18)
        self.gpr[PC] = 0x18;
        self.flush_pipeline();
        
        // Clear pending IRQ
        self.irq_pending = false;
        
        println!("🔥 IRQ handler setup complete: PC=0x{:x}, LR=0x{:x}, CPSR.I={}", 
            self.gpr[PC], self.gpr[LR], self.cpsr.get_I());
        
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
        return next + cycle;
    }

    pub fn step<T>(&mut self, bus: &mut T, started: bool) -> Result<Cycle, ()>
    where
        T: BusAccessor,
    {
        // Check for pending IRQ before executing instruction
        if self.irq_pending && !self.cpsr.get_I() {
            println!("🔥 Processing pending IRQ");
            let irq_cycle = self.handle_irq(bus);
            return Ok(irq_cycle);
        }
        
        let cycle = if self.pipeline_wait > 0 { self.wait_pipeline_filled(bus) } else { 0 };
        // let log = format!("{:?}", self.gpr);
        // dbg!(&self.gpr);
        if self.gpr[15] == 134225604 {
            // panic!("aa")
        }

        match self.cpsr.get_cpu_state() {
            CpuState::ARM => {
                // dbg!(&self.gpr);

                if self.gpr[15] == 134220600 {
                    dbg!("hello", self.cpsr.get_Z());
                }
                let fetched = self.get_arm_executable(bus);
                let cond: Cond = fetched.wrapping_shr(28).into();
                let condition_result = self.cpsr.condition_ok(cond);
                // Log SWI fetch regardless of PC range to verify decode/cond behavior
                if (fetched & 0x0F00_0000) == 0x0F00_0000 {
                    println!(
                        "SWI fetched: PC=0x{:08X}, instr=0x{:08X}, cond={:?}, CPSR=0x{:08X}, cond_ok={}",
                        self.gpr[15] - 8,
                        fetched,
                        cond,
                        self.cpsr.get(),
                        condition_result
                    );
                }
                if self.gpr[15] >= 134217728 && self.gpr[15] <= 134225000 {
                    println!("ARM: PC=0x{:08X}, instr=0x{:08X}, cond={:?}, CPSR=0x{:08X}, condition_ok={}", 
                        self.gpr[15] - 8, fetched, cond, self.cpsr.get(), condition_result);
                }
                if !condition_result {
                    let s = bus.compute_cycle(self.gpr[PC], AccessType::Seq(AccessWidth::Word));
                    self.increment_pc();
                    return Ok(s + cycle);
                }
                let instruction = arm::decode(fetched);
                if (fetched & 0x0F00_0000) == 0x0F00_0000 {
                    println!(
                        "Decoded at SWI site: PC=0x{:08X}, instr=0x{:08X}, variant={:?}",
                        self.gpr[15] - 8,
                        fetched,
                        instruction
                    );
                }
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
        bus.read_word(self.get_inst_addr())
    }

    fn get_thumb_executable<T>(&mut self, bus: &mut T) -> HalfWord
    where
        T: BusAccessor,
    {
        bus.read_halfword(self.get_inst_addr())
    }

    fn execute_arm<T>(&mut self, instruction: arm::Instruction, bus: &mut T) -> Result<Cycle, ()>
    where
        T: BusAccessor,
    {
        // // dbg!(&instruction, &self.gpr);
        //if (self.gpr[15] >= 134221712 && self.gpr[15] <= 134221748) {
        //         dbg!(&self.gpr);
        //     // dbg!("0");

        // if self.gpr[15] >= 134225848 && self.gpr[15] <= 134224860 {
        //     dbg!('🔥', &instruction, &self.gpr);
        // }
        // }
        let (cycle, pipeline_status) = {
            if let arm::Instruction::SWI = &instruction {
                println!("about to execute SWI (will unimplemented!)");
            }
            match instruction {
                arm::Instruction::AND(dec) => exec_arm_and(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::EOR(dec) => exec_arm_eor(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::SUB(dec) => exec_arm_sub(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::RSB(dec) => exec_arm_rsb(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::ADD(dec) => exec_arm_add(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::ADC(dec) => exec_arm_adc(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::SBC(dec) => exec_arm_sbc(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::RSC(dec) => exec_arm_rsc(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::TST(dec) => exec_arm_tst(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::TEQ(dec) => exec_arm_teq(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::CMP(dec) => exec_arm_cmp(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::CMN(dec) => exec_arm_cmn(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::ORR(dec) => exec_arm_orr(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::MOV(dec) => exec_arm_mov(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::LSL(dec) => exec_arm_shift(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::LSR(dec) => exec_arm_shift(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::ASR(dec) => exec_arm_shift(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::RRX(dec) => exec_arm_rrx(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::ROR(dec) => exec_arm_shift(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::BIC(dec) => exec_arm_bic(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::MVN(dec) => exec_arm_mvn(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::MUL(dec) => exec_arm_mul(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::MLA(dec) => exec_arm_mla(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::UMULL(dec) => exec_arm_umull(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::UMLAL(dec) => exec_arm_umlal(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::SMULL(dec) => exec_arm_smull(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::SMLAL(dec) => exec_arm_smlal(bus, dec, &mut self.gpr, &mut self.cpsr)?,
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
                arm::Instruction::STM(dec) => exec_arm_stm(bus, dec, &mut self.gpr)?,
                arm::Instruction::MRS(dec) => exec_arm_mrs(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr)?,
                arm::Instruction::MSR(dec) => exec_arm_msr(bus, dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr, &mut self.bank_gpr, &mut self.bank_spsr)?,
                arm::Instruction::SWP(dec) => exec_arm_swp(bus, dec, &mut self.gpr)?,
                arm::Instruction::SWPB(dec) => exec_arm_swpb(bus, dec, &mut self.gpr)?,
                arm::Instruction::Undefined => unimplemented!(),
                arm::Instruction::SWI => unimplemented!(),
                // ArmOpcode::Unknown => self.execute_unknown(dec),
                _ => unimplemented!(),
            }
        };
        match pipeline_status {
            PipelineStatus::Continue => {
                self.increment_pc();
                Ok(cycle)
            }
            PipelineStatus::Flush => {
                self.flush_pipeline();
                Ok(cycle + self.wait_pipeline_filled(bus))
            }
        }
    }

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
                thumb::Instruction::STRB_IMM_OFFET(dec) => exec_thumb_strb_imm_offset(bus, dec, &mut self.gpr),
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
                _ => {
                    // dbg!(&instruction, &self.gpr);
                    unimplemented!();
                }
            }
        };

        if self.gpr[15] % 2 == 1 {
            // // dbg!("aaaa!!!", &b, &self.gpr);
        }
        match pipeline_status {
            PipelineStatus::Continue => {
                self.increment_pc();
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
    extern crate byteorder;
    // extern crate env_logger;

    use super::*;
    use byteorder::{ByteOrder, LittleEndian};

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
            MockBus { mem: vec![0; 1024] }
        }

        pub fn set(&mut self, addr: Word, data: Word) {
            LittleEndian::write_u32(&mut self.mem[(addr as usize)..], data);
        }

        pub fn get_mem(&self, addr: usize) -> u32 {
            LittleEndian::read_u32(&self.mem[(addr as usize)..])
        }
    }

    impl BusAccessor for MockBus {
        fn read_byte(&self, addr: Word) -> Byte {
            self.mem[addr as usize]
        }

        fn read_halfword(&self, addr: Word) -> HalfWord {
            LittleEndian::read_u16(&self.mem[(addr as usize)..])
        }

        fn read_word(&self, addr: Word) -> Word {
            LittleEndian::read_u32(&self.mem[(addr as usize)..])
        }

        fn write_byte(&mut self, addr: Word, data: Byte) {
            self.mem[(addr as usize)] = data;
        }

        fn write_halfword(&mut self, addr: Word, data: HalfWord) {
            LittleEndian::write_u16(&mut self.mem[(addr as usize)..], data);
        }

        fn write_word(&mut self, addr: Word, data: Word) {
            LittleEndian::write_u32(&mut self.mem[(addr as usize)..], data);
        }

        fn compute_cycle(&self, addr: Word, access_type: AccessType) -> Cycle {
            1
        }
    }

    impl CpuTest for ARM {
        fn run_immediately<T>(&mut self, bus: &mut T)
        where
            T: BusAccessor,
        {
            for _ in 0..(INITIAL_PIPELINE_WAIT + 1) {
                self.step(bus, false);
            }
        }
    }

    fn setup() {
        use std::sync::{Once, ONCE_INIT};
        static INIT: Once = ONCE_INIT;
        // INIT.call_once(|| env_logger::init());
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

    #[test]
    // mov r0, #1
    fn mov_r0_imm1() {
        setup();
        let mut bus = MockBus::new();
        &bus.set(0x0, 0xE3A0_0001);
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
        &bus.set(0x0, 0xE001_3002);
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
        &bus.set(0x0, 0xE021_3002);
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
        &bus.set(0x0, 0xE041_3002);
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
        &bus.set(0x0, 0xE061_3002);
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
        &bus.set(0x0, 0xE081_3002);
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
        &bus.set(0x0, 0xE0A1_3002);
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
        &bus.set(0x0, 0xE0C1_3002);
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
        &bus.set(0x0, 0xE0C1_3002);
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
        &bus.set(0x0, 0xE061_3002);
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
        &bus.set(0x0, 0xE110_0001);
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
        &bus.set(0x0, 0xE111_0242);
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
        &bus.set(0x0, 0xE111_0242);
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
        &bus.set(0x0, 0xE131_0002);
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
        &bus.set(0x0, 0xE131_0002);
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
        &bus.set(0x0, 0xE151_0002);
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
        &bus.set(0x0, 0xE151_0002);
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
        &bus.set(0x0, 0xE151_0002);
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
        &bus.set(0x0, 0xE171_0001);
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
        &bus.set(0x0, 0xE182_1003);
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
        &bus.set(0x0, 0xE1A0_1802);
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
        &bus.set(0x0, 0xE1A0_1822);
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
        &bus.set(0x0, 0xE1A0_1842);
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
        &bus.set(0x0, 0xE1A0_2061);
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
        &bus.set(0x0, 0xE1A0_1862);
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
        &bus.set(0x0, 0xE1C2_1003);
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
        &bus.set(0x0, 0xE1E0_1002);
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
        &bus.set(0x0, 0xE001_0392);
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
        &bus.set(0x0, 0xE021_4392);
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
        &bus.set(0x0, 0xE082_1493);
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
        &bus.set(0x0, 0xE0A2_1493);
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
        &bus.set(0x0, 0xE0C2_1493);
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
        &bus.set(0x0, 0xE0E2_1493);
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
        &bus.set(0x0, 0xE51F_F004);
        &bus.set(0x4, 0x0000_0010);
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
        &bus.set(0x0, 0xE5D0_1000);
        &bus.set(0x100, 0xAAAA_5555);
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
        &bus.set(0x0, 0xE491_0004);
        &bus.set(0x100, 0xAAAA_5555);
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
        &bus.set(0x0, 0xE799_8102);
        &bus.set(0x140, 0xAA55_55AA);
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
        &bus.set(0x0, 0xE583_4000);
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
        &bus.set(0x0, 0xE5C3_4000);
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
        &bus.set(0x0, 0xE1C2_10B0);
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
        &bus.set(0x0, 0xE1C2_1FBF);
        let mut arm = ARM::new();
        arm.set_gpr(2, 0x200);
        arm.set_gpr(1, 0x1155_55AA);
        arm.run_immediately(&mut bus);
        assert_eq!(bus.get_mem(0x2FF), 0x0000_55AA);
    }

    #[test]
    // ldrh r1, [r2]
    fn ldrh_r1_r2() {
        setup();
        let mut bus = MockBus::new();
        &bus.set(0x0, 0xE1D2_10B0);
        &bus.set(0x200, 0xA5A5_5A5A);
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
        &bus.set(0x0, 0xE1D2_10D0);
        &bus.set(0x200, 0xA5A5_5AFF);
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
        &bus.set(0x0, 0xE1D2_10D0);
        &bus.set(0x200, 0xA5A5_FFFE);
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
        &bus.set(0x0000_0000, 0xEAFF_FFFE);
        let mut arm = ARM::new();
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(PC), 0x0000_0008);
    }

    #[test]
    // bl pc-2
    fn bl_pc_sub_2() {
        setup();
        let mut bus = MockBus::new();
        &bus.set(0x0000_0000, 0xEBFF_FFFE);
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
        &bus.set(0x0000_0000, 0xE8B0_0FF0);
        for i in 0..0x10 {
            &bus.set(0x100 + (i * 4), 0xA000_0000 + i);
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
        &bus.set(0x0000_0000, 0xE8A0_0FF0);
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
}
