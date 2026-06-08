//! A tiny, dependency-free replacement for the `bitfield` crate's `bitfield!`
//! macro, covering exactly the subset this codebase uses:
//!
//! ```ignore
//! bitfield! {
//!     #[derive(Debug, Clone, Copy)]
//!     pub struct Reg(u16);
//!     pub getter, setter: msb, lsb;  // range  -> reads/writes the base int type
//!     pub getter, _:      msb, lsb;  // range, read-only
//!     pub getter, setter: bit;       // single bit -> reads/writes `bool`
//!     pub getter, _:      bit;       // single bit, read-only
//! }
//! ```
//!
//! Field entries may carry their own attributes (e.g. `#[allow(non_snake_case)]`)
//! and an optional visibility. The wrapped value is a `pub` tuple field, matching
//! the original crate so existing `Reg(raw)` construction keeps working across
//! modules. Generated accessors get `#[allow(clippy::identity_op)]` because
//! `<< 0` / `>> 0` for lsb-0 fields is expected and harmless.


macro_rules! bitfield {
    // Entry point: struct definition followed by field declarations.
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident($t:ty);
        $($fields:tt)*
    ) => {
        $(#[$meta])*
        $vis struct $name(pub $t);

        impl $name {
            bitfield!(@fields $t; $($fields)*);
        }
    };

    // No more fields.
    (@fields $t:ty;) => {};

    // Range field with a setter: `vis getter, setter: msb, lsb;`
    (@fields $t:ty;
        $(#[$fmeta:meta])*
        $fvis:vis $getter:ident, $setter:ident: $msb:literal, $lsb:literal;
        $($rest:tt)*
    ) => {
        $(#[$fmeta])*
        #[allow(clippy::identity_op)]
        $fvis fn $getter(&self) -> $t {
            (self.0 >> $lsb) & (<$t>::MAX >> (<$t>::BITS - ($msb - $lsb + 1)))
        }
        $(#[$fmeta])*
        #[allow(clippy::identity_op)]
        $fvis fn $setter(&mut self, value: $t) {
            let mask: $t = (<$t>::MAX >> (<$t>::BITS - ($msb - $lsb + 1))) << $lsb;
            self.0 = (self.0 & !mask) | ((value << $lsb) & mask);
        }
        bitfield!(@fields $t; $($rest)*);
    };

    // Range field, read-only: `vis getter, _: msb, lsb;`
    (@fields $t:ty;
        $(#[$fmeta:meta])*
        $fvis:vis $getter:ident, _: $msb:literal, $lsb:literal;
        $($rest:tt)*
    ) => {
        $(#[$fmeta])*
        #[allow(clippy::identity_op)]
        $fvis fn $getter(&self) -> $t {
            (self.0 >> $lsb) & (<$t>::MAX >> (<$t>::BITS - ($msb - $lsb + 1)))
        }
        bitfield!(@fields $t; $($rest)*);
    };

    // Single-bit field with a setter: `vis getter, setter: bit;`
    (@fields $t:ty;
        $(#[$fmeta:meta])*
        $fvis:vis $getter:ident, $setter:ident: $bit:literal;
        $($rest:tt)*
    ) => {
        $(#[$fmeta])*
        #[allow(clippy::identity_op)]
        $fvis fn $getter(&self) -> bool {
            (self.0 & (1 << $bit)) != 0
        }
        $(#[$fmeta])*
        #[allow(clippy::identity_op)]
        $fvis fn $setter(&mut self, value: bool) {
            if value {
                self.0 |= 1 << $bit;
            } else {
                self.0 &= !(1 << $bit);
            }
        }
        bitfield!(@fields $t; $($rest)*);
    };

    // Single-bit field, read-only: `vis getter, _: bit;`
    (@fields $t:ty;
        $(#[$fmeta:meta])*
        $fvis:vis $getter:ident, _: $bit:literal;
        $($rest:tt)*
    ) => {
        $(#[$fmeta])*
        #[allow(clippy::identity_op)]
        $fvis fn $getter(&self) -> bool {
            (self.0 & (1 << $bit)) != 0
        }
        bitfield!(@fields $t; $($rest)*);
    };
}
