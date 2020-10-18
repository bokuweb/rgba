use crate::types::*;

pub trait BusAccessor {
    fn compute_cycle(&self, addr: Word, access_type: AccessType) -> Cycle;
    fn read_byte(&self, addr: Word) -> Byte;
    fn read_halfword(&self, addr: Word) -> HalfWord;
    fn read_word(&self, addr: Word) -> Word;
    fn write_byte(&mut self, addr: Word, data: Byte);
    fn write_halfword(&mut self, addr: Word, data: HalfWord);
    fn write_word(&mut self, addr: Word, data: Word);
}
