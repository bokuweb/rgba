//! Flappy Enaga in Rust: a port of ../flappy-enaga/src/main.c that plays the
//! same and draws the same art.
#![no_std]
#![no_main]

mod audio;
mod gba;
mod sin_lut;

use audio::{Mixer, SAMPLE_RATE};
use gba::*;

/// 4-byte aligned data, so DMA and word copies can read it.
#[repr(C, align(4))]
pub struct Align4<T: ?Sized>(pub T);

mod assets {
    use super::Align4;
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));
}
use assets::*;

// Cartridge header, stacks, .iwram/.data copy and .bss clear, shared with the
// C version. It ends by calling `main`.
core::arch::global_asm!(include_str!("../../flappy-enaga/rt/crt0.s"), options(raw));

// Player (positions and velocity are 20.12 fixed point)
const PLAYER_MAX_VELOCITY: i32 = 3 << 12;
const PLAYER_ACCELERATION: i32 = -669; // 9.8/60 = ~.163 = 669/4096
const PLAYER_X_OFFSET: i32 = 40;
const PLAYER_DIMS: i32 = 32;
const PLAYER_ANIMATION_SPEED: u32 = 8; // frames

// Columns
const PIPE_DIM_X: i32 = 4 * 8;
const PIPE_PIXEL_GAP: i32 = 56;
const PIPE_SPEED: i32 = 1 << 12;
// Two columns on screen, evenly spaced as they wrap around
const PIPE_COUNT: usize = 2;
const PIPE_SPACING: i32 = 136;

// Sprite tiles in object VRAM
const BIRD_TILE_BASE: u32 = 0;
const BIRD_PAL_IDX: usize = 0;
const PIPE_TILE_BASE: u32 = BIRD_TILE_BASE + 64;
const PIPE_PAL_IDX: usize = 1;
const BTN_TILE_BASE: u32 = PIPE_TILE_BASE + 128;
const BTN_PAL_IDX: usize = 2;
const TEXT_TILE_BASE: u32 = BTN_TILE_BASE + 12;
const TEXT_TITLE: u32 = TEXT_TILE_BASE;
const TEXT_GAME_OVER: u32 = TEXT_TILE_BASE + 48;
const TEXT_DIGITS: u32 = TEXT_TILE_BASE + 96;
const TEXT_PAL_IDX: usize = 3;

// OAM layout; lower indices draw on top
const SCORE_OBJECT_IDX: usize = 0;
const SCORE_DIGITS: usize = 3;
const BTN_OBJECT_IDX: usize = 3;
const TEXT_OBJECT_IDX: usize = 6;
const PLAYER_OBJECT_IDX: usize = 9;
const PIPE_OBJECT_IDX: usize = 10;
const PLAYER_AFFINE_IDX: usize = 0;

// Frames before a key press can leave the game over screen
const DEAD_INPUT_DELAY: u32 = 30;

// Player sprite tile animation offsets
const PLAYER_FLAP_ANIM: [u32; 4] = [16, 32, 48, 32];

#[derive(Clone, Copy, PartialEq, Eq)]
enum GameState {
    Menu,
    Playing,
    Dead,
}

#[derive(Default)]
struct Player {
    vel_y: i32,
    pos_x: i32,
    pos_y: i32,
}

#[derive(Default, Clone, Copy)]
struct Pipe {
    pos_x: i32,
    pos_y: i32,
}

/// The C library's rand(), so the same seed gives the same columns.
struct Rand(u32);

impl Rand {
    fn seed(&mut self, seed: u32) {
        self.0 = seed;
    }
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1103515245).wrapping_add(12345);
        (self.0 >> 16) & 0x7FFF
    }
}

struct Game {
    oam: Oam,
    keys: Keys,
    mixer: Mixer,
    rand: Rand,
    player: Player,
    pipes: [Pipe; PIPE_COUNT],
    bg_off: i32,
    mode: GameState,
    next_mode: GameState,
    score: u32,
    frame: u32,
    paused: bool,
    dead_frames: u32,
}

#[allow(clippy::too_many_arguments)]
fn aabb_collision(ax: i32, ay: i32, aw: i32, ah: i32, bx: i32, by: i32, bw: i32, bh: i32) -> bool {
    ax <= bx + bw && ax + aw >= bx && ay <= by + bh && ay + ah >= by
}

/// Copies `src` to VRAM with DMA3, a halfword at a time.
fn dma3_copy16(dst: usize, src: &[u16]) {
    dma_copy(3, dst, src.as_ptr() as usize, src.len() as u32, DMA_NOW);
}

// BG1: a 256x160 sky that scrolls slowly
fn init_background() {
    REG_BG1CNT.write(bg_char_base(0) | bg_screen_base(31) | bg_priority(1));
    copy_halfwords(tile_addr(0, 0), &BACKGROUND_TILES.0);
    copy_halfwords(bg_pal16_addr(1), &BACKGROUND_PAL.0);
    copy_halfwords(screen_block_addr(31), &BACKGROUND_MAP.0);
}

// BG0: 8 tiles of ground repeated across the 32 tile map, so it wraps
// without a seam
fn init_foreground() {
    REG_BG0CNT.write(bg_char_base(1) | bg_screen_base(30) | bg_priority(0));
    copy_halfwords(tile_addr(1, 0), &GROUND_TILES.0);
    copy_halfwords(bg_pal16_addr(0), &GROUND_PAL.0);

    let map = screen_block_addr(30) as *mut u16;
    for y in 0..4 {
        for x in 0..32 {
            let v = GROUND_MAP.0[y * 8 + x % 8];
            unsafe { core::ptr::write_volatile(map.add((y + 16) * 32 + x), v) }
        }
    }
}

// Sprite tiles go in character block 4, the first block of object VRAM
fn init_sprite_vram() {
    dma3_copy16(tile_addr(4, BIRD_TILE_BASE as usize), &ENAGA_TILES.0);
    copy_halfwords(obj_pal16_addr(BIRD_PAL_IDX), &ENAGA_PAL.0);
    dma3_copy16(tile_addr(4, PIPE_TILE_BASE as usize), &COLUMN_TILES.0);
    copy_halfwords(obj_pal16_addr(PIPE_PAL_IDX), &COLUMN_PAL.0);
    dma3_copy16(tile_addr(4, BTN_TILE_BASE as usize), &BUTTON_TILES.0);
    copy_halfwords(obj_pal16_addr(BTN_PAL_IDX), &BUTTON_PAL.0);
    dma3_copy16(tile_addr(4, TEXT_TILE_BASE as usize), &TEXT_TILES.0);
    copy_halfwords(obj_pal16_addr(TEXT_PAL_IDX), &TEXT_PAL.0);
}

impl Game {
    fn sfx(&mut self, channel: usize, data: &'static [i8]) {
        self.mixer.play(channel, data, SAMPLE_RATE, 2);
    }

    fn init_player(&mut self) {
        self.player = Player {
            vel_y: 0,
            pos_x: ((SCREEN_WIDTH - PLAYER_DIMS) / 2 - PLAYER_X_OFFSET) << 12,
            pos_y: ((SCREEN_HEIGHT - PLAYER_DIMS + 16) / 2) << 12,
        };
        let (x, y) = (self.player.pos_x >> 12, self.player.pos_y >> 12);
        let tile = BIRD_TILE_BASE + PLAYER_FLAP_ANIM[0];
        self.oam.init(PLAYER_OBJECT_IDX, x, y, ObjSize::S32x32, tile, 0);
        self.oam.set_rot_scale(PLAYER_OBJECT_IDX);
        self.oam.affine_identity(PLAYER_AFFINE_IDX);
    }

    // Pick a new gap and place the column pair at screen position x
    fn reset_pipe(&mut self, i: usize, x: i32) {
        let pipe = &mut self.pipes[i];
        pipe.pos_x = x << 12;
        // [-120-PIPE_PIXEL_GAP, -8+PIPE_PIXEL_GAP]
        let r = self.rand.next() as i32 % (112 - PIPE_PIXEL_GAP);
        pipe.pos_y = -((r + 8 + PIPE_PIXEL_GAP) << 12);
    }

    fn init_pipes(&mut self) {
        for i in 0..PIPE_COUNT {
            self.reset_pipe(i, SCREEN_WIDTH + i as i32 * PIPE_SPACING);
            let x = self.pipes[i].pos_x >> 12;
            for j in 0..2 {
                let upper = PIPE_OBJECT_IDX + i * 4 + j;
                let tile = PIPE_TILE_BASE + j as u32 * 32;
                self.oam.init(upper, x, 0, ObjSize::S32x64, tile, PIPE_PAL_IDX as u32);
                self.oam.set_priority(upper, 1);

                let lower = PIPE_OBJECT_IDX + i * 4 + 2 + j;
                let tile = PIPE_TILE_BASE + 64 + j as u32 * 32;
                self.oam.init(lower, x, 0, ObjSize::S32x64, tile, PIPE_PAL_IDX as u32);
                self.oam.set_priority(lower, 1);
            }
        }
    }

    fn player_jump(&mut self) {
        self.player.vel_y = PLAYER_MAX_VELOCITY;
        self.sfx(0, &SFX_WING.0);
    }

    // Show the score centered at the top of the screen
    fn update_score(&mut self) {
        let s = self.score;
        let digits = [s / 100 % 10, s / 10 % 10, s % 10];
        let first = if s >= 100 {
            0
        } else if s >= 10 {
            1
        } else {
            2
        };
        let width = (SCORE_DIGITS - first) as i32 * 12;
        let x = (SCREEN_WIDTH - width) / 2;
        for (i, &d) in digits.iter().enumerate() {
            let obj = SCORE_OBJECT_IDX + i;
            if i < first {
                self.oam.hide(obj);
            } else {
                let dx = x + (i - first) as i32 * 12;
                self.oam.init(obj, dx, 4, ObjSize::S16x16, TEXT_DIGITS + d * 4, TEXT_PAL_IDX as u32);
            }
        }
    }

    fn update_controls(&mut self) {
        if self.keys.hit(KEY_START) {
            self.paused = !self.paused;
        }
        if self.keys.hit(KEY_A | KEY_B) && !self.paused {
            self.player_jump();
        }
    }

    fn rotate_player(&mut self) {
        let angle = 90 * self.player.vel_y / PLAYER_MAX_VELOCITY;
        self.oam.affine_rotate(PLAYER_AFFINE_IDX, angle as u16);
    }

    fn update_player(&mut self, anim_only: bool) {
        let anim_frame = (self.frame / PLAYER_ANIMATION_SPEED) % 4;
        self.oam.set_tile(PLAYER_OBJECT_IDX, BIRD_TILE_BASE + PLAYER_FLAP_ANIM[anim_frame as usize]);

        if anim_only || self.paused {
            return;
        }

        let p = &mut self.player;
        // Gravity, then velocity
        p.vel_y = (p.vel_y + PLAYER_ACCELERATION).clamp(-PLAYER_MAX_VELOCITY, PLAYER_MAX_VELOCITY);
        p.pos_y = (p.pos_y - p.vel_y).clamp(-12 << 12, SCREEN_HEIGHT << 12);
        if p.pos_y <= -12 << 12 {
            // Hit the top: lose velocity
            p.vel_y = 0;
        }
        let (px, py) = (p.pos_x, p.pos_y);

        self.oam.set_position(PLAYER_OBJECT_IDX, px >> 12, py >> 12);
        self.rotate_player();

        // Ground
        if py > 104 << 12 {
            self.next_mode = GameState::Dead;
            return;
        }

        for i in 0..PIPE_COUNT {
            let pipe = self.pipes[i];
            let (bx, by, bw) = (px + (8 << 12), py + (8 << 12), 16 << 12);
            let upper = aabb_collision(bx, by, bw, bw, pipe.pos_x, pipe.pos_y, 32 << 12, 128 << 12);
            let lower_y = pipe.pos_y + ((128 + PIPE_PIXEL_GAP) << 12);
            let lower = aabb_collision(bx, by, bw, bw, pipe.pos_x, lower_y, 32 << 12, 128 << 12);
            if upper || lower {
                self.next_mode = GameState::Dead;
                return;
            }

            // Passed the column this frame?
            if px < pipe.pos_x + PIPE_SPEED && px >= pipe.pos_x {
                self.score += 1;
                self.sfx(3, &SFX_POINT.0);
            }
        }
    }

    // Fall to the ground after dying
    fn update_fall(&mut self) {
        let p = &mut self.player;
        p.vel_y = (p.vel_y + PLAYER_ACCELERATION).clamp(-PLAYER_MAX_VELOCITY, PLAYER_MAX_VELOCITY);
        p.pos_y = (p.pos_y - p.vel_y).min(104 << 12);
        let (x, y) = (p.pos_x >> 12, p.pos_y >> 12);
        self.oam.set_position(PLAYER_OBJECT_IDX, x, y);
        self.rotate_player();
    }

    // Move columns and wrap them around after they leave the left edge
    fn update_pipes(&mut self) {
        for i in 0..PIPE_COUNT {
            if !self.paused {
                self.pipes[i].pos_x -= PIPE_SPEED;
                if (self.pipes[i].pos_x >> 12) + PIPE_DIM_X <= 0 {
                    self.reset_pipe(i, PIPE_SPACING * PIPE_COUNT as i32 - PIPE_DIM_X);
                }
            }

            let (x, y) = (self.pipes[i].pos_x >> 12, self.pipes[i].pos_y >> 12);
            let base = PIPE_OBJECT_IDX + i * 4;
            self.oam.set_position(base, x, y);
            self.oam.set_position(base + 1, x, y + 64);
            self.oam.set_position(base + 2, x, y + 128 + PIPE_PIXEL_GAP);
            self.oam.set_position(base + 3, x, y + 64 + 128 + PIPE_PIXEL_GAP);
        }
    }

    // Scroll the ground, and the sky at a quarter of the speed
    fn update_background(&mut self) {
        if !self.paused {
            self.bg_off += 1;
            REG_BG0HOFS.write(self.bg_off as u16);
            REG_BG1HOFS.write((self.bg_off / 4) as u16);
        }
    }

    fn enter_menu(&mut self) {
        self.bg_off = 0;
        REG_BG0HOFS.write(0);
        REG_BG1HOFS.write(0);

        self.init_player();
        self.init_pipes();
        self.update_pipes();

        for i in 0..SCORE_DIGITS {
            self.oam.hide(SCORE_OBJECT_IDX + i);
        }

        let (btn_x, btn_y) = ((SCREEN_WIDTH - 6 * 8) / 2, 112);
        for i in 0..3 {
            let tile = BTN_TILE_BASE + i as u32 * 4;
            let x = btn_x + i as i32 * 16;
            self.oam.init(BTN_OBJECT_IDX + i, x, btn_y, ObjSize::S16x16, tile, BTN_PAL_IDX as u32);
        }
        self.show_banner(TEXT_TITLE, 22);
    }

    fn show_banner(&mut self, tile_base: u32, y: i32) {
        let x = (SCREEN_WIDTH - 12 * 8) / 2;
        for i in 0..3 {
            let tile = tile_base + i as u32 * 16;
            let ox = x + i as i32 * 32;
            self.oam.init(TEXT_OBJECT_IDX + i, ox, y, ObjSize::S32x32, tile, TEXT_PAL_IDX as u32);
        }
    }

    fn enter_playing(&mut self) {
        // Different columns every game
        self.rand.seed(self.frame);
        self.init_pipes();

        // Hide the title and the button
        for i in BTN_OBJECT_IDX..TEXT_OBJECT_IDX + 3 {
            self.oam.hide(i);
        }

        self.score = 0;
        // Start with a jump
        self.player_jump();
    }

    fn enter_dead(&mut self) {
        self.show_banner(TEXT_GAME_OVER, 36);

        // Stop flapping
        self.oam.set_tile(PLAYER_OBJECT_IDX, BIRD_TILE_BASE + PLAYER_FLAP_ANIM[1]);
        self.dead_frames = 0;
        self.paused = false;
        self.player.vel_y = 0;

        self.sfx(2, &SFX_HIT.0);
        self.sfx(1, &SFX_DIE.0);
    }

    fn step(&mut self) {
        self.keys.poll();
        self.mixer.mix();

        if self.mode != self.next_mode {
            self.mode = self.next_mode;
            match self.mode {
                GameState::Menu => self.enter_menu(),
                GameState::Playing => self.enter_playing(),
                GameState::Dead => self.enter_dead(),
            }
        }

        match self.mode {
            GameState::Menu => {
                // Animate the bird only
                self.update_player(true);
                if self.keys.hit(KEY_A | KEY_B) {
                    self.next_mode = GameState::Playing;
                }
            }
            GameState::Playing => {
                self.update_controls();
                self.update_pipes();
                self.update_player(false);
                self.update_background();
                self.update_score();
            }
            GameState::Dead => {
                self.update_fall();
                self.dead_frames += 1;
                if self.dead_frames > DEAD_INPUT_DELAY && self.keys.hit(KEY_A | KEY_B | KEY_START) {
                    self.next_mode = GameState::Menu;
                }
            }
        }

        self.oam.copy_to_hw();
        self.frame = self.frame.wrapping_add(1);
    }
}

#[unsafe(no_mangle)]
extern "C" fn main() -> ! {
    // Faster ROM access
    REG_WAITCNT.write(0x4317);

    let mixer = Mixer::init();

    REG_DISPCNT.write(MODE_0 | BG0_ON | BG1_ON | OBJ_ON | OBJ_1D);
    REG_BLDCNT.write(BLEND_MODE_ALPHA | BLEND_BG0_DST | BLEND_BG1_DST);
    REG_COLEV.write(colev_src(8) | colev_dst(8));

    audio::irq_init();

    let mut game = Game {
        oam: Oam::new(),
        keys: Keys::default(),
        mixer,
        rand: Rand(1),
        player: Player::default(),
        pipes: [Pipe::default(); PIPE_COUNT],
        bg_off: 0,
        mode: GameState::Menu,
        next_mode: GameState::Menu,
        score: 0,
        frame: 0,
        paused: false,
        dead_frames: 0,
    };

    init_background();
    init_foreground();
    init_sprite_vram();
    game.enter_menu();

    loop {
        vblank_intr_wait();
        game.step();
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
