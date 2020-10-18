use crate::cpu::types::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CpuState {
    ARM,
    Thumb,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Mode {
    User = 0,
    FIQ,
    IRQ,
    Supervisor,
    Abort,
    Undefined,
    System,
}

const RAW_DEFAULT: u32 = MODE_SUPERVISOR | (1 << IRQ_DISABLE_BIT) | (1 << FIQ_DISABLE_BIT);

const IRQ_DISABLE_BIT: u32 = 7;
const FIQ_DISABLE_BIT: u32 = 6;

const MODE_SUPERVISOR: u32 = 0b1_0011;

bitfield! {
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct PSR(u32);
    #[allow(non_snake_case)]
    pub get_N, set_N: 31;
    #[allow(non_snake_case)]
    pub get_Z, set_Z: 30;
    #[allow(non_snake_case)]
    pub get_C, set_C: 29;
    #[allow(non_snake_case)]
    pub get_V, set_V: 28;
    pub get_flag_bits, set_flag_bits: 31, 28;
    #[allow(non_snake_case)]
    pub get_Q, set_Q: 27;
    #[allow(non_snake_case)]
    pub get_J, set_J: 24;
    #[allow(non_snake_case)]
    pub get_I, set_I: 7;
    #[allow(non_snake_case)]
    pub get_F, set_F: 6;
    #[allow(non_snake_case)]
    pub get_T, set_T: 5;
    #[allow(non_snake_case)]
    pub get_M, set_M: 4, 0;
}

impl PSR {
    pub fn get(&self) -> u32 {
        self.0
    }

    pub fn set(&mut self, value: u32) {
        self.0 = value
    }

    pub fn set_cpu_state(&mut self, state: CpuState) {
        match state {
            CpuState::ARM => self.set_T(false),
            CpuState::Thumb => self.set_T(true),
        }
    }

    pub fn get_cpu_state(&self) -> CpuState {
        if self.get_T() {
            CpuState::Thumb
        } else {
            CpuState::ARM
        }
    }

    pub fn get_mode(&self) -> Mode {
        let m = self.get_M();
        match m {
            0b10000 => Mode::User,
            0b10001 => Mode::FIQ,
            0b10010 => Mode::IRQ,
            0b10011 => Mode::Supervisor,
            0b10111 => Mode::Abort,
            0b11011 => Mode::Undefined,
            0b11111 => Mode::System,
            _ => panic!("{:x} is illegal mode", m),
        }
    }

    pub fn set_flags(&mut self, value: u32) {
        self.set_flag_bits(value >> 28);
    }

    pub fn set_N_from(&mut self, reg: u32) {
        self.set_N(reg >> 31 == 0x01);
    }

    pub fn set_Z_from(&mut self, reg: u32) {
        self.set_Z(reg == 0x0);
    }

    pub fn set_C_from(&mut self, reg: u64) {
        self.set_C(reg > 0xFFFF_FFFF);
    }

    pub fn set_V_from(&mut self, cur: u32, reg: u32) {
        let v = (cur >> 31) != 0 && (((cur >> 31) ^ reg) >> 31) != 0 && (reg >> 31) == 0;
        self.set_V(v);
    }

    pub fn condition_ok(&self, cond: Cond) -> bool {
        match cond {
            Cond::EQ => self.get_Z(),
            Cond::NE => !self.get_Z(),
            Cond::CS_HS => self.get_C(),
            Cond::CC_LO => !self.get_C(),
            Cond::MI => self.get_N(),
            Cond::PL => !self.get_N(),
            Cond::VS => self.get_V(),
            Cond::VC => !self.get_V(),
            Cond::HI => self.get_C() && !self.get_Z(),
            Cond::LS => !self.get_C() || self.get_Z(),
            Cond::GE => !self.get_N() == !self.get_V(),
            Cond::LT => !self.get_N() != !self.get_V(),
            Cond::GT => !self.get_Z() && !self.get_N() == !self.get_V(),
            Cond::LE => self.get_Z() || !self.get_N() != !self.get_V(),
            Cond::AL => true,
        }
    }
}

impl Default for PSR {
    fn default() -> PSR {
        PSR(RAW_DEFAULT)
    }
}
