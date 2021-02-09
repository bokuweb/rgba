use crate::cpu::registers::Mode;
use crate::cpu::registers::PSR;

#[derive(Debug, PartialEq, Default)]
pub struct BankSpsr {
    fiq: PSR,
    svc: PSR,
    abt: PSR,
    irq: PSR,
    und: PSR,
    escaped: PSR,
}

impl BankSpsr {
    pub fn write(&mut self, mode: Mode, value: PSR) {
        match mode {
            Mode::User => panic!("user has no bank spsr"),
            Mode::FIQ => self.fiq = value,
            Mode::IRQ => self.irq = value,
            Mode::Supervisor => self.svc = value,
            Mode::Abort => self.abt = value,
            Mode::Undefined => self.und = value,
            Mode::System => panic!("system has no bank spsr"),
        }
    }

    pub fn read(&self, mode: Mode) -> PSR {
        match mode {
            Mode::User => panic!("user has no bank spsr"),
            Mode::FIQ => self.fiq,
            Mode::IRQ => self.irq,
            Mode::Supervisor => self.svc,
            Mode::Abort => self.abt,
            Mode::Undefined => self.und,
            Mode::System => panic!("system has no bank spsr"),
        }
    }

    pub fn push(&mut self, value: PSR) {
        self.escaped = value
    }

    pub fn pop(&mut self) -> PSR {
        self.escaped
    }
}
