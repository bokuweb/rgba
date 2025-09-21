use crate::types::*;

#[derive(Debug, Clone, Copy)]
pub struct DMAChannel {
    pub source: Word,      // Source Address
    pub destination: Word, // Destination Address
    pub count: HalfWord,   // Word Count
    pub control: HalfWord, // Control Register
    pub enabled: bool,     // Enable flag
}

impl DMAChannel {
    pub fn new() -> Self {
        DMAChannel {
            source: 0,
            destination: 0,
            count: 0,
            control: 0,
            enabled: false,
        }
    }

    pub fn set_source(&mut self, addr: Word) {
        self.source = addr;
    }

    pub fn set_destination(&mut self, addr: Word) {
        self.destination = addr;
    }

    pub fn set_count(&mut self, count: HalfWord) {
        self.count = count;
    }

    pub fn set_control(&mut self, control: HalfWord) {
        self.control = control;
        self.enabled = (control & 0x8000) != 0; // Bit 15: DMA Enable
    }

    pub fn get_transfer_size(&self) -> usize {
        if (self.control & 0x0400) != 0 { 4 } else { 2 } // Bit 10: DMA Transfer Type (0=16bit, 1=32bit)
    }

    pub fn get_source_control(&self) -> u8 {
        ((self.control >> 7) & 0x03) as u8 // Bit 7-8: Source Address Control
    }

    pub fn get_dest_control(&self) -> u8 {
        ((self.control >> 5) & 0x03) as u8 // Bit 5-6: Dest Address Control
    }

    pub fn get_timing(&self) -> u8 {
        ((self.control >> 12) & 0x03) as u8 // Bit 12-13: DMA Start Timing
    }

    pub fn is_repeat(&self) -> bool {
        (self.control & 0x0200) != 0 // Bit 9: DMA Repeat
    }
}

pub struct DMAController {
    pub channels: [DMAChannel; 4], // DMA0-3
}

impl DMAController {
    pub fn new() -> Self {
        DMAController {
            channels: [DMAChannel::new(); 4],
        }
    }

    pub fn write_source(&mut self, channel: usize, addr: Word) {
        if channel < 4 {
            self.channels[channel].set_source(addr);
            println!("🔧 DMA{} Source Address: 0x{:08x}", channel, addr);
        }
    }

    pub fn write_destination(&mut self, channel: usize, addr: Word) {
        if channel < 4 {
            self.channels[channel].set_destination(addr);
            println!("🔧 DMA{} Destination Address: 0x{:08x}", channel, addr);
        }
    }

    pub fn write_count(&mut self, channel: usize, count: HalfWord) {
        if channel < 4 {
            self.channels[channel].set_count(count);
            println!("🔧 DMA{} Word Count: 0x{:04x}", channel, count);
        }
    }

    pub fn write_control(&mut self, channel: usize, control: HalfWord) {
        if channel < 4 {
            let was_enabled = self.channels[channel].enabled;
            self.channels[channel].set_control(control);
            
            println!("🔧 DMA{} Control: 0x{:04x} (Enable: {}, Size: {}bit, Timing: {})", 
                channel, control, 
                self.channels[channel].enabled,
                if self.channels[channel].get_transfer_size() == 4 { 32 } else { 16 },
                self.channels[channel].get_timing()
            );

            // If DMA was just enabled, trigger transfer
            if !was_enabled && self.channels[channel].enabled {
                self.trigger_transfer(channel);
            }
        }
    }

    fn trigger_transfer(&mut self, channel: usize) {
        let dma = &mut self.channels[channel];
        if !dma.enabled {
            return;
        }

        println!("🚀 DMA{} Transfer: 0x{:08x} -> 0x{:08x}, Count: {}, Size: {}bit", 
            channel, dma.source, dma.destination, dma.count, 
            if dma.get_transfer_size() == 4 { 32 } else { 16 }
        );

        // Note: Actual memory transfer will be handled by the bus
        // This is just logging for now
    }

    pub fn get_pending_transfer(&mut self, channel: usize) -> Option<(Word, Word, usize, usize)> {
        if channel < 4 && self.channels[channel].enabled {
            let dma = &self.channels[channel];
            let count = if dma.count == 0 {
                match channel {
                    3 => 0x10000, // DMA3: 64KB max
                    _ => 0x4000,  // DMA0-2: 16KB max
                }
            } else {
                dma.count as usize
            };
            
            Some((dma.source, dma.destination, count, dma.get_transfer_size()))
        } else {
            None
        }
    }

    pub fn complete_transfer(&mut self, channel: usize) {
        if channel < 4 {
            if !self.channels[channel].is_repeat() {
                self.channels[channel].enabled = false;
                self.channels[channel].control &= !0x8000; // Clear enable bit
                println!("✅ DMA{} Transfer completed", channel);
            }
        }
    }
}
