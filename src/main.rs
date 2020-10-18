#[macro_use]
extern crate log;

#[macro_use]
extern crate bitfield;

mod cpu;
mod gba;
mod lcd;
mod types;
mod memory;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::render::WindowCanvas;
use sdl2::Sdl;

use std::time::{Duration, SystemTime};

const WIDTH: u32 = 240;
const HEIGHT: u32 = 160;

fn main() {
    std::env::set_var("RUST_LOG", "info");
    pretty_env_logger::init();

    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("rustynes", WIDTH, HEIGHT)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();
    let mut prev_time = SystemTime::now();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        let buf = gba::frame();

        for i in 0..HEIGHT {
            for j in 0..WIDTH {
                let base = ((i * WIDTH + j) * 4) as usize;
                let r = buf.get(base).expect("should get pixel data");
                let g = buf.get(base + 1).expect("should get pixel data");
                let b = buf.get(base + 2).expect("should get pixel data");
                canvas.set_draw_color(Color::RGB(*r, *g, *b));
                let _ = canvas.draw_point(Point::new(j as i32, i as i32));
            }
        }
        canvas.present();

        let elapsed_time = SystemTime::now().duration_since(prev_time).expect("Time went backwards").as_nanos();
        let wait = if elapsed_time < 1_000_000_000u128 / 60 {
            1_000_000_000u32 / 60 - (elapsed_time as u32)
        } else {
            0
        };
        ::std::thread::sleep(Duration::new(0, wait));
        prev_time = SystemTime::now();
        // panic!("")
    }
}
