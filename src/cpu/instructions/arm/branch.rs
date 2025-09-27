use crate::cpu::constants::*;
use crate::cpu::decoder::arm::*;
use crate::cpu::instructions::*;
use crate::types::*;

pub fn exec_arm_bl(dec: Branch, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()> {
    gpr[LR] = gpr[PC] - 4;
    exec_arm_b(dec, gpr)
}

pub fn exec_arm_b(dec: Branch, gpr: &mut [Word; 16]) -> Result<ExecuteResult, ()> {
    let imm = dec.get_offset();
    let imm = (if imm & 0x0080_0000 != 0 {
        imm | 0xFF00_0000
    } else {
        imm
    }) as i32;
    let old_pc = gpr[PC];
    let new_pc = (gpr[PC] as i32 + imm * 4) as Word;
    println!("Branch executed: PC 0x{:08X} -> 0x{:08X} (offset: {})", old_pc, new_pc, imm);

    // Test 005 debugging - check if this is the MI branch failure
    if old_pc == 0x08000154 || old_pc == 0x08000158 {
        println!("DEBUG: Test 005 MI branch - offset={}, expected offset=2 for success", imm);
        if imm == -2 {
            println!("WARNING: MI branch failed! Jumping backwards to m_exit 5");
        } else if imm == 2 {
            println!("SUCCESS: MI branch taken forward to t006 at PC=0x{:08X}", new_pc);
        }
    }

    gpr[PC] = new_pc;

    // Critical debug for Test 005
    if old_pc >= 0x08000150 && old_pc <= 0x08000160 {
        println!("BRANCH EXECUTION: PC changed from 0x{:08X} to 0x{:08X}, returning PipelineStatus::Flush", old_pc, new_pc);
    }

    // dbg!("b");
    // b does not consume extra cycle.
    Ok((0, PipelineStatus::Flush))
}
