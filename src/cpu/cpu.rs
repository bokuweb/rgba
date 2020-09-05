use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::{arm, thumb};
use crate::cpu::instructions::arm::{
    block_data_transfer::*, branch::*, branch_and_exchange::*, data::*, extra_memory::*, memory::*,
    multiple::*, psr_transfer::*,
};

use crate::cpu::instructions::thumb::*;

use crate::cpu::instructions::PipelineStatus;
use crate::cpu::registers::psr::{CpuState, PSR};
use crate::cpu::types::*;

pub const INITIAL_PIPELINE_WAIT: u8 = 2;

enum Arm {
    NOP,
    NOP_RAW,
}

enum CpuMode {
    System,
    Supervisor,
    FIQ,
}

pub struct ARM {
    pub gpr: [u32; 16],
    /// - 0-4:   r8_fiq - r12_fiq
    /// - 5-6:   r13_fiq & r14_fiq
    /// - 7-8:   r13_svc & r14_svc
    /// - 9-10:  r13_abt & r14_abt
    /// - 11-12: r13_irq & r14_irq
    /// - 13-14: r13_und & r14_und
    bank_gpr: [u32; 15],
    pipeline_wait: u8,
    cpsr: PSR,
    spsr: PSR,

    bank_spsr: [PSR; 5],
    mode: CpuMode,
    irq_disable: bool,
    fiq_disable: bool,
    optimise_swi: bool,
}

impl ARM {
    pub fn new() -> ARM {
        ARM {
            pipeline_wait: INITIAL_PIPELINE_WAIT,
            gpr: [0; 16],
            bank_gpr: [0; 15],
            cpsr: PSR::default(),
            spsr: PSR::default(),
            bank_spsr: [PSR::default(); 5],
            mode: CpuMode::System,
            irq_disable: false,
            fiq_disable: false,
            optimise_swi: false,
        }
    }

    pub fn reset(&mut self) {
        self.gpr[PC] = 0x00000000;

        self.cpsr = PSR::default();

        self.mode = CpuMode::Supervisor;
        self.irq_disable = true;
        self.fiq_disable = true;
    }

    fn flush_pipeline(&mut self) {
        self.pipeline_wait = INITIAL_PIPELINE_WAIT;
    }

    fn increment_pc(&mut self) {
        let next = if self.cpsr.get_cpu_state() == CpuState::ARM {
            4
        } else {
            2
        };
        self.gpr[PC] = self.gpr[PC].wrapping_add(next);
    }

    fn execute_arm<T>(&mut self, instruction: arm::Instruction, bus: &mut T) -> Result<(), ()>
    where
        T: BusAccessor,
    {
        debug!("execute {:?}", &instruction);
        let pipeline_status = {
            match instruction {
                arm::Instruction::AND(dec) => exec_and(bus, dec, &mut self.gpr)?,
                arm::Instruction::EOR(dec) => exec_eor(bus, dec, &mut self.gpr)?,
                arm::Instruction::SUB(dec) => exec_sub(bus, dec, &mut self.gpr)?,
                arm::Instruction::RSB(dec) => exec_rsb(bus, dec, &mut self.gpr)?,
                arm::Instruction::ADD(dec) => exec_add(bus, dec, &mut self.gpr)?,
                arm::Instruction::ADC(dec) => exec_adc(bus, dec, &mut self.gpr, &self.cpsr)?,
                arm::Instruction::SBC(dec) => exec_sbc(bus, dec, &mut self.gpr, &self.cpsr)?,
                arm::Instruction::RSC(dec) => exec_rsc(bus, dec, &mut self.gpr, &self.cpsr)?,
                arm::Instruction::TST(dec) => exec_tst(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::TEQ(dec) => exec_teq(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::CMP(dec) => exec_cmp(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::CMN(dec) => exec_cmn(bus, dec, &mut self.gpr, &mut self.cpsr)?,
                arm::Instruction::ORR(dec) => exec_orr(bus, dec, &mut self.gpr)?,
                arm::Instruction::MOV(dec) => exec_mov(bus, dec, &mut self.gpr)?,
                arm::Instruction::LSL(dec) => exec_shift(bus, dec, &mut self.gpr)?,
                arm::Instruction::LSR(dec) => exec_shift(bus, dec, &mut self.gpr)?,
                arm::Instruction::ASR(dec) => exec_shift(bus, dec, &mut self.gpr)?,
                arm::Instruction::RRX(dec) => exec_rrx(bus, dec, &mut self.gpr, &self.cpsr)?,
                arm::Instruction::ROR(dec) => exec_shift(bus, dec, &mut self.gpr)?,
                arm::Instruction::BIC(dec) => exec_bic(bus, dec, &mut self.gpr)?,
                arm::Instruction::MVN(dec) => exec_mvn(bus, dec, &mut self.gpr)?,
                arm::Instruction::MUL(dec) => exec_mul(bus, dec, &mut self.gpr, &self.cpsr)?,
                arm::Instruction::MLA(dec) => exec_mla(bus, dec, &mut self.gpr, &self.cpsr)?,
                arm::Instruction::UMULL(dec) => exec_umull(bus, dec, &mut self.gpr, &self.cpsr)?,
                arm::Instruction::UMLAL(dec) => exec_umlal(bus, dec, &mut self.gpr, &self.cpsr)?,
                arm::Instruction::SMULL(dec) => exec_smull(bus, dec, &mut self.gpr, &self.cpsr)?,
                arm::Instruction::SMLAL(dec) => exec_smlal(bus, dec, &mut self.gpr, &self.cpsr)?,
                arm::Instruction::LDR(dec) => exec_ldr(bus, dec, &mut self.gpr)?,
                arm::Instruction::STR(dec) => exec_str(bus, dec, &mut self.gpr)?,
                arm::Instruction::LDRB(dec) => exec_ldrb(bus, dec, &mut self.gpr)?,
                arm::Instruction::STRB(dec) => exec_strb(bus, dec, &mut self.gpr)?,
                arm::Instruction::STRH(dec) => exec_strh(bus, dec, &mut self.gpr)?,
                arm::Instruction::LDRH(dec) => exec_ldrh(bus, dec, &mut self.gpr)?,
                arm::Instruction::LDRSB(dec) => exec_ldrsb(bus, dec, &mut self.gpr)?,
                arm::Instruction::LDRSH(dec) => exec_ldrsh(bus, dec, &mut self.gpr)?,
                arm::Instruction::B(dec) => exec_b(dec, &mut self.gpr)?,
                arm::Instruction::BL(dec) => exec_bl(dec, &mut self.gpr)?,
                arm::Instruction::BX(dec) => exec_bx(dec, &mut self.cpsr, &mut self.gpr)?,
                arm::Instruction::LDM(dec) => exec_ldm(bus, dec, &mut self.gpr)?,
                arm::Instruction::STM(dec) => exec_stm(bus, dec, &mut self.gpr)?,
                arm::Instruction::MRS(dec) => {
                    exec_mrs(dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr)?
                }
                arm::Instruction::MSR(dec) => {
                    exec_msr(dec, &mut self.gpr, &mut self.cpsr, &mut self.spsr)?
                }
                arm::Instruction::Undefined => unimplemented!(),
                //arm::Opcode::NOP => unimplemented!(),
                //// arm::Opcode::SWI => unimplemented!(),
                // ArmOpcode::Unknown => self.execute_unknown(dec),
                _ => unimplemented!(),
            }
        };
        match pipeline_status {
            PipelineStatus::Continue => self.increment_pc(),
            PipelineStatus::Flush => self.flush_pipeline(),
        };
        Ok(())
    }

    fn execute_thumb<T>(&mut self, instruction: thumb::Instruction, bus: &mut T) -> Result<(), ()>
    where
        T: BusAccessor,
    {
        debug!("execute thumb {:?}", &instruction);
        let pipeline_status = {
            match instruction {
                thumb::Instruction::LDR3(dec) => exec_thumb_ldr3(bus, dec, &mut self.gpr)?,
                thumb::Instruction::ADD3(dec) => {
                    exec_thumb_add3(dec, &mut self.gpr, &mut self.cpsr)?
                }
                thumb::Instruction::BIC(dec) => {
                    exec_thumb_bic(dec, &mut self.gpr, &mut self.cpsr)?
                }
                thumb::Instruction::LSL(dec) => {
                    exec_thumb_lsl(bus, dec, &mut self.gpr, &mut self.cpsr)?
                }
                thumb::Instruction::MOV1(dec) => {
                    exec_thumb_mov1(dec, &mut self.gpr, &mut self.cpsr)?
                }
                thumb::Instruction::B(dec) => exec_thumb_b(dec, &mut self.gpr, &mut self.cpsr)?,
                thumb::Instruction::BL(dec) => exec_thumb_bl(dec, &mut self.gpr)?,
                _ => unimplemented!(),
            }
        };
        match pipeline_status {
            PipelineStatus::Continue => self.increment_pc(),
            PipelineStatus::Flush => self.flush_pipeline(),
        };
        Ok(())
    }

    pub fn tick<T>(&mut self, bus: &mut T) -> Result<(), ()>
    where
        T: BusAccessor,
    {
        if self.pipeline_wait > 0 {
            dbg!("inc pc");
            self.pipeline_wait -= 1;
            self.increment_pc();
            return Ok(());
        }
        debug!("registers = {:?} {:?}", self.gpr, self.cpsr.get_cpu_state());
        match self.cpsr.get_cpu_state() {
            CpuState::ARM => {
                let fetched = self.prefetch_arm(bus);
                let instruction = arm::decode(fetched);
                self.execute_arm(instruction, bus)
            }
            CpuState::Thumb => {
                dbg!("thumb!!");
                let fetched = self.prefetch_thumb(bus);
                debug!("{:x}", fetched);
                let instruction = thumb::decode(fetched);
                self.execute_thumb(instruction, bus)
            }
        }
    }

    fn prefetch_arm<T>(&mut self, bus: &mut T) -> Word
    where
        T: BusAccessor,
    {
        let fetched = bus.read_word(self.gpr[PC] - (PC_OFFSET * 4) as u32);
        fetched
    }

    fn prefetch_thumb<T>(&mut self, bus: &mut T) -> HalfWord
    where
        T: BusAccessor,
    {
        let fetched = bus.read_halfword(self.gpr[PC] - (PC_OFFSET * 2) as u32);
        fetched
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
}

#[cfg(test)]
mod test {
    extern crate byteorder;
    // extern crate env_logger;

    use super::*;
    use byteorder::{ByteOrder, LittleEndian};
    // use ctare::memory::readable::*;
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

        fn write_byte(&mut self, addr: Word, data: u8) {
            self.mem[(addr as usize)] = data;
        }

        fn write_word(&mut self, addr: Word, data: u32) {
            LittleEndian::write_u32(&mut self.mem[(addr as usize)..], data);
        }
    }

    impl CpuTest for ARM {
        fn run_immediately<T>(&mut self, bus: &mut T)
        where
            T: BusAccessor,
        {
            for _ in 0..(INITIAL_PIPELINE_WAIT + 1) {
                self.tick(bus);
            }
        }
    }

    fn setup() {
        use std::sync::{Once, ONCE_INIT};
        static INIT: Once = ONCE_INIT;
        // INIT.call_once(|| env_logger::init());
    }

    #[test]
    // tick
    fn increment_pc_by_tick() {
        setup();
        let mut bus = MockBus::new();
        let mut arm = ARM::new();
        arm.tick(&mut bus);
        assert_eq!(arm.get_gpr(PC), 0x0000_0004);
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
        &bus.set(0x0, 0xE0E1_3002);
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
        &bus.set(0x0, 0xE0E1_3002);
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
        arm.set_gpr(1, 0x00AA_AA55);
        arm.cpsr.set_C(true);
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(2), 0x8055_552A);
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
        &bus.set(0x4, 0x8000_0000);
        let mut arm = ARM::new();
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(PC), 0x8000_0000);
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
        assert_eq!(arm.get_gpr(PC), 0x0000_0000);
    }

    #[test]
    // bl pc-2
    fn bl_pc_sub_2() {
        setup();
        let mut bus = MockBus::new();
        &bus.set(0x0000_0000, 0xEBFF_FFFE);
        let mut arm = ARM::new();
        arm.run_immediately(&mut bus);
        assert_eq!(arm.get_gpr(PC), 0x0000_0000);
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
        assert_eq!(arm.get_gpr(PC), 0x0000_000C);
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
        assert_eq!(arm.get_gpr(PC), 0x0000_000C);
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
