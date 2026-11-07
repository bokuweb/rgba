#include "types.h"
#include "memory.h"
#include "io.h"
#include "irq.h"
#include "video.h"
#include "dma.h"
#include "timer.h"
#include "audio.h"
#include "systemcall.h"

#include "assets.h"

#include <string.h>
#include <stdlib.h>

#define min(a, b) (((a) < (b)) ? (a) : (b))
#define max(a, b) (((a) > (b)) ? (a) : (b))
#define clamp(a, _min, _max) min(max(a, _min), _max)

// Player defs
#define PLAYER_MAX_VELOCITY (3<<12) // (20.12 fixed point)
#define PLAYER_ACCELERATION -669 // 9.8/60 = ~.163 = 669/4096 (20.12 fixed point)
#define PLAYER_X_OFFSET 40 // pixels
#define PLAYER_DIMS 32 // pixels
#define PLAYER_ANIMATION_SPEED 8 // frames

// Pipe defs
#define PIPE_TILE_DIM_X 4
#define PIPE_TILE_DIM_Y 16
#define PIPE_DIM_X (PIPE_TILE_DIM_X*8)
#define PIPE_DIM_Y (PIPE_TILE_DIM_Y*8)
#define PIPE_PIXEL_GAP 56
#define PIPE_SPEED (1<<12)

// Sprite tiles in OVRAM
#define BIRD_TILE_BASE 0
#define BIRD_TILE_SIZE 64
#define BIRD_PAL_IDX 0
#define PIPE_TILE_BASE (BIRD_TILE_BASE+BIRD_TILE_SIZE)
#define PIPE_TILE_SIZE 128
#define PIPE_PAL_IDX 1
#define BTN_TILE_BASE (PIPE_TILE_BASE+PIPE_TILE_SIZE)
#define BTN_TILE_SIZE 12
#define BTN_PAL_IDX 2
#define TEXT_TILE_BASE (BTN_TILE_BASE+BTN_TILE_SIZE)
#define TEXT_TITLE TEXT_TILE_BASE
#define TEXT_GAME_OVER (TEXT_TILE_BASE+48)
#define TEXT_DIGITS (TEXT_TILE_BASE+96)
#define TEXT_PAL_IDX 3

// OAM layout; lower indices draw on top
#define SCORE_OBJECT_IDX 0
#define SCORE_DIGITS 3
#define BTN_OBJECT_IDX1 3
#define BTN_OBJECT_IDX2 4
#define BTN_OBJECT_IDX3 5
#define TEXT_OBJECT_IDX1 6
#define TEXT_OBJECT_IDX2 7
#define TEXT_OBJECT_IDX3 8
#define PLAYER_OBJECT_IDX 9
#define PIPE_OBJECT_IDX 10

// Two columns on screen, evenly spaced as they wrap around
#define PIPE_COUNT 2
#define PIPE_SPACING 136

// Frames before a key press can leave the game over screen
#define DEAD_INPUT_DELAY 30

#define SFX(ch, name) audio_play(ch, (s8*)name, name##Len, 10512, 2, 0)

// Player sprite tile animation offsets
static const u32 player_flap_anim[4] = { 16, 32, 48, 32 };

// The player character
typedef struct Player {
	ObjAttr	*sprite; 	// hw renderable
	ObjAffine *tform;	// transform
	s32 vel_y;			// velocity y (20.12 fixed point)
	s32 pos_x, pos_y;	// position x,y (20.12 fixed point)
} Player;

typedef struct Pipe {
	ObjAttr	*sprite_upper[2], *sprite_lower[2]; 	// hw renderables
	s32 pos_x, pos_y;	// position x,y (20.12 fixed point)
} Pipe;

typedef enum GameState {
	GAME_STATE_MENU,
	GAME_STATE_PLAYING,
	GAME_STATE_DEAD,
} GameState;

// The player instance
Player player;

// Column pairs
Pipe pipes[PIPE_COUNT];

// Background scroll offset
s32 bg_off;

// Game mode
u32 game_mode, next_game_mode;

// The players score
u32 player_score;

// Frame number
u32 frame;

// game pause
u8 game_paused;

// Frames since dying
u32 dead_frames;

ObjAttr objects[OAM_OBJ_SIZE];
ObjAffine *objects_affine= (ObjAffine*)objects;

static int aabb_aabb_collision(s32 ax, s32 ay, s32 aw, s32 ah, s32 bx, s32 by, s32 bw, s32 bh) {
	s32 ar = ax + aw;
	s32 br = bx + bw;
	s32 ab = ay + ah;
	s32 bb = by + bh;
	if (ax <= br && ar >= bx) {
		if (ay <= bb && ab >= by) {
			return 1;
		}
	}
	return 0;
}

// Initialize BG1: a 256x160 sky that scrolls slowly
static void init_background() {
	REG_BG1CNT = BG_CHAR_BASE(0)|BG_SCREEN_BASE(31)|BG_PRIORITY(1);

	memcpy(&tile_mem[0][0], backgroundTiles, backgroundTilesLen);
	memcpy(&bg_pal16_mem[1], backgroundPal, backgroundPalLen);
	memcpy(&se_mem[31][0], backgroundMap, backgroundMapLen);
}

// Initialize BG0: 8 tiles of ground repeated across the 32 tile map, so it
// wraps without a seam
static void init_foreground() {
	REG_BG0CNT = BG_CHAR_BASE(1)|BG_SCREEN_BASE(30)|BG_PRIORITY(0);

	memcpy(&tile_mem[1][0], groundTiles, groundTilesLen);
	memcpy(&bg_pal16_mem[0], groundPal, groundPalLen);

	for (u32 y = 0; y < 4; y++) {
		for (u32 x = 0; x < 32; x++) {
			se_mem[30][(y+16)*32+x] = groundMap[y*8+x%8];
		}
	}

	bg_off = 0;
}

// Initialize Sprite VRAM (Charbocks 4/5)
static void init_sprite_vram() {
	dma3_copy(&tile_mem[4][BIRD_TILE_BASE], hopperTiles, hopperTilesLen/2, DMA_NOW);
	memcpy(&sprite_pal16_mem[BIRD_PAL_IDX], hopperPal, hopperPalLen);

	dma3_copy(&tile_mem[4][PIPE_TILE_BASE], columnTiles, columnTilesLen/2, DMA_NOW);
	memcpy(&sprite_pal16_mem[PIPE_PAL_IDX], columnPal, columnPalLen);

	dma3_copy(&tile_mem[4][BTN_TILE_BASE], buttonTiles, buttonTilesLen/2, DMA_NOW);
	memcpy(&sprite_pal16_mem[BTN_PAL_IDX], buttonPal, buttonPalLen);

	dma3_copy(&tile_mem[4][TEXT_TILE_BASE], textTiles, textTilesLen/2, DMA_NOW);
	memcpy(&sprite_pal16_mem[TEXT_PAL_IDX], textPal, textPalLen);
}

// Initialize plauer sprite
static void init_player() {
	//	Init player object
	memset(&player, 0, sizeof(Player));
	player.pos_x = ((M4_WIDTH - PLAYER_DIMS)/2 - PLAYER_X_OFFSET)<<12;
	player.pos_y = ((M4_HEIGHT - PLAYER_DIMS + 16)/2)<<12;

	// Init obj attr
	ObjAttr *sprite = player.sprite = &objects[PLAYER_OBJECT_IDX];
	obj_attr_init(sprite, player.pos_x>>12, player.pos_y>>12, OA_SIZE_32x32, BIRD_TILE_BASE+player_flap_anim[0], 0);
	obj_attr_set_flags(sprite, A0_ROT_SCALE_ON, 0, 0);

	// Init obj 2x2 affine matrix
	ObjAffine *tform = player.tform = &objects_affine[0];
	obj_aff_identity(tform);
}

// Pick a new gap and place the column pair at screen position x
static void reset_pipe(Pipe *pipe, s32 x) {
	pipe->pos_x = x<<12;
	// [-120-PIPE_PIXEL_GAP, -8+PIPE_PIXEL_GAP]
	pipe->pos_y = -((rand()%(112-PIPE_PIXEL_GAP)+8+PIPE_PIXEL_GAP)<<12);
}

// Initialize column sprites
static void init_pipes() {
	for (u32 i = 0; i < PIPE_COUNT; i++) {
		Pipe *pipe = &pipes[i];
		reset_pipe(pipe, M4_WIDTH+i*PIPE_SPACING);

		for (u32 j = 0; j < 2; j++) {
			ObjAttr *upper = pipe->sprite_upper[j] = &objects[PIPE_OBJECT_IDX+i*4+j];
			obj_attr_init(upper, pipe->pos_x>>12, 0, OA_SIZE_32x64, PIPE_TILE_BASE+j*32, PIPE_PAL_IDX);
			obj_attr_set_priority(upper, 1);

			ObjAttr *lower = pipe->sprite_lower[j] = &objects[PIPE_OBJECT_IDX+i*4+2+j];
			obj_attr_init(lower, pipe->pos_x>>12, 0, OA_SIZE_32x64, PIPE_TILE_BASE+64+j*32, PIPE_PAL_IDX);
			obj_attr_set_priority(lower, 1);
		}
	}
}

// Jump logic
static void player_jump() {
	player.vel_y = PLAYER_MAX_VELOCITY;
	SFX(0, sfxWing);
}

// Show the score centered at the top of the screen
static void update_score() {
	u32 digits[SCORE_DIGITS];
	digits[0] = player_score/100%10;
	digits[1] = player_score/10%10;
	digits[2] = player_score%10;

	u32 first = player_score >= 100 ? 0 : player_score >= 10 ? 1 : 2;
	u32 width = (SCORE_DIGITS-first)*12;
	u32 x = (M4_WIDTH-width)/2;
	for (u32 i = 0; i < SCORE_DIGITS; i++) {
		ObjAttr *obj = &objects[SCORE_OBJECT_IDX+i];
		if (i < first) {
			obj_attr_hide(obj);
		} else {
			obj_attr_init(obj, x+(i-first)*12, 4, OA_SIZE_16x16, TEXT_DIGITS+digits[i]*4, TEXT_PAL_IDX);
		}
	}
}

// Player input
static void update_controls() {
	if (key_hit(KEY_START)) {
		game_paused = !game_paused;
	}
	if (key_hit(KEY_A|KEY_B) && !game_paused) {
		player_jump();
	}
}

// Advance player state each frame
static void update_player(u32 anim_only) {
	ObjAttr* sprite = player.sprite;

	// advance animation frame (0-3)
	u32 animFrame = (frame/PLAYER_ANIMATION_SPEED)%4;
	obj_attr_set_tile_base(sprite, BIRD_TILE_BASE+player_flap_anim[animFrame]);

	// Bird physics
	if (!anim_only && !game_paused) {
		// Apply gravity
		player.vel_y = clamp(player.vel_y+PLAYER_ACCELERATION, -PLAYER_MAX_VELOCITY, PLAYER_MAX_VELOCITY);

		// Apply player velocity
		player.pos_y = clamp(player.pos_y-player.vel_y, -12<<12, M4_HEIGHT<<12);
		if (player.pos_y<=(-12<<12)) { // hit top, lose velocity
			player.vel_y = 0;
		}

		// Update obj attr
		obj_attr_set_position(sprite, player.pos_x>>12, player.pos_y>>12);

		// Update rotation
		u32 angle = 90*player.vel_y/PLAYER_MAX_VELOCITY;
		obj_aff_rotscale(player.tform, 1<<12, 1<<12, angle);

		// Bird x Ground collision
		if (player.pos_y > (104<<12)) {
			next_game_mode = GAME_STATE_DEAD;
			return;
		}

		for (u32 i = 0; i < PIPE_COUNT; i++) {
			Pipe *pipe = &pipes[i];

			// Bird x upper column collision
			if (aabb_aabb_collision(player.pos_x+(8<<12), player.pos_y+(8<<12), 16<<12, 16<<12
					, pipe->pos_x, pipe->pos_y, 32<<12, 128<<12)) {
				next_game_mode = GAME_STATE_DEAD;
				return;
			}

			// Bird x lower column collision
			if (aabb_aabb_collision(player.pos_x+(8<<12), player.pos_y+(8<<12), 16<<12, 16<<12
					, pipe->pos_x, pipe->pos_y+((128+PIPE_PIXEL_GAP)<<12), 32<<12, 128<<12)) {
				next_game_mode = GAME_STATE_DEAD;
				return;
			}

			// Did we pass the column this frame?
			if (player.pos_x < pipe->pos_x+PIPE_SPEED && player.pos_x >= pipe->pos_x) {
				player_score++;
				SFX(3, sfxPoint);
			}
		}
	}
}

// Fall to the ground after dying
static void update_fall() {
	player.vel_y = clamp(player.vel_y+PLAYER_ACCELERATION, -PLAYER_MAX_VELOCITY, PLAYER_MAX_VELOCITY);
	player.pos_y = min(player.pos_y-player.vel_y, 104<<12);
	obj_attr_set_position(player.sprite, player.pos_x>>12, player.pos_y>>12);
	u32 angle = 90*player.vel_y/PLAYER_MAX_VELOCITY;
	obj_aff_rotscale(player.tform, 1<<12, 1<<12, angle);
}

// Move columns and wrap them around after they leave the left edge
static void update_pipes() {
	for (u32 i = 0; i < PIPE_COUNT; i++) {
		Pipe *pipe = &pipes[i];
		if (!game_paused) {
			pipe->pos_x -= PIPE_SPEED;

			if (((pipe->pos_x>>12) + PIPE_DIM_X) <= 0) {
				reset_pipe(pipe, PIPE_SPACING*PIPE_COUNT-PIPE_DIM_X);
			}
		}

		s32 x = pipe->pos_x>>12, y = pipe->pos_y>>12;
		obj_attr_set_position(pipe->sprite_upper[0], x, y);
		obj_attr_set_position(pipe->sprite_upper[1], x, y+64);
		obj_attr_set_position(pipe->sprite_lower[0], x, y+128+PIPE_PIXEL_GAP);
		obj_attr_set_position(pipe->sprite_lower[1], x, y+64+128+PIPE_PIXEL_GAP);
	}
}

// Scroll backtground and foreground tilemaps via hw
static void update_background() {
	if (!game_paused) {
		REG_BG0HOFS = ++bg_off;
		REG_BG1HOFS = bg_off/4;
	}
}

// Enter menu state logic
static void set_game_state_menu() {
	// Reset background
	bg_off = 0;
	REG_BG0HOFS = 0;
	REG_BG1HOFS = 0;

	init_player();
	init_pipes();
	update_pipes();

	for (u32 i = 0; i < SCORE_DIGITS; i++) {
		obj_attr_hide(&objects[SCORE_OBJECT_IDX+i]);
	}

	u32 btn_pos_x = (M4_WIDTH-6*8)/2;
	u32 btn_pos_y = 112;
	for (u32 i = 0; i < 3; i++) {
		obj_attr_init(&objects[BTN_OBJECT_IDX1+i], btn_pos_x+i*16, btn_pos_y, OA_SIZE_16x16, BTN_TILE_BASE+i*4, BTN_PAL_IDX);
	}

	u32 text_pos_x = (M4_WIDTH-12*8)/2;
	u32 text_pos_y = 22;
	for (u32 i = 0; i < 3; i++) {
		obj_attr_init(&objects[TEXT_OBJECT_IDX1+i], text_pos_x+i*32, text_pos_y, OA_SIZE_32x32, TEXT_TITLE+i*16, TEXT_PAL_IDX);
	}
}

// Enter playing state logic
static void set_game_state_playing() {
	// Different columns every game
	srand(frame);
	init_pipes();

	// Hide start game text and button
	for (u32 i = BTN_OBJECT_IDX1; i <= TEXT_OBJECT_IDX3; i++) {
		obj_attr_hide(&objects[i]);
	}

	// Reset score
	player_score = 0;

	// Start with a jump
	player_jump();
}

// Enter death state logic
static void set_game_state_dead() {
	// Set gameover text
	u32 text_pos_x = (M4_WIDTH-12*8)/2;
	u32 text_pos_y = 36;
	for (u32 i = 0; i < 3; i++) {
		obj_attr_init(&objects[TEXT_OBJECT_IDX1+i], text_pos_x+i*32, text_pos_y, OA_SIZE_32x32, TEXT_GAME_OVER+i*16, TEXT_PAL_IDX);
	}

	// Stop flapping
	obj_attr_set_tile_base(player.sprite, BIRD_TILE_BASE+player_flap_anim[1]);
	dead_frames = 0;
	game_paused = 0;
	player.vel_y = 0;

	SFX(2, sfxHit);
	SFX(1, sfxDie);
}

int main(int argc, char* argv[]) {
	// Faster ROM access
	REG_WAITCNT = 0x4317;

	// Init DirectSound mixer
	audio_init();

	// Setup PPU video mode 0 (tile map)
	REG_DISPCNT = MODE_0
		| BG0_ON | BG1_ON
		| OBJ_ON | OBJ_1D;

	// Setup blending
	REG_BLDCNT = BLEND_MODE_ALPHA|BLEND_BG0_DST|BLEND_BG1_DST;
	REG_COLEV = SET_COLEV_SRC_ALPHA(8)|SET_COLEV_DST_ALPHA(8);

	// Init inturrupt handler
	irq_init();

	// Init sprite OAM and ERAM shadow
	oam_init(objects, OAM_OBJ_SIZE);

	// One-time game init
	init_background();
	init_foreground();
	init_sprite_vram();

	// Enter initial state
	game_mode = next_game_mode = GAME_STATE_MENU;
	set_game_state_menu();

	frame = 0;
	while(1) {
		// CPU halt until vblank interval
		VBlankIntrWait();

		// Poll keystate
		key_poll();

		// Mix audio
		audio_mix();

		// Switch game state
		if (game_mode != next_game_mode) {
			game_mode = next_game_mode;
			switch (game_mode) {
				case GAME_STATE_MENU: set_game_state_menu(); break;
				case GAME_STATE_PLAYING: set_game_state_playing(); break;
				case GAME_STATE_DEAD: set_game_state_dead(); break;
			}
		}

		// State specific update
		if (game_mode == GAME_STATE_MENU) {
			// Update player anim only
			update_player(1);

			if (key_hit(KEY_A|KEY_B)) {
				next_game_mode = GAME_STATE_PLAYING;
			}
		} else if (game_mode == GAME_STATE_PLAYING) {
			// Apply input
			update_controls();

			// Update columns
			update_pipes();

			// Update player
			update_player(0);

			// Update background
			update_background();

			update_score();
		} else if (game_mode == GAME_STATE_DEAD) {
			update_fall();

			if (++dead_frames > DEAD_INPUT_DELAY && key_hit(KEY_A|KEY_B|KEY_START)) {
				next_game_mode = GAME_STATE_MENU;
			}
		}

		// Copy ERAM object shadows to VRAM
		oam_copy(oam_mem, objects, OAM_OBJ_SIZE);

		frame++;
	}
	return 0;
}
