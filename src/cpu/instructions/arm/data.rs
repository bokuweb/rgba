use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::shift::{is_carry_over, ror, shift};
use crate::cpu::instructions::ExecuteResult;
use crate::cpu::registers::psr::{Mode, PSR};
use crate::cpu::registers::{BankGpr, BankSpsr};
use crate::types::*;

pub fn exec_data_processing<T, F>(bus: &T, gpr: &mut [Word; 16], dec: DataProcessing, cpsr: &mut PSR, data_process: &mut F) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
    F: FnMut(&mut [Word; 16], Word, bool, &mut PSR),
{
    let mut cycle = 0;

    let (value, carry) = if dec.get_I() {
        let imm = dec.get_imm();
        let rot = dec.get_rotate(); // 0..15
        if rot == 0 {
            (imm, cpsr.get_C())
        } else {
            let amount = (rot * 2) & 31; // 2,4,...,30
            let res = ror(imm, amount, cpsr.get_C(), false);
            let carry = (res & 0x8000_0000) != 0; // 即値回転のCは結果bit31
            (res, carry)
        }
    } else if dec.get_bit4() {
        // Shift by register: PC is treated as PC+12 (PC+8 already stored, add +4 here)
        let rm = get_operand(&*gpr, dec.get_Rm(), dec.get_I(), dec.get_bit4());
        // if shifted by register, consume 1I cycle.
        cycle += 1;
        // only lower 8bit of Rs is used. Rs should not be PC per ARM spec.
        let rs = dec.get_Rs() as usize;
        let rs = gpr[rs];
        let shift_value = rs & 0xFF;
        // dbg!("shift", rm, shift_value);
        (
            shift(dec.get_sh().into(), rm, shift_value, cpsr.get_C(), true),
            is_carry_over(dec.get_sh().into(), rm, shift_value, cpsr.get_C(), true),
        )
    } else {
        let shift_value = dec.get_shamt5();
        // Shift by immediate: our gpr[PC] already reflects PC+8 in ARM state
        // so do NOT add any extra offset here.
        let rm = get_operand(&*gpr, dec.get_Rm(), dec.get_I(), dec.get_bit4());
        (
            shift(dec.get_sh().into(), rm, shift_value, cpsr.get_C(), dec.get_bit4()),
            is_carry_over(dec.get_sh().into(), rm, shift_value, cpsr.get_C(), dec.get_bit4()),
        )
    };

    data_process(gpr, value, carry, cpsr);

    if dec.get_Rd() == PC as u32 {
        // waiting for pipeline filled is executed by caller.
        Ok((cycle, PipelineStatus::Flush))
    } else {
        let s = bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::Word));
        Ok((s + cycle, PipelineStatus::Continue))
    }
}

pub fn exec_arm_mov<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, c, cpsr| {
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(value as u32);
                cpsr.set_Z_from(value as u32);
                cpsr.set_C(c);
            }
        }
        if gpr[15] >= 134224832 && gpr[15] <= 134224892 {
            dbg!("🔥value", value);
        }
        gpr[rd] = value;

        // Critical debug: detect m_exit 5 execution
        if rd == 12 && value == 5 {
            println!("🚨 CRITICAL: m_exit 5 executed! R12=5 set at PC=0x{:08X}", gpr[PC]);
        }
    })
}

pub fn exec_arm_and<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn_value = get_operand(&*gpr, dec.get_Rn(), dec.get_I(), dec.get_bit4());
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, c, cpsr| {
        let d = rn_value & value;
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d as u32);
                cpsr.set_Z_from(d as u32);
                cpsr.set_C(c);
            }
        }
        gpr[rd] = d;

        // Critical debug: detect m_exit 5 execution
        if rd == 12 && d == 5 {
            println!("🚨 CRITICAL: m_exit 5 executed! R12=5 set at PC=0x{:08X}", gpr[PC]);
        }
    })
}

pub fn exec_arm_eor<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn_value = get_operand(&*gpr, dec.get_Rn(), dec.get_I(), dec.get_bit4());
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, carry, cpsr| {
        let d = rn_value ^ value;
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                // dbg!("EOR", carry, rd);
                cpsr.set_N_from(d);
                cpsr.set_Z_from(d);
                cpsr.set_C(carry);
            }
        }
        gpr[rd] = d;

        // Critical debug: detect m_exit 5 execution
        if rd == 12 && d == 5 {
            println!("🚨 CRITICAL: m_exit 5 executed! R12=5 set at PC=0x{:08X}", gpr[PC]);
        }
    })
}

pub fn exec_arm_sub<T>(
    bus: &T,
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
    spsr: &mut PSR,
    bank_gpr: &mut BankGpr,
    bank_spsr: &mut BankSpsr,
) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn_value = get_operand(&*gpr, dec.get_Rn(), dec.get_I(), dec.get_bit4());
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        let d = rn_value.wrapping_sub(value);
        if s {
            if rd == PC && cpsr.get_mode() != Mode::User {
                cpsr.restore(spsr, gpr, bank_gpr, bank_spsr);
            } else {
                cpsr.set_N_from(d as u32);
                cpsr.set_Z_from(d as u32);
                cpsr.set_C(rn_value >= value);
                let (_, v) = (rn_value as i32).overflowing_sub(value as i32);
                cpsr.set_V(v);
            }
        }
        gpr[rd] = d;

        // Critical debug: detect m_exit 5 execution
        if rd == 12 && d == 5 {
            println!("🚨 CRITICAL: m_exit 5 executed! R12=5 set at PC=0x{:08X}", gpr[PC]);
        }
    })
}

pub fn exec_arm_rsb<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn_value = get_operand(&*gpr, dec.get_Rn(), dec.get_I(), dec.get_bit4());
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        let d = value.wrapping_sub(rn_value);
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d as u32);
                cpsr.set_Z_from(d as u32);
                cpsr.set_C(value >= rn_value);
                let (_, v) = (value as i32).overflowing_sub(rn_value as i32);
                cpsr.set_V(v);
            }
        }
        gpr[rd] = d;

        // Critical debug: detect m_exit 5 execution
        if rd == 12 && d == 5 {
            println!("🚨 CRITICAL: m_exit 5 executed! R12=5 set at PC=0x{:08X}", gpr[PC]);
        }
    })
}

fn get_operand(gpr: &[Word; 16], reg: u32, I: bool, R: bool) -> u32 {
    // When using R15 as operand (Rm or Rn)]
    // the returned value depends on the instruction: PC+12 if I=0,R=1 (shift by register)
    // otherwise PC+8 (shift by immediate).
    if reg as usize == PC && !I && R {
        return gpr[reg as usize] + 4;
    }
    gpr[reg as usize]
}

pub fn exec_arm_add<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    // Rn はその時点のレジスタ値を使用（PC は既にパイプライン相当オフセット込み）
    let op1 = get_operand(&*gpr, dec.get_Rn(), dec.get_I(), dec.get_bit4());
    let result = exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        let d = op1 as u64 + value as u64;
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d as u32);
                cpsr.set_Z_from(d as u32);
                cpsr.set_C_from(d);
                let (_, v) = (op1 as i32).overflowing_add(value as i32);
                cpsr.set_V(v);
            }
        }
        gpr[rd] = d as u32;
    });
    result
}

pub fn exec_arm_adc<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let c_flag = cpsr.get_C();
    let rn_value = get_operand(&*gpr, dec.get_Rn(), dec.get_I(), dec.get_bit4());
    let result = exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        let c = if c_flag { 1 } else { 0 } as u32;
        let d = rn_value as u64 + value as u64 + c as u64;
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d as u32);
                cpsr.set_Z_from(d as u32);
                cpsr.set_C_from(d);
                // V: (~(op1 ^ (op2 + Cin)) & (op1 ^ result)) の MSB
                let op1 = rn_value as i32;
                let op2 = value as i32;
                let cin = if c_flag { 1i32 } else { 0i32 };
                let result = op1.wrapping_add(op2).wrapping_add(cin);
                let op2c = op2.wrapping_add(cin);
                let v = (((!(op1 ^ op2c)) & (op1 ^ result)) as u32 & 0x8000_0000) != 0;
                cpsr.set_V(v);
            }
        }
        gpr[rd] = d as u32;
    });
    result
}

pub fn exec_arm_sbc<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let cin = cpsr.get_C();
    let rn_value = get_operand(&*gpr, dec.get_Rn(), dec.get_I(), dec.get_bit4());
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        let borrow_in = if cin { 0u32 } else { 1u32 };
        let d = rn_value.wrapping_sub(value).wrapping_sub(borrow_in);
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d);
                cpsr.set_Z_from(d);
                // C（ノーボロー）を 64bit で厳密に
                let rn64 = rn_value as u64;
                let sub64 = (value as u64) + (borrow_in as u64);
                cpsr.set_C(rn64 >= sub64);
                // V（符号オーバーフロー）
                let op1 = rn_value as i32;
                let op2c = (value as i32).wrapping_add(borrow_in as i32);
                let (_, v) = op1.overflowing_sub(op2c);
                cpsr.set_V(v);
            }
        }
        gpr[rd] = d
    })
}

pub fn exec_arm_rsc<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let cin = cpsr.get_C() as u32;
    let rn_value = get_operand(&*gpr, dec.get_Rn(), dec.get_I(), dec.get_bit4());
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        let borrow_in = 1 - cin;
        let d = value.wrapping_sub(rn_value.wrapping_add(borrow_in));

        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d);
                cpsr.set_Z_from(d);
                // C（ノーボロー）を 64bit で厳密に
                let op2_64 = value as u64;
                let subtrahend64 = (rn_value as u64) + (borrow_in as u64);
                cpsr.set_C(op2_64 >= subtrahend64);
                // V（符号オーバーフロー）
                let (_, v) = (value as i32)
                    .overflowing_sub((rn_value as i32).wrapping_add(borrow_in as i32));
                cpsr.set_V(v);
            }
        }
        gpr[rd] = d;

        // Critical debug: detect m_exit 5 execution
        if rd == 12 && d == 5 {
            println!("🚨 CRITICAL: m_exit 5 executed! R12=5 set at PC=0x{:08X}", gpr[PC]);
        }
    })
}

pub fn exec_arm_tst<T>(
    bus: &T,
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
    spsr: &mut PSR,
    bank_gpr: &mut BankGpr,
    bank_spsr: &mut BankSpsr,
) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    let rn_value = get_operand(&*gpr, dec.get_Rn(), dec.get_I(), dec.get_bit4());
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, carry, cpsr| {
        // Bad TST with Rn == PC in privileged mode: restore CPSR from SPSR, do not update flags
        if rn == PC && cpsr.get_mode() != Mode::User {
            cpsr.restore(spsr, gpr, bank_gpr, bank_spsr);
            return;
        }
        let tst = rn_value & value;
        cpsr.set_N(tst >> 31 != 0);
        cpsr.set_Z(tst == 0);
        cpsr.set_C(carry);
    })
}

pub fn exec_arm_teq<T>(
    bus: &T,
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
    spsr: &mut PSR,
    bank_gpr: &mut BankGpr,
    bank_spsr: &mut BankSpsr,
) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    let rn_value = get_operand(&*gpr, dec.get_Rn(), dec.get_I(), dec.get_bit4());
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, carry, cpsr| {
        // Bad TEQ with Rn == PC in privileged mode
        if rn == PC && cpsr.get_mode() != Mode::User {
            cpsr.restore(spsr, gpr, bank_gpr, bank_spsr);
            return;
        }
        let teq = rn_value ^ value;
        cpsr.set_N(teq >> 31 != 0);
        cpsr.set_Z(teq == 0);
        cpsr.set_C(carry);
    })
}

pub fn exec_arm_cmp<T>(
    bus: &T,
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
    spsr: &mut PSR,
    bank_gpr: &mut BankGpr,
    bank_spsr: &mut BankSpsr,
) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    let rn_value = get_operand(&*gpr, dec.get_Rn(), dec.get_I(), dec.get_bit4());
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        // Bad CMP with Rn == PC in privileged mode
        if rn == PC && cpsr.get_mode() != Mode::User {
            cpsr.restore(spsr, gpr, bank_gpr, bank_spsr);
            return;
        }
        let cmp = rn_value.wrapping_sub(value);
        cpsr.set_N(cmp >> 31 != 0);
        cpsr.set_Z(cmp == 0);
        let (_, v) = (rn_value as i32).overflowing_sub(value as i32);
        cpsr.set_V(v);
        cpsr.set_C(rn_value >= value);
    })
}

pub fn exec_arm_cmn<T>(
    bus: &T,
    dec: DataProcessing,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
    spsr: &mut PSR,
    bank_gpr: &mut BankGpr,
    bank_spsr: &mut BankSpsr,
) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    let rn_value = get_operand(&*gpr, dec.get_Rn(), dec.get_I(), dec.get_bit4());
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, _, cpsr| {
        // Bad CMN with Rn == PC in privileged mode
        if rn == PC && cpsr.get_mode() != Mode::User {
            cpsr.restore(spsr, gpr, bank_gpr, bank_spsr);
            return;
        }
        let cmn = (rn_value as u64).wrapping_add(value as u64);
        let res = rn_value.wrapping_add(value);
        cpsr.set_N_from(res);
        cpsr.set_Z_from(res);
        let (_, v) = (rn_value as i32).overflowing_add(value as i32);
        cpsr.set_V(v);
        cpsr.set_C(cmn > 0xFFFF_FFFF);
    })
}

pub fn exec_arm_orr<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn_value = get_operand(&*gpr, dec.get_Rn(), dec.get_I(), dec.get_bit4());
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, carry, cpsr| {
        let d = rn_value | value;
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d);
                cpsr.set_Z_from(d);
                cpsr.set_C(carry);
            }
        }
        gpr[rd] = d;

        // Critical debug: detect m_exit 5 execution
        if rd == 12 && d == 5 {
            println!("🚨 CRITICAL: m_exit 5 executed! R12=5 set at PC=0x{:08X}", gpr[PC]);
        }
    })
}

pub fn exec_arm_shift<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, carry, cpsr| {
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(value);
                cpsr.set_Z_from(value);
                cpsr.set_C(carry);
            }
        }
        gpr[rd] = value;

        // Critical debug: detect m_exit 5 execution
        if rd == 12 && value == 5 {
            println!("🚨 CRITICAL: m_exit 5 executed! R12=5 set at PC=0x{:08X}", gpr[PC]);
        }
    })
}

pub fn exec_arm_bic<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    let rn_value = get_operand(&*gpr, dec.get_Rn(), dec.get_I(), dec.get_bit4());
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, carry, cpsr| {
        let d = rn_value & !value;
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d);
                cpsr.set_Z_from(d);
                cpsr.set_C(carry);
            }
        }
        gpr[rd] = d;

        // Critical debug: detect m_exit 5 execution
        if rd == 12 && d == 5 {
            println!("🚨 CRITICAL: m_exit 5 executed! R12=5 set at PC=0x{:08X}", gpr[PC]);
        }
    })
}

pub fn exec_arm_mvn<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, carry, cpsr| {
        let d = !value;
        if s {
            if rd == PC {
                unimplemented!("data processing Rd = PC with S flag.");
            } else {
                cpsr.set_N_from(d);
                cpsr.set_Z_from(d);
                cpsr.set_C(carry);
            }
        }
        gpr[rd] = d;

        // Critical debug: detect m_exit 5 execution
        if rd == 12 && d == 5 {
            println!("🚨 CRITICAL: m_exit 5 executed! R12=5 set at PC=0x{:08X}", gpr[PC]);
        }
    })
}

pub fn exec_arm_rrx<T>(bus: &T, dec: DataProcessing, gpr: &mut [Word; 16], cpsr: &mut PSR) -> Result<ExecuteResult, ()>
where
    T: BusAccessor,
{
    let s = dec.get_S();
    let rd = dec.get_Rd() as usize;
    exec_data_processing(bus, gpr, dec, cpsr, &mut |gpr, value, carry: bool, cpsr| {
        if s {
            cpsr.set_N_from(value);
            cpsr.set_Z_from(value);
            cpsr.set_C(carry);
        }
        gpr[rd] = value;

        // Critical debug: detect m_exit 5 execution
        if rd == 12 && value == 5 {
            println!("🚨 CRITICAL: m_exit 5 executed! R12=5 set at PC=0x{:08X}", gpr[PC]);
        }
    })
}
