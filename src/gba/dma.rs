use crate::types::*;

#[derive(Debug, Clone, Copy)]
pub struct DMAChannel {
    pub source: Word,      // Source Address
    pub destination: Word, // Destination Address
    pub count: HalfWord,   // Word Count
    pub control: HalfWord, // Control Register
    pub initial_source: Word,
    pub initial_destination: Word,
    pub initial_count: HalfWord,
    pub enabled: bool,     // Enable flag
    pub pending: bool,     // Pending immediate transfer request
}

impl DMAChannel {
    pub fn new() -> Self {
        DMAChannel {
            source: 0,
            destination: 0,
            count: 0,
            control: 0,
            initial_source: 0,
            initial_destination: 0,
            initial_count: 0,
            enabled: false,
            pending: false,
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

    pub fn handle_irq(&self) -> bool {
        (self.control & 0x4000) != 0 // Bit 14: IRQ upon end
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
        }
    }

    pub fn write_destination(&mut self, channel: usize, addr: Word) {
        if channel < 4 {
            self.channels[channel].set_destination(addr);
        }
    }

    pub fn write_count(&mut self, channel: usize, count: HalfWord) {
        if channel < 4 {
            self.channels[channel].set_count(count);
        }
    }

    pub fn write_control(&mut self, channel: usize, control: HalfWord) {
        if channel < 4 {
            let ch = &mut self.channels[channel];
            let was_enabled = ch.enabled;
            let prev_timing = ch.get_timing();
            ch.set_control(control);
            let now_timing = ch.get_timing();
            if std::env::var("AGB_TRACE_DMA").ok().as_deref() == Some("1") {
                println!(
                    "DMA ctrl ch={} was_en={} en={} ctrl={:04x} timing={} repeat={} irq={}",
                    channel,
                    was_enabled,
                    ch.enabled,
                    control,
                    now_timing,
                    ch.is_repeat(),
                    ch.handle_irq()
                );
            }
            
            // Pending scheduling policy:
            // - When enabling (edge 0->1): schedule immediately only if timing==Immediate
            // - When already enabled and timing changed to Immediate: do NOT start immediately (HW behavior)
            // - Otherwise: clear pending
            if !was_enabled && ch.enabled {
                ch.initial_source = ch.source;
                ch.initial_destination = ch.destination;
                ch.initial_count = ch.count;
                ch.pending = now_timing == 0; // Immediate only
            } else if was_enabled && ch.enabled {
                // Mode change while enabled shouldn't start transfer immediately
                if prev_timing != now_timing {
                    ch.pending = false;
                }
            } else {
                ch.pending = false;
            }
        }
    }

    pub fn trigger_timing_event(&mut self, channel: usize) {
        if channel < 4 && self.channels[channel].enabled {
            self.channels[channel].pending = true;
        }
    }

    fn trigger_transfer(&mut self, _channel: usize) {}

    pub fn get_pending_transfer(&mut self, channel: usize) -> Option<(Word, Word, usize, usize)> {
        if channel < 4 && self.channels[channel].enabled && self.channels[channel].pending {
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
            let repeat = self.channels[channel].is_repeat();
            let dest_control = self.channels[channel].get_dest_control();
            // Clear pending regardless
            self.channels[channel].pending = false;
            if !repeat {
                self.channels[channel].enabled = false;
                self.channels[channel].control &= !0x8000; // Clear enable bit
            } else {
                // For repeat mode, restore word count and destination if increment/reload.
                self.channels[channel].count = self.channels[channel].initial_count;
                if dest_control == 3 {
                    self.channels[channel].destination = self.channels[channel].initial_destination;
                }
            }
        }
    }
}
