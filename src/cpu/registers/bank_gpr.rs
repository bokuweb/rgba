use crate::cpu::constants::*;
use crate::cpu::registers::Mode;

// - 0-4:   r8_fiq - r12_fiq
// - 5-6:   r13_fiq & r14_fiq
// - 7-8:   r13_svc & r14_svc
// - 9-10:  r13_abt & r14_abt
// - 11-12: r13_irq & r14_irq
// - 13-14: r13_und & r14_und
#[derive(Debug, PartialEq, Default)]
pub struct BankGpr {
    r8_fiq: u32,
    r9_fiq: u32,
    r10_fiq: u32,
    r11_fiq: u32,
    r12_fiq: u32,
    r13_fiq: u32,
    r14_fiq: u32,
    r13_svc: u32,
    r14_svc: u32,
    r13_abt: u32,
    r14_abt: u32,
    r13_irq: u32,
    r14_irq: u32,
    r13_und: u32,
    r14_und: u32,
    // escaped
    r8_escaped: u32,
    r9_escaped: u32,
    r10_escaped: u32,
    r11_escaped: u32,
    r12_escaped: u32,
    r13_escaped: u32,
    r14_escaped: u32,
}

impl BankGpr {
    pub(crate) fn write(&mut self, mode: Mode, index: usize, value: u32) {
        match mode {
            Mode::User => panic!("user has no bank register"),
            Mode::FIQ => match index {
                8 => self.r8_fiq = value,
                9 => self.r9_fiq = value,
                10 => self.r10_fiq = value,
                11 => self.r11_fiq = value,
                12 => self.r12_fiq = value,
                SP => self.r13_fiq = value,
                LR => self.r14_fiq = value,
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::IRQ => match index {
                SP => self.r13_irq = value,
                LR => self.r14_irq = value,
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::Supervisor => match index {
                SP => self.r13_svc = value,
                LR => self.r14_svc = value,
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::Abort => match index {
                SP => self.r13_abt = value,
                LR => self.r14_abt = value,
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::Undefined => match index {
                SP => self.r13_und = value,
                LR => self.r14_und = value,
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::System => panic!("system has no bank register"),
        }
    }

    pub(crate) fn push(&mut self, index: usize, value: u32) {
        match index {
            8 => self.r8_escaped = value,
            9 => self.r9_escaped = value,
            10 => self.r10_escaped = value,
            11 => self.r11_escaped = value,
            12 => self.r12_escaped = value,
            SP => self.r13_escaped = value,
            LR => self.r14_escaped = value,
            _ => panic!("unexpected gpr index detected."),
        }
    }

    pub(crate) fn pop(&mut self, index: usize) -> u32 {
        match index {
            8 => self.r8_escaped,
            9 => self.r9_escaped,
            10 => self.r10_escaped,
            11 => self.r11_escaped,
            12 => self.r12_escaped,
            SP => self.r13_escaped,
            LR => self.r14_escaped,
            _ => panic!("unexpected gpr index detected."),
        }
    }

    pub(crate) fn read(&mut self, mode: Mode, index: usize) -> u32 {
        dbg!(index);
        match mode {
            Mode::User => panic!("user has no bank register"),
            Mode::FIQ => match index {
                8 => self.r8_fiq,
                9 => self.r9_fiq,
                10 => self.r10_fiq,
                11 => self.r11_fiq,
                12 => self.r12_fiq,
                SP => self.r13_fiq,
                LR => self.r14_fiq,
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::IRQ => match index {
                SP => self.r13_irq,
                LR => self.r14_irq,
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::Supervisor => match index {
                SP => self.r13_svc,
                LR => self.r14_svc,
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::Abort => match index {
                SP => self.r13_abt,
                LR => self.r14_abt,
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::Undefined => match index {
                SP => self.r13_und,
                LR => self.r14_und,
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::System => panic!("system has no bank register"),
        }
    }
}
