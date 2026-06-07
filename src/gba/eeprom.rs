//! Serial EEPROM backup memory.
//!
//! GBA EEPROM is a bit-serial device mapped in the GamePak `0x0D000000` region
//! and driven exclusively by DMA: every 16-bit bus access transfers a single
//! bit (in bit 0). Two capacities exist — 4 Kbit (512 bytes, 6-bit dword
//! address) and 64 Kbit (8 KiB, 14-bit dword address). The address width is
//! discovered from the length of the driving DMA, which the bus reports via
//! [`Eeprom::prepare`].
//!
//! Command framing (MSB first), one bit per halfword:
//! * **Write**: `1 0`, address bits, 64 data bits, `0` stop.
//! * **Read**:  `1 1`, address bits, `0` stop — then the game DMA-reads 68
//!   bits: 4 ignored bits followed by the 64 data bits.

/// Largest supported capacity (64 Kbit = 8 KiB).
const MAX_BYTES: usize = 0x2000;
/// Bits clocked out for a read: 4 dummy bits + 64 data bits.
const READ_BITS: u8 = 68;

/// Protocol phase of the bit-serial state machine.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Idle between commands; the stop bit of a command is consumed here.
    Idle,
    /// Collecting the two opcode bits.
    Opcode,
    /// Collecting the dword address bits (`writing` selects the follow-up).
    Address { writing: bool },
    /// Collecting the 64 data bits of a write command.
    WriteData,
    /// Streaming a dword back to the host (4 dummy bits then 64 data bits).
    Reading,
}

/// A serial EEPROM save device.
pub struct Eeprom {
    /// Backing store (`0xFF`-initialised, like erased EEPROM).
    data: Vec<u8>,
    /// Dword-address width in bits: 6 (512 B) or 14 (8 KiB). 0 until detected.
    addr_bits: u8,
    /// Current protocol phase.
    phase: Phase,
    /// Bits collected so far in the current phase.
    bit_count: u8,
    /// Two-bit opcode accumulator.
    opcode: u8,
    /// Address accumulator.
    addr: u16,
    /// 64-bit data accumulator for write commands.
    data_reg: u64,
    /// Byte offset of the dword currently being read back.
    read_offset: usize,
    /// Number of bits already streamed during the current read (0..=67).
    read_index: u8,
    /// Set when the contents change, so the host can persist lazily.
    dirty: bool,
}

impl Eeprom {
    /// Create an EEPROM seeded with `initial` (e.g. a `.sav` file). The width is
    /// detected at runtime, so the full 8 KiB is allocated up front.
    #[must_use]
    pub fn new(initial: &[u8]) -> Self {
        let mut data = vec![0xFF; MAX_BYTES];
        let n = initial.len().min(MAX_BYTES);
        data[..n].copy_from_slice(&initial[..n]);
        Self {
            data,
            addr_bits: 0,
            phase: Phase::Idle,
            bit_count: 0,
            opcode: 0,
            addr: 0,
            data_reg: 0,
            read_offset: 0,
            read_index: 0,
            dirty: false,
        }
    }

    /// Capacity in bytes implied by the detected address width.
    fn capacity(&self) -> usize {
        if self.addr_bits == 6 {
            0x200
        } else {
            MAX_BYTES
        }
    }

    /// Raw backing bytes, trimmed to the detected capacity (for persistence).
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.data[..self.capacity()]
    }

    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Notify the device that a DMA of `count` units is about to access the
    /// EEPROM. `to_eeprom` is true for a write (host -> EEPROM), false for a
    /// read. The command transfer length identifies the address width.
    pub fn prepare(&mut self, to_eeprom: bool, count: usize) {
        if to_eeprom {
            // Command stream. 9/73 bits -> 6-bit address, 17/81 -> 14-bit.
            match count {
                9 | 73 => self.addr_bits = 6,
                17 | 81 => self.addr_bits = 14,
                _ => {}
            }
            self.phase = Phase::Opcode;
            self.bit_count = 0;
            self.opcode = 0;
            self.addr = 0;
            self.data_reg = 0;
        } else {
            // Read-data stream begins; output starts with the 4 dummy bits.
            self.phase = Phase::Reading;
            self.read_index = 0;
        }
    }

    /// Read one bit (returned in bit 0) during a read-data transfer.
    #[must_use]
    pub fn read_bit(&mut self) -> u16 {
        if self.phase != Phase::Reading {
            return 1;
        }
        let bit = if self.read_index < 4 {
            0 // four leading dummy bits
        } else {
            let data_index = u32::from(self.read_index - 4); // 0..=63
            let byte = self.data[self.read_offset + (data_index / 8) as usize];
            (byte >> (7 - (data_index % 8))) & 1
        };
        self.read_index += 1;
        if self.read_index >= READ_BITS {
            self.phase = Phase::Idle;
        }
        u16::from(bit)
    }

    /// Write one command/data bit (taken from bit 0).
    pub fn write_bit(&mut self, value: u16) {
        if self.addr_bits == 0 {
            return; // width not yet known; ignore stray accesses
        }
        let bit = (value & 1) as u8;
        match self.phase {
            Phase::Opcode => {
                self.opcode = (self.opcode << 1) | bit;
                self.bit_count += 1;
                if self.bit_count == 2 {
                    self.bit_count = 0;
                    self.phase = match self.opcode {
                        0b10 => Phase::Address { writing: true },
                        0b11 => Phase::Address { writing: false },
                        _ => Phase::Idle,
                    };
                }
            }
            Phase::Address { writing } => {
                self.addr = (self.addr << 1) | u16::from(bit);
                self.bit_count += 1;
                if self.bit_count == self.addr_bits {
                    self.bit_count = 0;
                    if writing {
                        self.phase = Phase::WriteData;
                    } else {
                        self.read_offset = self.offset_for(self.addr);
                        // The host now reads; the trailing stop bit is ignored.
                        self.phase = Phase::Idle;
                    }
                }
            }
            Phase::WriteData => {
                self.data_reg = (self.data_reg << 1) | u64::from(bit);
                self.bit_count += 1;
                if self.bit_count == 64 {
                    self.commit_write();
                    self.phase = Phase::Idle;
                }
            }
            Phase::Idle | Phase::Reading => {} // stop bit / stray access
        }
    }

    /// Byte offset of dword `addr`, wrapped into the detected capacity.
    fn offset_for(&self, addr: u16) -> usize {
        (usize::from(addr) * 8) % self.capacity()
    }

    fn commit_write(&mut self) {
        let offset = self.offset_for(self.addr);
        for (i, slot) in self.data[offset..offset + 8].iter_mut().enumerate() {
            *slot = ((self.data_reg >> (56 - i * 8)) & 0xFF) as u8;
        }
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Clock a slice of bits into the device as a write transfer.
    fn send_command(eeprom: &mut Eeprom, bits: &[u8]) {
        eeprom.prepare(true, bits.len());
        for &b in bits {
            eeprom.write_bit(u16::from(b));
        }
    }

    /// Read `n` bits back from the device.
    fn read_back(eeprom: &mut Eeprom, n: usize) -> Vec<u8> {
        eeprom.prepare(false, n);
        (0..n).map(|_| eeprom.read_bit() as u8).collect()
    }

    fn bits_of(value: u64, width: usize) -> Vec<u8> {
        (0..width).map(|i| ((value >> (width - 1 - i)) & 1) as u8).collect()
    }

    #[test]
    fn detects_512b_from_transfer_length() {
        let mut e = Eeprom::new(&[]);
        // 9-bit read command -> 6-bit addressing -> 512 bytes.
        send_command(&mut e, &[1, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(e.bytes().len(), 0x200);
    }

    #[test]
    fn write_then_read_roundtrips_512b() {
        let mut e = Eeprom::new(&[]);
        // Write 0x0102_0304_0506_0708 to dword address 5.
        let mut cmd = vec![1, 0]; // write opcode
        cmd.extend(bits_of(5, 6)); // 6-bit address
        cmd.extend(bits_of(0x0102_0304_0506_0708, 64)); // data
        cmd.push(0); // stop
        assert_eq!(cmd.len(), 73);
        send_command(&mut e, &cmd);
        assert!(e.is_dirty());

        // Read it back: read command (9 bits) then 68 data bits.
        let mut rd = vec![1, 1];
        rd.extend(bits_of(5, 6));
        rd.push(0);
        send_command(&mut e, &rd);
        let out = read_back(&mut e, 68);
        // Skip the 4 dummy bits; reassemble the 64 data bits.
        let mut got = 0u64;
        for &b in &out[4..] {
            got = (got << 1) | u64::from(b);
        }
        assert_eq!(got, 0x0102_0304_0506_0708);
    }

    #[test]
    fn detects_8k_from_transfer_length() {
        let mut e = Eeprom::new(&[]);
        let mut cmd = vec![1, 0];
        cmd.extend(bits_of(1234, 14)); // 14-bit address
        cmd.extend(bits_of(0xDEAD_BEEF_0000_1111, 64));
        cmd.push(0);
        assert_eq!(cmd.len(), 81);
        send_command(&mut e, &cmd);
        assert_eq!(e.bytes().len(), MAX_BYTES);

        let mut rd = vec![1, 1];
        rd.extend(bits_of(1234, 14));
        rd.push(0);
        send_command(&mut e, &rd);
        let out = read_back(&mut e, 68);
        let mut got = 0u64;
        for &b in &out[4..] {
            got = (got << 1) | u64::from(b);
        }
        assert_eq!(got, 0xDEAD_BEEF_0000_1111);
    }

    #[test]
    fn unwritten_cell_reads_all_ones() {
        let mut e = Eeprom::new(&[]);
        let mut rd = vec![1, 1];
        rd.extend(bits_of(0, 6));
        rd.push(0);
        send_command(&mut e, &rd);
        let out = read_back(&mut e, 68);
        assert!(out[4..].iter().all(|&b| b == 1), "erased EEPROM reads as 1s");
    }
}
