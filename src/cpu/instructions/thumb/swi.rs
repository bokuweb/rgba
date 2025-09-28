use crate::cpu::bus::accessor::BusAccessor;
use crate::cpu::decoder::thumb::ThumbSoftwareInterrupt;
use crate::cpu::instructions::{ExecuteResult, PipelineStatus};
use crate::cpu::registers::psr::{Mode, PSR};
use crate::cpu::bios::Bios;
use crate::types::Word;

// Thumb SWI は ARM と同様に SVC モードへ遷移し BIOS へ入るが、
// 返りアドレスは Thumb の 2 バイト幅に合わせて PC-2 を LR_svc に置く。
pub fn exec_thumb_swi<T: BusAccessor>(
    _bus: &T,
    dec: ThumbSoftwareInterrupt,
    gpr: &mut [Word; 16],
    cpsr: &mut PSR,
    spsr: &mut PSR,
) -> ExecuteResult {
    // 原因: Thumb の SWI が未実装（decode が todo!、実行関数不在）だったため、
    //       多数の Thumb テストが失敗していた。ここで実装する。

    // 現在の CPSR を保存
    *spsr = *cpsr;

    // LR_svc に復帰先 (PC - 2) を保存（Thumb は 2 バイト命令）
    let lr_svc = gpr[15] - 2;

    // SVC へ遷移、IRQ 無効、ARM モードへ（BIOS は ARM コード）
    cpsr.set_mode(Mode::Supervisor);
    cpsr.set_I(true);
    cpsr.set_T(false);
    gpr[14] = lr_svc;

    // 8bit 即値を BIOS ディスパッチへ渡す
    let swi_number = dec.get_immediate() as u32;
    Bios::execute_swi(_bus, swi_number, gpr);

    // エミュレート BIOS: 呼び出し後に元の CPSR に戻し、PC を LR_svc に復帰
    *cpsr = *spsr;
    gpr[15] = lr_svc;
    (1, PipelineStatus::Flush)
}


