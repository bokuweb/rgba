use crate::types::HalfWord;

/// DMA channel configuration
#[derive(Debug, Clone, Copy)]
pub struct DMAChannel {
    /// Source address
    pub source: u32,
    /// Destination address  
    pub destination: u32,
    /// Word count
    pub count: u32,
    /// Control register
    pub control: u16,
    /// Enable flag
    pub enable: bool,
    /// Internal source address (for transfers)
    internal_source: u32,
    /// Internal destination address (for transfers)
    internal_destination: u32,
    /// Internal count (for transfers)
    internal_count: u32,
}

impl Default for DMAChannel {
    fn default() -> Self {
        Self {
            source: 0,
            destination: 0,
            count: 0,
            control: 0,
            enable: false,
            internal_source: 0,
            internal_destination: 0,
            internal_count: 0,
        }
    }
}

impl DMAChannel {
    /// Create a new DMA channel
    pub fn new() -> Self {
        Self::default()
    }

    /// Set source address
    pub fn set_source(&mut self, addr: u32) {
        self.source = addr;
    }

    /// Set destination address
    pub fn set_destination(&mut self, addr: u32) {
        self.destination = addr;
    }

    /// Set word count
    pub fn set_count(&mut self, count: u32) {
        self.count = count;
    }

    /// Set control register
    pub fn set_control(&mut self, control: u16) {
        self.control = control;
        self.enable = (control & 0x8000) != 0; // Bit 15 is enable
        
        if self.enable {
            // Initialize internal registers when DMA is enabled
            self.internal_source = self.source;
            self.internal_destination = self.destination;
            self.internal_count = if self.count == 0 {
                // Count of 0 means maximum transfer size
                match self.get_channel_id() {
                    0 => 0x4000,  // DMA0: 16KB max
                    1 => 0x4000,  // DMA1: 16KB max  
                    2 => 0x4000,  // DMA2: 16KB max
                    3 => 0x10000, // DMA3: 64KB max
                    _ => 0x4000,
                }
            } else {
                self.count
            };
        }
    }

    /// Get channel ID (would need to be set externally in real implementation)
    fn get_channel_id(&self) -> u8 {
        3 // For now, assume DMA3 (used by EEPROM)
    }

    /// Check if DMA is enabled
    pub fn is_enabled(&self) -> bool {
        self.enable
    }

    /// Get current count
    pub fn get_count(&self) -> u32 {
        self.internal_count
    }

    /// Get destination address type
    pub fn get_dest_addr_control(&self) -> u8 {
        ((self.control >> 5) & 0x3) as u8
    }

    /// Get source address type  
    pub fn get_source_addr_control(&self) -> u8 {
        ((self.control >> 7) & 0x3) as u8
    }

    /// Get transfer type (0=16bit, 1=32bit)
    pub fn get_transfer_type(&self) -> bool {
        (self.control & 0x400) != 0
    }

    /// Get start timing
    pub fn get_start_timing(&self) -> u8 {
        ((self.control >> 12) & 0x3) as u8
    }

    /// Check if IRQ is enabled
    pub fn is_irq_enabled(&self) -> bool {
        (self.control & 0x4000) != 0
    }

    /// Check if repeat is enabled
    pub fn is_repeat_enabled(&self) -> bool {
        (self.control & 0x200) != 0
    }
}

/// DMA Controller
#[derive(Debug)]
pub struct DMAController {
    /// DMA channels (0-3)
    pub channels: [DMAChannel; 4],
}

impl Default for DMAController {
    fn default() -> Self {
        Self {
            channels: [DMAChannel::default(); 4],
        }
    }
}

impl DMAController {
    /// Create a new DMA controller
    pub fn new() -> Self {
        Self::default()
    }

    /// Get DMA channel
    pub fn get_channel(&self, channel: usize) -> Option<&DMAChannel> {
        self.channels.get(channel)
    }

    /// Get mutable DMA channel
    pub fn get_channel_mut(&mut self, channel: usize) -> Option<&mut DMAChannel> {
        self.channels.get_mut(channel)
    }

    /// Read DMA register
    pub fn read_register(&self, addr: u32) -> HalfWord {
        let channel = ((addr - 0x0400_00B0) / 12) as usize;
        let reg = (addr - 0x0400_00B0) % 12;

        if channel >= 4 {
            return 0;
        }

        match reg {
            0 => (self.channels[channel].source & 0xFFFF) as u16,        // DMAxxSAD_L
            2 => (self.channels[channel].source >> 16) as u16,           // DMAxxSAD_H
            4 => (self.channels[channel].destination & 0xFFFF) as u16,   // DMAxxDAD_L
            6 => (self.channels[channel].destination >> 16) as u16,      // DMAxxDAD_H
            8 => (self.channels[channel].count & 0xFFFF) as u16,         // DMAxxCNT_L
            10 => self.channels[channel].control,                        // DMAxxCNT_H
            _ => 0,
        }
    }

    /// Write DMA register
    pub fn write_register(&mut self, addr: u32, value: HalfWord) {
        let channel = ((addr - 0x0400_00B0) / 12) as usize;
        let reg = (addr - 0x0400_00B0) % 12;

        if channel >= 4 {
            return;
        }

        match reg {
            0 => {
                // DMAxxSAD_L
                self.channels[channel].source = (self.channels[channel].source & 0xFFFF_0000) | value as u32;
            }
            2 => {
                // DMAxxSAD_H  
                self.channels[channel].source = (self.channels[channel].source & 0x0000_FFFF) | ((value as u32) << 16);
            }
            4 => {
                // DMAxxDAD_L
                self.channels[channel].destination = (self.channels[channel].destination & 0xFFFF_0000) | value as u32;
            }
            6 => {
                // DMAxxDAD_H
                self.channels[channel].destination = (self.channels[channel].destination & 0x0000_FFFF) | ((value as u32) << 16);
            }
            8 => {
                // DMAxxCNT_L
                self.channels[channel].count = (self.channels[channel].count & 0xFFFF_0000) | value as u32;
            }
            10 => {
                // DMAxxCNT_H
                self.channels[channel].set_control(value);
            }
            _ => {}
        }
    }
}
