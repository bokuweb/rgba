//! GBA audio processing unit (APU).
//!
//! Implements the four PSG (Game Boy compatible) channels and the two
//! DirectSound FIFO channels, mixes them into a stereo stream, and exposes the
//! sound I/O registers (0x04000060..=0x040000A7).
//!
//! The DSP runs off the same master clock as the rest of the system: the bus
//! calls [`Apu::tick`] with the elapsed CPU cycles and [`Apu::on_timer_overflow`]
//! whenever timer 0/1 overflows (which clocks the DirectSound FIFOs). Output is
//! resampled to [`SAMPLE_RATE`] and collected into an interleaved L/R `i16`
//! buffer that the host drains once per frame.

use std::collections::VecDeque;

/// CPU / master clock frequency.
const CPU_HZ: u32 = 16_777_216;
/// Host output sample rate (a clean divisor of the CPU clock).
pub const SAMPLE_RATE: u32 = 32_768;
/// CPU cycles between output samples (16777216 / 32768 = 512).
const CYCLES_PER_SAMPLE: i32 = (CPU_HZ / SAMPLE_RATE) as i32;
/// Frame-sequencer period: 512 Hz -> 32768 cycles.
const SEQ_PERIOD: i32 = (CPU_HZ / 512) as i32;

/// Duty cycle patterns for the square channels (8 steps each).
const DUTY: [[u8; 8]; 4] = [
    [0, 0, 0, 0, 0, 0, 0, 1], // 12.5%
    [1, 0, 0, 0, 0, 0, 0, 1], // 25%
    [1, 0, 0, 0, 0, 1, 1, 1], // 50%
    [0, 1, 1, 1, 1, 1, 1, 0], // 75%
];

/// Noise divisor ratios indexed by the 3-bit divisor code.
const NOISE_DIVISOR: [u32; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

/// A pulse / square-wave PSG channel.
///
/// Used for both channel 1 and channel 2; channel 1 additionally drives the
/// frequency-sweep unit (enabled via [`Square::has_sweep`]). The channel mixes
/// a selectable duty cycle with a volume envelope and an optional length
/// counter, all clocked by the 512 Hz frame sequencer plus a per-channel
/// frequency timer.
#[derive(Default)]
struct Square {
    /// Whether the channel is currently producing sound.
    enabled: bool,
    /// Whether the channel's DAC is powered (envelope volume / direction != 0).
    dac_on: bool,
    /// Duty-cycle selector (index into [`DUTY`]).
    duty: u8,
    /// Current position (0..7) within the duty pattern.
    duty_pos: u8,
    /// 11-bit frequency code; higher means a higher pitch.
    freq: u16,
    /// Down-counter (in master cycles) until the next duty step.
    timer: i32,
    /// Remaining length-counter ticks; reaching 0 disables the channel.
    length_counter: u16,
    /// Whether the length counter is active.
    length_enable: bool,
    /// Initial envelope volume loaded on trigger (0..15).
    env_initial: u8,
    /// Envelope direction: `true` increases volume, `false` decreases it.
    env_dir_up: bool,
    /// Envelope step period in frame-sequencer ticks (0 disables stepping).
    env_period: u8,
    /// Current envelope output volume (0..15).
    env_volume: u8,
    /// Envelope step down-counter.
    env_timer: u8,
    /// Whether this instance owns a frequency-sweep unit (channel 1 only).
    has_sweep: bool,
    /// Sweep step period in sweep ticks (0 treated as 8).
    sweep_period: u8,
    /// Sweep direction: `true` subtracts (pitch down), `false` adds.
    sweep_dir_down: bool,
    /// Sweep shift amount applied to the shadow frequency.
    sweep_shift: u8,
    /// Sweep step down-counter.
    sweep_timer: u8,
    /// Whether the sweep unit is currently active.
    sweep_enabled: bool,
    /// Working copy of the frequency used by the sweep calculation.
    shadow_freq: u16,
}

impl Square {
    /// Create a channel, optionally with a frequency-sweep unit.
    fn new(has_sweep: bool) -> Self {
        Self { has_sweep, ..Default::default() }
    }

    /// Master cycles between successive duty steps for the current frequency.
    fn period(&self) -> i32 {
        ((2048 - self.freq as i32) * 16).max(1)
    }

    /// Advance the duty-cycle generator by `cycles` master cycles.
    fn tick(&mut self, cycles: i32) {
        self.timer -= cycles;
        while self.timer <= 0 {
            self.timer += self.period();
            self.duty_pos = (self.duty_pos + 1) & 7;
        }
    }

    /// Restart the channel (NRx4 trigger): reload the timers, the length and
    /// envelope, and arm the sweep unit.
    fn trigger(&mut self) {
        self.enabled = self.dac_on;
        if self.length_counter == 0 {
            self.length_counter = 64;
        }
        self.timer = self.period();
        self.env_volume = self.env_initial;
        self.env_timer = if self.env_period == 0 { 8 } else { self.env_period };
        if self.has_sweep {
            self.shadow_freq = self.freq;
            self.sweep_timer = if self.sweep_period == 0 { 8 } else { self.sweep_period };
            self.sweep_enabled = self.sweep_period != 0 || self.sweep_shift != 0;
            if self.sweep_shift != 0 {
                self.sweep_calc(); // initial overflow check
            }
        }
    }

    /// Compute the next swept frequency, disabling the channel if it overflows
    /// the 11-bit range (the hardware overflow check).
    const fn sweep_calc(&mut self) -> u16 {
        let delta = self.shadow_freq >> self.sweep_shift;
        let new = if self.sweep_dir_down {
            self.shadow_freq.wrapping_sub(delta)
        } else {
            self.shadow_freq.wrapping_add(delta)
        };
        if new > 2047 {
            self.enabled = false;
        }
        new
    }

    /// Advance the frequency sweep one tick (128 Hz). Applies the new frequency
    /// and re-checks for overflow when the sweep period elapses.
    const fn clock_sweep(&mut self) {
        if !self.has_sweep || !self.sweep_enabled {
            return;
        }
        if self.sweep_timer > 0 {
            self.sweep_timer -= 1;
        }
        if self.sweep_timer == 0 {
            self.sweep_timer = if self.sweep_period == 0 { 8 } else { self.sweep_period };
            if self.sweep_period != 0 {
                let new = self.sweep_calc();
                if new <= 2047 && self.sweep_shift != 0 {
                    self.freq = new;
                    self.shadow_freq = new;
                    self.sweep_calc();
                }
            }
        }
    }

    /// Advance the length counter one tick (256 Hz), disabling the channel when
    /// it reaches zero.
    const fn clock_length(&mut self) {
        if self.length_enable && self.length_counter > 0 {
            self.length_counter -= 1;
            if self.length_counter == 0 {
                self.enabled = false;
            }
        }
    }

    /// Advance the volume envelope one tick (64 Hz).
    const fn clock_envelope(&mut self) {
        if self.env_period == 0 {
            return;
        }
        if self.env_timer > 0 {
            self.env_timer -= 1;
        }
        if self.env_timer == 0 {
            self.env_timer = self.env_period;
            if self.env_dir_up && self.env_volume < 15 {
                self.env_volume += 1;
            } else if !self.env_dir_up && self.env_volume > 0 {
                self.env_volume -= 1;
            }
        }
    }

    /// Current DAC output level (0..15): the envelope volume while the duty
    /// pattern is high, otherwise zero.
    const fn output(&self) -> u8 {
        if self.enabled && self.dac_on && DUTY[self.duty as usize][self.duty_pos as usize] == 1 {
            self.env_volume
        } else {
            0
        }
    }
}

/// The wave-table PSG channel (channel 3).
///
/// Plays back user-supplied 4-bit samples from wave RAM at a programmable rate,
/// scaled by a coarse volume selector. In 64-sample mode both RAM banks are
/// played back to back; otherwise one selected 32-sample bank is used.
#[derive(Default)]
struct Wave {
    /// Whether the channel is currently producing sound.
    enabled: bool,
    /// Whether the DAC is powered (SOUND3CNT_L bit 7).
    dac_on: bool,
    /// 11-bit frequency code.
    freq: u16,
    /// Down-counter (in master cycles) until the next sample step.
    timer: i32,
    /// Current sample index within the selected bank(s).
    pos: u8,
    /// Remaining length-counter ticks (channel 3 uses a 256-step counter).
    length_counter: u16,
    /// Whether the length counter is active.
    length_enable: bool,
    /// Volume selector: 0 = mute, 1 = 100%, 2 = 50%, 3 = 25%.
    volume_code: u8,
    /// Force 75% volume regardless of `volume_code` (SOUND3CNT_H bit 7).
    force_75: bool,
    /// 64-sample mode using both banks (SOUND3CNT_L bit 5).
    two_banks: bool,
    /// Selected bank in 32-sample mode (SOUND3CNT_L bit 6).
    bank: u8,
    /// Wave RAM: two banks of 16 bytes (64 nibbles total).
    ram: [u8; 32],
}

impl Wave {
    /// Master cycles between successive sample steps.
    fn period(&self) -> i32 {
        ((2048 - self.freq as i32) * 8).max(1)
    }

    /// Advance the sample pointer by `cycles` master cycles.
    fn tick(&mut self, cycles: i32) {
        if !self.enabled {
            return;
        }
        self.timer -= cycles;
        while self.timer <= 0 {
            self.timer += self.period();
            self.pos = (self.pos + 1) & if self.two_banks { 63 } else { 31 };
        }
    }

    /// Restart playback from the first sample (NR34 trigger).
    fn trigger(&mut self) {
        self.enabled = self.dac_on;
        if self.length_counter == 0 {
            self.length_counter = 256;
        }
        self.timer = self.period();
        self.pos = 0;
    }

    /// Advance the length counter one tick, disabling the channel at zero.
    const fn clock_length(&mut self) {
        if self.length_enable && self.length_counter > 0 {
            self.length_counter -= 1;
            if self.length_counter == 0 {
                self.enabled = false;
            }
        }
    }

    /// Current DAC output (0..15): the selected wave-RAM nibble scaled by the
    /// volume selector.
    const fn output(&self) -> u8 {
        if !self.enabled || !self.dac_on {
            return 0;
        }
        // `ram` holds 64 nibbles (two banks of 32). Index into it.
        let base = if self.two_banks { 0 } else { (self.bank as usize) * 32 };
        let idx = (base + self.pos as usize) & 63;
        let byte = self.ram[idx / 2 % 32]; // 32 bytes store 64 nibbles
        let nibble = if idx & 1 == 0 { byte >> 4 } else { byte & 0x0F };
        if self.force_75 {
            (nibble * 3) / 4
        } else {
            match self.volume_code {
                0 => 0,
                1 => nibble,
                2 => nibble / 2,
                3 => nibble / 4,
                _ => 0,
            }
        }
    }
}

/// The noise PSG channel (channel 4).
///
/// Generates pseudo-random noise from a linear-feedback shift register clocked
/// at a programmable rate, shaped by a volume envelope and a length counter.
/// The LFSR can run in 15-bit or 7-bit (`width7`) mode for different timbres.
#[derive(Default)]
struct Noise {
    /// Whether the channel is currently producing sound.
    enabled: bool,
    /// Whether the DAC is powered (envelope volume / direction != 0).
    dac_on: bool,
    /// Down-counter (in master cycles) until the next LFSR step.
    timer: i32,
    /// 15-bit linear-feedback shift register state.
    lfsr: u16,
    /// 7-bit LFSR mode (NR43 bit 3) for a more periodic, metallic tone.
    width7: bool,
    /// Frequency divisor selector (index into [`NOISE_DIVISOR`]).
    divisor_code: u8,
    /// Pre-scaler shift applied on top of the divisor.
    shift: u8,
    /// Remaining length-counter ticks.
    length_counter: u16,
    /// Whether the length counter is active.
    length_enable: bool,
    /// Initial envelope volume loaded on trigger (0..15).
    env_initial: u8,
    /// Envelope direction: `true` increases volume, `false` decreases it.
    env_dir_up: bool,
    /// Envelope step period in frame-sequencer ticks (0 disables stepping).
    env_period: u8,
    /// Current envelope output volume (0..15).
    env_volume: u8,
    /// Envelope step down-counter.
    env_timer: u8,
}

impl Noise {
    /// Master cycles between successive LFSR steps for the current divisor /
    /// shift settings.
    fn period(&self) -> i32 {
        // (divisor << shift) in 4.19 MHz units -> *4 for the GBA clock.
        let div = NOISE_DIVISOR[self.divisor_code as usize & 7];
        ((div << self.shift as u32) as i32 * 4).max(1)
    }

    /// Advance the LFSR by `cycles` master cycles. Shift codes >= 14 stop the
    /// register, as on hardware.
    fn tick(&mut self, cycles: i32) {
        if !self.enabled || self.shift >= 14 {
            return;
        }
        self.timer -= cycles;
        while self.timer <= 0 {
            self.timer += self.period();
            let bit = (self.lfsr ^ (self.lfsr >> 1)) & 1;
            self.lfsr >>= 1;
            self.lfsr |= bit << 14;
            if self.width7 {
                self.lfsr = (self.lfsr & !(1 << 6)) | (bit << 6);
            }
        }
    }

    /// Restart the channel (NR44 trigger): reset the LFSR to all-ones and reload
    /// the timers, length and envelope.
    fn trigger(&mut self) {
        self.enabled = self.dac_on;
        if self.length_counter == 0 {
            self.length_counter = 64;
        }
        self.timer = self.period();
        self.lfsr = 0x7FFF;
        self.env_volume = self.env_initial;
        self.env_timer = if self.env_period == 0 { 8 } else { self.env_period };
    }

    /// Advance the length counter one tick, disabling the channel at zero.
    const fn clock_length(&mut self) {
        if self.length_enable && self.length_counter > 0 {
            self.length_counter -= 1;
            if self.length_counter == 0 {
                self.enabled = false;
            }
        }
    }

    /// Advance the volume envelope one tick (64 Hz).
    const fn clock_envelope(&mut self) {
        if self.env_period == 0 {
            return;
        }
        if self.env_timer > 0 {
            self.env_timer -= 1;
        }
        if self.env_timer == 0 {
            self.env_timer = self.env_period;
            if self.env_dir_up && self.env_volume < 15 {
                self.env_volume += 1;
            } else if !self.env_dir_up && self.env_volume > 0 {
                self.env_volume -= 1;
            }
        }
    }

    /// Current DAC output (0..15): the envelope volume while the LFSR's low bit
    /// is clear, otherwise zero.
    const fn output(&self) -> u8 {
        if self.enabled && self.dac_on && (self.lfsr & 1) == 0 {
            self.env_volume
        } else {
            0
        }
    }
}

/// A DirectSound FIFO channel (A or B).
///
/// Streams signed 8-bit PCM bytes that the game pushes (usually by DMA) into a
/// 32-byte FIFO. One byte is latched as the current output each time the
/// channel's selected timer overflows; when the FIFO runs low the bus refills
/// it from the associated DMA channel.
#[derive(Default)]
struct Fifo {
    /// Pending PCM bytes (max 32).
    buf: VecDeque<i8>,
    /// Sample currently driving the DAC, latched on the last timer tick.
    sample: i8,
    /// Timer that clocks this FIFO: 0 = Timer 0, 1 = Timer 1.
    timer_sel: u8,
    /// Mix this channel into the right output.
    enable_right: bool,
    /// Mix this channel into the left output.
    enable_left: bool,
    /// Output level: `false` = 50%, `true` = 100%.
    full_volume: bool,
}

impl Fifo {
    /// Enqueue the four bytes of a 32-bit word (low byte first).
    fn push_word(&mut self, word: u32) {
        for i in 0..4 {
            self.push_byte(((word >> (i * 8)) & 0xFF) as u8);
        }
    }

    /// Enqueue a single PCM byte, dropping it if the FIFO is full.
    fn push_byte(&mut self, byte: u8) {
        if self.buf.len() < 32 {
            self.buf.push_back(byte as i8);
        }
    }

    /// Latch the next queued sample. Called once per driving-timer overflow.
    fn tick(&mut self) {
        if let Some(s) = self.buf.pop_front() {
            self.sample = s;
        }
    }

    /// Whether the FIFO has drained to half or less and should be refilled.
    fn needs_refill(&self) -> bool {
        self.buf.len() <= 16
    }

    /// Clear the FIFO and latched sample (SOUNDCNT_H reset bit).
    fn reset(&mut self) {
        self.buf.clear();
        self.sample = 0;
    }
}

/// Result of [`Apu::on_timer_overflow`]: which DirectSound FIFOs dropped low
/// enough that the bus should top them up via DMA.
#[derive(Default, Clone, Copy)]
pub struct RefillRequest {
    /// FIFO A needs a DMA refill.
    pub fifo_a: bool,
    /// FIFO B needs a DMA refill.
    pub fifo_b: bool,
}

/// The audio processing unit: the four PSG channels, the two DirectSound FIFO
/// channels, the global mix controls, and the output resampler.
/// One-pole DC-blocking high-pass filter (`y = x - x₋₁ + R·y₋₁`).
///
/// The PSG channels swing between 0 and their volume rather than around zero,
/// so the mixed signal carries a large DC offset that would otherwise pop the
/// speakers and waste headroom. This removes it while leaving the audio band
/// essentially untouched. `R ≈ 0.995` as a 10-bit fixed-point fraction.
#[derive(Default)]
struct DcBlocker {
    x_prev: i32,
    y_prev: i32,
}

impl DcBlocker {
    const R_NUM: i32 = 1019; // ≈ 0.995 * 1024
    const R_SHIFT: i32 = 10;

    fn process(&mut self, x: i32) -> i32 {
        let y = x - self.x_prev + ((self.y_prev * Self::R_NUM) >> Self::R_SHIFT);
        self.x_prev = x;
        self.y_prev = y;
        y
    }
}

pub struct Apu {
    /// PSG channel 1 (square with sweep).
    ch1: Square,
    /// PSG channel 2 (square).
    ch2: Square,
    /// PSG channel 3 (wave table).
    ch3: Wave,
    /// PSG channel 4 (noise).
    ch4: Noise,
    /// DirectSound FIFO A.
    fifo_a: Fifo,
    /// DirectSound FIFO B.
    fifo_b: Fifo,

    /// Master sound enable (SOUNDCNT_X bit 7); when clear the PSG is powered off.
    master_enable: bool,
    /// Left master volume, 0..7 (SOUNDCNT_L).
    vol_left: u8,
    /// Right master volume, 0..7 (SOUNDCNT_L).
    vol_right: u8,
    /// Per-PSG-channel left mix enables (SOUNDCNT_L).
    psg_left_enable: [bool; 4],
    /// Per-PSG-channel right mix enables (SOUNDCNT_L).
    psg_right_enable: [bool; 4],
    /// PSG mix ratio (SOUNDCNT_H bits 0-1): 0 = 25%, 1 = 50%, 2 = 100%.
    psg_volume_code: u8,
    /// SOUNDBIAS register; bits 0-9 are the DAC bias used to centre the output.
    soundbias: u16,

    /// DC-blocking high-pass filters for the left and right output.
    dc_left: DcBlocker,
    dc_right: DcBlocker,

    /// Down-counter (master cycles) until the next output sample is emitted.
    sample_timer: i32,
    /// Down-counter (master cycles) until the next frame-sequencer step.
    seq_timer: i32,
    /// Current frame-sequencer step (0..7).
    seq_step: u8,

    /// Interleaved L/R output samples awaiting playback.
    out: Vec<i16>,
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}

impl Apu {
    /// Create a powered-off APU with empty FIFOs and channels.
    pub fn new() -> Self {
        Self {
            ch1: Square::new(true),
            ch2: Square::new(false),
            ch3: Wave::default(),
            ch4: Noise::default(),
            fifo_a: Fifo::default(),
            fifo_b: Fifo::default(),
            master_enable: false,
            vol_left: 0,
            vol_right: 0,
            psg_left_enable: [false; 4],
            psg_right_enable: [false; 4],
            psg_volume_code: 0,
            soundbias: 0x200,
            dc_left: DcBlocker::default(),
            dc_right: DcBlocker::default(),
            sample_timer: CYCLES_PER_SAMPLE,
            seq_timer: SEQ_PERIOD,
            seq_step: 0,
            out: Vec::with_capacity(2048),
        }
    }

    /// Drain all queued interleaved L/R samples for the host to play.
    pub fn take_samples(&mut self) -> Vec<i16> {
        std::mem::take(&mut self.out)
    }

    /// Enqueue a 32-bit word into DirectSound FIFO A (the normal DMA feed).
    pub fn push_fifo_a(&mut self, word: u32) {
        self.fifo_a.push_word(word);
    }

    /// Enqueue a 32-bit word into DirectSound FIFO B.
    pub fn push_fifo_b(&mut self, word: u32) {
        self.fifo_b.push_word(word);
    }

    /// Clock the DirectSound FIFOs driven by `timer_index`, advancing the
    /// latched sample once per overflow and reporting which FIFOs now need a DMA
    /// refill.
    pub fn on_timer_overflow(&mut self, timer_index: usize, count: u64) -> RefillRequest {
        let mut req = RefillRequest::default();
        for _ in 0..count {
            if self.fifo_a.timer_sel as usize == timer_index {
                self.fifo_a.tick();
            }
            if self.fifo_b.timer_sel as usize == timer_index {
                self.fifo_b.tick();
            }
        }
        if self.fifo_a.timer_sel as usize == timer_index && self.fifo_a.needs_refill() {
            req.fifo_a = true;
        }
        if self.fifo_b.timer_sel as usize == timer_index && self.fifo_b.needs_refill() {
            req.fifo_b = true;
        }
        req
    }

    /// Advance the DSP by `cycles` master cycles, emitting samples.
    pub fn tick(&mut self, cycles: u32) {
        let mut remaining = cycles as i32;
        while remaining > 0 {
            // Step in chunks bounded by the next sample / sequencer edge so the
            // channel timers stay phase-accurate.
            let step = remaining.min(self.sample_timer).min(self.seq_timer).max(1);

            self.ch1.tick(step);
            self.ch2.tick(step);
            self.ch3.tick(step);
            self.ch4.tick(step);

            self.seq_timer -= step;
            if self.seq_timer <= 0 {
                self.seq_timer += SEQ_PERIOD;
                self.clock_sequencer();
            }

            self.sample_timer -= step;
            if self.sample_timer <= 0 {
                self.sample_timer += CYCLES_PER_SAMPLE;
                self.emit_sample();
            }

            remaining -= step;
        }
    }

    /// Run one step of the 512 Hz frame sequencer, which clocks the length
    /// counters (256 Hz, even steps), the sweep unit (128 Hz, steps 2 and 6) and
    /// the volume envelopes (64 Hz, step 7).
    const fn clock_sequencer(&mut self) {
        match self.seq_step {
            0 | 4 => {
                self.ch1.clock_length();
                self.ch2.clock_length();
                self.ch3.clock_length();
                self.ch4.clock_length();
            }
            2 | 6 => {
                self.ch1.clock_length();
                self.ch2.clock_length();
                self.ch3.clock_length();
                self.ch4.clock_length();
                self.ch1.clock_sweep();
            }
            7 => {
                self.ch1.clock_envelope();
                self.ch2.clock_envelope();
                self.ch4.clock_envelope();
            }
            _ => {}
        }
        self.seq_step = (self.seq_step + 1) & 7;
    }

    /// Mix the current channel states, DC-block the result, and append one
    /// interleaved L/R sample pair to the output buffer.
    fn emit_sample(&mut self) {
        let (raw_l, raw_r) = if self.master_enable { self.mix() } else { (0, 0) };
        let l = self.dc_left.process(raw_l).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
        let r = self.dc_right.process(raw_r).clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16;
        self.out.push(l);
        self.out.push(r);
    }

    /// Mix the PSG and DirectSound channels into a centred 16-bit sample pair
    /// (before DC blocking), modelling the GBA's 10-bit output DAC.
    fn mix(&self) -> (i32, i32) {
        let psg = [
            i32::from(self.ch1.output()),
            i32::from(self.ch2.output()),
            i32::from(self.ch3.output()),
            i32::from(self.ch4.output()),
        ];

        // Sum the enabled PSG channels (each 0..15) per side.
        let mut psg_l = 0i32;
        let mut psg_r = 0i32;
        for i in 0..4 {
            if self.psg_left_enable[i] {
                psg_l += psg[i];
            }
            if self.psg_right_enable[i] {
                psg_r += psg[i];
            }
        }
        // SOUNDCNT_L master volume (0..7 -> x1..x8) and SOUNDCNT_H ratio.
        let ratio_shift = match self.psg_volume_code {
            0 => 2, // 25%
            1 => 1, // 50%
            _ => 0, // 100%
        };
        psg_l = (psg_l * (i32::from(self.vol_left) + 1)) >> ratio_shift;
        psg_r = (psg_r * (i32::from(self.vol_right) + 1)) >> ratio_shift;

        // DirectSound: signed 8-bit sample, optionally halved at 50% volume.
        let ds = |f: &Fifo, left: bool| -> i32 {
            let on = if left { f.enable_left } else { f.enable_right };
            if !on {
                return 0;
            }
            let v = i32::from(f.sample);
            if f.full_volume { v } else { v / 2 }
        };
        let ds_l = ds(&self.fifo_a, true) + ds(&self.fifo_b, true);
        let ds_r = ds(&self.fifo_a, false) + ds(&self.fifo_b, false);

        // GBA mixes into a 10-bit DAC centred on SOUNDBIAS (default 0x200). The
        // PSG contributes about half its raw range so DirectSound can sit on
        // top; the sum saturates the DAC exactly as the hardware does.
        let bias = i32::from(self.soundbias & 0x3FF);
        let mix_l = (psg_l >> 1) + ds_l;
        let mix_r = (psg_r >> 1) + ds_r;
        let dac_l = (bias + mix_l).clamp(0, 0x3FF);
        let dac_r = (bias + mix_r).clamp(0, 0x3FF);
        // Centre on the bias and scale the 10-bit value up towards 16-bit.
        ((dac_l - bias) << 6, (dac_r - bias) << 6)
    }

    /// Read a sound I/O register byte. `addr` is the absolute bus address in
    /// `0x04000060..=0x040000A7` (or wave RAM `0x90..=0x9F`); unmapped offsets
    /// read back as 0.
    pub fn read_register(&self, addr: u32) -> u8 {
        let off = addr & 0xFF;
        match off {
            // Channel 1
            0x60 => self.ch1.sweep_byte(),
            0x62 => (self.ch1.duty << 6) | 0x3F, // length write-only
            0x63 => self.ch1.envelope_byte(),
            0x65 => 0xBF | if self.ch1.length_enable { 0x40 } else { 0 },
            // Channel 2
            0x68 => (self.ch2.duty << 6) | 0x3F,
            0x69 => self.ch2.envelope_byte(),
            0x6D => 0xBF | if self.ch2.length_enable { 0x40 } else { 0 },
            // Channel 3
            0x70 => self.ch3.cnt_l_byte(),
            0x73 => self.ch3.volume_byte(),
            0x75 => 0xBF | if self.ch3.length_enable { 0x40 } else { 0 },
            // Channel 4
            0x79 => self.ch4.envelope_byte(),
            0x7C => self.ch4.poly_byte(),
            0x7D => 0xBF | if self.ch4.length_enable { 0x40 } else { 0 },
            // SOUNDCNT_L
            0x80 => (self.vol_right & 7) | ((self.vol_left & 7) << 4),
            0x81 => {
                let mut v = 0u8;
                for i in 0..4 {
                    if self.psg_right_enable[i] {
                        v |= 1 << i;
                    }
                    if self.psg_left_enable[i] {
                        v |= 1 << (4 + i);
                    }
                }
                v
            }
            // SOUNDCNT_H low byte: PSG mix ratio (0-1) + DirectSound A/B volume.
            0x82 => {
                (self.psg_volume_code & 3)
                    | ((self.fifo_a.full_volume as u8) << 2)
                    | ((self.fifo_b.full_volume as u8) << 3)
            }
            // SOUNDCNT_H high byte: DirectSound enable/timer-select bits.
            0x83 => {
                let mut h = 0u8;
                if self.fifo_a.enable_right {
                    h |= 1 << 0;
                }
                if self.fifo_a.enable_left {
                    h |= 1 << 1;
                }
                if self.fifo_a.timer_sel == 1 {
                    h |= 1 << 2;
                }
                if self.fifo_b.enable_right {
                    h |= 1 << 4;
                }
                if self.fifo_b.enable_left {
                    h |= 1 << 5;
                }
                if self.fifo_b.timer_sel == 1 {
                    h |= 1 << 6;
                }
                h
            }
            // SOUNDCNT_X
            0x84 => {
                let mut v = 0u8;
                if self.ch1.enabled {
                    v |= 1 << 0;
                }
                if self.ch2.enabled {
                    v |= 1 << 1;
                }
                if self.ch3.enabled {
                    v |= 1 << 2;
                }
                if self.ch4.enabled {
                    v |= 1 << 3;
                }
                if self.master_enable {
                    v |= 1 << 7;
                }
                v
            }
            0x88 => (self.soundbias & 0xFF) as u8,
            0x89 => ((self.soundbias >> 8) & 0xFF) as u8,
            // Wave RAM (reads back the inactive bank's bytes).
            0x90..=0x9F => self.ch3.ram_read((off - 0x90) as usize),
            _ => 0,
        }
    }

    /// Write a sound I/O register byte. `addr` is the absolute bus address.
    /// While the APU master is disabled, all registers except SOUNDCNT_X and
    /// wave RAM ignore writes, matching hardware.
    pub fn write_register(&mut self, addr: u32, data: u8) {
        let off = addr & 0xFF;
        // With the master disabled, only SOUNDCNT_X and wave RAM are writable.
        if !self.master_enable && off != 0x84 && !(0x90..=0x9F).contains(&off) {
            return;
        }
        match off {
            0x60 => self.ch1.write_sweep(data),
            0x62 => self.ch1.write_duty_length(data),
            0x63 => self.ch1.write_envelope(data),
            0x64 => self.ch1.write_freq_lo(data),
            0x65 => self.ch1.write_freq_hi_trigger(data),
            0x68 => self.ch2.write_duty_length(data),
            0x69 => self.ch2.write_envelope(data),
            0x6C => self.ch2.write_freq_lo(data),
            0x6D => self.ch2.write_freq_hi_trigger(data),
            0x70 => self.ch3.write_cnt_l(data),
            0x72 => self.ch3.write_length(data),
            0x73 => self.ch3.write_volume(data),
            0x74 => self.ch3.write_freq_lo(data),
            0x75 => self.ch3.write_freq_hi_trigger(data),
            0x78 => self.ch4.write_length(data),
            0x79 => self.ch4.write_envelope(data),
            0x7C => self.ch4.write_poly(data),
            0x7D => self.ch4.write_trigger(data),
            0x80 => {
                self.vol_right = data & 7;
                self.vol_left = (data >> 4) & 7;
            }
            0x81 => {
                for i in 0..4 {
                    self.psg_right_enable[i] = data & (1 << i) != 0;
                    self.psg_left_enable[i] = data & (1 << (4 + i)) != 0;
                }
            }
            0x82 => {
                self.psg_volume_code = data & 3;
                self.fifo_a.full_volume = data & (1 << 2) != 0;
                self.fifo_b.full_volume = data & (1 << 3) != 0;
            }
            0x83 => {
                self.fifo_a.enable_right = data & (1 << 0) != 0;
                self.fifo_a.enable_left = data & (1 << 1) != 0;
                self.fifo_a.timer_sel = (data >> 2) & 1;
                if data & (1 << 3) != 0 {
                    self.fifo_a.reset();
                }
                self.fifo_b.enable_right = data & (1 << 4) != 0;
                self.fifo_b.enable_left = data & (1 << 5) != 0;
                self.fifo_b.timer_sel = (data >> 6) & 1;
                if data & (1 << 7) != 0 {
                    self.fifo_b.reset();
                }
            }
            0x84 => {
                let en = data & 0x80 != 0;
                self.master_enable = en;
                if !en {
                    self.reset_on_disable();
                }
            }
            0x88 => self.soundbias = (self.soundbias & 0xFF00) | data as u16,
            0x89 => self.soundbias = (self.soundbias & 0x00FF) | ((data as u16) << 8),
            0x90..=0x9F => self.ch3.ram_write((off - 0x90) as usize, data),
            // Sound FIFO byte writes (normally fed 32-bit via DMA).
            0xA0..=0xA3 => self.fifo_a.push_byte(data),
            0xA4..=0xA7 => self.fifo_b.push_byte(data),
            _ => {}
        }
    }

    /// Clear the PSG channel state when the master enable is turned off,
    /// preserving wave RAM (which stays accessible while powered down).
    fn reset_on_disable(&mut self) {
        self.ch1 = Square::new(true);
        self.ch2 = Square::new(false);
        let ram = self.ch3.ram;
        self.ch3 = Wave { ram, ..Default::default() };
        self.ch4 = Noise::default();
    }
}

// Register encode/decode helpers for the individual channels. Each method reads
// or writes one byte of a channel's NRxx register set.

impl Square {
    /// SOUND1CNT_L (sweep) read-back, with the unused high bit set.
    const fn sweep_byte(&self) -> u8 {
        (self.sweep_shift & 7)
            | ((self.sweep_dir_down as u8) << 3)
            | ((self.sweep_period & 7) << 4)
            | 0x80
    }
    /// NRx2 (envelope) read-back.
    const fn envelope_byte(&self) -> u8 {
        (self.env_period & 7) | ((self.env_dir_up as u8) << 3) | (self.env_initial << 4)
    }
    /// Write SOUND1CNT_L: sweep shift, direction and period.
    const fn write_sweep(&mut self, d: u8) {
        self.sweep_shift = d & 7;
        self.sweep_dir_down = d & 0x08 != 0;
        self.sweep_period = (d >> 4) & 7;
    }
    /// Write NRx1: duty cycle and initial length.
    const fn write_duty_length(&mut self, d: u8) {
        self.duty = (d >> 6) & 3;
        self.length_counter = 64 - (d & 0x3F) as u16;
    }
    /// Write NRx2: envelope and DAC power (disabling the channel when the DAC
    /// turns off).
    const fn write_envelope(&mut self, d: u8) {
        self.env_period = d & 7;
        self.env_dir_up = d & 0x08 != 0;
        self.env_initial = (d >> 4) & 0x0F;
        self.dac_on = (d & 0xF8) != 0;
        if !self.dac_on {
            self.enabled = false;
        }
    }
    /// Write the low 8 bits of the frequency (NRx3).
    const fn write_freq_lo(&mut self, d: u8) {
        self.freq = (self.freq & 0x0700) | d as u16;
    }
    /// Write NRx4: the high frequency bits, length enable, and trigger.
    fn write_freq_hi_trigger(&mut self, d: u8) {
        self.freq = (self.freq & 0x00FF) | (((d & 7) as u16) << 8);
        self.length_enable = d & 0x40 != 0;
        if d & 0x80 != 0 {
            self.trigger();
        }
    }
}

impl Wave {
    /// SOUND3CNT_L read-back: bank mode, selected bank and DAC power.
    const fn cnt_l_byte(&self) -> u8 {
        ((self.two_banks as u8) << 5) | ((self.bank) << 6) | ((self.dac_on as u8) << 7) | 0x1F
    }
    /// SOUND3CNT_H volume-byte read-back.
    const fn volume_byte(&self) -> u8 {
        ((self.volume_code & 3) << 5) | ((self.force_75 as u8) << 7)
    }
    /// Write SOUND3CNT_L: bank mode, selected bank and DAC power.
    const fn write_cnt_l(&mut self, d: u8) {
        self.two_banks = d & 0x20 != 0;
        self.bank = (d >> 6) & 1;
        self.dac_on = d & 0x80 != 0;
        if !self.dac_on {
            self.enabled = false;
        }
    }
    /// Write the initial length (256-step counter).
    const fn write_length(&mut self, d: u8) {
        self.length_counter = 256 - d as u16;
    }
    /// Write SOUND3CNT_H: volume selector and the 75% override.
    const fn write_volume(&mut self, d: u8) {
        self.volume_code = (d >> 5) & 3;
        self.force_75 = d & 0x80 != 0;
    }
    /// Write the low 8 bits of the frequency.
    const fn write_freq_lo(&mut self, d: u8) {
        self.freq = (self.freq & 0x0700) | d as u16;
    }
    /// Write the high frequency bits, length enable, and trigger.
    fn write_freq_hi_trigger(&mut self, d: u8) {
        self.freq = (self.freq & 0x00FF) | (((d & 7) as u16) << 8);
        self.length_enable = d & 0x40 != 0;
        if d & 0x80 != 0 {
            self.trigger();
        }
    }
    /// Read a wave-RAM byte. The CPU sees the bank that is *not* currently
    /// playing (approximated here as the opposite of `bank`).
    const fn ram_read(&self, i: usize) -> u8 {
        let base = if self.bank == 0 { 16 } else { 0 };
        self.ram[base + i]
    }
    /// Write a wave-RAM byte into the non-playing bank.
    const fn ram_write(&mut self, i: usize, d: u8) {
        let base = if self.bank == 0 { 16 } else { 0 };
        self.ram[base + i] = d;
    }
}

impl Noise {
    /// NR42 (envelope) read-back.
    const fn envelope_byte(&self) -> u8 {
        (self.env_period & 7) | ((self.env_dir_up as u8) << 3) | (self.env_initial << 4)
    }
    /// NR43 (polynomial counter) read-back: divisor, width and shift.
    const fn poly_byte(&self) -> u8 {
        (self.divisor_code & 7) | ((self.width7 as u8) << 3) | (self.shift << 4)
    }
    /// Write NR41: initial length.
    const fn write_length(&mut self, d: u8) {
        self.length_counter = 64 - (d & 0x3F) as u16;
    }
    /// Write NR42: envelope and DAC power.
    const fn write_envelope(&mut self, d: u8) {
        self.env_period = d & 7;
        self.env_dir_up = d & 0x08 != 0;
        self.env_initial = (d >> 4) & 0x0F;
        self.dac_on = (d & 0xF8) != 0;
        if !self.dac_on {
            self.enabled = false;
        }
    }
    /// Write NR43: noise divisor, LFSR width and pre-scaler shift.
    const fn write_poly(&mut self, d: u8) {
        self.divisor_code = d & 7;
        self.width7 = d & 0x08 != 0;
        self.shift = (d >> 4) & 0x0F;
    }
    /// Write NR44: length enable and trigger.
    fn write_trigger(&mut self, d: u8) {
        self.length_enable = d & 0x40 != 0;
        if d & 0x80 != 0 {
            self.trigger();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifo_push_and_timer_pop() {
        let mut apu = Apu::new();
        apu.master_enable = true;
        apu.write_register(0x0400_0083, 0b0000_0011); // FIFO A: enable L/R, timer0
        apu.push_fifo_a(0x0403_0201);
        // Four samples queued; ticking timer0 pops them in order.
        apu.on_timer_overflow(0, 1);
        assert_eq!(apu.fifo_a.sample, 0x01);
        apu.on_timer_overflow(0, 2);
        assert_eq!(apu.fifo_a.sample, 0x03);
        // Below half (<=16) -> refill requested.
        let req = apu.on_timer_overflow(0, 1);
        assert!(req.fifo_a);
    }

    #[test]
    fn square_produces_nonzero_when_triggered() {
        let mut apu = Apu::new();
        apu.write_register(0x0400_0084, 0x80); // master on
        apu.write_register(0x0400_0080, 0x77); // full L/R master volume
        apu.write_register(0x0400_0081, 0x11); // ch1 enabled L+R
        apu.write_register(0x0400_0062, 0x80); // duty 50%? (0b10<<6) -> duty=2
        apu.write_register(0x0400_0063, 0xF0); // envelope: volume 15, DAC on
        apu.write_register(0x0400_0064, 0x00); // freq lo
        apu.write_register(0x0400_0065, 0x87); // trigger + freq hi
        // Run ~1/60s and confirm the output isn't all silence.
        apu.tick(CPU_HZ / 60);
        let s = apu.take_samples();
        assert!(!s.is_empty());
        assert!(s.iter().any(|&v| v != 0), "square channel produced only silence");
    }

    #[test]
    fn noise_lfsr_advances() {
        let mut apu = Apu::new();
        apu.write_register(0x0400_0084, 0x80);
        apu.write_register(0x0400_0079, 0xF0); // envelope vol 15, DAC on
        apu.write_register(0x0400_007C, 0x00); // divisor 0, shift 0
        apu.write_register(0x0400_007D, 0x80); // trigger
        let before = apu.ch4.lfsr;
        apu.ch4.tick(10_000);
        assert_ne!(apu.ch4.lfsr, before, "noise LFSR should advance");
    }

    #[test]
    fn master_disable_silences_psg() {
        let mut apu = Apu::new();
        apu.write_register(0x0400_0084, 0x80);
        apu.write_register(0x0400_0063, 0xF0); // ch1 DAC on
        apu.write_register(0x0400_0065, 0x80); // trigger
        assert!(apu.ch1.enabled);
        apu.write_register(0x0400_0084, 0x00); // master off
        assert!(!apu.ch1.enabled);
    }

    #[test]
    fn dc_blocker_removes_constant_offset() {
        let mut dc = DcBlocker::default();
        let mut last = 0;
        for _ in 0..20_000 {
            last = dc.process(5_000); // constant input
        }
        assert!(last.abs() < 50, "DC offset not removed: {}", last);
    }

    #[test]
    fn silence_stays_centred_on_zero() {
        let mut apu = Apu::new();
        apu.write_register(0x0400_0084, 0x80); // master on, but no channels enabled
        apu.tick(CPU_HZ / 100);
        let s = apu.take_samples();
        assert!(!s.is_empty());
        assert!(s.iter().all(|&v| v == 0), "silence is not centred on zero");
    }

    #[test]
    fn output_stays_within_range_under_full_load() {
        let mut apu = Apu::new();
        apu.write_register(0x0400_0084, 0x80);
        apu.write_register(0x0400_0080, 0x77); // full master volume
        apu.write_register(0x0400_0081, 0xFF); // all PSG channels L+R
        apu.write_register(0x0400_0082, 0x0E); // 100% PSG ratio + DS A/B full vol
        apu.write_register(0x0400_0083, 0x33); // DS A/B enabled L+R
        // Trigger every PSG channel.
        apu.write_register(0x0400_0063, 0xF0);
        apu.write_register(0x0400_0065, 0x87);
        apu.write_register(0x0400_0069, 0xF0);
        apu.write_register(0x0400_006D, 0x87);
        // Pump the DirectSound FIFOs with the maximum sample.
        for _ in 0..8 {
            apu.push_fifo_a(0x7F7F_7F7F);
            apu.push_fifo_b(0x7F7F_7F7F);
        }
        apu.on_timer_overflow(0, 4);
        apu.tick(CPU_HZ / 200);
        // No panic above means no overflow; values are i16 by construction.
        assert!(!apu.take_samples().is_empty());
    }
}
