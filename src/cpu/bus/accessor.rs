use crate::types::*;

pub trait BusAccessor {
    fn compute_cycle(&self, addr: Word, access_type: AccessType) -> Cycle;
    fn set_open_bus_context(&mut self, _pc: Word, _instruction_width: Word) {}
    fn set_cpu_halted(&mut self, _halted: bool) {}
    fn is_cpu_halted(&self) -> bool { false }
    fn has_pending_interrupt_flags(&self) -> bool { false }
    /// BIOS interrupt-check flags (0x03007FF8), maintained by the game's IRQ
    /// handler and polled by IntrWait / VBlankIntrWait.
    fn read_bios_if(&self) -> HalfWord { 0 }
    /// Clear the given bits from the BIOS interrupt-check flags.
    fn clear_bios_if(&mut self, _mask: HalfWord) {}
    /// Arm an IntrWait: record the interrupt mask the CPU is blocked on.
    fn set_intr_wait(&mut self, _mask: HalfWord) {}
    /// Clear the armed IntrWait (the awaited interrupt arrived).
    fn clear_intr_wait(&mut self) {}
    /// The interrupt mask the CPU is currently blocked on via IntrWait, if any.
    fn intr_wait_mask(&self) -> Option<HalfWord> { None }
    fn read_byte(&self, addr: Word) -> Byte;
    fn read_halfword(&self, addr: Word) -> HalfWord;
    fn read_word(&self, addr: Word) -> Word;
    fn write_byte(&mut self, addr: Word, data: Byte);
    fn write_halfword(&mut self, addr: Word, data: HalfWord);
    fn write_word(&mut self, addr: Word, data: Word);
}
