#![no_std]
#![feature(start)]

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[start]
fn main(_argc: isize, _argv: *const *const u8) -> isize {
    unsafe {
        (0x400_0000 as *mut u16).write_volatile(0x0403); // BG2 Mode 3
        (0x600_0000 as *mut u16).offset((80 * 240 + 120) as isize).write_volatile(0x001F);
    }
    0
}
