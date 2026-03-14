use super::super::PipelineStatus;

use crate::cpu::bus::accessor::*;
use crate::cpu::constants::*;
use crate::cpu::decoder::thumb::*;
use crate::cpu::instructions::*;
use crate::types::*;

pub fn exec_thumb_stmia<T>(bus: &mut T, dec: BlockDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let rn = dec.get_Rn() as usize;
    let mut base = gpr[rn];
    let register_list = dec.get_register_list();

    let mut cycle: Cycle = 0;
    let mut is_n_cycle = true;

    // 特殊ケース: 空のrlistはPCをストアし、ベースを+0x40進める（gba-tests準拠）
    if register_list == 0 {
        cycle += bus.compute_cycle(base, AccessType::NonSeq(AccessWidth::Word));
        // 空rlistのSTMは PC+2 を書き込む（次命令で読み出すPCと一致させるため）。
        bus.write_word(base & 0xFFFF_FFFC, gpr[PC].wrapping_add(2));
        gpr[rn] = base.wrapping_add(0x40);
        // 次命令プリフェッチ相当
        let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::NonSeq(AccessWidth::HalfWord));
        return (cycle, PipelineStatus::Continue);
    }

    // 総転送バイト数と初期ベースを保存（ベースがrlistに含まれる場合の格納値に使用）
    let total_bytes = (register_list.count_ones() as u32) * 4;
    let initial_base = base;
    let first_index = register_list.trailing_zeros() as usize; // 最初に格納されるレジスタ番号
    for i in 0..0x8 {
        if register_list & (1 << i) != 0 {
            let access_type = if is_n_cycle {
                is_n_cycle = false;
                AccessType::NonSeq(AccessWidth::Word)
            } else {
                AccessType::Seq(AccessWidth::Word)
            };
            cycle += bus.compute_cycle(base, access_type);

            // 修正(THUMB.15 test 230/232): ベースがrlistに含まれる場合の格納値は位置依存。
            // - ベースがrlistの先頭（最小レジスタ）なら initial_base を格納（t232）
            // - それ以外の位置なら 最終ベース(initial_base + total_bytes) を格納（t230）
            let value = if i as usize == rn {
                if rn == first_index { initial_base } else { initial_base.wrapping_add(total_bytes) }
            } else {
                gpr[i as usize]
            };
            bus.write_word(base, value);
            base = base.wrapping_add(4);
        }
    }
    gpr[rn] = base;
    // consume 1N cycle to prefetch next cycle
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::NonSeq(AccessWidth::HalfWord));
    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_ldmia<T>(bus: &T, dec: BlockDataTransfer, gpr: &mut [Word; 16], started: bool) -> ExecuteResult
where
    T: BusAccessor,
{
    if started {
    }
    let rn = dec.get_Rn();
    let rn_idx = rn as usize;
    let mut base = gpr[rn_idx];
    let register_list = dec.get_register_list();

    let mut cycle: Cycle = 0;
    let mut is_n_cycle = true;

    // 特殊ケース: 空のrlistは [base] からPCをロードし、ベースを+0x40進める（gba-tests準拠）
    if register_list == 0 {
        let data = bus.read_word(base & 0xFFFF_FFFC);
        cycle += bus.compute_cycle(base, AccessType::NonSeq(AccessWidth::Word));
        gpr[PC] = data & 0xFFFF_FFFE;
        base = base.wrapping_add(0x40);
        gpr[rn_idx] = base;
        // Iサイクル + 次プリフェッチ相当
        let cycle = cycle + 1 + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
        return (cycle, PipelineStatus::Flush);
    }

    let total_bytes = (register_list.count_ones() as u32) * 4;
    for i in 0..0x8 {
        if register_list & (1 << i) != 0 {
            let d = bus.read_word(base & 0xFFFF_FFFC);
            // dbg!(base, d);
            // 読み出し値は常にメモリの内容。ベースがrlistに含まれても、後段の書き戻しで最終アドレスをRnへ設定する。
            gpr[i] = d;
            let access_type = if is_n_cycle {
                is_n_cycle = false;
                AccessType::NonSeq(AccessWidth::Word)
            } else {
                AccessType::Seq(AccessWidth::Word)
            };
            cycle += bus.compute_cycle(base, access_type);

            base = base.wrapping_add(4);
        }
    }
    // ベースはリストに含まれていても必ず書き戻す（最終アドレス）。
    gpr[rn_idx] = base;

    if started {
    }

    // Consume 1I cycle.
    let cycle = cycle + 1;
    // consume merged I-S cycle
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));

    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_push<T>(bus: &mut T, dec: BlockDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let mut addr = gpr[SP] - 4;
    let register_list = dec.get_register_list();

    let mut cycle: Cycle = 0;
    let mut is_n_cycle = true;

    if dec.get_R() {
        bus.write_word(addr, gpr[LR]);
        addr = addr - 4;
    }

    for i in 0..0x8 {
        let i = 7 - i;
        if register_list & (1 << i) != 0 {
            bus.write_word(addr & 0xFFFF_FFFC, gpr[i]);

            let access_type = if is_n_cycle {
                is_n_cycle = false;
                AccessType::NonSeq(AccessWidth::Word)
            } else {
                AccessType::Seq(AccessWidth::Word)
            };
            cycle += bus.compute_cycle(addr, access_type);

            addr -= 4;
        }
    }

    gpr[SP] = addr + 4;
    // Consume 1N cycle to next prefetch
    let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::NonSeq(AccessWidth::HalfWord));
    // dbg!(&gpr, "after push");
    (cycle, PipelineStatus::Continue)
}

pub fn exec_thumb_pop<T>(bus: &mut T, dec: BlockDataTransfer, gpr: &mut [Word; 16]) -> ExecuteResult
where
    T: BusAccessor,
{
    let mut cycle: Cycle = 0;
    let mut is_n_cycle = true;

    let mut addr = gpr[SP];
    let register_list = dec.get_register_list();

    // dbg!("befpre pop", &gpr);

    for i in 0..0x8 {
        if register_list & (1 << i) != 0 {
            let data = bus.read_word(addr & 0xFFFF_FFFC);
            let access_type = if is_n_cycle {
                is_n_cycle = false;
                AccessType::NonSeq(AccessWidth::Word)
            } else {
                AccessType::Seq(AccessWidth::Word)
            };
            cycle += bus.compute_cycle(addr, access_type);

            gpr[i] = data;
            addr += 4;
        }
    }

    if dec.get_R() {
        let data = bus.read_word(addr);
        gpr[PC] = data & 0xFFFF_FFFE;
        addr += 4;
    }

    gpr[SP] = addr;
    // dbg!("after pop", &gpr);

    // Consume 1I cycle.
    let cycle = cycle + 1;

    if dec.get_R() {
        (cycle, PipelineStatus::Flush)
    } else {
        // consume merged I-S cycle
        let cycle = cycle + bus.compute_cycle(gpr[PC], AccessType::Seq(AccessWidth::HalfWord));
        (cycle, PipelineStatus::Continue)
    }
}
