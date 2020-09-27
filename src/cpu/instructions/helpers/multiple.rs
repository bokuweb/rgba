use crate::types::*;

pub fn compute_multiple_cycle(rs: Word) -> Cycle {
    let masked = rs & 0xffff_ff00;
    if ((masked) == 0xffff_ff00) || (masked) == 0 {
        return 1;
    }

    let masked = rs & 0xffff_0000;
    if ((masked) == 0xffff0000) || (masked) == 0 {
        return 2;
    }

    let masked = rs & 0xff00_0000;
    if ((masked) == 0xff000000) || (masked) == 0 {
        return 3;
    }
    4
}
