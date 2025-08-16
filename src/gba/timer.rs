use crate::types::HalfWord;
use crate::gba::interrupt::InterruptType;

#[derive(Debug, Clone, Copy)]
pub struct Timer {
    /// Timer Counter/Reload value (TM0CNT_L etc.)
    counter: u16,
    reload: u16,
    
    /// Timer Control register (TM0CNT_H etc.)
    control: HalfWord,
    
    /// Internal cycle counter for prescaler
    cycle_counter: u32,
    
    /// Timer ID (0-3)
    id: u8,
}

impl Timer {
    pub fn new(id: u8) -> Self {
        Self {
            counter: 0,
            reload: 0,
            control: 0,
            cycle_counter: 0,
            id,
        }
    }
    
    /// Read counter value (TM0CNT_L)
    pub fn read_counter(&self) -> HalfWord {
        self.counter
    }
    
    /// Write reload value (TM0CNT_L)
    pub fn write_counter(&mut self, value: HalfWord) {
        self.reload = value;
        println!("Timer {} reload set to: 0x{:04x}", self.id, value);
    }
    
    /// Read control register (TM0CNT_H)
    pub fn read_control(&self) -> HalfWord {
        self.control
    }
    
    /// Write control register (TM0CNT_H)
    pub fn write_control(&mut self, value: HalfWord) {
        let old_enabled = self.is_enabled();
        self.control = value;
        let new_enabled = self.is_enabled();
        
        // If timer is being enabled, reset counter to reload value
        if !old_enabled && new_enabled {
            self.counter = self.reload;
            self.cycle_counter = 0;
            println!("Timer {} enabled with reload: 0x{:04x}, prescaler: {}", 
                     self.id, self.reload, self.get_prescaler_value());
        } else if old_enabled && !new_enabled {
            println!("Timer {} disabled", self.id);
        }
    }
    
    /// Check if timer is enabled
    pub fn is_enabled(&self) -> bool {
        (self.control & 0x80) != 0
    }
    
    /// Check if IRQ is enabled
    pub fn is_irq_enabled(&self) -> bool {
        (self.control & 0x40) != 0
    }
    
    /// Get prescaler value
    fn get_prescaler_value(&self) -> u32 {
        match self.control & 0x03 {
            0 => 1,      // F/1
            1 => 64,     // F/64
            2 => 256,    // F/256
            3 => 1024,   // F/1024
            _ => 1,
        }
    }
    
    /// Check if count-up timing is enabled (not applicable to Timer 0)
    pub fn is_count_up(&self) -> bool {
        (self.control & 0x04) != 0 && self.id > 0
    }
    
    /// Update timer (returns true if overflow occurred)
    pub fn update(&mut self, cycles: u32) -> bool {
        if !self.is_enabled() || self.is_count_up() {
            return false;
        }
        
        let prescaler = self.get_prescaler_value();
        self.cycle_counter += cycles;
        
        let mut overflow = false;
        while self.cycle_counter >= prescaler {
            self.cycle_counter -= prescaler;
            
            if self.counter == 0xFFFF {
                // Timer overflow
                self.counter = self.reload;
                overflow = true;
                if self.is_irq_enabled() {
                    println!("Timer {} overflow! IRQ enabled, generating interrupt", self.id);
                }
            } else {
                self.counter += 1;
            }
        }
        
        overflow
    }
    
    /// Increment from previous timer overflow (for count-up timing)
    pub fn increment_from_overflow(&mut self) -> bool {
        if !self.is_enabled() || !self.is_count_up() {
            return false;
        }
        
        if self.counter == 0xFFFF {
            self.counter = self.reload;
            if self.is_irq_enabled() {
                println!("Timer {} count-up overflow! IRQ enabled, generating interrupt", self.id);
            }
            true
        } else {
            self.counter += 1;
            false
        }
    }
    
    /// Get the interrupt type for this timer
    pub fn get_interrupt_type(&self) -> InterruptType {
        match self.id {
            0 => InterruptType::Timer0,
            1 => InterruptType::Timer1,
            2 => InterruptType::Timer2,
            3 => InterruptType::Timer3,
            _ => panic!("Invalid timer ID: {}", self.id),
        }
    }
}

pub struct TimerController {
    timers: [Timer; 4],
}

impl TimerController {
    pub fn new() -> Self {
        Self {
            timers: [
                Timer::new(0),
                Timer::new(1),
                Timer::new(2),
                Timer::new(3),
            ],
        }
    }
    
    /// Update all timers and return interrupts that should be generated
    pub fn update(&mut self, cycles: u32) -> Vec<InterruptType> {
        let mut interrupts = Vec::new();
        
        // Update Timer 0 first
        if self.timers[0].update(cycles) && self.timers[0].is_irq_enabled() {
            interrupts.push(InterruptType::Timer0);
        }
        
        // Handle count-up timing for other timers
        let mut overflow = self.timers[0].update(cycles);
        for i in 1..4 {
            if overflow {
                overflow = self.timers[i].increment_from_overflow();
                if overflow && self.timers[i].is_irq_enabled() {
                    interrupts.push(self.timers[i].get_interrupt_type());
                }
            }
            
            // Also update timer normally if not in count-up mode
            if !self.timers[i].is_count_up() && self.timers[i].update(cycles) && self.timers[i].is_irq_enabled() {
                interrupts.push(self.timers[i].get_interrupt_type());
            }
        }
        
        interrupts
    }
    
    /// Read timer register
    pub fn read(&self, addr: u32) -> HalfWord {
        match addr {
            0x0400_0100 => self.timers[0].read_counter(),
            0x0400_0102 => self.timers[0].read_control(),
            0x0400_0104 => self.timers[1].read_counter(),
            0x0400_0106 => self.timers[1].read_control(),
            0x0400_0108 => self.timers[2].read_counter(),
            0x0400_010A => self.timers[2].read_control(),
            0x0400_010C => self.timers[3].read_counter(),
            0x0400_010E => self.timers[3].read_control(),
            _ => 0,
        }
    }
    
    /// Write timer register
    pub fn write(&mut self, addr: u32, value: HalfWord) {
        match addr {
            0x0400_0100 => self.timers[0].write_counter(value),
            0x0400_0102 => self.timers[0].write_control(value),
            0x0400_0104 => self.timers[1].write_counter(value),
            0x0400_0106 => self.timers[1].write_control(value),
            0x0400_0108 => self.timers[2].write_counter(value),
            0x0400_010A => self.timers[2].write_control(value),
            0x0400_010C => self.timers[3].write_counter(value),
            0x0400_010E => self.timers[3].write_control(value),
            _ => {}
        }
    }
}
