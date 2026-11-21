//! DirectSound mixer and the VBlank interrupt handler that swaps its buffers.
//! Port of the C version's audio.c and irq.c (mixer adapted from
//! http://deku.gbadev.org).

use crate::gba::*;
use core::cell::UnsafeCell;

pub const SAMPLE_RATE: u32 = 10512;
const SAMPLES_PER_FRAME: usize = 176;
const TM0_START: u16 = 63940;
const MAX_CHANNELS: usize = 4;

/// Two frames of 8-bit samples; DMA1 plays one while the game mixes the other.
#[repr(C, align(4))]
struct MixBuffers([[i8; SAMPLES_PER_FRAME]; 2]);

/// State shared between the game loop and the interrupt handler. The GBA has
/// one core, and the handler only touches `active_buffer` and `cur_mix`.
struct Shared {
    buffers: UnsafeCell<MixBuffers>,
    active_buffer: UnsafeCell<u8>,
    cur_mix: UnsafeCell<*mut i8>,
}

unsafe impl Sync for Shared {}

static SHARED: Shared = Shared {
    buffers: UnsafeCell::new(MixBuffers([[0; SAMPLES_PER_FRAME]; 2])),
    active_buffer: UnsafeCell::new(1),
    cur_mix: UnsafeCell::new(core::ptr::null_mut()),
};

fn buffer_base() -> *mut i8 {
    SHARED.buffers.get() as *mut i8
}

#[derive(Clone, Copy)]
struct Channel {
    data: Option<&'static [i8]>,
    pos: u32,         // 22.10 fixed point
    length: u32,      // 22.10 fixed point
    stride: u32,      // 22.10 fixed point
    vol: u32,         // 1.6 fixed point
    loop_length: u32, // 22.10 fixed point
}

pub struct Mixer {
    channels: [Channel; MAX_CHANNELS],
}

impl Mixer {
    /// Starts the sound hardware: timer 0 sets the sample rate, DMA1 feeds
    /// FIFO A.
    pub fn init() -> Self {
        REG_SOUNDBIAS.write(0x4200);
        REG_SOUNDCNT_H.write(SDS_DMG100 | SDS_A100 | SDS_B100 | SDS_AR | SDS_AL | SDS_ATMR0 | SDS_ARESET);
        REG_SOUNDCNT_X.write(SOUND_ENABLE);

        // 0xFFFF - round(cpuFreq / playbackFreq)
        REG_TM0D.write(TM0_START);
        REG_TM0CNT.write(TIMER_ENABLE);

        dma_fifo_repeat(1, REG_FIFO_A, buffer_base() as usize);

        let idle = Channel {
            data: None,
            pos: 0,
            length: 0,
            stride: 0,
            vol: 0,
            loop_length: 0,
        };
        Mixer {
            channels: [idle; MAX_CHANNELS],
        }
    }

    /// Mixes the next frame into the buffer DMA1 is not playing.
    pub fn mix(&mut self) {
        let mut temp = [0i16; SAMPLES_PER_FRAME];

        for ch in self.channels.iter_mut() {
            let Some(data) = ch.data else { continue };
            for t in temp.iter_mut() {
                let sample = data[(ch.pos >> 10) as usize] as i32 as u32;
                *t = t.wrapping_add(sample.wrapping_mul(ch.vol) as i16);
                ch.pos += ch.stride;

                if ch.pos >= ch.length {
                    if ch.loop_length == 0 {
                        ch.data = None;
                        break;
                    }
                    while ch.pos >= ch.length {
                        ch.pos -= ch.loop_length;
                    }
                }
            }
        }

        // 16 bit down to 8: >>6 for the volume, >>2 for four channels.
        let out = unsafe { *SHARED.cur_mix.get() };
        if out.is_null() {
            return;
        }
        for (i, &t) in temp.iter().enumerate() {
            unsafe { *out.add(i) = (t >> 8) as i8 }
        }
    }

    /// Plays a one-shot sound recorded at `freq` Hz on `channel` at `vol`
    /// (0-64 is 0.0-1.0 in 1.6 fixed point, shifted down by 6).
    pub fn play(&mut self, channel: usize, data: &'static [i8], freq: u32, vol: u32) {
        let length = data.len() as u32;
        self.channels[channel] = Channel {
            data: Some(data),
            pos: 0,
            length: length << 10,
            stride: (freq << 10) / SAMPLE_RATE,
            vol: vol << 6,
            loop_length: 0,
        };
    }
}

/// Called by the BIOS on every VBlank. The BIOS enters it with `ldr pc`, which
/// on the ARM7TDMI cannot switch to Thumb, so it has to be ARM code. It runs
/// from IWRAM, whose 32-bit bus fetches ARM code without wait states.
#[unsafe(link_section = ".iwram.irq_handler")]
#[instruction_set(arm::a32)]
extern "C" fn irq_handler() {
    let ime = REG_IME.read();
    REG_IME.write(0);

    unsafe {
        let active = SHARED.active_buffer.get();
        let cur = SHARED.cur_mix.get();
        if *active == 1 {
            // Buffer 1 just finished: restart DMA on buffer 0, mix into 1.
            dma_fifo_repeat(1, REG_FIFO_A, buffer_base() as usize);
            *cur = buffer_base().add(SAMPLES_PER_FRAME);
            *active = 0;
        } else {
            // Buffer 0 just finished and DMA has run on into buffer 1.
            *cur = buffer_base();
            *active = 1;
        }
    }

    REG_IF.write(IRQ_VBLANK);
    REG_IFBIOS.write(REG_IFBIOS.read() | IRQ_VBLANK);
    REG_IME.write(ime);
}

/// Installs the handler and enables the VBlank interrupt.
pub fn irq_init() {
    REG_IRQ_VECTOR.write(irq_handler as *const () as usize as u32);
    REG_DISPSTAT.write(VBL_IRQ | HBL_IRQ);
    REG_IE.write(IRQ_VBLANK);
    REG_IME.write(1);
}
