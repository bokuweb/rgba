use crate::cpu::bus::accessor::BusAccessor;
use crate::types::Word;

/// BIOS function implementations
/// These implement the GBA BIOS System Call functions
pub struct Bios;

impl Bios {
    /// SWI 0x00 - SoftReset
    /// Clears 0x7E00-0x7FFF in IWRAM and sets up registers for reset
    pub const fn soft_reset<T: BusAccessor>(
        _bus: &mut T,
        gpr: &mut [Word; 16],
        _iwram_flag: u8,
    ) {
        // Clear most of IWRAM (0x7E00-0x7FFF)
        // Set LR based on flag at 0x7FFA
        // For now, just set a default return address
        gpr[14] = 0x0800_0000; // Default to ROM
    }

    /// SWI 0x02 - Halt
    /// Halts the CPU until an interrupt occurs
    pub fn halt<T: BusAccessor>(bus: &mut T, _gpr: &mut [Word; 16]) {
        bus.set_cpu_halted(true);
    }

    /// SWI 0x04 - IntrWait
    /// Waits for specific interrupts
    pub fn intr_wait<T: BusAccessor>(
        bus: &mut T,
        gpr: &mut [Word; 16],
    ) {
        let discard_old = gpr[0] != 0;
        let interrupt_flags = (gpr[1] & 0xFFFF) as u16;
        let trace_dma = std::env::var("AGB_TRACE_DMA").ok().as_deref() == Some("1");
        if trace_dma {
            println!(
                "BIOS IntrWait enter discard_old={} mask=0x{:04x} IF=0x{:04x} IME=0x{:04x}",
                discard_old,
                interrupt_flags,
                bus.read_halfword(0x0400_0202),
                bus.read_halfword(0x0400_0208)
            );
        }

        // Match JS behavior: ensure IME is enabled while waiting.
        if (bus.read_halfword(0x0400_0208) & 0x0001) == 0 {
            bus.write_halfword(0x0400_0208, 0x0001);
        }

        // If caller does not request discarding old flags and target IF is already set, return immediately.
        let current_if = bus.read_halfword(0x0400_0202);
        if !discard_old && (current_if & interrupt_flags) != 0 {
            if trace_dma {
                println!("BIOS IntrWait immediate return IF=0x{current_if:04x}");
            }
            return;
        }

        // Clear latched interrupt flags before waiting.
        bus.write_halfword(0x0400_0202, 0xFFFF);
        if trace_dma {
            println!(
                "BIOS IntrWait halt IF(after clear)=0x{:04x}",
                bus.read_halfword(0x0400_0202)
            );
        }
        bus.set_cpu_halted(true);
    }

    /// SWI 0x05 - VBlankIntrWait
    /// Waits for VBlank interrupt
    pub fn vblank_intr_wait<T: BusAccessor>(
        bus: &mut T,
        gpr: &mut [Word; 16],
    ) {
        // Set up for VBlank interrupt
        gpr[0] = 1; // Discard old interrupt
        gpr[1] = 1; // VBlank interrupt flag
        Self::intr_wait(bus, gpr);
    }

    /// SWI 0x06 - Div
    /// Signed division r0/r1
    pub const fn div<T: BusAccessor>(_bus: &mut T, gpr: &mut [Word; 16]) {
        let numerator = gpr[0] as i32;
        let denominator = gpr[1] as i32;


        if denominator == 0 {
            // Division by zero typically causes endless loop in BIOS
            // For testing, we'll return max values
            gpr[0] = if numerator >= 0 { 0x7FFF_FFFF } else { 0x8000_0000 };
            gpr[1] = numerator as u32;
            gpr[3] = 0x7FFF_FFFF;
        } else {
            let result = numerator / denominator;
            let remainder = numerator % denominator;
            gpr[0] = result as u32;
            gpr[1] = remainder as u32;
            gpr[3] = result.unsigned_abs();
        }
    }

    /// SWI 0x07 - DivArm
    /// Division with swapped parameters (ARM library compatibility)
    pub const fn div_arm<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16]) {
        // Swap parameters and call regular div
        gpr.swap(0, 1);
        Self::div(bus, gpr);
    }

    /// SWI 0x08 - Sqrt
    /// Calculate integer square root
    pub fn sqrt<T: BusAccessor>(_bus: &mut T, gpr: &mut [Word; 16]) {
        let value = gpr[0];

        let result = if value == 0 {
            0
        } else {
            // Integer square root
            (value as f64).sqrt() as u32
        };

        gpr[0] = result;
    }

    /// SWI 0x09 - ArcTan
    /// Calculate arctangent
    pub fn arc_tan<T: BusAccessor>(_bus: &mut T, gpr: &mut [Word; 16]) {
        let tan_value = gpr[0] as i16; // 16-bit signed fixed point

        // Convert from 14-bit decimal part to floating point
        let tan_float = (tan_value as f64) / 16384.0;
        let atan_result = tan_float.atan();

        // Convert back to fixed point range C000h-4000h for -PI/2 to PI/2
        let result = (atan_result / std::f64::consts::FRAC_PI_2 * 16384.0) as i32;
        gpr[0] = result as u32;
    }

    /// SWI 0x0A - ArcTan2
    /// Calculate arctangent with quadrant correction
    pub fn arc_tan2<T: BusAccessor>(_bus: &mut T, gpr: &mut [Word; 16]) {
        let x = gpr[0] as i16;
        let y = gpr[1] as i16;

        let x_float = (x as f64) / 16384.0;
        let y_float = (y as f64) / 16384.0;
        let atan2_result = y_float.atan2(x_float);

        // Convert to 0-FFFFh range for 0-2PI
        let normalized = if atan2_result < 0.0 {
            2.0f64.mul_add(std::f64::consts::PI, atan2_result)
        } else {
            atan2_result
        };

        let result = (normalized / (2.0 * std::f64::consts::PI) * 65536.0) as u32;
        gpr[0] = result & 0xFFFF;
    }

    /// SWI 0x0B - CpuSet
    /// Memory copy/fill function
    pub fn cpu_set<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16]) {
        let source = gpr[0];
        let dest = gpr[1];
        let control = gpr[2];

        let count = control & 0x000F_FFFF;
        let fill_mode = (control & 0x0100_0000) != 0;
        let word_size = if (control & 0x0400_0000) != 0 { 4 } else { 2 };

        if fill_mode {
            // Fill mode - repeat first value
            let fill_value = if word_size == 4 {
                bus.read_word(source & !3)
            } else {
                bus.read_halfword(source & !1) as u32
            };

            for i in 0..count {
                let dst_addr = dest + i * word_size;
                if word_size == 4 {
                    bus.write_word(dst_addr & !3, fill_value);
                } else {
                    bus.write_halfword(dst_addr & !1, (fill_value & 0xFFFF) as u16);
                }
            }
        } else {
            // Copy mode
            for i in 0..count {
                let src_addr = source + i * word_size;
                let dst_addr = dest + i * word_size;

                if word_size == 4 {
                    let value = bus.read_word(src_addr & !3);
                    bus.write_word(dst_addr & !3, value);
                } else {
                    let value = bus.read_halfword(src_addr & !1);
                    bus.write_halfword(dst_addr & !1, value);
                }
            }
        }
    }

    /// SWI 0x0C - CpuFastSet
    /// Fast memory copy (32-bit aligned)
    pub fn cpu_fast_set<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16]) {
        let source = gpr[0] & !3; // Force word alignment
        let dest = gpr[1] & !3;   // Force word alignment
        let control = gpr[2];

        let count = control & 0x000F_FFFF;
        let count = ((count + 7) >> 3) << 3; // Rounded up to multiples of 8 words
        let fill_mode = (control & 0x0100_0000) != 0;

        if fill_mode {
            let fill_value = bus.read_word(source);
            for i in 0..count {
                let dst_addr = dest + i * 4;
                bus.write_word(dst_addr, fill_value);
            }
        } else {
            for i in 0..count {
                let src_addr = source + i * 4;
                let dst_addr = dest + i * 4;
                let value = bus.read_word(src_addr);
                bus.write_word(dst_addr, value);
            }
        }
    }

    /// SWI 0x0E - BgAffineSet
    /// Generates affine transform matrices for BG rotation/scaling.
    pub fn bg_affine_set<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16]) {
        let mut offset = gpr[0];
        let mut destination = gpr[1];
        let mut count = gpr[2];

        while count > 0 {
            let ox = (bus.read_word(offset) as i32 as f64) / 256.0;
            let oy = (bus.read_word(offset + 4) as i32 as f64) / 256.0;
            let cx = bus.read_halfword(offset + 8) as i16 as f64;
            let cy = bus.read_halfword(offset + 10) as i16 as f64;
            let sx = (bus.read_halfword(offset + 12) as i16 as f64) / 256.0;
            let sy = (bus.read_halfword(offset + 14) as i16 as f64) / 256.0;
            let theta_u8 = ((bus.read_halfword(offset + 16) >> 8) & 0xFF) as f64;
            let theta = theta_u8 / 128.0 * std::f64::consts::PI;
            offset += 20;

            let mut a = theta.cos();
            let mut b = theta.sin();
            let mut c = theta.sin();
            let mut d = theta.cos();

            a *= sx;
            b *= -sx;
            c *= sy;
            d *= sy;

            let rx = ox - (a * cx + b * cy);
            let ry = oy - (c * cx + d * cy);

            bus.write_halfword(destination, ((a * 256.0) as i32) as u16);
            bus.write_halfword(destination + 2, ((b * 256.0) as i32) as u16);
            bus.write_halfword(destination + 4, ((c * 256.0) as i32) as u16);
            bus.write_halfword(destination + 6, ((d * 256.0) as i32) as u16);
            bus.write_word(destination + 8, ((rx * 256.0) as i32) as u32);
            bus.write_word(destination + 12, ((ry * 256.0) as i32) as u32);

            destination += 16;
            count -= 1;
        }
    }

    /// SWI 0x0F - ObjAffineSet
    /// Generates affine matrices for OBJ entries.
    pub fn obj_affine_set<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16]) {
        let mut offset = gpr[0];
        let mut destination = gpr[1];
        let mut count = gpr[2];
        let diff = gpr[3];

        while count > 0 {
            let sx = (bus.read_halfword(offset) as i16 as f64) / 256.0;
            let sy = (bus.read_halfword(offset + 2) as i16 as f64) / 256.0;
            let theta_u8 = ((bus.read_halfword(offset + 4) >> 8) & 0xFF) as f64;
            let theta = theta_u8 / 128.0 * std::f64::consts::PI;
            offset += 6;

            let mut a = theta.cos();
            let mut b = theta.sin();
            let mut c = theta.sin();
            let mut d = theta.cos();

            a *= sx;
            b *= -sx;
            c *= sy;
            d *= sy;

            bus.write_halfword(destination, ((a * 256.0) as i32) as u16);
            bus.write_halfword(destination + diff, ((b * 256.0) as i32) as u16);
            bus.write_halfword(destination + diff * 2, ((c * 256.0) as i32) as u16);
            bus.write_halfword(destination + diff * 3, ((d * 256.0) as i32) as u16);
            destination += diff * 4;
            count -= 1;
        }
    }

    /// SWI 0x01 - RegisterRamReset
    /// Clears the RAM/IO regions selected by the bitmask in r0.
    pub fn register_ram_reset<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16]) {
        let flags = gpr[0];
        let clear = |bus: &mut T, start: Word, end: Word| {
            let mut a = start;
            while a < end {
                bus.write_word(a, 0);
                a += 4;
            }
        };
        if flags & 0x01 != 0 {
            clear(bus, 0x0200_0000, 0x0204_0000); // EWRAM
        }
        if flags & 0x02 != 0 {
            clear(bus, 0x0300_0000, 0x0300_7E00); // IWRAM (excluding BIOS/stack tail)
        }
        if flags & 0x04 != 0 {
            clear(bus, 0x0500_0000, 0x0500_0400); // Palette
        }
        if flags & 0x08 != 0 {
            clear(bus, 0x0600_0000, 0x0601_8000); // VRAM
        }
        if flags & 0x10 != 0 {
            clear(bus, 0x0700_0000, 0x0700_0400); // OAM
        }
        if flags & 0x80 != 0 {
            // Reset the LCD into forced blank, a sane default for the IO reset bit.
            bus.write_halfword(0x0400_0000, 0x0080);
        }
    }

    /// SWI 0x0D - GetBiosChecksum
    /// Returns the checksum of the GBA BIOS (a fixed constant on real hardware).
    pub const fn get_bios_checksum(gpr: &mut [Word; 16]) {
        gpr[0] = 0xBAAE_187F;
    }

    /// SWI 0x11/0x12 - LZ77UnComp (write 8-bit / 16-bit)
    /// Decompresses LZ77-compressed data from r0 to r1. The 16-bit variant is
    /// VRAM-safe (writes whole halfwords).
    pub fn lz77_uncomp<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16], vram: bool) {
        let src = gpr[0];
        let dst = gpr[1];
        let header = bus.read_word(src);
        let size = (header >> 8) as usize;
        let mut out: Vec<u8> = Vec::with_capacity(size);
        let mut sp = src + 4;
        while out.len() < size {
            let flags = bus.read_byte(sp);
            sp += 1;
            for bit in 0..8 {
                if out.len() >= size {
                    break;
                }
                if flags & (0x80 >> bit) != 0 {
                    let b0 = bus.read_byte(sp) as usize;
                    let b1 = bus.read_byte(sp + 1) as usize;
                    sp += 2;
                    let len = (b0 >> 4) + 3;
                    let disp = ((b0 & 0x0F) << 8 | b1) + 1;
                    for _ in 0..len {
                        if out.len() >= size || disp > out.len() {
                            break;
                        }
                        let idx = out.len() - disp;
                        out.push(out[idx]);
                    }
                } else {
                    out.push(bus.read_byte(sp));
                    sp += 1;
                }
            }
        }
        Self::write_decompressed(bus, dst, &out, vram);
    }

    /// SWI 0x14/0x15 - RLUnComp (write 8-bit / 16-bit)
    pub fn rl_uncomp<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16], vram: bool) {
        let src = gpr[0];
        let dst = gpr[1];
        let header = bus.read_word(src);
        let size = (header >> 8) as usize;
        let mut out: Vec<u8> = Vec::with_capacity(size);
        let mut sp = src + 4;
        while out.len() < size {
            let flag = bus.read_byte(sp);
            sp += 1;
            if flag & 0x80 != 0 {
                // Compressed run: (flag & 0x7F) + 3 copies of the next byte.
                let len = (flag & 0x7F) as usize + 3;
                let b = bus.read_byte(sp);
                sp += 1;
                for _ in 0..len {
                    if out.len() >= size {
                        break;
                    }
                    out.push(b);
                }
            } else {
                // Uncompressed run: (flag & 0x7F) + 1 literal bytes.
                let len = (flag & 0x7F) as usize + 1;
                for _ in 0..len {
                    if out.len() >= size {
                        break;
                    }
                    out.push(bus.read_byte(sp));
                    sp += 1;
                }
            }
        }
        Self::write_decompressed(bus, dst, &out, vram);
    }

    /// SWI 0x16/0x17/0x18 - Diff(8/16)bitUnFilter
    /// Reverses delta filtering: each element is the running sum of the deltas.
    /// `elem` is 1 (8-bit) or 2 (16-bit); `vram` selects 16-bit writes.
    pub fn diff_unfilter<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16], elem: usize, vram: bool) {
        let src = gpr[0];
        let dst = gpr[1];
        let header = bus.read_word(src);
        let size = (header >> 8) as usize; // size in bytes
        let mut out: Vec<u8> = Vec::with_capacity(size);
        let mut sp = src + 4;
        if elem == 1 {
            let mut acc: u8 = 0;
            while out.len() < size {
                acc = acc.wrapping_add(bus.read_byte(sp));
                sp += 1;
                out.push(acc);
            }
        } else {
            let mut acc: u16 = 0;
            while out.len() < size {
                let d = bus.read_byte(sp) as u16 | ((bus.read_byte(sp + 1) as u16) << 8);
                sp += 2;
                acc = acc.wrapping_add(d);
                out.push((acc & 0xFF) as u8);
                out.push((acc >> 8) as u8);
            }
        }
        Self::write_decompressed(bus, dst, &out, vram);
    }

    /// SWI 0x13 - HuffUnComp
    /// Decompresses Huffman-compressed data from r0 to r1 (always 32-bit writes).
    pub fn huff_uncomp<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16]) {
        let src = gpr[0];
        let dst = gpr[1];
        let header = bus.read_word(src);
        let data_bits = (header & 0x0F).max(1); // symbol size in bits (4 or 8)
        let size = (header >> 8) as usize; // decompressed size in bytes
        let tree_base = src + 4;
        let tree_size = (bus.read_byte(tree_base) as u32 + 1) * 2; // bytes
        let mut bitstream = tree_base + tree_size;

        let mut out: Vec<u8> = Vec::with_capacity(size);
        let mut cur_word: u32 = 0; // assembled output bits (LSB first)
        let mut cur_bits: u32 = 0;
        let mut word = bus.read_word(bitstream);
        bitstream += 4;
        let mut remaining = 32u32;
        // `node` is the address of the current tree node byte; root is at tree_base+1.
        let mut node = tree_base + 1;
        let mut guard = 0usize;
        while out.len() < size {
            guard += 1;
            if guard > size * 64 + 1024 {
                break; // safety net against malformed input
            }
            if remaining == 0 {
                word = bus.read_word(bitstream);
                bitstream += 4;
                remaining = 32;
            }
            let bit = (word >> 31) & 1;
            word <<= 1;
            remaining -= 1;

            let node_val = bus.read_byte(node);
            let offset = (node_val & 0x3F) as u32;
            let next_base = (node & !1).wrapping_add(offset * 2 + 2);
            let child = next_base + bit;
            let is_data = if bit == 0 { node_val & 0x80 != 0 } else { node_val & 0x40 != 0 };
            if is_data {
                let data = bus.read_byte(child) as u32 & ((1u32 << data_bits) - 1);
                cur_word |= data << cur_bits;
                cur_bits += data_bits;
                while cur_bits >= 8 {
                    out.push((cur_word & 0xFF) as u8);
                    cur_word >>= 8;
                    cur_bits -= 8;
                }
                node = tree_base + 1; // back to root
            } else {
                node = child;
            }
        }
        Self::write_decompressed(bus, dst, &out, false);
    }

    /// SWI 0x10 - BitUnPack
    /// Expands packed bit groups (1/2/4/8-bit) into wider units per the unpack
    /// info structure pointed to by r2.
    pub fn bit_unpack<T: BusAccessor>(bus: &mut T, gpr: &mut [Word; 16]) {
        let mut src = gpr[0];
        let mut dst = gpr[1];
        let info = gpr[2];
        let src_len = bus.read_halfword(info) as usize; // bytes
        let src_width = bus.read_byte(info + 2) as u32; // 1,2,4,8
        let dst_width = bus.read_byte(info + 3) as u32; // 1,2,4,8,16,32
        let param = bus.read_word(info + 4);
        let data_offset = param & 0x7FFF_FFFF;
        let zero_flag = (param & 0x8000_0000) != 0;

        if src_width == 0 || dst_width == 0 {
            return;
        }
        let mut out_word: u32 = 0;
        let mut out_bits: u32 = 0;
        let src_end = src + src_len as u32;
        let mask = (1u32 << src_width) - 1;
        while src < src_end {
            let byte = bus.read_byte(src) as u32;
            src += 1;
            let mut bitpos = 0u32;
            while bitpos < 8 {
                let val = (byte >> bitpos) & mask;
                let out_val = if val != 0 || zero_flag { val + data_offset } else { 0 };
                out_word |= (out_val & ((1u32 << dst_width) - 1)) << out_bits;
                out_bits += dst_width;
                if out_bits >= 32 {
                    bus.write_word(dst, out_word);
                    dst += 4;
                    out_word = 0;
                    out_bits = 0;
                }
                bitpos += src_width;
            }
        }
        if out_bits > 0 {
            bus.write_word(dst, out_word);
        }
    }

    /// Write a decompressed byte buffer to `dst`. When `vram` is set, writes are
    /// performed as 16-bit halfwords (VRAM/Palette/OAM are not byte-writable).
    fn write_decompressed<T: BusAccessor>(bus: &mut T, dst: Word, out: &[u8], vram: bool) {
        if vram {
            let mut i = 0;
            while i + 1 < out.len() {
                let hw = out[i] as u16 | ((out[i + 1] as u16) << 8);
                bus.write_halfword(dst + i as u32, hw);
                i += 2;
            }
            if i < out.len() {
                bus.write_halfword(dst + i as u32, out[i] as u16);
            }
        } else {
            for (i, b) in out.iter().enumerate() {
                bus.write_byte(dst + i as u32, *b);
            }
        }
    }

    /// Execute BIOS function based on SWI number
    pub fn execute_swi<T: BusAccessor>(
        bus: &mut T,
        swi_number: u32,
        gpr: &mut [Word; 16],
    ) {
        match swi_number {
            0x00 => Self::soft_reset(bus, gpr, 0),
            0x01 => Self::register_ram_reset(bus, gpr),
            0x02 => Self::halt(bus, gpr),
            0x04 => Self::intr_wait(bus, gpr),
            0x05 => Self::vblank_intr_wait(bus, gpr),
            0x06 => Self::div(bus, gpr),
            0x07 => Self::div_arm(bus, gpr),
            0x08 => Self::sqrt(bus, gpr),
            0x09 => Self::arc_tan(bus, gpr),
            0x0A => Self::arc_tan2(bus, gpr),
            0x0B => Self::cpu_set(bus, gpr),
            0x0C => Self::cpu_fast_set(bus, gpr),
            0x0D => Self::get_bios_checksum(gpr),
            0x0E => Self::bg_affine_set(bus, gpr),
            0x0F => Self::obj_affine_set(bus, gpr),
            0x10 => Self::bit_unpack(bus, gpr),
            0x11 => Self::lz77_uncomp(bus, gpr, false),
            0x12 => Self::lz77_uncomp(bus, gpr, true),
            0x13 => Self::huff_uncomp(bus, gpr),
            0x14 => Self::rl_uncomp(bus, gpr, false),
            0x15 => Self::rl_uncomp(bus, gpr, true),
            0x16 => Self::diff_unfilter(bus, gpr, 1, false),
            0x17 => Self::diff_unfilter(bus, gpr, 1, true),
            0x18 => Self::diff_unfilter(bus, gpr, 2, true),
            _ => {
                unimplemented!("BIOS: Unimplemented SWI 0x{:02X}", swi_number);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Bios;
    use crate::cpu::bus::accessor::BusAccessor;
    use crate::types::*;
    use std::collections::HashMap;

    // Mock bus for testing
    struct MockBus {
        memory: HashMap<Word, Word>,
    }

    impl MockBus {
        fn new() -> Self {
            Self {
                memory: HashMap::new(),
            }
        }

        fn write_32(&mut self, addr: Word, value: Word) {
            self.memory.insert(addr & !3, value);
        }
    }

    impl BusAccessor for MockBus {
        fn compute_cycle(&self, _addr: Word, _access_type: crate::types::AccessType) -> crate::types::Cycle {
            1 // Mock implementation
        }

        fn read_byte(&self, addr: Word) -> Byte {
            let word_addr = addr & !3;
            let byte_offset = addr & 3;
            let word = self.memory.get(&word_addr).unwrap_or(&0);
            ((word >> (byte_offset * 8)) & 0xFF) as Byte
        }

        fn read_halfword(&self, addr: Word) -> HalfWord {
            let word_addr = addr & !3;
            let is_upper = (addr & 2) != 0;
            let word = self.memory.get(&word_addr).unwrap_or(&0);
            if is_upper {
                (word >> 16) as HalfWord
            } else {
                (word & 0xFFFF) as HalfWord
            }
        }

        fn read_word(&self, addr: Word) -> Word {
            *self.memory.get(&(addr & !3)).unwrap_or(&0)
        }

        fn write_byte(&mut self, addr: Word, value: Byte) {
            let word_addr = addr & !3;
            let shift = (addr & 3) * 8;
            let cur = *self.memory.get(&word_addr).unwrap_or(&0);
            let new = (cur & !(0xFFu32 << shift)) | ((value as u32) << shift);
            self.memory.insert(word_addr, new);
        }

        fn write_halfword(&mut self, addr: Word, value: HalfWord) {
            let word_addr = addr & !3;
            let shift = (addr & 2) * 8;
            let cur = *self.memory.get(&word_addr).unwrap_or(&0);
            let new = (cur & !(0xFFFFu32 << shift)) | ((value as u32) << shift);
            self.memory.insert(word_addr, new);
        }

        fn write_word(&mut self, addr: Word, value: Word) {
            self.memory.insert(addr & !3, value);
        }
    }

    #[test]
    fn test_div_basic() {
        let mut bus = MockBus::new();
        let mut gpr = [0u32; 16];

        // Test: 100 / 10 = 10 remainder 0
        gpr[0] = 100;
        gpr[1] = 10;

        Bios::div(&mut bus, &mut gpr);

        assert_eq!(gpr[0], 10);  // quotient
        assert_eq!(gpr[1], 0);   // remainder
        assert_eq!(gpr[3], 10);  // absolute quotient
    }

    #[test]
    fn test_div_negative() {
        let mut bus = MockBus::new();
        let mut gpr = [0u32; 16];

        // Test: -123 / 10 = -12 remainder -3
        gpr[0] = (-123i32) as u32;
        gpr[1] = 10;

        Bios::div(&mut bus, &mut gpr);

        assert_eq!(gpr[0] as i32, -12); // quotient
        assert_eq!(gpr[1] as i32, -3);  // remainder
        assert_eq!(gpr[3], 12);         // absolute quotient
    }

    #[test]
    fn test_sqrt() {
        let mut bus = MockBus::new();
        let mut gpr = [0u32; 16];

        // Test sqrt(16) = 4
        gpr[0] = 16;
        Bios::sqrt(&mut bus, &mut gpr);
        assert_eq!(gpr[0], 4);

        // Test sqrt(0) = 0
        gpr[0] = 0;
        Bios::sqrt(&mut bus, &mut gpr);
        assert_eq!(gpr[0], 0);
    }

    fn put(bus: &mut MockBus, addr: Word, bytes: &[u8]) {
        for (i, b) in bytes.iter().enumerate() {
            bus.write_byte(addr + i as u32, *b);
        }
    }

    fn read_bytes(bus: &MockBus, addr: Word, n: usize) -> Vec<u8> {
        (0..n).map(|i| bus.read_byte(addr + i as u32)).collect()
    }

    #[test]
    fn test_get_bios_checksum() {
        let mut gpr = [0u32; 16];
        Bios::get_bios_checksum(&mut gpr);
        assert_eq!(gpr[0], 0xBAAE_187F);
    }

    #[test]
    fn test_lz77_uncomp() {
        let mut bus = MockBus::new();
        let src = 0x0200_0000;
        let dst = 0x0200_1000;
        // Header: size=4 (<<8) | type 1 (<<4) -> 0x00000410.
        // flag 0x40: bit0 literal 'A', bit1 compressed (disp=1,len=3) -> "AAAA".
        put(&mut bus, src, &[0x10, 0x04, 0x00, 0x00, 0x40, 0x41, 0x00, 0x00]);
        let mut gpr = [0u32; 16];
        gpr[0] = src;
        gpr[1] = dst;
        Bios::lz77_uncomp(&mut bus, &mut gpr, false);
        assert_eq!(read_bytes(&bus, dst, 4), vec![0x41, 0x41, 0x41, 0x41]);
    }

    #[test]
    fn test_rl_uncomp() {
        let mut bus = MockBus::new();
        let src = 0x0200_0000;
        let dst = 0x0200_1000;
        // size=4. flag 0x81 = compressed run of (1+3)=4 of the next byte 'A'.
        put(&mut bus, src, &[0x30, 0x04, 0x00, 0x00, 0x81, 0x41]);
        let mut gpr = [0u32; 16];
        gpr[0] = src;
        gpr[1] = dst;
        Bios::rl_uncomp(&mut bus, &mut gpr, false);
        assert_eq!(read_bytes(&bus, dst, 4), vec![0x41, 0x41, 0x41, 0x41]);
    }

    #[test]
    fn test_diff8_unfilter() {
        let mut bus = MockBus::new();
        let src = 0x0200_0000;
        let dst = 0x0200_1000;
        // size=4. deltas [0x10, 1, 1, 1] -> running sum [0x10, 0x11, 0x12, 0x13].
        put(&mut bus, src, &[0x10, 0x04, 0x00, 0x00, 0x10, 0x01, 0x01, 0x01]);
        let mut gpr = [0u32; 16];
        gpr[0] = src;
        gpr[1] = dst;
        Bios::diff_unfilter(&mut bus, &mut gpr, 1, false);
        assert_eq!(read_bytes(&bus, dst, 4), vec![0x10, 0x11, 0x12, 0x13]);
    }

    #[test]
    fn test_register_ram_reset_clears_ewram() {
        let mut bus = MockBus::new();
        bus.write_word(0x0200_0000, 0xDEAD_BEEF);
        bus.write_word(0x0203_FFFC, 0x1234_5678);
        let mut gpr = [0u32; 16];
        gpr[0] = 0x01; // reset EWRAM
        Bios::register_ram_reset(&mut bus, &mut gpr);
        assert_eq!(bus.read_word(0x0200_0000), 0);
        assert_eq!(bus.read_word(0x0203_FFFC), 0);
    }
}
