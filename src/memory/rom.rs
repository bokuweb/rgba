use super::readable::*;
use super::Raw;

#[derive(Debug)]
pub struct Rom(Vec<u8>);

impl Rom {
    pub fn new(size: usize, init: &[u8]) -> Self {
        let _ = size;
        Rom(init.to_vec())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Raw for Rom {
    fn raw(&self, addr: u32) -> &[u8] {
        &self.0[(addr as usize)..]
    }
}

impl ByteReadable for Rom {
    fn read_byte(&self, addr: u32) -> u8 {
        let len = self.0.len();
        if len == 0 { return 0xFF; }
        self.0[(addr as usize) % len]
    }
}

impl HalfWordReadable for Rom {
    fn read_halfword(&self, addr: u32) -> u16 {
        let len = self.0.len();
        if len == 0 { return 0xFFFF; }
        let i = (addr as usize) % len;
        let b0 = self.0[i] as u16;
        let b1 = self.0[(i + 1) % len] as u16;
        b0 | (b1 << 8)
    }
}

impl WordReadable for Rom {
    fn read_word(&self, addr: u32) -> u32 {
        let len = self.0.len();
        if len == 0 { return 0xFFFF_FFFF; }
        let i = (addr as usize) % len;
        let b0 = self.0[i] as u32;
        let b1 = self.0[(i + 1) % len] as u32;
        let b2 = self.0[(i + 2) % len] as u32;
        let b3 = self.0[(i + 3) % len] as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }
}

#[test]
fn rom_read_byte() {
    let rom = Rom::new(4, &vec![0x01, 0x00, 0x00, 0x00]);
    assert_eq!(rom.read_byte(0), 0x01);
}

#[test]
fn rom_read_halfword() {
    let rom = Rom::new(4, &vec![0x01, 0x02, 0x00, 0x00]);
    assert_eq!(rom.read_halfword(0), 0x0201);
}

#[test]
fn rom_read_word() {
    let rom = Rom::new(4, &vec![0x01, 0x02, 0x03, 0x04]);
    assert_eq!(rom.read_word(0), 0x0403_0201);
}
