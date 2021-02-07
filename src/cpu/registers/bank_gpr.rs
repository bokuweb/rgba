use crate::cpu::constants::*;
use crate::cpu::registers::Mode;

// - 0-4:   r8_fiq - r12_fiq
// - 5-6:   r13_fiq & r14_fiq
// - 7-8:   r13_svc & r14_svc
// - 9-10:  r13_abt & r14_abt
// - 11-12: r13_irq & r14_irq
// - 13-14: r13_und & r14_und
#[derive(Debug, PartialEq, Default)]
pub struct BankGpr([u32; 15]);

impl BankGpr {
    pub(crate) fn write(&mut self, mode: Mode, index: usize, value: u32) {
        dbg!(index);
        match mode {
            Mode::User => panic!("user has no bank register"),
            Mode::FIQ => {
                todo!()
            }
            Mode::IRQ => match index {
                SP => self.0[11] = value,
                LR => self.0[12] = value,
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::Supervisor => match index {
                SP => self.0[7] = value,
                LR => self.0[8] = value,
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::Abort => match index {
                SP => self.0[9] = value,
                LR => self.0[10] = value,
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::Undefined => match index {
                SP => self.0[13] = value,
                LR => self.0[14] = value,
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::System => panic!("system has no bank register"),
        }
    }

    pub(crate) fn read(&mut self, mode: Mode, index: usize) -> u32 {
        dbg!(index);
        match mode {
            Mode::User => panic!("user has no bank register"),
            Mode::FIQ => {
                todo!()
            }
            Mode::IRQ => match index {
                SP => self.0[11],
                LR => self.0[12],
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::Supervisor => match index {
                SP => self.0[7],
                LR => self.0[8],
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::Abort => match index {
                SP => self.0[9],
                LR => self.0[10],
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::Undefined => match index {
                SP => self.0[13],
                LR => self.0[14],
                _ => panic!("unexpected gpr index detected."),
            },
            Mode::System => panic!("system has no bank register"),
        }
    }
}
