use super::MutRaw;

pub trait ByteWritable: MutRaw {
    fn write_byte(&mut self, addr: u32, data: u8) {
        self.mut_raw(addr)[0] = data;
    }
}

pub trait HalfWordWritable: MutRaw {
    fn write_halfword(&mut self, addr: u32, data: u16) {
        self.mut_raw(addr)[..2].copy_from_slice(&data.to_le_bytes());
    }
}

pub trait WordWritable: MutRaw {
    fn write_word(&mut self, addr: u32, data: u32) {
        self.mut_raw(addr)[..4].copy_from_slice(&data.to_le_bytes());
    }
}