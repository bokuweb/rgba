#![no_std]
#![feature(start)]

mod font;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

static BG_PALETTE: usize = 0x0500_0000;
static VRAM: usize = 0x0600_0000;

static LCD_CTRL: usize = 0x0400_0000;
static LCD_BG0: usize = 0x0400_0008;
static KEY_STATUS: usize = 0x0400_0130;

static LCD_SIZE3232: u16 = 0x0000;
static LCD_COLOR256: u16 = 0x0080;
static LCD_MODE0: u16 = 0x0000;
static LCD_BG0EN: u16 = 0x0100;

macro_rules! bgr {
    ( $r:expr, $g:expr, $b:expr ) => {
        ($b << 10) + ($g << 5) + $r
    };
}

macro_rules! vram_tile {
    ( $n:expr ) => {
        VRAM + $n * 0x4000
    };
}

macro_rules! vram_map {
    ( $n:expr ) => {
        VRAM + $n * 0x0800
    };
}

macro_rules! lcd_bg_tile {
    ( $n:expr ) => {
        $n << 2
    };
}

macro_rules! lcd_bg_map {
    ( $n:expr ) => {
        $n << 8
    };
}

fn prints(s: &str, x: usize, y: usize) {
    unsafe {
        for (i, c) in s.chars().enumerate() {
            (vram_map!(21) as *mut u16)
                .offset(y as isize * 32 + (i as isize + x as isize))
                .write_volatile(c as u16);
        }
    }
}

#[start]
fn main(_argc: isize, _argv: *const *const u8) -> isize {
    init_screen();

    prints("Hit any key, please.", 0, 0);

    let mut y = 1;
    let mut prev: Option<u16> = None;

    loop {
        let key = unsafe { core::ptr::read_volatile(KEY_STATUS as *const u16) & 0x3FF };
        if key == 0x03FF {
            prev = None;
            continue;
        }

        if let Some(prev) = prev {
            if prev == key {
                continue;
            }
        }

        prev = Some(key);

        prints("Key pressed ", 0, y);

        match key {
            0x03FE => prints("A", 12, y),
            0x03FD => prints("B", 12, y),
            0x03FB => prints("SELECT", 12, y),
            0x03F7 => prints("START", 12, y),
            0x03EF => prints("RIGHT", 12, y),
            0x03DF => prints("LEFT", 12, y),
            0x03BF => prints("UP", 12, y),
            0x037F => prints("DOWN", 12, y),
            0x02FF => prints("R", 12, y),
            0x01FF => prints("L", 12, y),
            _ => {
                // multi input is not supported.
                continue;
            }
        }
        y += 1;
    }
}

fn init_screen() {
    unsafe {
        (BG_PALETTE as *mut u16).offset(1).write_volatile(bgr!(0, 0, 0));
        (BG_PALETTE as *mut u16).offset(2).write_volatile(bgr!(31, 31, 31));

        let mut val = 0;
        for i in 0..256 {
            for row in 0..8 {
                for col in 0..8 {
                    let col = 7 - col;
                    let b = if font::CHAR8X8[i][row] & (1 << col) != 0 { 2 } else { 1 };
                    if col % 2 != 0 {
                        val = b
                    } else {
                        (vram_tile!(3) as *mut u16)
                            .offset((i * 32 + (row * 4) + ((7 - col) / 2)) as isize)
                            .write_volatile(val + (b << 8));
                    }
                }
            }
        }

        (LCD_BG0 as *mut u16).write_volatile(LCD_SIZE3232 | LCD_COLOR256 | lcd_bg_tile!(3) | lcd_bg_map!(21));
        (LCD_CTRL as *mut u16).write_volatile(LCD_MODE0 | LCD_BG0EN);
    }
}
