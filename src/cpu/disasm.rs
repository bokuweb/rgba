//! Disassembler — turns raw ARM/THUMB instruction words back into assembly
//! text. It reuses the emulator's own decoders ([`super::decoder`]) so the
//! decode logic stays in one place; this module only renders operands.
//!
//! ARM coverage is complete for every variant the decoder produces. THUMB
//! covers the common instruction set; any variant whose operands aren't
//! rendered yet falls back to `<mnemonic>` and the raw hex is always shown by
//! the caller, so nothing is ever silently mis-decoded.

use crate::cpu::decoder::{arm, thumb};

/// Register name with the conventional aliases for r13/r14/r15.
fn reg(n: u32) -> &'static str {
    const NAMES: [&str; 16] = [
        "r0", "r1", "r2", "r3", "r4", "r5", "r6", "r7", "r8", "r9", "r10", "r11", "r12", "sp", "lr",
        "pc",
    ];
    NAMES[(n & 0xF) as usize]
}

/// Condition-code suffix (empty for AL).
fn cond(bits: u32) -> &'static str {
    match bits & 0xF {
        0 => "eq",
        1 => "ne",
        2 => "cs",
        3 => "cc",
        4 => "mi",
        5 => "pl",
        6 => "vs",
        7 => "vc",
        8 => "hi",
        9 => "ls",
        10 => "ge",
        11 => "lt",
        12 => "gt",
        13 => "le",
        14 => "", // AL — omitted
        _ => "nv",
    }
}

fn shift_name(sh: u32) -> &'static str {
    match sh & 3 {
        0 => "lsl",
        1 => "lsr",
        2 => "asr",
        _ => "ror",
    }
}

/// Render the 16-bit register list of an LDM/STM/PUSH/POP as `{r0,r4-r7,lr}`.
fn reg_list(mask: u32) -> String {
    let mut parts = Vec::new();
    let mut i = 0u32;
    while i < 16 {
        if mask & (1 << i) != 0 {
            let start = i;
            while i + 1 < 16 && mask & (1 << (i + 1)) != 0 {
                i += 1;
            }
            if i == start {
                parts.push(reg(start).to_string());
            } else {
                parts.push(format!("{}-{}", reg(start), reg(i)));
            }
        }
        i += 1;
    }
    format!("{{{}}}", parts.join(","))
}

/// Format an ARM data-processing operand 2 (immediate or shifted register).
fn arm_op2(dp: &arm::DataProcessing) -> String {
    if dp.get_I() {
        let imm = dp.get_imm();
        let rot = dp.get_rotate() * 2;
        let val = imm.rotate_right(rot);
        format!("#0x{val:x}")
    } else {
        let rm = reg(dp.get_Rm());
        let sh = dp.get_sh();
        if dp.get_R() {
            // Shift amount in a register.
            format!("{rm}, {} {}", shift_name(sh), reg(dp.get_Rs()))
        } else {
            let amt = dp.get_shamt5();
            if amt == 0 {
                match sh & 3 {
                    0 => rm.to_string(),          // lsl #0 == no shift
                    3 => format!("{rm}, rrx"),    // ror #0 == rrx
                    _ => format!("{rm}, {} #32", shift_name(sh)), // lsr/asr #0 == #32
                }
            } else {
                format!("{rm}, {} #{amt}", shift_name(sh))
            }
        }
    }
}

/// Disassemble one ARM instruction located at `pc`.
#[allow(clippy::too_many_lines)]
pub fn arm(raw: u32, pc: u32) -> String {
    use arm::Instruction as I;
    let c = cond(raw >> 28);
    let ins = arm::decode(raw);

    // Data-processing helpers.
    let dp3 = |m: &str, dp: &arm::DataProcessing| {
        let s = if dp.get_S() { "s" } else { "" };
        format!("{m}{c}{s}\t{}, {}, {}", reg(dp.get_Rd()), reg(dp.get_Rn()), arm_op2(dp))
    };
    let dp_cmp = |m: &str, dp: &arm::DataProcessing| {
        format!("{m}{c}\t{}, {}", reg(dp.get_Rn()), arm_op2(dp))
    };
    let dp_mov = |m: &str, dp: &arm::DataProcessing| {
        let s = if dp.get_S() { "s" } else { "" };
        format!("{m}{c}{s}\t{}, {}", reg(dp.get_Rd()), arm_op2(dp))
    };

    match &ins {
        I::AND(d) => dp3("and", d),
        I::EOR(d) => dp3("eor", d),
        I::SUB(d) => dp3("sub", d),
        I::RSB(d) => dp3("rsb", d),
        I::ADD(d) => dp3("add", d),
        I::ADC(d) => dp3("adc", d),
        I::SBC(d) => dp3("sbc", d),
        I::RSC(d) => dp3("rsc", d),
        I::ORR(d) => dp3("orr", d),
        I::BIC(d) => dp3("bic", d),
        I::TST(d) => dp_cmp("tst", d),
        I::TEQ(d) => dp_cmp("teq", d),
        I::CMP(d) => dp_cmp("cmp", d),
        I::CMN(d) => dp_cmp("cmn", d),
        I::MOV(d) => dp_mov("mov", d),
        I::MVN(d) => dp_mov("mvn", d),
        // MOV-with-shift variants: render as the shift mnemonic, Rd, Rm[, amt].
        I::LSL(d) | I::LSR(d) | I::ASR(d) | I::ROR(d) | I::RRX(d) => {
            let s = if d.get_S() { "s" } else { "" };
            let m = match &ins {
                I::LSL(_) => "lsl",
                I::LSR(_) => "lsr",
                I::ASR(_) => "asr",
                I::ROR(_) => "ror",
                _ => "rrx",
            };
            if matches!(ins, I::RRX(_)) {
                format!("{m}{c}{s}\t{}, {}", reg(d.get_Rd()), reg(d.get_Rm()))
            } else if d.get_R() {
                format!("{m}{c}{s}\t{}, {}, {}", reg(d.get_Rd()), reg(d.get_Rm()), reg(d.get_Rs()))
            } else {
                format!("{m}{c}{s}\t{}, {}, #{}", reg(d.get_Rd()), reg(d.get_Rm()), d.get_shamt5())
            }
        }

        I::MUL(m) => {
            let s = if m.get_S() { "s" } else { "" };
            format!("mul{c}{s}\t{}, {}, {}", reg(m.get_Rd()), reg(m.get_Rm()), reg(m.get_Rs()))
        }
        I::MLA(m) => {
            let s = if m.get_S() { "s" } else { "" };
            format!(
                "mla{c}{s}\t{}, {}, {}, {}",
                reg(m.get_Rd()),
                reg(m.get_Rm()),
                reg(m.get_Rs()),
                reg(m.get_Rn())
            )
        }
        I::UMULL(m) | I::UMLAL(m) | I::SMULL(m) | I::SMLAL(m) => {
            let mn = match &ins {
                I::UMULL(_) => "umull",
                I::UMLAL(_) => "umlal",
                I::SMULL(_) => "smull",
                _ => "smlal",
            };
            let s = if m.get_S() { "s" } else { "" };
            format!(
                "{mn}{c}{s}\t{}, {}, {}, {}",
                reg(m.get_RdLo()),
                reg(m.get_RdHi()),
                reg(m.get_Rm()),
                reg(m.get_Rs())
            )
        }

        I::STR(m) | I::LDR(m) | I::LDRB(m) | I::STRB(m) => {
            let mn = match &ins {
                I::STR(_) => "str",
                I::LDR(_) => "ldr",
                I::LDRB(_) => "ldrb",
                _ => "strb",
            };
            format!("{mn}{c}\t{}, {}", reg(m.get_Rd()), arm_mem_addr(m))
        }
        I::STRH(m) | I::LDRH(m) | I::LDRSB(m) | I::LDRSH(m) => {
            let mn = match &ins {
                I::STRH(_) => "strh",
                I::LDRH(_) => "ldrh",
                I::LDRSB(_) => "ldrsb",
                _ => "ldrsh",
            };
            format!("{mn}{c}\t{}, {}", reg(m.get_Rd()), arm_extra_mem_addr(m))
        }

        I::B(b) | I::BL(b) => {
            let mn = if matches!(ins, I::BL(_)) { "bl" } else { "b" };
            // 24-bit signed offset, shifted left 2, relative to pc+8.
            let off = ((b.get_offset() << 8) as i32) >> 6; // sign-extend then *4
            let target = pc.wrapping_add(8).wrapping_add(off as u32);
            format!("{mn}{c}\t#0x{target:x}")
        }
        I::BX(b) => format!("bx{c}\t{}", reg(b.get_Rm())),

        I::LDM(b) | I::STM(b) => {
            let mn = if b.get_L() { "ldm" } else { "stm" };
            let mode = match (b.get_P(), b.get_U()) {
                (false, true) => "ia",
                (true, true) => "ib",
                (false, false) => "da",
                (true, false) => "db",
            };
            let wb = if b.get_W() { "!" } else { "" };
            let user = if b.get_S() { "^" } else { "" };
            format!("{mn}{c}{mode}\t{}{wb}, {}{user}", reg(b.get_Rn()), reg_list(b.get_register_list()))
        }

        I::SWP(s) | I::SWPB(s) => {
            let mn = if matches!(ins, I::SWPB(_)) { "swpb" } else { "swp" };
            format!("{mn}{c}\t{}, {}, [{}]", reg(s.get_Rd()), reg(s.get_Rm()), reg(s.get_Rn()))
        }

        I::MRS(p) => {
            let psr = if p.get_Pd() { "spsr" } else { "cpsr" };
            format!("mrs{c}\t{}, {psr}", reg(p.get_Rd()))
        }
        I::MSR(p) => {
            let psr = if p.get_Pd() { "spsr" } else { "cpsr" };
            let mut fields = String::new();
            if p.get_C() {
                fields.push('c');
            }
            if p.get_X() {
                fields.push('x');
            }
            if p.get_S() {
                fields.push('s');
            }
            if p.get_F() {
                fields.push('f');
            }
            let src = if p.get_I() {
                let val = p.get_imm().rotate_right(p.get_rotate() * 2);
                format!("#0x{val:x}")
            } else {
                reg(p.get_Rm()).to_string()
            };
            format!("msr{c}\t{psr}_{fields}, {src}")
        }

        I::SWI(s) => format!("swi{c}\t#0x{:x}", s.raw & 0x00FF_FFFF),
        I::Undefined => "undefined".to_string(),
    }
}

/// Render an ARM word/byte load/store addressing mode.
fn arm_mem_addr(m: &arm::Memory) -> String {
    let rn = reg(m.get_Rn());
    let sign = if m.get_U() { "" } else { "-" };
    let wb = if m.get_W() { "!" } else { "" };
    let offset = if m.get_I() {
        // Register offset (note: ARM memory I-bit is inverted vs data-proc).
        let rm = reg(m.get_Rm());
        let sh = m.get_sh();
        let amt = m.get_shamt5();
        if amt == 0 && sh == 0 {
            format!("{sign}{rm}")
        } else {
            format!("{sign}{rm}, {} #{amt}", shift_name(sh))
        }
    } else {
        format!("#{sign}0x{:x}", m.get_imm())
    };
    if m.get_P() {
        format!("[{rn}, {offset}]{wb}")
    } else {
        format!("[{rn}], {offset}")
    }
}

/// Render an ARM halfword / signed-byte load/store addressing mode.
fn arm_extra_mem_addr(m: &arm::ExtraMemory) -> String {
    let rn = reg(m.get_Rn());
    let sign = if m.get_U() { "" } else { "-" };
    let wb = if m.get_W() { "!" } else { "" };
    let offset = if m.get_I() {
        let imm = (m.get_imm7_4() << 4) | m.get_imm3_0();
        format!("#{sign}0x{imm:x}")
    } else {
        format!("{sign}{}", reg(m.get_Rm()))
    };
    if m.get_P() {
        format!("[{rn}, {offset}]{wb}")
    } else {
        format!("[{rn}], {offset}")
    }
}

/// Disassemble one THUMB instruction located at `pc`. Common instructions get
/// full operands; rarer variants render just the mnemonic (the caller always
/// shows the raw hex alongside).
#[allow(clippy::too_many_lines)]
pub fn thumb(raw: u16, pc: u32) -> String {
    use thumb::Instruction as I;
    let ins = thumb::decode(raw);

    // 3-bit register fields used by most THUMB encodings.
    let rd2_0 = (raw & 0x7) as u32;
    let rn5_3 = ((raw >> 3) & 0x7) as u32;
    let rm5_3 = ((raw >> 3) & 0x7) as u32; // alias
    let rd10_8 = ((raw >> 8) & 0x7) as u32;

    match &ins {
        // Format 1: move shifted register — LSL/LSR/ASR Rd, Rm, #imm5
        I::LSLThumb1(_) | I::LSRThumb1(_) | I::ASRThumb1(_) => {
            let m = match &ins {
                I::LSLThumb1(_) => "lsl",
                I::LSRThumb1(_) => "lsr",
                _ => "asr",
            };
            let imm5 = ((raw >> 6) & 0x1F) as u32;
            format!("{m}\t{}, {}, #{imm5}", reg(rd2_0), reg(rn5_3))
        }
        // Format 2: ADD/SUB Rd, Rn, Rm | #imm3
        I::ADD3(_) | I::SUB3(_) => {
            let m = if matches!(ins, I::ADD3(_)) { "add" } else { "sub" };
            format!("{m}\t{}, {}, {}", reg(rd2_0), reg(rn5_3), reg(((raw >> 6) & 0x7) as u32))
        }
        I::ADD1(_) | I::SUB2(_) => {
            let m = if matches!(ins, I::ADD1(_)) { "add" } else { "sub" };
            let imm3 = ((raw >> 6) & 0x7) as u32;
            format!("{m}\t{}, {}, #{imm3}", reg(rd2_0), reg(rn5_3))
        }
        // Format 3: MOV/CMP/ADD/SUB Rd, #imm8
        I::MOV1(_) | I::CMP1(_) | I::ADD2(_) | I::SUB1(_) => {
            let m = match &ins {
                I::MOV1(_) => "mov",
                I::CMP1(_) => "cmp",
                I::ADD2(_) => "add",
                _ => "sub",
            };
            format!("{m}\t{}, #0x{:x}", reg(rd10_8), raw & 0xFF)
        }
        // Format 4: ALU operations Rd, Rm
        I::AND(_) | I::EOR(_) | I::LSLThumb4(_) | I::LSR2(_) | I::ASRThumb4(_) | I::ADCThumb4(_)
        | I::SBC(_) | I::RORThumb4(_) | I::TST(_) | I::NEG(_) | I::CMP2(_) | I::CMNThumb4(_)
        | I::ORR(_) | I::MUL(_) | I::BIC(_) | I::MVN(_) => {
            let m = match &ins {
                I::AND(_) => "and",
                I::EOR(_) => "eor",
                I::LSLThumb4(_) => "lsl",
                I::LSR2(_) => "lsr",
                I::ASRThumb4(_) => "asr",
                I::ADCThumb4(_) => "adc",
                I::SBC(_) => "sbc",
                I::RORThumb4(_) => "ror",
                I::TST(_) => "tst",
                I::NEG(_) => "neg",
                I::CMP2(_) => "cmp",
                I::CMNThumb4(_) => "cmn",
                I::ORR(_) => "orr",
                I::MUL(_) => "mul",
                I::BIC(_) => "bic",
                _ => "mvn",
            };
            format!("{m}\t{}, {}", reg(rd2_0), reg(rm5_3))
        }
        // Format 5: Hi-register ops / BX
        I::ADDHiRegister(_) | I::CMPThumb5(_) | I::MOV3(_) => {
            let m = match &ins {
                I::ADDHiRegister(_) => "add",
                I::CMPThumb5(_) => "cmp",
                _ => "mov",
            };
            let h1 = ((raw >> 7) & 1) as u32;
            let h2 = ((raw >> 6) & 1) as u32;
            let rd = (raw & 0x7) as u32 | (h1 << 3);
            let rm = ((raw >> 3) & 0x7) as u32 | (h2 << 3);
            format!("{m}\t{}, {}", reg(rd), reg(rm))
        }
        I::BX(_) => {
            let h2 = ((raw >> 6) & 1) as u32;
            let rm = ((raw >> 3) & 0x7) as u32 | (h2 << 3);
            format!("bx\t{}", reg(rm))
        }

        // PC-relative / SP-relative loads
        I::LDR3(_) => format!("ldr\t{}, [pc, #0x{:x}]", reg(rd10_8), ((raw & 0xFF) as u32) << 2),
        I::LDR4(_) => format!("ldr\t{}, [sp, #0x{:x}]", reg(rd10_8), ((raw & 0xFF) as u32) << 2),
        I::STR3(_) => format!("str\t{}, [sp, #0x{:x}]", reg(rd10_8), ((raw & 0xFF) as u32) << 2),

        // Load/store with immediate offset
        I::LDR1(_) | I::STR1(_) | I::LDRB(_) | I::STRB_IMM_OFFET(_) => {
            let (m, scale) = match &ins {
                I::LDR1(_) => ("ldr", 2),
                I::STR1(_) => ("str", 2),
                I::LDRB(_) => ("ldrb", 0),
                _ => ("strb", 0),
            };
            let off = (((raw >> 6) & 0x1F) as u32) << scale;
            format!("{m}\t{}, [{}, #0x{off:x}]", reg(rd2_0), reg(rn5_3))
        }
        I::LDRHThumb8(_) | I::STRH(_) => {
            let m = if matches!(ins, I::STRH(_)) { "strh" } else { "ldrh" };
            let off = (((raw >> 6) & 0x1F) as u32) << 1;
            format!("{m}\t{}, [{}, #0x{off:x}]", reg(rd2_0), reg(rn5_3))
        }
        // Load/store with register offset
        I::LDRRegOffset(_) | I::STRRegOffset(_) | I::LDRBRegOffset(_) | I::STRBRegOffset(_)
        | I::LDRH(_) | I::STRHRegOffset(_) | I::LDSBThumb8(_) | I::LDSHThumb8(_) => {
            let m = match &ins {
                I::LDRRegOffset(_) => "ldr",
                I::STRRegOffset(_) => "str",
                I::LDRBRegOffset(_) => "ldrb",
                I::STRBRegOffset(_) => "strb",
                I::LDRH(_) => "ldrh",
                I::STRHRegOffset(_) => "strh",
                I::LDSBThumb8(_) => "ldrsb",
                _ => "ldrsh",
            };
            let rm = ((raw >> 6) & 0x7) as u32;
            format!("{m}\t{}, [{}, {}]", reg(rd2_0), reg(rn5_3), reg(rm))
        }

        // ADD Rd, pc/sp, #imm8*4  (formats 12)
        I::ADD6(_) => format!("add\t{}, sp, #0x{:x}", reg(rd10_8), ((raw & 0xFF) as u32) << 2),
        // ADD/SUB sp, #imm7*4 (format 13)
        I::ADD7(_) => {
            let imm = ((raw & 0x7F) as u32) << 2;
            if raw & 0x80 != 0 {
                format!("sub\tsp, #0x{imm:x}")
            } else {
                format!("add\tsp, #0x{imm:x}")
            }
        }

        // Block transfer / stack
        I::PUSH(_) => {
            let mut mask = (raw & 0xFF) as u32;
            if raw & 0x100 != 0 {
                mask |= 1 << 14; // lr
            }
            format!("push\t{}", reg_list(mask))
        }
        I::POP(_) => {
            let mut mask = (raw & 0xFF) as u32;
            if raw & 0x100 != 0 {
                mask |= 1 << 15; // pc
            }
            format!("pop\t{}", reg_list(mask))
        }
        I::LDMIA(_) | I::STMIA(_) => {
            let m = if matches!(ins, I::LDMIA(_)) { "ldmia" } else { "stmia" };
            format!("{m}\t{}!, {}", reg(rd10_8), reg_list((raw & 0xFF) as u32))
        }

        // Branches
        I::B(_) => {
            // Unconditional: 11-bit signed offset *2 relative to pc+4.
            let off = ((((raw & 0x7FF) as i32) << 21) >> 20) as i64;
            let target = (pc as i64).wrapping_add(4).wrapping_add(off) as u32;
            format!("b\t#0x{target:x}")
        }
        I::B2(_) => {
            let cnd = ((raw >> 8) & 0xF) as u32;
            // 8-bit signed offset *2 relative to pc+4.
            let off = ((((raw & 0xFF) as i32) << 24) >> 23) as i64;
            let target = (pc as i64).wrapping_add(4).wrapping_add(off) as u32;
            format!("b{}\t#0x{target:x}", cond(cnd))
        }
        I::BL(_) => {
            // Half-instruction; the assembler emits two halfwords. Render the
            // raw offset field; full target needs both halves.
            format!("bl\t(half) #0x{:x}", raw & 0x7FF)
        }
        I::SWI(_) => format!("swi\t#0x{:x}", raw & 0xFF),
        I::Undefined => "undefined".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm(s: String) -> String {
        s.replace('\t', " ")
    }

    #[test]
    fn arm_common() {
        assert_eq!(norm(arm(0xE3A0_0001, 0)), "mov r0, #0x1");
        assert_eq!(norm(arm(0xE281_1001, 0)), "add r1, r1, #0x1");
        assert_eq!(norm(arm(0xE12F_FF1E, 0)), "bx lr");
        // STMDB sp!, {lr}  (push lr)
        assert_eq!(norm(arm(0xE92D_4000, 0)), "stmdb sp!, {lr}");
        // Branch to self: target == pc (matches the No$gba screenshot's `b`).
        assert_eq!(norm(arm(0xEAFF_FFFE, 0x0800_00EC)), "b #0x80000ec");
        // LDR r0, [r1, #4]
        assert_eq!(norm(arm(0xE591_0004, 0)), "ldr r0, [r1, #0x4]");
        // CMP r0, #0
        assert_eq!(norm(arm(0xE350_0000, 0)), "cmp r0, #0x0");
    }

    #[test]
    fn thumb_common() {
        // movs r0, #1  (0x2001)
        assert_eq!(norm(thumb(0x2001, 0)), "mov r0, #0x1");
        // push {lr} (0xB500)
        assert_eq!(norm(thumb(0xB500, 0)), "push {lr}");
        // pop {pc} (0xBD00)
        assert_eq!(norm(thumb(0xBD00, 0)), "pop {pc}");
        // bx lr (0x4770)
        assert_eq!(norm(thumb(0x4770, 0)), "bx lr");
        // add r0, r1, r2 (0x1888)
        assert_eq!(norm(thumb(0x1888, 0)), "add r0, r1, r2");
    }
}
