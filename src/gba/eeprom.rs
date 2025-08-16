use crate::types::{HalfWord, Word, Byte};

/// EEPROM sizes supported by GBA
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EEPROMSize {
    /// 512 bytes (4Kbit) - 6-bit addressing
    Size512 = 512,
    /// 8KB (64Kbit) - 14-bit addressing  
    Size8K = 8192,
}

/// EEPROM command states based on JS implementation
#[derive(Debug, Clone, Copy, PartialEq)]
enum EEPROMCommand {
    Null = 0,
    Pending = 1,
    Write = 2,
    ReadPending = 3,
    Read = 4,
}

/// GBA EEPROM emulation
/// 
/// Based on JS implementation and gbatek documentation:
/// - Connected to bit 0 of the data bus
/// - Accessed via DMA3 transfers
/// - Commands are sent as bit streams
/// - Address range: 0xD000000-0xDFFFFFF (but typically 0xDFFFF00-0xDFFFFFF)
#[derive(Debug)]
pub struct EEPROM {
    /// EEPROM data storage
    data: Vec<u8>,
    /// EEPROM size configuration
    size: EEPROMSize,
    /// Number of address bits (6 for 512B, 14 for 8KB)
    address_bits: u8,
    /// Real size in bytes (auto-detected based on DMA count)
    real_size: usize,
    /// Current command state
    command: EEPROMCommand,
    /// Remaining command bits to process
    command_bits_remaining: u32,
    /// Write address (for write operations)
    write_address: u32,
    /// Read address (for read operations)  
    read_address: u32,
    /// Remaining read bits to send
    read_bits_remaining: u32,
    /// Write pending flag
    write_pending: bool,
}

impl EEPROM {
    /// Create a new EEPROM instance
    pub fn new(size: EEPROMSize) -> Self {
        let data_size = size as usize;
        let address_bits = match size {
            EEPROMSize::Size512 => 6,
            EEPROMSize::Size8K => 14,
        };

        Self {
            data: vec![0xFF; data_size], // EEPROM defaults to 0xFF when erased
            size,
            address_bits,
            real_size: 0, // Auto-detected
            command: EEPROMCommand::Null,
            command_bits_remaining: 0,
            write_address: 0,
            read_address: 0,
            read_bits_remaining: 0,
            write_pending: false,
        }
    }

    /// Read a halfword from EEPROM (used for bit-by-bit communication)
    /// Based on JS implementation loadU16 method
    pub fn read_halfword(&mut self, _addr: u32, dma_enabled: bool) -> HalfWord {
        if self.command != EEPROMCommand::Read || !dma_enabled {
            return 1; // Return 1 when not in read mode or DMA disabled
        }

        if self.read_bits_remaining == 0 {
            self.command = EEPROMCommand::Null;
            return 1;
        }

        self.read_bits_remaining -= 1;
        
        if self.read_bits_remaining < 64 {
            let step = 63 - self.read_bits_remaining;
            let byte_index = (self.read_address + step) >> 3;
            let bit_index = 0x7 - (step & 0x7);
            
            if (byte_index as usize) < self.data.len() {
                let data = self.data[byte_index as usize] >> bit_index;
                if self.read_bits_remaining == 0 {
                    self.command = EEPROMCommand::Null;
                }
                return (data & 0x1) as HalfWord;
            }
        }
        
        0
    }

    /// Peek at EEPROM read value without changing state (for read-only contexts)
    pub fn peek_read_halfword(&self, _addr: u32, dma_enabled: bool) -> HalfWord {
        if self.command != EEPROMCommand::Read || !dma_enabled {
            return 1; // Return 1 when not in read mode or DMA disabled
        }

        if self.read_bits_remaining == 0 {
            return 1;
        }

        if self.read_bits_remaining <= 64 {
            let step = 63 - (self.read_bits_remaining - 1);
            let byte_index = (self.read_address + step) >> 3;
            let bit_index = 0x7 - (step & 0x7);
            
            if (byte_index as usize) < self.data.len() {
                let data = self.data[byte_index as usize] >> bit_index;
                return (data & 0x1) as HalfWord;
            }
        }
        
        0
    }

    /// Write a halfword to EEPROM (used for bit-by-bit communication)
    /// Based on JS implementation store16 method
    pub fn write_halfword(&mut self, _addr: u32, value: HalfWord, dma_count: u32) {
        let bit = value & 0x1;
        
        match self.command {
            // Read header
            EEPROMCommand::Null => {
                self.command = if bit != 0 { EEPROMCommand::Pending } else { EEPROMCommand::Null };
            }
            
            EEPROMCommand::Pending => {
                // Shift command and add new bit (matching JS implementation)
                let mut new_command = (self.command as u32) << 1;
                new_command |= bit as u32;
                
                if new_command == EEPROMCommand::Write as u32 {
                    // Write command detected (0b10)
                    if self.real_size == 0 {
                        // Auto-detect size based on DMA count
                        let bits = dma_count.saturating_sub(67); // Total bits - (2 command + 64 data + 1 stop)
                        self.real_size = 8 << bits;
                        self.address_bits = bits as u8;
                    }
                    self.command = EEPROMCommand::Write;
                    self.command_bits_remaining = self.address_bits as u32 + 64 + 1;
                    self.write_address = 0;
                } else {
                    // Read command (0b11)
                    if self.real_size == 0 {
                        // Auto-detect size based on DMA count  
                        let bits = dma_count.saturating_sub(3); // Total bits - (2 command + 1 stop)
                        self.real_size = 8 << bits;
                        self.address_bits = bits as u8;
                    }
                    self.command = EEPROMCommand::ReadPending;
                    self.command_bits_remaining = self.address_bits as u32 + 1;
                    self.read_address = 0;
                }
            }
            
            // Do commands
            EEPROMCommand::Write => {
                if self.command_bits_remaining > 0 {
                    self.command_bits_remaining -= 1;
                }
                
                if self.command_bits_remaining > 64 {
                    // Reading address bits
                    self.write_address <<= 1;
                    self.write_address |= (bit << 6) as u32;
                } else if self.command_bits_remaining == 0 {
                    // Write complete
                    self.command = EEPROMCommand::Null;
                    self.write_pending = true;
                } else {
                    // Reading data bits
                    let byte_index = (self.write_address >> 3) as usize;
                    let bit_index = 0x7 - (self.write_address & 0x7);
                    
                    if byte_index < self.data.len() {
                        let mut current = self.data[byte_index];
                        current &= !(1 << bit_index);
                        current |= ((bit & 0x1) << bit_index) as u8;
                        self.data[byte_index] = current;
                    }
                    
                    self.write_address += 1;
                }
            }
            
            EEPROMCommand::ReadPending => {
                if self.command_bits_remaining > 0 {
                    self.command_bits_remaining -= 1;
                }
                
                if self.command_bits_remaining > 0 {
                    // Reading address bits
                    self.read_address <<= 1;
                    if bit != 0 {
                        self.read_address |= 0x40;
                    }
                } else {
                    // Address complete, start read
                    self.read_bits_remaining = 68; // 4 dummy + 64 data bits
                    self.command = EEPROMCommand::Read;
                }
            }
            
            EEPROMCommand::Read => {
                // Ignore writes during read
            }
        }
    }

    /// Get EEPROM size
    pub fn size(&self) -> EEPROMSize {
        self.size
    }

    /// Check if write is pending
    pub fn is_write_pending(&self) -> bool {
        self.write_pending
    }

    /// Clear write pending flag
    pub fn clear_write_pending(&mut self) {
        self.write_pending = false;
    }

    /// Load EEPROM data from external source
    pub fn load_data(&mut self, data: &[u8]) {
        let copy_len = std::cmp::min(data.len(), self.data.len());
        self.data[..copy_len].copy_from_slice(&data[..copy_len]);
        println!("[EEPROM] Loaded {} bytes of data", copy_len);
    }

    /// Save EEPROM data
    pub fn save_data(&self) -> &[u8] {
        &self.data
    }

    /// Replace EEPROM data (for save state loading)
    pub fn replace_data(&mut self, data: Vec<u8>) {
        if data.len() == self.data.len() {
            self.data = data;
        } else {
            println!("[EEPROM] Warning: Data size mismatch during replace");
        }
    }

    /// Handle byte access (should not be used for EEPROM)
    pub fn read_byte(&self, _addr: u32) -> Byte {
        println!("[EEPROM] Warning: Byte access not supported");
        0
    }

    /// Handle byte write (should not be used for EEPROM)
    pub fn write_byte(&mut self, _addr: u32, _value: Byte) {
        println!("[EEPROM] Warning: Byte write not supported");
    }

    /// Handle word access (should not be used for EEPROM)
    pub fn read_word(&self, _addr: u32) -> Word {
        println!("[EEPROM] Warning: Word access not supported");
        0
    }

    /// Handle word write (should not be used for EEPROM)
    pub fn write_word(&mut self, _addr: u32, _value: Word) {
        println!("[EEPROM] Warning: Word write not supported");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eeprom_creation() {
        let eeprom = EEPROM::new(EEPROMSize::Size512);
        assert_eq!(eeprom.size(), EEPROMSize::Size512);
        assert_eq!(eeprom.data.len(), 512);
        assert_eq!(eeprom.address_bits, 6);
    }

    #[test]
    fn test_eeprom_8k_creation() {
        let eeprom = EEPROM::new(EEPROMSize::Size8K);
        assert_eq!(eeprom.size(), EEPROMSize::Size8K);
        assert_eq!(eeprom.data.len(), 8192);
        assert_eq!(eeprom.address_bits, 14);
    }

    #[test]
    fn test_eeprom_initial_state() {
        let eeprom = EEPROM::new(EEPROMSize::Size512);
        assert_eq!(eeprom.command, EEPROMCommand::Null);
        assert_eq!(eeprom.real_size, 0);
        assert!(!eeprom.is_write_pending());
    }

    #[test]
    fn test_eeprom_command_detection() {
        let mut eeprom = EEPROM::new(EEPROMSize::Size512);
        
        // Test write command (0b10)
        eeprom.write_halfword(0, 1, 73); // Start bit
        assert_eq!(eeprom.command, EEPROMCommand::Pending);
        
        eeprom.write_halfword(0, 0, 73); // Second bit (making 0b10 = write)
        assert_eq!(eeprom.command, EEPROMCommand::Write);
        
        // Test read command (0b11)
        let mut eeprom2 = EEPROM::new(EEPROMSize::Size512);
        eeprom2.write_halfword(0, 1, 9); // Start bit
        assert_eq!(eeprom2.command, EEPROMCommand::Pending);
        
        eeprom2.write_halfword(0, 1, 9); // Second bit (making 0b11 = read)
        assert_eq!(eeprom2.command, EEPROMCommand::ReadPending);
    }

    #[test]
    fn test_eeprom_data_persistence() {
        let mut eeprom = EEPROM::new(EEPROMSize::Size512);
        let test_data = vec![0x55, 0xAA, 0xFF, 0x00];
        
        // Load test data
        eeprom.load_data(&test_data);
        
        // Verify data was loaded
        let saved_data = eeprom.save_data();
        assert_eq!(saved_data[0], 0x55);
        assert_eq!(saved_data[1], 0xAA);
        assert_eq!(saved_data[2], 0xFF);
        assert_eq!(saved_data[3], 0x00);
    }
}