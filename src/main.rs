mod cpu;
mod lcd;
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
    // cpu::run();
    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("rustynes", WIDTH, HEIGHT)
        .position_centered()
        .build()
        .unwrap();
    let canvas = window.into_canvas().build().unwrap();
    let mut pad = 0;
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

        let elapsed_time = SystemTime::now()
            .duration_since(prev_time)
            .expect("Time went backwards")
            .as_nanos();
        let wait = if elapsed_time < 1_000_000_000u128 / 60 {
            1_000_000_000u32 / 60 - (elapsed_time as u32)
        } else {
            0
        };
        ::std::thread::sleep(Duration::new(0, wait));
        prev_time = SystemTime::now();
    }

    let buf = vec![0; 240 * 160 * 4];
    loop {
        // renderer.render(&buf);
    }
}
