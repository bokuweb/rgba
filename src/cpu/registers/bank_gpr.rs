use crate::cpu::constants::*;
use crate::cpu::registers::Mode;

// - 0-4:   r8_fiq - r12_fiq
// - 5-6:   r13_fiq & r14_fiq
// - 7-8:   r13_svc & r14_svc
// - 9-10:  r13_abt & r14_abt
// - 11-12: r13_irq & r14_irq
// - 13-14: r13_und & r14_und
#[derive(Debug, PartialEq, Eq, Default)]
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
    // LDM^ glitch support: one-shot overlay for next instruction
    glitch_mask: u16,
    glitch_values: [u32; 16],
    glitch_backup: [u32; 16],
    // 0: Inactive, 1: Armed (apply after current inst), 2: Active (restore after current inst)
    glitch_state: u8,
}

impl BankGpr {
    pub(crate) const fn is_glitch_active_or_armed(&self) -> bool {
        self.glitch_state == 1 || self.glitch_state == 2
    }

    pub(crate) const fn get_glitch_backup(&self, index: usize) -> u32 {
        self.glitch_backup[index]
    }

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

    // ========== LDM^ glitch helpers ==========
    // Arm glitch for specified registers. This does NOT modify gpr immediately;
    // actual application happens on advance_glitch_window() call.
    pub(crate) fn glitch_arm(&mut self, mask: u16, overlays: &[(usize, u32)], gpr: &mut [u32; 16]) {
        if mask == 0 {
            return;
        }
        self.glitch_mask = mask;
        // Backup current visible registers for restoration later
        for i in 0..16 {
            if (mask & (1 << i)) != 0 {
                self.glitch_backup[i] = gpr[i];
            }
        }
        // Store overlay values
        for (idx, val) in overlays {
            self.glitch_values[*idx] = *val;
        }
        // Mark as Armed so it will be applied after current instruction retires
        self.glitch_state = 1;
    }

    // Advance glitch window across instruction boundary.
    // - Armed -> Active: apply overlay into gpr (to affect next instruction)
    // - Active -> Inactive: restore original values and clear mask
    pub(crate) fn advance_glitch_window(&mut self, gpr: &mut [u32; 16]) {
        match self.glitch_state {
            0 => {
                // Inactive: nothing to do
            }
            1 => {
                // Armed -> apply overlay now
                println!("[GLITCH] Apply overlay (mask=0x{:04X})", self.glitch_mask);
                for i in 0..16 {
                    if (self.glitch_mask & (1 << i)) != 0 {
                        println!(
                            "  r{}: user|curr = 0x{:08X} | 0x{:08X} -> 0x{:08X}",
                            i, self.glitch_backup[i], gpr[i], self.glitch_values[i]
                        );
                        gpr[i] = self.glitch_values[i];
                    }
                }
                self.glitch_state = 2; // Active
            }
            2 => {
                // Active -> restore and clear
                println!("[GLITCH] Restore (mask=0x{:04X})", self.glitch_mask);
                for i in 0..16 {
                    if (self.glitch_mask & (1 << i)) != 0 {
                        // もし対象レジスタが次命令で書き換えられている場合は復元しない
                        // （overlay 値と現在値が異なれば write と判断）
                        if gpr[i] == self.glitch_values[i] {
                            println!("  r{}: restore to 0x{:08X}", i, self.glitch_backup[i]);
                            gpr[i] = self.glitch_backup[i];
                        }
                        // Clear stored values
                        self.glitch_values[i] = 0;
                        self.glitch_backup[i] = 0;
                    }
                }
                self.glitch_mask = 0;
                self.glitch_state = 0; // Inactive
            }
            _ => {}
        }
    }

    // Returns Some(backup) only if the index is covered by current glitch mask
    pub(crate) const fn glitch_backup_if_masked(&self, index: usize) -> Option<u32> {
        if (self.glitch_mask & (1 << index)) != 0 {
            Some(self.glitch_backup[index])
        } else {
            None
        }
    }

    // Remove specified indices from active overlay (if active), restoring original values.
    // Use when next instruction must see the pre-overlay values for certain registers (e.g., multiplicands).
    pub(crate) fn refine_remove_indices_from_overlay(&mut self, indices: &[usize], gpr: &mut [u32; 16]) {
        if self.glitch_state != 2 {
            // Not active yet; nothing to refine
            return;
        }
        for &i in indices {
            if (self.glitch_mask & (1 << i)) != 0 {
                // If the overlay value is still present, restore the backup
                if gpr[i] == self.glitch_values[i] {
                    gpr[i] = self.glitch_backup[i];
                }
                // Clear this bit from the overlay and backups
                self.glitch_mask &= !(1 << i);
                self.glitch_values[i] = 0;
                self.glitch_backup[i] = 0;
            }
        }
    }
}
