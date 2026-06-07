//! Cartridge backup (save) memory.
//!
//! Models the two parallel-bus backup types that live in the GamePak SRAM
//! region `0x0E000000..=0x0E00FFFF`:
//!
//! * **SRAM / FRAM** — 32 KiB of plain, directly addressable byte memory.
//! * **Flash** — 64 KiB (512 Kbit) or 128 KiB (1 Mbit) flash driven by a
//!   command state machine (software-ID, chip/sector erase, byte program and,
//!   for 1 Mbit parts, bank switching).
//!
//! EEPROM is a *serial* device mapped in the `0x0D000000` region and is not
//! handled here.

/// Detected backup kind for a cartridge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveKind {
    /// 32 KiB battery-backed SRAM / FRAM.
    Sram,
    /// 64 KiB (512 Kbit) flash.
    Flash512,
    /// 128 KiB (1 Mbit) flash, two 64 KiB banks.
    Flash1M,
}

impl SaveKind {
    /// Size in bytes of the backing store for this kind.
    pub const fn size(self) -> usize {
        match self {
            Self::Sram => 0x8000,      // 32 KiB
            Self::Flash512 => 0x1_0000, // 64 KiB
            Self::Flash1M => 0x2_0000,  // 128 KiB
        }
    }

    /// Detect the backup kind from ASCII markers placed in the ROM image by the
    /// official SDK (e.g. `FLASH512_V`, `FLASH1M_V`, `SRAM_V`, `EEPROM_V`).
    /// Defaults to [`SaveKind::Sram`] when nothing matches.
    pub fn detect(rom: &[u8]) -> Self {
        // Order matters: check the more specific markers first.
        if contains(rom, b"FLASH1M_V") {
            Self::Flash1M
        } else if contains(rom, b"FLASH512_V") || contains(rom, b"FLASH_V") {
            Self::Flash512
        } else {
            // SRAM_V / SRAM_F_V (and EEPROM_V carts, whose serial device lives in
            // the 0x0D region and is modeled separately) fall back to plain SRAM,
            // which is also the safe default.
            Self::Sram
        }
    }
}

/// Whether the ROM declares serial EEPROM backup (an `EEPROM_V` marker). EEPROM
/// is a separate device from [`Backup`]; the bus models it in the 0x0D region.
#[must_use]
pub fn is_eeprom(rom: &[u8]) -> bool {
    contains(rom, b"EEPROM_V")
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() || haystack.len() < needle.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|w| w == needle)
}

// Software-ID (manufacturer, device) pairs reported by reads at offsets 0/1
// while the chip is in ID mode. These are the IDs the matching SDK save
// routines probe for.
const FLASH512_MANUFACTURER: u8 = 0x32; // Panasonic
const FLASH512_DEVICE: u8 = 0x1B;
const FLASH1M_MANUFACTURER: u8 = 0x62; // Sanyo
const FLASH1M_DEVICE: u8 = 0x13;

/// Backup memory device: a byte store plus, for flash, its command state.
#[derive(Debug)]
pub struct Backup {
    kind: SaveKind,
    data: Vec<u8>,
    /// Number of accepted command-prefix bytes (`AA`,`55`) so far: 0, 1 or 2.
    prefix: u8,
    /// Software-ID (autoselect) mode active.
    id_mode: bool,
    /// A chip/sector erase command (`0x80`) has been armed.
    erase_armed: bool,
    /// The next byte write is the data for a program (`0xA0`) command.
    write_armed: bool,
    /// The next byte write selects the active bank (`0xB0`, 1 Mbit only).
    bank_armed: bool,
    /// Currently selected 64 KiB bank (1 Mbit only; always 0 otherwise).
    bank: usize,
    /// Set whenever the contents change, so the host can persist lazily.
    dirty: bool,
}

impl Backup {
    /// Create a backup device of `kind`, seeding it with `initial` (e.g. the
    /// contents of a `.sav` file). Erased flash reads back as `0xFF`, so the
    /// store is padded with `0xFF`; plain SRAM is conventionally `0x00`.
    pub fn new(kind: SaveKind, initial: &[u8]) -> Self {
        let size = kind.size();
        let fill = match kind {
            SaveKind::Sram => 0x00,
            SaveKind::Flash512 | SaveKind::Flash1M => 0xFF,
        };
        let mut data = vec![fill; size];
        let n = initial.len().min(size);
        data[..n].copy_from_slice(&initial[..n]);
        Self {
            kind,
            data,
            prefix: 0,
            id_mode: false,
            erase_armed: false,
            write_armed: false,
            bank_armed: false,
            bank: 0,
            dirty: false,
        }
    }

    pub const fn kind(&self) -> SaveKind {
        self.kind
    }

    /// Raw backing bytes (for persistence).
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    /// Whether the contents have changed since the last [`Backup::clear_dirty`].
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub const fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    const fn is_flash(&self) -> bool {
        matches!(self.kind, SaveKind::Flash512 | SaveKind::Flash1M)
    }

    const fn id_bytes(&self) -> (u8, u8) {
        match self.kind {
            SaveKind::Flash1M => (FLASH1M_MANUFACTURER, FLASH1M_DEVICE),
            _ => (FLASH512_MANUFACTURER, FLASH512_DEVICE),
        }
    }

    /// Read a byte from the backup region. `addr` is the absolute bus address.
    pub fn read(&self, addr: u32) -> u8 {
        let off = (addr & 0xFFFF) as usize;
        if self.is_flash() {
            if self.id_mode && off <= 1 {
                let (man, dev) = self.id_bytes();
                return if off == 0 { man } else { dev };
            }
            let idx = (self.bank * 0x1_0000 + off) & (self.data.len() - 1);
            self.data[idx]
        } else {
            // SRAM: 32 KiB, mirrored across the 64 KiB region.
            self.data[off & 0x7FFF]
        }
    }

    /// Write a byte to the backup region. `addr` is the absolute bus address.
    pub fn write(&mut self, addr: u32, value: u8) {
        if !self.is_flash() {
            self.data[(addr as usize & 0xFFFF) & 0x7FFF] = value;
            self.dirty = true;
            return;
        }
        self.flash_write((addr & 0xFFFF) as usize, value);
    }

    fn flash_write(&mut self, off: usize, value: u8) {
        // A pending program writes the data byte to the current bank, then ends
        // the command — regardless of the target address (including 0x5555).
        if self.write_armed {
            let idx = self.bank * 0x1_0000 + off;
            if idx < self.data.len() {
                self.data[idx] = value;
                self.dirty = true;
            }
            self.write_armed = false;
            self.prefix = 0;
            return;
        }

        // A pending bank switch consumes the next write (1 Mbit only).
        if self.bank_armed {
            if self.kind == SaveKind::Flash1M {
                self.bank = (value & 1) as usize;
            }
            self.bank_armed = false;
            self.prefix = 0;
            return;
        }

        // Command prefix: AA @ 0x5555, then 55 @ 0x2AAA.
        match self.prefix {
            0 => {
                if off == 0x5555 && value == 0xAA {
                    self.prefix = 1;
                }
                return;
            }
            1 => {
                if off == 0x2AAA && value == 0x55 {
                    self.prefix = 2;
                } else {
                    self.prefix = 0;
                }
                return;
            }
            _ => {}
        }

        // prefix == 2: command byte.
        self.prefix = 0;
        match value {
            0x90 => self.id_mode = true,            // enter software-ID
            0xF0 => self.id_mode = false,           // exit software-ID
            0xA0 => self.write_armed = true,        // program a byte next
            0xB0 => self.bank_armed = true,         // bank switch next (1M)
            0x80 => self.erase_armed = true,        // erase command coming
            0x10 if self.erase_armed => {
                // Chip erase.
                for b in &mut self.data {
                    *b = 0xFF;
                }
                self.erase_armed = false;
                self.dirty = true;
            }
            0x30 if self.erase_armed => {
                // Sector erase: 4 KiB sector containing `off` in the current bank.
                let len = self.data.len();
                let base = (self.bank * 0x1_0000 + (off & 0xF000)).min(len);
                let end = (base + 0x1000).min(len);
                for b in &mut self.data[base..end] {
                    *b = 0xFF;
                }
                self.erase_armed = false;
                self.dirty = true;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_flash_kinds() {
        assert_eq!(SaveKind::detect(b"...FLASH512_V131..."), SaveKind::Flash512);
        assert_eq!(SaveKind::detect(b"...FLASH1M_V102..."), SaveKind::Flash1M);
        assert_eq!(SaveKind::detect(b"...FLASH_V123..."), SaveKind::Flash512);
        assert_eq!(SaveKind::detect(b"...SRAM_V112..."), SaveKind::Sram);
        assert_eq!(SaveKind::detect(b"...EEPROM_V120..."), SaveKind::Sram);
        assert_eq!(SaveKind::detect(b"no marker here"), SaveKind::Sram);
    }

    #[test]
    fn sram_is_plain_byte_memory() {
        let mut b = Backup::new(SaveKind::Sram, &[]);
        b.write(0x0E00_0000, 0x12);
        b.write(0x0E00_7FFF, 0x34);
        assert_eq!(b.read(0x0E00_0000), 0x12);
        assert_eq!(b.read(0x0E00_7FFF), 0x34);
        // 32 KiB mirror within the 64 KiB window.
        assert_eq!(b.read(0x0E00_8000), 0x12);
    }

    fn cmd(b: &mut Backup, off_val: &[(usize, u8)]) {
        for &(off, v) in off_val {
            b.write(0x0E00_0000 + off as u32, v);
        }
    }

    #[test]
    fn flash_software_id() {
        let mut b = Backup::new(SaveKind::Flash512, &[]);
        cmd(&mut b, &[(0x5555, 0xAA), (0x2AAA, 0x55), (0x5555, 0x90)]);
        assert_eq!(b.read(0x0E00_0000), FLASH512_MANUFACTURER);
        assert_eq!(b.read(0x0E00_0001), FLASH512_DEVICE);
        // Exit ID mode -> normal reads (erased = 0xFF).
        cmd(&mut b, &[(0x5555, 0xAA), (0x2AAA, 0x55), (0x5555, 0xF0)]);
        assert_eq!(b.read(0x0E00_0000), 0xFF);
    }

    #[test]
    fn flash_program_and_chip_erase() {
        let mut b = Backup::new(SaveKind::Flash512, &[]);
        // Program 0x42 at offset 0x20.
        cmd(&mut b, &[(0x5555, 0xAA), (0x2AAA, 0x55), (0x5555, 0xA0), (0x20, 0x42)]);
        assert_eq!(b.read(0x0E00_0020), 0x42);
        assert!(b.is_dirty());
        // Chip erase -> all 0xFF.
        cmd(
            &mut b,
            &[
                (0x5555, 0xAA), (0x2AAA, 0x55), (0x5555, 0x80),
                (0x5555, 0xAA), (0x2AAA, 0x55), (0x5555, 0x10),
            ],
        );
        assert_eq!(b.read(0x0E00_0020), 0xFF);
    }

    #[test]
    fn flash_sector_erase_only_clears_its_sector() {
        let mut b = Backup::new(SaveKind::Flash512, &[]);
        cmd(&mut b, &[(0x5555, 0xAA), (0x2AAA, 0x55), (0x5555, 0xA0), (0x0000, 0x11)]);
        cmd(&mut b, &[(0x5555, 0xAA), (0x2AAA, 0x55), (0x5555, 0xA0), (0x1000, 0x22)]);
        // Erase the sector at 0x1000 only.
        cmd(
            &mut b,
            &[
                (0x5555, 0xAA), (0x2AAA, 0x55), (0x5555, 0x80),
                (0x5555, 0xAA), (0x2AAA, 0x55), (0x1000, 0x30),
            ],
        );
        assert_eq!(b.read(0x0E00_0000), 0x11); // untouched sector
        assert_eq!(b.read(0x0E00_1000), 0xFF); // erased sector
    }

    #[test]
    fn flash_1m_bank_switch() {
        let mut b = Backup::new(SaveKind::Flash1M, &[]);
        // Program 0xAB at bank 0, offset 0.
        cmd(&mut b, &[(0x5555, 0xAA), (0x2AAA, 0x55), (0x5555, 0xA0), (0x0000, 0xAB)]);
        // Switch to bank 1, program 0xCD at offset 0.
        cmd(&mut b, &[(0x5555, 0xAA), (0x2AAA, 0x55), (0x5555, 0xB0), (0x0000, 0x01)]);
        cmd(&mut b, &[(0x5555, 0xAA), (0x2AAA, 0x55), (0x5555, 0xA0), (0x0000, 0xCD)]);
        assert_eq!(b.read(0x0E00_0000), 0xCD); // bank 1
        // Back to bank 0.
        cmd(&mut b, &[(0x5555, 0xAA), (0x2AAA, 0x55), (0x5555, 0xB0), (0x0000, 0x00)]);
        assert_eq!(b.read(0x0E00_0000), 0xAB); // bank 0
        assert_eq!(b.bytes().len(), 0x2_0000);
    }
}
