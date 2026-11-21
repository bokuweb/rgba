//! Registers, DMA, OAM helpers and BIOS calls. Mirrors the C version's
//! memory.h, video.h, dma.h, io.h, timer.h and systemcall.s.

use core::ptr::{read_volatile, write_volatile};

pub const IO_BASE: usize = 0x0400_0000;
pub const IWRAM: usize = 0x0300_0000;
pub const VRAM: usize = 0x0600_0000;
pub const PALETTE_RAM: usize = 0x0500_0000;
pub const OAM: usize = 0x0700_0000;

/// A memory-mapped register of width `T`.
#[derive(Clone, Copy)]
pub struct Reg<T>(usize, core::marker::PhantomData<T>);

impl<T: Copy> Reg<T> {
    pub const fn at(addr: usize) -> Self {
        Reg(addr, core::marker::PhantomData)
    }
    #[inline(always)]
    pub fn read(self) -> T {
        unsafe { read_volatile(self.0 as *const T) }
    }
    #[inline(always)]
    pub fn write(self, v: T) {
        unsafe { write_volatile(self.0 as *mut T, v) }
    }
}

const fn io<T: Copy>(off: usize) -> Reg<T> {
    Reg::at(IO_BASE + off)
}

// Video
pub const REG_DISPCNT: Reg<u32> = io(0x0000);
pub const REG_DISPSTAT: Reg<u16> = io(0x0004);
pub const REG_BG0CNT: Reg<u16> = io(0x0008);
pub const REG_BG1CNT: Reg<u16> = io(0x000A);
pub const REG_BG0HOFS: Reg<u16> = io(0x0010);
pub const REG_BG1HOFS: Reg<u16> = io(0x0014);
pub const REG_BLDCNT: Reg<u16> = io(0x0050);
pub const REG_COLEV: Reg<u16> = io(0x0052);

pub const MODE_0: u32 = 0;
pub const OBJ_1D: u32 = 1 << 6;
pub const BG0_ON: u32 = 1 << 8;
pub const BG1_ON: u32 = 1 << 9;
pub const OBJ_ON: u32 = 1 << 12;

pub const VBL_IRQ: u16 = 1 << 3;
pub const HBL_IRQ: u16 = 1 << 4;

pub const BLEND_MODE_ALPHA: u16 = 1 << 6;
pub const BLEND_BG0_DST: u16 = 1 << 8;
pub const BLEND_BG1_DST: u16 = 1 << 9;

pub const fn bg_priority(p: u16) -> u16 {
    p & 0x3
}
pub const fn bg_char_base(b: u16) -> u16 {
    (b << 2) & 0xC
}
pub const fn bg_screen_base(b: u16) -> u16 {
    (b << 8) & 0x1F00
}
pub const fn colev_src(a: u16) -> u16 {
    a & 0x1F
}
pub const fn colev_dst(a: u16) -> u16 {
    (a & 0x1F) << 8
}

pub const SCREEN_WIDTH: i32 = 240;
pub const SCREEN_HEIGHT: i32 = 160;

/// Address of tile `tile` in character block `block` (4bpp, 32 bytes a tile).
pub const fn tile_addr(block: usize, tile: usize) -> usize {
    VRAM + block * 0x4000 + tile * 32
}
/// Address of screen block `block` (32x32 entries).
pub const fn screen_block_addr(block: usize) -> usize {
    VRAM + block * 0x800
}
pub const fn bg_pal16_addr(bank: usize) -> usize {
    PALETTE_RAM + bank * 32
}
pub const fn obj_pal16_addr(bank: usize) -> usize {
    PALETTE_RAM + 0x200 + bank * 32
}

/// Copies halfwords one at a time. VRAM and palette RAM ignore byte writes, so
/// this is the narrowest copy that works for both.
pub fn copy_halfwords(dst: usize, src: &[u16]) {
    let dst = dst as *mut u16;
    for (i, &v) in src.iter().enumerate() {
        unsafe { write_volatile(dst.add(i), v) }
    }
}

// Interrupts
pub const REG_IE: Reg<u16> = io(0x0200);
pub const REG_IF: Reg<u16> = io(0x0202);
pub const REG_WAITCNT: Reg<u16> = io(0x0204);
pub const REG_IME: Reg<u16> = io(0x0208);
/// Flags the BIOS's IntrWait functions check, at the top of IWRAM.
pub const REG_IFBIOS: Reg<u16> = Reg::at(IWRAM + 0x7FF8);
/// Where the BIOS jumps on an IRQ.
pub const REG_IRQ_VECTOR: Reg<u32> = Reg::at(IWRAM + 0x7FFC);
pub const IRQ_VBLANK: u16 = 1 << 0;

// Keys
pub const REG_KEYINPUT: Reg<u16> = io(0x0130);
pub const KEY_A: u16 = 1 << 0;
pub const KEY_B: u16 = 1 << 1;
pub const KEY_START: u16 = 1 << 3;
const KEY_MASK: u16 = 0x03FF;

/// Key state for this frame and the last one; set bits are pressed keys.
#[derive(Default)]
pub struct Keys {
    curr: u16,
    prev: u16,
}

impl Keys {
    pub fn poll(&mut self) {
        self.prev = self.curr;
        self.curr = !REG_KEYINPUT.read() & KEY_MASK;
    }
    /// Keys in `mask` that went down this frame.
    pub fn hit(&self, mask: u16) -> bool {
        mask & self.curr & !self.prev != 0
    }
}

// Timers
pub const REG_TM0D: Reg<u16> = io(0x0100);
pub const REG_TM0CNT: Reg<u16> = io(0x0102);
pub const TIMER_ENABLE: u16 = 1 << 7;

// Sound
pub const REG_SOUNDCNT_H: Reg<u16> = io(0x0082);
pub const REG_SOUNDCNT_X: Reg<u16> = io(0x0084);
pub const REG_SOUNDBIAS: Reg<u16> = io(0x0088);
pub const REG_FIFO_A: usize = IO_BASE + 0x00A0;
pub const SDS_DMG100: u16 = 2;
pub const SDS_A100: u16 = 1 << 2;
pub const SDS_B100: u16 = 1 << 3;
pub const SDS_AR: u16 = 1 << 8;
pub const SDS_AL: u16 = 1 << 9;
pub const SDS_ATMR0: u16 = 0;
pub const SDS_ARESET: u16 = 1 << 11;
pub const SOUND_ENABLE: u16 = 1 << 7;

// DMA
const DMA_SRC_INC: u32 = 0;
const DMA_DST_INC: u32 = 0;
const DMA_REPEAT: u32 = 1 << 25;
pub const DMA_32: u32 = 1 << 26;
pub const DMA_NOW: u32 = 0;
const DMA_AT_REFRESH: u32 = 3 << 28;
const DMA_ENABLE: u32 = 1 << 31;

/// Keeps the compiler from moving memory accesses across this point. An empty
/// `asm!` without `nomem` does that; `compiler_fence` would call
/// `__sync_synchronize`, which ARMv4T has no instruction for.
#[inline(always)]
fn memory_barrier() {
    unsafe { core::arch::asm!("", options(nostack, preserves_flags)) }
}

#[inline(always)]
fn dma_regs(channel: usize) -> (Reg<u32>, Reg<u32>, Reg<u32>) {
    let base = 0x00B0 + channel * 12;
    (io(base), io(base + 4), io(base + 8))
}

/// Starts an immediate DMA of `count` units (halfwords, or words with
/// `DMA_32`). The CPU stops until it finishes.
#[inline(always)]
pub fn dma_copy(channel: usize, dst: usize, src: usize, count: u32, mode: u32) {
    let (sad, dad, cnt) = dma_regs(channel);
    // Plain stores to the source have to land before the DMA reads it.
    memory_barrier();
    cnt.write(0);
    sad.write(src as u32);
    dad.write(dst as u32);
    cnt.write((count & 0xFFFF) | mode | DMA_ENABLE);
    memory_barrier();
}

/// Feeds a sound FIFO from `src`, a word each time the FIFO asks for more.
#[inline(always)]
pub fn dma_fifo_repeat(channel: usize, dst: usize, src: usize) {
    let (sad, dad, cnt) = dma_regs(channel);
    cnt.write(0);
    sad.write(src as u32);
    dad.write(dst as u32);
    cnt.write(DMA_AT_REFRESH | DMA_32 | DMA_ENABLE | DMA_SRC_INC | DMA_DST_INC | DMA_REPEAT);
}

// OAM
pub const OAM_OBJ_COUNT: usize = 128;
const A0_ROT_SCALE_ON: u16 = 1 << 8;
const A0_HIDE: u16 = 1 << 9;

#[derive(Clone, Copy)]
pub enum ObjSize {
    S16x16 = 0x01,
    S32x32 = 0x02,
    S32x64 = 0x0B,
}

/// One OAM entry: three attributes plus the halfword that holds a piece of an
/// affine matrix.
#[derive(Clone, Copy, Default)]
#[repr(C, align(4))]
pub struct ObjAttr {
    pub attr: [u16; 3],
    pub fill: u16,
}

/// A shadow of OAM in IWRAM, copied over every frame.
#[repr(C, align(4))]
pub struct Oam(pub [ObjAttr; OAM_OBJ_COUNT]);

impl Oam {
    /// Every object hidden, also written to OAM.
    pub fn new() -> Self {
        let mut oam = Oam([ObjAttr {
            attr: [A0_HIDE, 0, 0],
            fill: 0,
        }; OAM_OBJ_COUNT]);
        oam.copy_to_hw();
        oam
    }

    pub fn copy_to_hw(&mut self) {
        let words = (core::mem::size_of::<Self>() / 4) as u32;
        dma_copy(3, OAM, self.0.as_ptr() as usize, words, DMA_NOW | DMA_32);
    }

    pub fn init(&mut self, i: usize, x: i32, y: i32, size: ObjSize, tile: u32, pal: u32) {
        let s = size as u16;
        self.0[i].attr = [
            ((s & 0x0C) << 12) | (y as u16 & 0x00FF),
            ((s & 0x03) << 14) | (x as u16 & 0x01FF),
            (tile as u16 & 0x03FF) | ((pal as u16 & 0x0F) << 12),
        ];
    }

    pub fn hide(&mut self, i: usize) {
        // The C version builds this from attr[1], a typo. The object is
        // disabled either way.
        let a = &mut self.0[i].attr;
        a[0] = (a[0] & !A0_ROT_SCALE_ON) | A0_HIDE;
    }

    pub fn set_rot_scale(&mut self, i: usize) {
        self.0[i].attr[0] |= A0_ROT_SCALE_ON;
    }

    pub fn set_priority(&mut self, i: usize, priority: u32) {
        let a = &mut self.0[i].attr;
        a[2] = (a[2] & !0x03) | ((priority as u16 & 0x03) << 10);
    }

    pub fn set_position(&mut self, i: usize, x: i32, y: i32) {
        let a = &mut self.0[i].attr;
        a[0] = (a[0] & !0x00FF) | (y as u16 & 0x00FF);
        a[1] = (a[1] & !0x01FF) | (x as u16 & 0x01FF);
    }

    pub fn set_tile(&mut self, i: usize, tile: u32) {
        let a = &mut self.0[i].attr;
        a[2] = (a[2] & !0x3FF) | (tile as u16 & 0x3FF);
    }

    /// Affine matrix `n` lives in the spare halfwords of objects 4n..4n+3.
    pub fn set_affine(&mut self, n: usize, pa: i16, pb: i16, pc: i16, pd: i16) {
        self.0[n * 4].fill = pa as u16;
        self.0[n * 4 + 1].fill = pb as u16;
        self.0[n * 4 + 2].fill = pc as u16;
        self.0[n * 4 + 3].fill = pd as u16;
    }

    pub fn affine_identity(&mut self, n: usize) {
        self.set_affine(n, 1 << 8, 0, 0, 1 << 8);
    }

    /// Rotation only; the C version ignores its scale arguments too.
    pub fn affine_rotate(&mut self, n: usize, angle: u16) {
        let ss = ((crate::sin_lut::lu_sin(angle as u32) as i32 - 0x4000) >> 6) as i16;
        let cc = ((crate::sin_lut::lu_cos(angle as u32) as i32 - 0x4000) >> 6) as i16;
        self.set_affine(n, cc, -ss, ss, cc);
    }
}

// BIOS
/// Halts until the next VBlank interrupt (SWI 0x05).
#[inline(always)]
pub fn vblank_intr_wait() {
    // The BIOS call clobbers r0-r3 and r12. Leaving out `nomem` keeps memory
    // accesses on the right side of the wait, as the C call did.
    unsafe {
        core::arch::asm!(
            "swi #0x05",
            out("r0") _, out("r1") _, out("r2") _, out("r3") _, out("r12") _,
        );
    }
}
