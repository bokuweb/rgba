use super::super::types::{Byte, HalfWord, Word};

pub trait BusAccessor {
    fn read_byte(&self, addr: Word) -> Byte;
    fn read_halfword(&self, addr: Word) -> HalfWord;
    fn read_word(&self, addr: Word) -> Word;
    fn write_byte(&mut self, addr: Word, data: Byte);
    fn write_word(&mut self, addr: Word, data: Word);
}
