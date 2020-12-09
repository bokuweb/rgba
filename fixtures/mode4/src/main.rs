#![no_std]
#![feature(start)]

static LCD_CTRL: usize = 0x0400_0000;
static REG_VCOUNT: usize = 0x0400_0006;
static BG_PALETTE: usize = 0x0500_0000;
static FRAME0: usize = 0x0600_0000;
static FRAME1: usize = 0x0600_A000;

static BACKBUFFER: u16 = 0x10;
static LCD_MODE4: u16 = 0x0004;
static LCD_BG2EN: u16 = 0x0400;

macro_rules! rgb {
    ( $r:expr, $g:expr, $b:expr ) => {
        (($b as u16) << 10) + (($g as u16) << 5) + $r as u16
    };
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

enum CurrentFrame {
    Frame0,
    Frame1,
}

struct Game {
    pub line: usize,
    pub current_frame: CurrentFrame,
}

impl Game {
    pub fn new() -> Self {
        Self {
            line: 0,
            current_frame: CurrentFrame::Frame0,
        }
    }

    fn wait_for_vsync(&self) {
        unsafe {
            while core::ptr::read_volatile(REG_VCOUNT as *const u16) >= 160 {}
            while core::ptr::read_volatile(REG_VCOUNT as *const u16) < 160 {}
        }
    }

    fn switch_frame(&mut self) {
        unsafe {
            let c = core::ptr::read_volatile(LCD_CTRL as *const u16);
            (LCD_CTRL as *mut u16).offset(0).write_volatile(c ^ BACKBUFFER);
            if c & BACKBUFFER != 0 {
                self.current_frame = CurrentFrame::Frame0
            } else {
                self.current_frame = CurrentFrame::Frame1
            }
        }
    }

    fn init_palette(&self) {
        unsafe {
            for i in 0..32 {
                (BG_PALETTE as *mut u16).offset(i).write_volatile(rgb!(31, 31 - i, 31 - i));
            }

            for i in 0..32 {
                (BG_PALETTE as *mut u16).offset(32 + i).write_volatile(rgb!(31 - i, i, 0));
            }

            for i in 0..32 {
                (BG_PALETTE as *mut u16).offset(64 + i).write_volatile(rgb!(0, 31, i));
            }

            for i in 0..32 {
                (BG_PALETTE as *mut u16).offset(96 + i).write_volatile(rgb!(0, 31 - i, 31));
            }

            for i in 0..32 {
                (BG_PALETTE as *mut u16).offset(128 + i).write_volatile(rgb!(i, 0, 31));
            }

            for i in 0..32 {
                (BG_PALETTE as *mut u16).offset(160 + i).write_volatile(rgb!(31, 0, 31 - i));
            }

            for i in 0..32 {
                (BG_PALETTE as *mut u16).offset(192 + i).write_volatile(rgb!(31, i, 0));
            }

            for i in 0..32 {
                (BG_PALETTE as *mut u16).offset(224 + i).write_volatile(rgb!(31, 31, i));
            }
        }
    }

    fn put_line(&self, line: usize, index: usize) {
        let v = index | index.wrapping_shl(8) | index.wrapping_shl(16) | index.wrapping_shl(24);
        let base = line as isize * 60;
        for i in 0..60 {
            match self.current_frame {
                CurrentFrame::Frame0 => unsafe {
                    (FRAME0 as *mut u32).offset(base + i).write_volatile(v as u32);
                },
                CurrentFrame::Frame1 => unsafe {
                    (FRAME1 as *mut u32).offset(base + i).write_volatile(v as u32);
                },
            }
        }
    }

    fn draw(&mut self) {
        for j in 0..160 {
            self.put_line(j, (self.line + j) & 0xFF);
        }
        self.line += 1;
    }
}

#[start]
fn main(_argc: isize, _argv: *const *const u8) -> isize {
    unsafe {
        (LCD_CTRL as *mut u16).write_volatile(LCD_MODE4 | LCD_BG2EN);
    }
    let mut game = Game::new();
    game.init_palette();

    loop {
        game.draw();
        game.wait_for_vsync();
        game.switch_frame();
    }
}
