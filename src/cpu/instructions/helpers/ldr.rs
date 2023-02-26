use crate::cpu::bus::accessor::*;
use crate::types::*;

pub fn read_ldr_data<T: BusAccessor>(bus: &T, addr: Word) -> Word {
    let data = bus.read_word(addr & 0xFFFF_FFFC);
    // The loaded data is rotated right by one, two or three bytes according to bits [1:0] of the address.
    // https://www.keil.com/support/man/docs/armasm/armasm_dom1359731171041.htm
    let rotate = addr & 0x03;
    data.rotate_right(rotate.wrapping_shl(3))
}
