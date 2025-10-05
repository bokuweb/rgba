use crate::cpu::constants::*;
use crate::cpu::registers::{BankGpr, BankSpsr};
use crate::cpu::types::*;
use crate::types::*;
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CpuState {
    ARM,
    Thumb,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Mode {
    User = 0x10,
    FIQ = 0x11,
    IRQ = 0x12,
    Supervisor = 0x13,
    Abort = 0x17,
    Undefined = 0x1B,
    System = 0x1F,
}

impl From<u32> for Mode {
    fn from(f: u32) -> Mode {
        match f {
            0x10 => Mode::User,
            0x11 => Mode::FIQ,
            0x12 => Mode::IRQ,
            0x13 => Mode::Supervisor,
            0x17 => Mode::Abort,
            0x1B => Mode::Undefined,
            0x1F => Mode::System,
            _ => panic!("illegal mode value({:x}) detected.", f),
        }
    }
}

const RAW_DEFAULT: u32 = MODE_SYSTEM; // | (1 << IRQ_DISABLE_BIT) | (1 << FIQ_DISABLE_BIT);

const IRQ_DISABLE_BIT: u32 = 7;
const FIQ_DISABLE_BIT: u32 = 6;

const MODE_SUPERVISOR: u32 = 0b1_0011;
const MODE_SYSTEM: u32 = 0b1_1111;

// Bit   Expl.
// 31    N - Sign Flag       (0=Not Signed, 1=Signed)               ;\
// 30    Z - Zero Flag       (0=Not Zero, 1=Zero)                   ; Condition
// 29    C - Carry Flag      (0=Borrow/No Carry, 1=Carry/No Borrow) ; Code Flags
// 28    V - Overflow Flag   (0=No Overflow, 1=Overflow)            ;/
// 27    Q - Sticky Overflow (1=Sticky Overflow, ARMv5TE and up only)
// 26-8  Reserved            (For future use) - Do not change manually!
// 7     I - IRQ disable     (0=Enable, 1=Disable)                     ;\
// 6     F - FIQ disable     (0=Enable, 1=Disable)                     ; Control
// 5     T - State Bit       (0=ARM, 1=THUMB) - Do not change manually!; Bits
// 4-0   M4-M0 - Mode Bits   (See below)                               ;/
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

    pub fn set_mode(&mut self, mode: Mode) {
        self.set_M(mode as u32);
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

    // pub fn set_V_from(&mut self, cur: u32, reg: u32) {
    //     let v = (cur >> 31) != 0 && (((cur >> 31) ^ reg) >> 31) != 0 && (reg >> 31) == 0;
    //     self.set_V(v);
    // }

    pub fn restore(&mut self, spsr: &mut PSR, gpr: &mut [Word; 16], bank_gpr: &mut BankGpr, bank_spsr: &mut BankSpsr) {
        self.switch_mode(spsr.get_mode(), gpr, spsr, bank_gpr, bank_spsr);
        self.set(spsr.get())
        // TODO: check irq??
    }

    pub fn switch_mode(&mut self, new_mode: Mode, gpr: &mut [Word; 16], spsr: &mut PSR, bank_gpr: &mut BankGpr, bank_spsr: &mut BankSpsr) {
        if new_mode == self.get_mode() {
            return;
        }

        // TODO: move to PSR?
        //       switch mode
        // let current_value = cpsr.get();
        // NOTE: new_mode が User/System 以外の特権モードのときのみ
        // バンキングの入れ替え処理を行うべきだが、以前は `||` だったため
        // 常に true となり不要なバンク切替が走っていた。
        // その結果、FIQ<->System 切替時に r8-r12 の入れ替えが誤って起こりうる。
        // ここを `&&` に修正し、User/System の場合はこの分岐を素通りする。
        if new_mode != Mode::User && new_mode != Mode::System {
            let current_mode = self.get_mode();
            // let new_mode = self.get_mode();
            if current_mode != new_mode {
                // TODO: support FIQ
                if current_mode == Mode::FIQ {
                    bank_gpr.write(current_mode, 8, gpr[8]);
                    bank_gpr.write(current_mode, 9, gpr[9]);
                    bank_gpr.write(current_mode, 10, gpr[10]);
                    bank_gpr.write(current_mode, 11, gpr[11]);
                    bank_gpr.write(current_mode, 12, gpr[12]);

                    gpr[8] = bank_gpr.pop(8);
                    gpr[9] = bank_gpr.pop(9);
                    gpr[10] = bank_gpr.pop(10);
                    gpr[11] = bank_gpr.pop(11);
                    gpr[12] = bank_gpr.pop(12);
                }

                if new_mode == Mode::FIQ {
                    bank_gpr.push(8, gpr[8]);
                    bank_gpr.push(9, gpr[9]);
                    bank_gpr.push(10, gpr[10]);
                    bank_gpr.push(11, gpr[11]);
                    bank_gpr.push(12, gpr[12]);

                    gpr[8] = bank_gpr.read(new_mode, 8);
                    gpr[9] = bank_gpr.read(new_mode, 9);
                    gpr[10] = bank_gpr.read(new_mode, 10);
                    gpr[11] = bank_gpr.read(new_mode, 11);
                    gpr[12] = bank_gpr.read(new_mode, 12);
                }

                if current_mode != Mode::System && current_mode != Mode::User {
                    bank_gpr.write(current_mode, SP, gpr[SP]);
                    bank_gpr.write(current_mode, LR, gpr[LR]);
                    bank_spsr.write(current_mode, *spsr);

                    gpr[SP] = bank_gpr.pop(SP);
                    gpr[LR] = bank_gpr.pop(LR);
                    *spsr = bank_spsr.pop()
                }

                if new_mode != Mode::System && new_mode != Mode::User {
                    bank_gpr.push(SP, gpr[SP]);
                    bank_gpr.push(LR, gpr[LR]);
                    bank_spsr.push(*spsr);

                    gpr[SP] = bank_gpr.read(new_mode, SP);
                    gpr[LR] = bank_gpr.read(new_mode, LR);
                    *spsr = bank_spsr.read(new_mode);
                }
            }
        }
        self.set_mode(new_mode);
    }

    pub fn condition_ok(&self, cond: Cond) -> bool {
        match cond {
            Cond::EQ => self.get_Z(),
            Cond::NE => !self.get_Z(),
            Cond::CS => self.get_C(),
            Cond::CC => !self.get_C(),
            Cond::MI => self.get_N(),
            Cond::PL => !self.get_N(),
            Cond::VS => self.get_V(),
            Cond::VC => !self.get_V(),
            Cond::HI => self.get_C() && !self.get_Z(),
            Cond::LS => !self.get_C() || self.get_Z(),
            Cond::GE => self.get_N() == self.get_V(),
            Cond::LT => self.get_N() != self.get_V(),
            Cond::GT => !self.get_Z() && (self.get_N() == self.get_V()),
            Cond::LE => self.get_Z() || (self.get_N() != self.get_V()),
            Cond::AL => true,
            Cond::NV => false, // Never condition (ARMv4)
        }
    }
}

impl Default for PSR {
    fn default() -> PSR {
        PSR(RAW_DEFAULT)
    }
}
