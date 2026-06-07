//! GamePak GPIO port and the Seiko S-3511A real-time clock.
//!
//! Cartridges such as Pokémon Ruby/Sapphire/Emerald and Boktai wire an RTC chip
//! to four general-purpose I/O pins exposed at the start of the ROM region:
//!
//! | Address      | Register        |
//! |--------------|-----------------|
//! | `0x080000C4` | GPIO data (4 b) |
//! | `0x080000C6` | GPIO direction  |
//! | `0x080000C8` | GPIO control    |
//!
//! The RTC uses three of those pins — SCK (clock, bit 0), SIO (data, bit 1) and
//! CS (chip select, bit 2) — and speaks a simple bit-serial protocol: a command
//! byte (MSB first, fixed `0110` prefix + 3-bit command + R/W bit) optionally
//! followed by BCD data bytes (LSB first). This module models the GPIO latch and
//! the chip's command state machine, reading the host clock for date/time.

const GPIO_DATA: u32 = 0x0800_00C4;
const GPIO_DIRECTION: u32 = 0x0800_00C6;
const GPIO_CONTROL: u32 = 0x0800_00C8;

const PIN_SCK: u8 = 1 << 0;
const PIN_SIO: u8 = 1 << 1;
const PIN_CS: u8 = 1 << 2;

/// RTC commands (the 3-bit field of the command byte).
const CMD_RESET: u8 = 0;
const CMD_CONTROL: u8 = 4;
const CMD_DATETIME: u8 = 2;
const CMD_TIME: u8 = 6;

/// Phase of the bit-serial exchange.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// CS is low; nothing in progress.
    Idle,
    /// Shifting in the 8-bit command byte (MSB first).
    Command,
    /// Streaming data bytes out to the host (LSB first).
    Reading,
    /// Shifting data bytes in from the host (LSB first).
    Writing,
}

/// A plain calendar date/time, BCD-encoded on demand.
#[derive(Clone, Copy)]
struct DateTime {
    year: u8,    // 0..99 (two-digit)
    month: u8,   // 1..12
    day: u8,     // 1..31
    weekday: u8, // 0..6
    hour: u8,    // 0..23
    minute: u8,  // 0..59
    second: u8,  // 0..59
}

/// GamePak GPIO + S-3511A RTC.
pub struct Rtc {
    /// CPU-written values of the GPIO data pins (bits 0..3).
    data_out: u8,
    /// Pin directions: 1 = CPU output, 0 = input (chip-driven).
    direction: u8,
    /// GPIO control: bit 0 = registers readable.
    read_enable: bool,

    // Serial chip state.
    phase: Phase,
    prev_sck: bool,
    prev_cs: bool,
    /// SIO level the chip currently drives (read back by the CPU).
    sio_out: bool,
    /// Bit accumulator for the byte currently being shifted.
    shift: u8,
    /// Number of bits shifted in/out of the current byte.
    bit_count: u8,
    /// Decoded command field of the active transaction.
    command: u8,
    /// Index of the data byte being transferred.
    byte_index: u8,
    /// Data bytes for the active read/write (date/time is 7, time is 3).
    buffer: [u8; 7],
    /// Number of data bytes the active command transfers.
    byte_total: u8,
    /// RTC control register (bit 6 = 24-hour mode).
    control_reg: u8,

    /// Fixed Unix time for deterministic tests; `None` uses the host clock.
    test_unix: Option<i64>,
}

impl Rtc {
    #[must_use]
    pub fn new() -> Self {
        Self {
            data_out: 0,
            direction: 0,
            read_enable: false,
            phase: Phase::Idle,
            prev_sck: false,
            prev_cs: false,
            sio_out: false,
            shift: 0,
            bit_count: 0,
            command: 0,
            byte_index: 0,
            buffer: [0; 7],
            byte_total: 0,
            control_reg: 0x40, // power-on default: 24-hour mode
            test_unix: None,
        }
    }

    /// Whether a half-word read of the GPIO region should return GPIO state
    /// (rather than falling through to ROM).
    #[must_use]
    pub const fn read_enabled(&self) -> bool {
        self.read_enable
    }

    /// Read one of the three GPIO registers.
    #[must_use]
    pub fn read(&self, addr: u32) -> u16 {
        match addr {
            GPIO_DATA => {
                // Output pins read back what the CPU wrote; input pins read the
                // level the chip drives (only SIO is ever chip-driven).
                let chip = if self.sio_out { PIN_SIO } else { 0 };
                let value = (self.data_out & self.direction) | (chip & !self.direction);
                u16::from(value & 0x0F)
            }
            GPIO_DIRECTION => u16::from(self.direction),
            GPIO_CONTROL => u16::from(u8::from(self.read_enable)),
            _ => 0,
        }
    }

    /// Write one of the three GPIO registers.
    pub fn write(&mut self, addr: u32, value: u16) {
        let byte = (value & 0xFF) as u8;
        match addr {
            GPIO_DATA => {
                self.data_out = byte & 0x0F;
                self.update_pins();
            }
            GPIO_DIRECTION => self.direction = byte & 0x0F,
            GPIO_CONTROL => self.read_enable = byte & 1 != 0,
            _ => {}
        }
    }

    /// Re-evaluate the chip from the current CPU-driven pin levels.
    fn update_pins(&mut self) {
        // Only consider pins the CPU currently drives as outputs.
        let driven = self.data_out & self.direction;
        let sck = driven & PIN_SCK != 0;
        let cs = driven & PIN_CS != 0;
        let sio = driven & PIN_SIO != 0;

        if !self.prev_cs && cs {
            // CS rising: begin a transaction.
            self.phase = Phase::Command;
            self.shift = 0;
            self.bit_count = 0;
        } else if self.prev_cs && !cs {
            self.phase = Phase::Idle;
        } else if cs && !self.prev_sck && sck {
            // Rising clock edge while selected: transfer one bit.
            self.clock_bit(sio);
        }

        self.prev_cs = cs;
        self.prev_sck = sck;
    }

    fn clock_bit(&mut self, sio_in: bool) {
        match self.phase {
            Phase::Command => {
                // Command byte arrives MSB first.
                self.shift = (self.shift << 1) | u8::from(sio_in);
                self.bit_count += 1;
                if self.bit_count == 8 {
                    self.decode_command();
                }
            }
            Phase::Writing => {
                // Data bytes arrive LSB first.
                self.shift = (self.shift >> 1) | (u8::from(sio_in) << 7);
                self.bit_count += 1;
                if self.bit_count == 8 {
                    self.store_write_byte();
                }
            }
            Phase::Reading => {
                // Present the next bit (LSB first), then advance.
                let bit = (self.buffer[self.byte_index as usize] >> self.bit_count) & 1;
                self.sio_out = bit != 0;
                self.bit_count += 1;
                if self.bit_count == 8 {
                    self.bit_count = 0;
                    self.byte_index += 1;
                    if self.byte_index >= self.byte_total {
                        self.phase = Phase::Idle;
                    }
                }
            }
            Phase::Idle => {}
        }
    }

    fn decode_command(&mut self) {
        // Accept either bit order of the fixed `0110` prefix.
        let byte = if self.shift & 0x0F == 0x06 {
            // Prefix in the low nibble: reverse to canonical form.
            self.shift.reverse_bits()
        } else {
            self.shift
        };
        self.command = (byte >> 1) & 0x07;
        let is_read = byte & 1 != 0;
        self.bit_count = 0;
        self.byte_index = 0;
        self.shift = 0;

        match self.command {
            CMD_RESET => {
                self.control_reg = 0x40;
                self.phase = Phase::Idle;
            }
            CMD_CONTROL => {
                self.byte_total = 1;
                if is_read {
                    self.buffer[0] = self.control_reg;
                    self.phase = Phase::Reading;
                } else {
                    self.phase = Phase::Writing;
                }
            }
            CMD_DATETIME | CMD_TIME => {
                self.byte_total = if self.command == CMD_DATETIME { 7 } else { 3 };
                if is_read {
                    self.load_datetime();
                    self.phase = Phase::Reading;
                } else {
                    // Setting the clock is accepted but ignored (host time wins).
                    self.phase = Phase::Writing;
                }
            }
            _ => self.phase = Phase::Idle,
        }
    }

    fn store_write_byte(&mut self) {
        if (self.byte_index as usize) < self.buffer.len() {
            self.buffer[self.byte_index as usize] = self.shift;
        }
        if self.command == CMD_CONTROL {
            self.control_reg = self.shift;
        }
        self.shift = 0;
        self.bit_count = 0;
        self.byte_index += 1;
        if self.byte_index >= self.byte_total {
            self.phase = Phase::Idle;
        }
    }

    /// Fill `buffer` with the current date/time in BCD for the active command.
    fn load_datetime(&mut self) {
        let dt = self.current_datetime();
        let hour_byte = if self.control_reg & 0x40 == 0 {
            // 12-hour mode: BCD hour 1..12 with the PM flag in bit 7.
            let h = dt.hour % 12;
            let h = if h == 0 { 12 } else { h };
            to_bcd(h) | if dt.hour >= 12 { 0x80 } else { 0 }
        } else {
            to_bcd(dt.hour) // 24-hour mode (the power-on default)
        };
        if self.command == CMD_DATETIME {
            self.buffer = [
                to_bcd(dt.year),
                to_bcd(dt.month),
                to_bcd(dt.day),
                dt.weekday,
                hour_byte,
                to_bcd(dt.minute),
                to_bcd(dt.second),
            ];
        } else {
            self.buffer = [hour_byte, to_bcd(dt.minute), to_bcd(dt.second), 0, 0, 0, 0];
        }
    }

    fn current_datetime(&self) -> DateTime {
        let secs = self.test_unix.unwrap_or_else(host_unix_seconds);
        civil_from_unix(secs)
    }
}

impl Default for Rtc {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a binary value (0..99) to packed BCD.
const fn to_bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
}

/// Seconds since the Unix epoch from the host clock (0 if the clock is before
/// the epoch, which never happens in practice).
fn host_unix_seconds() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// Break a Unix timestamp into a local-naïve calendar date/time (UTC).
fn civil_from_unix(secs: i64) -> DateTime {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let hour = (rem / 3_600) as u8;
    let minute = ((rem % 3_600) / 60) as u8;
    let second = (rem % 60) as u8;
    // Weekday: 1970-01-01 was a Thursday (4).
    let weekday = (days.rem_euclid(7) + 4).rem_euclid(7) as u8;

    // Howard Hinnant's civil-from-days algorithm.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as u8; // [1, 31]
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8; // [1, 12]
    let full_year = year + i64::from(month <= 2);

    DateTime {
        year: (full_year.rem_euclid(100)) as u8,
        month,
        day,
        weekday,
        hour,
        minute,
        second,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive a full read transaction over the GPIO pins for `command`, returning
    /// the `count` data bytes the chip streams back.
    fn read_command(rtc: &mut Rtc, command: u8, count: usize) -> Vec<u8> {
        // Direction: SCK, SIO, CS are CPU outputs while sending the command.
        rtc.write(GPIO_DIRECTION, u16::from(PIN_SCK | PIN_SIO | PIN_CS));
        // CS high to begin.
        set_pins(rtc, false, false, true);

        // Command byte: 0110 prefix, 3-bit command, R/W=1 (read), MSB first.
        let byte = 0b0110_0000 | (command << 1) | 1;
        for i in (0..8).rev() {
            let bit = (byte >> i) & 1 != 0;
            set_pins(rtc, false, bit, true); // set SIO with clock low
            set_pins(rtc, true, bit, true); // rising edge samples it
        }

        // Switch SIO to input so the chip drives it for the read.
        rtc.write(GPIO_DIRECTION, u16::from(PIN_SCK | PIN_CS));
        rtc.write(GPIO_CONTROL, 1); // enable register reads

        let mut out = Vec::new();
        for _ in 0..count {
            let mut byte = 0u8;
            for bit in 0..8 {
                set_pins(rtc, false, false, true); // SCK low
                set_pins(rtc, true, false, true); // rising edge presents the bit
                let level = rtc.read(GPIO_DATA) & u16::from(PIN_SIO) != 0;
                byte |= u8::from(level) << bit; // LSB first
            }
            out.push(byte);
        }
        out
    }

    fn set_pins(rtc: &mut Rtc, sck: bool, sio: bool, cs: bool) {
        let mut v = 0u16;
        if sck {
            v |= u16::from(PIN_SCK);
        }
        if sio {
            v |= u16::from(PIN_SIO);
        }
        if cs {
            v |= u16::from(PIN_CS);
        }
        rtc.write(GPIO_DATA, v);
    }

    #[test]
    fn datetime_read_returns_expected_bcd() {
        let mut rtc = Rtc::new();
        // 2023-08-15 (Tuesday) 13:37:42 UTC.
        // days from 1970-01-01 to 2023-08-15 = 19584.
        rtc.test_unix = Some(19_584 * 86_400 + 13 * 3_600 + 37 * 60 + 42);
        let out = read_command(&mut rtc, CMD_DATETIME, 7);
        assert_eq!(out[0], 0x23, "year");
        assert_eq!(out[1], 0x08, "month");
        assert_eq!(out[2], 0x15, "day");
        assert_eq!(out[4], 0x13, "hour (24h)");
        assert_eq!(out[5], 0x37, "minute");
        assert_eq!(out[6], 0x42, "second");
    }

    #[test]
    fn time_read_returns_hms() {
        let mut rtc = Rtc::new();
        rtc.test_unix = Some(9 * 3_600 + 5 * 60 + 1); // 09:05:01 on 1970-01-01
        let out = read_command(&mut rtc, CMD_TIME, 3);
        assert_eq!(out, vec![0x09, 0x05, 0x01]);
    }

    #[test]
    fn reads_disabled_until_control_set() {
        let rtc = Rtc::new();
        assert!(!rtc.read_enabled());
    }

    #[test]
    fn bcd_conversion() {
        assert_eq!(to_bcd(0), 0x00);
        assert_eq!(to_bcd(42), 0x42);
        assert_eq!(to_bcd(99), 0x99);
    }

    #[test]
    fn civil_from_unix_epoch_is_thursday() {
        let dt = civil_from_unix(0);
        assert_eq!(dt.year, 70);
        assert_eq!(dt.month, 1);
        assert_eq!(dt.day, 1);
        assert_eq!(dt.weekday, 4); // Thursday
    }
}
