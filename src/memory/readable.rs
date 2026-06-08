use super::Raw;

pub trait ByteReadable: Raw {
    fn read_byte(&self, addr: u32) -> u8 {
        self.raw(addr)[0]
    }
}

pub trait HalfWordReadable: Raw {
    fn read_halfword(&self, addr: u32) -> u16 {
        let b = self.raw(addr);
        u16::from_le_bytes([b[0], b[1]])
    }
}

pub trait WordReadable: Raw {
    fn read_word(&self, addr: u32) -> u32 {
        let b = self.raw(addr);
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    }
}