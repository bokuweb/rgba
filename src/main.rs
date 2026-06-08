// The native binary is a thin SDL2 frontend. It shares the whole `src/**` module
// tree with the library crate, but only drives the run-the-frame path — the
// debugger / assembler / disassembler API and various register accessors exist
// for the wasm frontend (and the test suite), so they read as "dead" here even
// though they are genuinely used elsewhere. Silence dead-code analysis for the
// binary rather than scatter per-item allows; the wasm build still exercises
// that surface.
#![allow(dead_code)]

#[macro_use]
mod macros;

mod cpu;
mod gba;
mod interrupt;
mod io;
mod lcd;
mod memory;
mod types;

use sdl2::audio::{AudioQueue, AudioSpecDesired};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Point;

use std::time::{Duration, SystemTime};

const WIDTH: u32 = 240;
const HEIGHT: u32 = 160;

fn main() {

    let sdl_context = sdl2::init().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("arm", WIDTH, HEIGHT).position_centered().build().unwrap();
    let mut canvas = window.into_canvas().build().unwrap();

    // Audio: a stereo i16 queue clocked at the APU's output rate. If the audio
    // device can't be opened (e.g. headless), run without sound.
    let audio_queue: Option<AudioQueue<i16>> = sdl_context.audio().ok().and_then(|audio| {
        let desired = AudioSpecDesired {
            freq: Some(gba::GBA::AUDIO_SAMPLE_RATE as i32),
            channels: Some(2),
            samples: Some(1024),
        };
        audio.open_queue::<i16, _>(None, &desired).ok()
    });
    if let Some(q) = &audio_queue {
        q.resume();
    }

    let mut prev_time = SystemTime::now();
    let mut gba = gba::GBA::new();
    let mut key = io::Key::new();
    let mut frames: u64 = 0;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown { keycode: Some(code), .. } => {
                    // dbg!("keydown", &code);
                    match code {
                        Keycode::X => key.set_B(io::KeyStatus::ON),
                        Keycode::Z => key.set_A(io::KeyStatus::ON),
                        Keycode::Space => key.set_SELECT(io::KeyStatus::ON),
                        Keycode::Return => key.set_START(io::KeyStatus::ON),
                        Keycode::Up => key.set_UP(io::KeyStatus::ON),
                        Keycode::Down => key.set_DOWN(io::KeyStatus::ON),
                        Keycode::Left => key.set_LEFT(io::KeyStatus::ON),
                        Keycode::Right => key.set_RIGHT(io::KeyStatus::ON),
                        Keycode::A => key.set_L(io::KeyStatus::ON),
                        Keycode::S => key.set_R(io::KeyStatus::ON),
                        _ => {}
                    }
                }
                Event::KeyUp { keycode: Some(code), .. } => match code {
                    Keycode::X => key.set_B(io::KeyStatus::OFF),
                    Keycode::Z => key.set_A(io::KeyStatus::OFF),
                    Keycode::Space => key.set_SELECT(io::KeyStatus::OFF),
                    Keycode::Return => key.set_START(io::KeyStatus::OFF),
                    Keycode::Up => key.set_UP(io::KeyStatus::OFF),
                    Keycode::Down => key.set_DOWN(io::KeyStatus::OFF),
                    Keycode::Left => key.set_LEFT(io::KeyStatus::OFF),
                    Keycode::Right => key.set_RIGHT(io::KeyStatus::OFF),
                    Keycode::A => key.set_L(io::KeyStatus::OFF),
                    Keycode::S => key.set_R(io::KeyStatus::OFF),
                    _ => {}
                },
                _ => {}
            }
        }

        gba.update_key(key);

        let buf = gba.frame(false);

        // Push this frame's audio. Drop samples if the queue is backing up (the
        // emulator ran ahead) to keep latency bounded.
        let samples = gba.take_audio();
        if let Some(q) = &audio_queue {
            if q.size() < (gba::GBA::AUDIO_SAMPLE_RATE * 2 * 2) / 4 {
                let _ = q.queue(&samples);
            }
        }

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

        // Persist battery-backed save memory roughly once a second (no-op unless
        // the game wrote to it). A final flush happens on exit below.
        frames = frames.wrapping_add(1);
        if frames.is_multiple_of(60) {
            gba.flush_save_if_dirty();
        }

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

    // Flush any pending save before exiting.
    gba.flush_save_if_dirty();
}
