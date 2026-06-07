use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::render::Canvas;
use sdl2::video::Window;

const WIDTH: u32 = 240;
const HEIGHT: u32 = 160;

pub struct Renderer {
    canvas: Canvas<Window>,
}

impl Renderer {
    pub fn new() -> Self {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let window = video_subsystem
            .window("rusty-gba", WIDTH, HEIGHT)
            .position_centered()
            .build()
            .expect("should create window.");
        let mut canvas = window.into_canvas().build().expect("should create canvas");
        canvas.clear();
        canvas.present();
        Self { canvas }
    }

    pub(crate) fn render(&mut self, buf: &[u8]) {
        for i in 0..HEIGHT {
            for j in 0..WIDTH {
                let base = ((i * WIDTH + j) * 4) as usize;
                let r = buf.get(base).expect("should get pixel data");
                let g = buf.get(base + 1).expect("should get pixel data");
                let b = buf.get(base + 2).expect("should get pixel data");
                self.canvas.set_draw_color(Color::RGB(*r, *g, *b));
                let _ = self.canvas.draw_point(Point::new(j as i32, i as i32));
            }
        }
        self.canvas.present();
    }
}
