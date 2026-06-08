//! A small two-pass ARM assembler — the `asm -> bin` direction.
//!
//! This is intentionally a *useful subset*, not a full GNU `as` clone. It
//! covers the instructions you actually hand-write when poking at a debugger:
//! data-processing, branches, `bx`, word/byte loads & stores, `swi`, `nop`, the
//! `.word` directive, and labels (resolved with a real two-pass pass so forward
//! branches work).
//!
//! The hard parts of "is an assembler difficult?" live here in miniature:
//!   * the immediate encoder ([`encode_imm12`]) — ARM can only express an 8-bit
//!     value rotated by an even amount, so not every constant is legal;
//!   * label resolution — pass 1 assigns an address to every line, pass 2
//!     encodes now that every label's address is known.
//!
//! For the full instruction set (THUMB, coprocessor, NEON, all addressing
//! modes) you'd reach for Keystone; this keeps the demo dependency-free.

/// Assemble `source` as ARM code that will live at `base` in memory.
/// Returns the encoded little-endian bytes, or a human-readable error.
pub fn assemble(source: &str, base: u32) -> Result<Vec<u8>, String> {
    // ---- Pass 1: collect labels and the instruction list -----------------
    let mut labels = std::collections::HashMap::new();
    let mut lines: Vec<(u32, String)> = Vec::new();
    let mut addr = base;
    for (lineno, raw) in source.lines().enumerate() {
        let mut line = strip_comment(raw).trim().to_string();
        // Leading "label:" (possibly multiple, possibly followed by an instr).
        while let Some(colon) = label_prefix(&line) {
            let name = line[..colon].trim().to_string();
            if labels.insert(name.clone(), addr).is_some() {
                return Err(format!("line {}: duplicate label '{name}'", lineno + 1));
            }
            line = line[colon + 1..].trim().to_string();
        }
        if line.is_empty() {
            continue;
        }
        lines.push((addr, line));
        addr = addr.wrapping_add(4); // every supported line is one 4-byte word
    }

    // ---- Pass 2: encode --------------------------------------------------
    let mut out = Vec::with_capacity(lines.len() * 4);
    for (i, (at, line)) in lines.iter().enumerate() {
        let word = encode_line(line, *at, &labels)
            .map_err(|e| format!("'{line}': {e}"))?;
        let _ = i;
        out.extend_from_slice(&word.to_le_bytes());
    }
    Ok(out)
}

fn strip_comment(s: &str) -> &str {
    for marker in [";", "//", "@"] {
        if let Some(idx) = s.find(marker) {
            return &s[..idx];
        }
    }
    s
}

/// If `line` starts with `label:`, return the byte index of the colon.
fn label_prefix(line: &str) -> Option<usize> {
    let colon = line.find(':')?;
    let name = line[..colon].trim();
    if !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        && name.chars().next().is_some_and(|c| !c.is_ascii_digit())
    {
        Some(colon)
    } else {
        None
    }
}

const COND: [(&str, u32); 15] = [
    ("eq", 0), ("ne", 1), ("cs", 2), ("cc", 3), ("mi", 4), ("pl", 5), ("vs", 6), ("vc", 7),
    ("hi", 8), ("ls", 9), ("ge", 10), ("lt", 11), ("gt", 12), ("le", 13), ("al", 14),
];

/// Split a mnemonic into (base, cond, s-flag). e.g. `addnes` -> ("add", ne, true).
fn split_mnemonic<'b>(m: &str, bases: &'b [&'b str]) -> Option<(&'b str, u32, bool)> {
    for base in bases {
        if let Some(rest) = m.strip_prefix(*base) {
            // optional condition, then optional 's'
            let (cond_bits, rest) = match COND.iter().find(|(c, _)| rest.starts_with(c)) {
                Some((c, b)) => (*b, &rest[c.len()..]),
                None => (14, rest),
            };
            let (s, rest) = if let Some(r) = rest.strip_prefix('s') { (true, r) } else { (false, rest) };
            if rest.is_empty() {
                return Some((*base, cond_bits, s));
            }
        }
    }
    None
}

fn cond_only(m: &str, base: &str) -> Option<u32> {
    let rest = m.strip_prefix(base)?;
    if rest.is_empty() {
        return Some(14);
    }
    COND.iter().find(|(c, _)| *c == rest).map(|(_, b)| *b)
}

fn reg(tok: &str) -> Result<u32, String> {
    let t = tok.trim().to_ascii_lowercase();
    match t.as_str() {
        "sp" => Ok(13),
        "lr" => Ok(14),
        "pc" => Ok(15),
        _ => t
            .strip_prefix('r')
            .and_then(|n| n.parse::<u32>().ok())
            .filter(|n| *n < 16)
            .ok_or_else(|| format!("bad register '{tok}'")),
    }
}

/// Parse `#123`, `#0x1f`, `#-4`, or a bare number.
fn imm(tok: &str) -> Result<i64, String> {
    let t = tok.trim().trim_start_matches('#').trim();
    let (neg, t) = t.strip_prefix('-').map_or((false, t), |r| (true, r));
    let v = if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16)
    } else {
        t.parse::<i64>()
    }
    .map_err(|_| format!("bad immediate '{tok}'"))?;
    Ok(if neg { -v } else { v })
}

/// Encode a 32-bit value as ARM's rotated 8-bit immediate, if possible.
pub fn encode_imm12(value: u32) -> Option<u32> {
    for rot in 0..16u32 {
        let rotated = value.rotate_left(rot * 2);
        if rotated <= 0xFF {
            return Some((rot << 8) | rotated);
        }
    }
    None
}

const DP_OPS: [(&str, u32); 16] = [
    ("and", 0), ("eor", 1), ("sub", 2), ("rsb", 3), ("add", 4), ("adc", 5), ("sbc", 6), ("rsc", 7),
    ("tst", 8), ("teq", 9), ("cmp", 10), ("cmn", 11), ("orr", 12), ("mov", 13), ("bic", 14),
    ("mvn", 15),
];

fn split_operands(rest: &str) -> Vec<String> {
    rest.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

fn encode_line(
    line: &str,
    at: u32,
    labels: &std::collections::HashMap<String, u32>,
) -> Result<u32, String> {
    let mut it = line.splitn(2, char::is_whitespace);
    let mnem = it.next().unwrap_or("").to_ascii_lowercase();
    let rest = it.next().unwrap_or("").trim();

    // .word <imm|label>
    if mnem == ".word" {
        return match labels.get(rest) {
            Some(&a) => Ok(a),
            None => Ok(imm(rest)? as u32),
        };
    }
    if mnem == "nop" {
        return Ok(0xE1A0_0000);
    }

    // Data processing
    let dp_bases: Vec<&str> = DP_OPS.iter().map(|(n, _)| *n).collect();
    if let Some((base, cond, s)) = split_mnemonic(&mnem, &dp_bases) {
        let opcode = DP_OPS.iter().find(|(n, _)| *n == base).unwrap().1;
        let ops = split_operands(rest);
        let is_cmp = matches!(base, "cmp" | "cmn" | "tst" | "teq");
        let is_mov = matches!(base, "mov" | "mvn");

        let (rd, rn, op2): (u32, u32, &str);
        if is_cmp {
            // cmp Rn, op2
            if ops.len() != 2 {
                return Err("expected 2 operands".into());
            }
            rd = 0;
            rn = reg(&ops[0])?;
            op2 = &ops[1];
        } else if is_mov {
            // mov Rd, op2
            if ops.len() != 2 {
                return Err("expected 2 operands".into());
            }
            rd = reg(&ops[0])?;
            rn = 0;
            op2 = &ops[1];
        } else {
            // op Rd, Rn, op2
            if ops.len() != 3 {
                return Err("expected 3 operands".into());
            }
            rd = reg(&ops[0])?;
            rn = reg(&ops[1])?;
            op2 = &ops[2];
        }

        let s_bit = u32::from(s || is_cmp); // cmp-family always set flags
        let (i_bit, operand2) = if op2.trim_start().starts_with('#') {
            let v = imm(op2)? as u32;
            let enc = encode_imm12(v).ok_or_else(|| format!("immediate 0x{v:x} not encodable"))?;
            (1, enc)
        } else {
            (0, reg(op2)?) // register operand, no shift
        };

        return Ok((cond << 28)
            | (i_bit << 25)
            | (opcode << 21)
            | (s_bit << 20)
            | (rn << 16)
            | (rd << 12)
            | operand2);
    }

    // Branch: b / bl
    if let Some(cond) = cond_only(&mnem, "bl").or_else(|| cond_only(&mnem, "b")) {
        let l = u32::from(mnem.starts_with("bl"));
        let target = resolve_target(rest, labels)?;
        let off = (target as i64) - (at as i64) - 8;
        if off % 4 != 0 {
            return Err("branch target not word-aligned".into());
        }
        let off24 = ((off >> 2) as u32) & 0x00FF_FFFF;
        return Ok((cond << 28) | (0b101 << 25) | (l << 24) | off24);
    }

    // bx Rm
    if let Some(cond) = cond_only(&mnem, "bx") {
        let rm = reg(rest)?;
        return Ok((cond << 28) | 0x012F_FF10 | rm);
    }

    // swi / svc
    if let Some(cond) = cond_only(&mnem, "swi").or_else(|| cond_only(&mnem, "svc")) {
        let comment = (imm(rest)? as u32) & 0x00FF_FFFF;
        return Ok((cond << 28) | 0x0F00_0000 | comment);
    }

    // Loads / stores: ldr str ldrb strb
    for (base, l, b) in [("ldrb", 1u32, 1u32), ("strb", 0, 1), ("ldr", 1, 0), ("str", 0, 0)] {
        if let Some(cond) = cond_only(&mnem, base) {
            return encode_mem(rest, cond, l, b);
        }
    }

    Err(format!("unsupported instruction '{mnem}'"))
}

fn resolve_target(tok: &str, labels: &std::collections::HashMap<String, u32>) -> Result<u32, String> {
    let t = tok.trim();
    if let Some(&a) = labels.get(t) {
        Ok(a)
    } else {
        Ok(imm(t)? as u32)
    }
}

/// Encode `Rd, [Rn]` / `[Rn, #imm]` / `[Rn, #imm]!` / `[Rn], #imm`.
fn encode_mem(rest: &str, cond: u32, l: u32, b: u32) -> Result<u32, String> {
    let comma = rest.find(',').ok_or("expected 'Rd, [Rn...]'")?;
    let rd = reg(&rest[..comma])?;
    let mem = rest[comma + 1..].trim();

    let open = mem.find('[').ok_or("expected '['")?;
    let close = mem.find(']').ok_or("expected ']'")?;
    let inside = &mem[open + 1..close];
    let after = mem[close + 1..].trim(); // "" | "!" | ", #imm" (post-index)
    let writeback = after == "!";
    let post = after.starts_with(',');

    let parts = split_operands(inside);
    let rn = reg(&parts[0])?;

    let (p, mut u, offset) = if post {
        // [Rn], #imm
        let v = imm(after.trim_start_matches(','))?;
        (0u32, 1u32, v)
    } else if parts.len() == 2 {
        // [Rn, #imm]
        let v = imm(&parts[1])?;
        (1, 1, v)
    } else {
        (1, 1, 0)
    };

    let mut off = offset;
    if off < 0 {
        u = 0;
        off = -off;
    }
    if off > 0xFFF {
        return Err("offset out of range (12-bit)".into());
    }
    let w = u32::from(writeback);
    Ok((cond << 28)
        | (0b01 << 26)
        | (p << 24)
        | (u << 23)
        | (b << 22)
        | (w << 21)
        | (l << 20)
        | (rn << 16)
        | (rd << 12)
        | (off as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(src: &str) -> u32 {
        let bytes = assemble(src, 0).expect("assemble");
        assert_eq!(bytes.len(), 4, "expected single word");
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    #[test]
    fn data_processing() {
        assert_eq!(one("mov r0, #1"), 0xE3A0_0001);
        assert_eq!(one("add r1, r1, #1"), 0xE281_1001);
        assert_eq!(one("cmp r0, #0"), 0xE350_0000);
        assert_eq!(one("sub r2, r3, r4"), 0xE043_2004);
        assert_eq!(one("nop"), 0xE1A0_0000);
        assert_eq!(one("movs r0, #1"), 0xE3B0_0001);
    }

    #[test]
    fn branches_and_misc() {
        assert_eq!(one("bx lr"), 0xE12F_FF1E);
        assert_eq!(one("swi #0"), 0xEF00_0000);
        // Branch to self at addr 0: target 0, off = 0-0-8 = -8 -> -2 words.
        assert_eq!(one("b 0"), 0xEAFF_FFFE);
    }

    #[test]
    fn memory() {
        assert_eq!(one("ldr r0, [r1, #4]"), 0xE591_0004);
        assert_eq!(one("str r0, [r1]"), 0xE581_0000);
        assert_eq!(one("ldr r0, [r1, #-4]"), 0xE511_0004);
        assert_eq!(one("ldrb r2, [r3, #8]"), 0xE5D3_2008);
    }

    #[test]
    fn labels_forward_and_back() {
        // loop forever: target == its own address.
        let bytes = assemble("start:\n  b start", 0x0800_0000).unwrap();
        let w = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(w, 0xEAFF_FFFE);

        // forward branch over one instruction
        let src = "  b skip\n  nop\nskip:\n  nop";
        let b = assemble(src, 0).unwrap();
        let first = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        // skip is at 0x8, branch at 0x0: off = 8-0-8 = 0 -> off24 = 0
        assert_eq!(first, 0xEA00_0000);
    }

    #[test]
    fn imm_encoding_limits() {
        assert!(encode_imm12(0xFF).is_some());
        assert!(encode_imm12(0x104).is_some()); // 0x41 rotated — IS expressible
        assert!(encode_imm12(0x101).is_none()); // bits 0 and 8 span 9 bits — not expressible
    }
}
