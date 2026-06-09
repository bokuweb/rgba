#[derive(Default)]
pub struct Timers {
    timer: [Timer; 4],
    prev_cycle: u64,
}

#[derive(Default, Clone, Copy)]
struct Timer {
    enable: bool,
    counter: u16,
    reload: u16,
    irq_enable: bool,
    countup_timing: bool, // 0: prescaler, 1: prev timer overflow
    prescaler: u8,        // 0:1, 1:64, 2:256, 3:1024
    fraction: u64,
    just_enabled: bool,
    next_edge: u64,
}

impl Timers {
    pub const fn read(&self, ofs: u32) -> u8 {
        match ofs {
            // TMxCNT_L
            0x00 | 0x01 | 0x04 | 0x05 | 0x08 | 0x09 | 0x0C | 0x0D => {
                let i = (ofs / 4) as usize;
                let b = ofs & 1;
                (self.timer[i].counter >> (b * 8)) as u8
            }
            // TMxCNT_H
            0x02 | 0x06 | 0x0A | 0x0E => {
                let i = (ofs / 4) as usize;
                let t = &self.timer[i];
                let mut v: u8 = 0;
                v |= t.prescaler & 0x3;           // bits 0..1
                v |= (t.countup_timing as u8) << 2;       // bit 2
                v |= (t.irq_enable as u8) << 6;           // bit 6
                v |= (t.enable as u8) << 7;               // bit 7
                v
            }
            // Not used
            0x03 | 0x07 | 0x0B | 0x0F => 0,
            _ => 0,
        }
    }

    pub const fn write(&mut self, ofs: u32, data: u8) {
        match ofs {
            // TMxCNT_L (reload value). Hardware-accurate: while disabled, also apply to the counter immediately.
            0x00 | 0x01 | 0x04 | 0x05 | 0x08 | 0x09 | 0x0C | 0x0D => {
                let i = (ofs / 4) as usize;
                if ofs & 1 == 0 {
                    self.timer[i].reload = (self.timer[i].reload & 0xFF00) | data as u16;
                } else {
                    self.timer[i].reload = ((data as u16) << 8) | (self.timer[i].reload & 0x00FF);
                }
                // When disabled, writing reload immediately propagates to counter (read-back matches).
                if !self.timer[i].enable {
                    self.timer[i].counter = self.timer[i].reload;
                }
            }
            // TMxCNT_H
            0x02 | 0x06 | 0x0A | 0x0E => {
                let i = (ofs / 4) as usize;
                let prev_enable = self.timer[i].enable;
                self.timer[i].prescaler = data & 0x3;
                // On Timer0, count-up (bit2) is ignored (hardware spec).
                self.timer[i].countup_timing = i != 0 && (data & 0x4) != 0;
                self.timer[i].irq_enable = (data & 0x40) != 0;
                self.timer[i].enable = (data & 0x80) != 0;
                if !prev_enable && self.timer[i].enable {
                    // On enable, load the reload value and clear the prescaler phase to 0 (hardware-accurate).
                    self.timer[i].counter = self.timer[i].reload;
                    self.timer[i].fraction = 0;
                    self.timer[i].just_enabled = true;
                    // Set the next increment edge to current cycle + prescaler.
                    let prescaler = match self.timer[i].prescaler { 0=>1,1=>64,2=>256,3=>1024,_=>1 } as u64;
                    // Match measured hardware: slightly delay the first increment edge right after enable.
                    let enable_phase_adjust: u64 = 2; // 2 cycles delay
                    self.timer[i].next_edge = self.prev_cycle
                        .saturating_add(prescaler)
                        .saturating_add(enable_phase_adjust);
                } else if prev_enable && !self.timer[i].enable {
                    // On disable, retain the counter.
                }
            }
            _ => {}
        }
    }

    /// Advance all timers to `cycles`, returning the per-channel overflow
    /// counts (used to clock the DirectSound FIFOs).
    pub fn tick(&mut self, cycles: u64, irq: &mut crate::interrupt::InterruptController) -> [u64; 4] {
        let elapsed = cycles - self.prev_cycle;
        self.prev_cycle = cycles;

        let mut overflows = [0u64; 4];
        let mut prev_overflow: u64 = 0;
        for ch in 0..4 {
            let overflow = self.timer_process(ch, elapsed, prev_overflow, irq);
            overflows[ch] = overflow;
            prev_overflow = overflow;
        }
        overflows
    }

    fn timer_process(
        &mut self,
        ch: usize,
        elapsed: u64,
        prev_overflow: u64,
        irq: &mut crate::interrupt::InterruptController,
    ) -> u64 {
        let t = &mut self.timer[ch];
        if !t.enable {
            return 0;
        }

        let mut inc: u64 = if t.countup_timing {
            prev_overflow
        } else {
            // Prescaler divisors per GBA spec (/1, /64, /256, /1024).
            let prescaler = match t.prescaler {
                0 => 1,
                1 => 64,
                2 => 256,
                3 => 1024,
                _ => 1,
            } as u64;
            // Increment on absolute cycle edges (accumulate to the next edge, increment once per crossed boundary).
            let mut produced = 0u64;
            if elapsed > 0 {
                let mut current = self.prev_cycle - elapsed;
                while current < self.prev_cycle {
                    if t.next_edge == 0 { t.next_edge = current.saturating_add(prescaler); }
                    if current >= t.next_edge { // passed edge; schedule next
                        produced += 1;
                        t.next_edge = t.next_edge.saturating_add(prescaler);
                    } else {
                        // fast-forward to next_edge or end of window
                        let step = (t.next_edge - current).min(self.prev_cycle - current);
                        current += step;
                        if current == t.next_edge {
                            produced += 1;
                            t.next_edge = t.next_edge.saturating_add(prescaler);
                        }
                    }
                }
            }
            produced
        };

        // Even right after enable, follow the prescaler edges precisely when accumulating.
        // On hardware, /1 starts counting immediately after enable, so do not suppress it.
        if t.just_enabled { t.just_enabled = false; }

        let mut overflow: u64 = 0;
        while inc > 0 {
            let room = 0x10000u64 - (t.counter as u64);
            if inc >= room {
                overflow += 1;
                inc -= room;
                t.counter = t.reload;

                if t.irq_enable {
                    let kind = match ch {
                        0 => crate::interrupt::InterruptType::Timer0,
                        1 => crate::interrupt::InterruptType::Timer1,
                        2 => crate::interrupt::InterruptType::Timer2,
                        3 => crate::interrupt::InterruptType::Timer3,
                        _ => crate::interrupt::InterruptType::Timer0,
                    };
                    irq.request_interrupt(kind);
                }
            } else {
                let add = inc as u16;
                t.counter = t.counter.wrapping_add(add);
                break;
            }
        }

        overflow
    }
}
