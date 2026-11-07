#!/usr/bin/env python3
"""Draws all of Flappy Hopper's art and sound from code and writes them as a C
source file. Nothing here comes from another game. Usage: gen_assets.py OUTDIR"""
import math
import random
import sys


def rgb(h):
    h = h.lstrip("#")
    r, g, b = (int(h[i : i + 2], 16) for i in (0, 2, 4))
    return (r >> 3) | ((g >> 3) << 5) | ((b >> 3) << 10)


def canvas(w, h, fill=0):
    return [[fill] * w for _ in range(h)]


def outline(img, color):
    """Paints `color` on every empty pixel that touches a filled one."""
    h, w = len(img), len(img[0])
    out = [row[:] for row in img]
    for y in range(h):
        for x in range(w):
            if img[y][x]:
                continue
            for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                nx, ny = x + dx, y + dy
                if 0 <= nx < w and 0 <= ny < h and img[ny][nx]:
                    out[y][x] = color
                    break
    return out


def blit(dst, src, ox, oy):
    for y, row in enumerate(src):
        for x, c in enumerate(row):
            if c:
                dst[oy + y][ox + x] = c


def tile_at(img, tx, ty):
    return tuple(img[ty * 8 + y][tx * 8 + x] for y in range(8) for x in range(8))


def tile_words(tile):
    """4bpp tile as 16 halfwords; the left pixel sits in the low nibble."""
    words = []
    for y in range(8):
        row = tile[y * 8 : y * 8 + 8]
        for i in (0, 4):
            words.append(row[i] | row[i + 1] << 4 | row[i + 2] << 8 | row[i + 3] << 12)
    return words


def sprite_tiles(img):
    """Tiles in the row-major order that 1D object mapping expects."""
    words = []
    for ty in range(len(img) // 8):
        for tx in range(len(img[0]) // 8):
            words += tile_words(tile_at(img, tx, ty))
    return words


def sprite_blocks(img, width):
    """Splits a sheet into objects `width` pixels wide, each with its own run of tiles."""
    words = []
    for x in range(0, len(img[0]), width):
        words += sprite_tiles([row[x : x + width] for row in img])
    return words


def tile_map(img, pal_bank):
    """Deduplicated tiles and a map. Tile 0 is always blank."""
    blank = (0,) * 64
    tiles = {blank: 0}
    order = [blank]
    entries = []
    for ty in range(len(img) // 8):
        for tx in range(len(img[0]) // 8):
            t = tile_at(img, tx, ty)
            if t not in tiles:
                tiles[t] = len(order)
                order.append(t)
            entries.append(tiles[t] | pal_bank << 12)
    words = []
    for t in order:
        words += tile_words(t)
    return words, entries, len(order)


# ---------------------------------------------------------------- background
SKY = ["#1b1f4b", "#2b2a66", "#43307a", "#6a3a8a", "#96478f", "#c8578a", "#ee7a78", "#f9a66c", "#fcd08a"]
BG_PAL = [rgb(c) for c in ["#000000"] + SKY + ["#fff3bf", "#ffd9c0", "#c98aa8", "#5f3f85", "#3a2a60"]]
SUN, CLOUD, CLOUD_DARK, FAR, NEAR = 10, 11, 12, 13, 14
BAYER = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]]


def ridge(x, base, parts):
    return base + sum(a * math.sin(2 * math.pi * k * x / 256 + p) for a, k, p in parts)


def background():
    rng = random.Random(7)
    w, h = 256, 160
    img = canvas(w, h)
    for y in range(h):
        pos = min(y, 118) / 118 * (len(SKY) - 1)
        band = int(pos)
        frac = pos - band
        for x in range(w):
            up = frac * 16 > BAYER[y % 4][x % 4]
            img[y][x] = min(band + (2 if up else 1), len(SKY))
    for _ in range(46):
        x, y = rng.randrange(w), rng.randrange(44)
        img[y][x] = SUN if rng.random() < 0.3 else CLOUD
    cx, cy, r = 176, 92, 22
    for y in range(h):
        for x in range(w):
            if (x - cx) ** 2 + (y - cy) ** 2 <= r * r:
                img[y][x] = SUN
    for cloud_x, cloud_y, size in [(40, 46, 1.0), (120, 30, 0.7), (214, 58, 0.85), (82, 72, 0.6)]:
        for ox, rx, ry in [(-16, 12, 5), (-4, 16, 7), (12, 13, 6), (24, 8, 3)]:
            rx, ry = rx * size, ry * size
            for y in range(int(cloud_y - ry), int(cloud_y + ry) + 1):
                for x in range(int(cloud_x + ox * size - rx), int(cloud_x + ox * size + rx) + 1):
                    if ((x - (cloud_x + ox * size)) / rx) ** 2 + ((y - cloud_y) / ry) ** 2 <= 1:
                        img[y][x % w] = CLOUD_DARK if y > cloud_y + ry * 0.2 else CLOUD
    far = [(10, 3, 0.4), (6, 7, 1.9), (2, 17, 0.2)]
    near = [(7, 5, 2.2), (4, 11, 0.7), (1.5, 29, 1.2)]
    for x in range(w):
        for y in range(int(ridge(x, 98, far)), h):
            img[y][x] = FAR
        for y in range(int(ridge(x, 114, near)), h):
            img[y][x] = NEAR
    return img


# -------------------------------------------------------------------- ground
GROUND_PAL = [rgb(c) for c in ["#000000", "#c9e86f", "#86c24f", "#5a9a3e", "#3d7434", "#9a6a43", "#7c5236", "#5e3d2a", "#452c20", "#b89a78", "#d6bd9a"]]


def ground():
    rng = random.Random(11)
    w, h = 64, 32
    img = canvas(w, h)
    for x in range(w):
        top = 2 + round(math.sin(2 * math.pi * x / 16) + 0.6 * math.sin(2 * math.pi * x / 32 + 1))
        for y in range(top, h):
            depth = y - top
            if depth == 0:
                c = 1
            elif depth < 3:
                c = 2
            elif depth < 5:
                c = 3
            elif depth == 5 or (depth == 6 and x % 5 < 2):
                c = 4
            else:
                c = 5 + min((y - 8) // 8, 2)
            img[y][x] = c
    for _ in range(9):
        px, py = rng.randrange(w), rng.randrange(13, 29)
        for dy in range(2):
            for dx in range(3):
                img[py + dy][(px + dx) % w] = 10 if dy == 0 and dx < 2 else 9
        img[py + 2][(px + 1) % w] = 8
    for _ in range(26):
        px, py = rng.randrange(w), rng.randrange(11, 31)
        if img[py][px] in (5, 6, 7):
            img[py][px] = 8
    return img


# ------------------------------------------------------------------- columns
COLUMN_PAL = [rgb(c) for c in ["#000000", "#f1e8da", "#d2c6bb", "#aa9fa9", "#807893", "#585173", "#38324f", "#8db850", "#5f8a3c"]]


def column_upper():
    """Stone column hanging from the sky, capital at the bottom."""
    rng = random.Random(3)
    w, h = 32, 128
    img = canvas(w, h)
    for y in range(h):
        course = y // 8
        for x in range(3, 29):
            shade = 1 + min((x - 3) * 5 // 26, 4)
            if y % 8 == 7 or (x + (4 if course % 2 else 12)) % 16 == 0:
                shade = 6
            elif y % 8 == 0 and shade < 5:
                shade = max(shade - 1, 1)
            img[y][x] = shade
        img[y][2] = img[y][29] = 6
    for y in range(h - 13, h):
        for x in range(32):
            shade = 1 + min(x * 5 // 32, 4)
            img[y][x] = shade
        img[y][0] = img[y][31] = 6
    for x in range(32):
        img[h - 13][x] = 6
        img[h - 1][x] = 6
        img[h - 5][x] = 5
    for _ in range(8):
        x = rng.randrange(1, 30)
        for dy in range(rng.randrange(1, 4)):
            for dx in range(rng.randrange(2, 4)):
                if x + dx < 31:
                    img[h - 2 - dy][x + dx] = 7 if dy == 0 else 8
    return img


def columns():
    upper = column_upper()
    lower = upper[::-1]
    out = canvas(64, 128)
    blit(out, upper, 0, 0)
    blit(out, lower, 32, 0)
    return out


# ---------------------------------------------------------------- treehopper
HOPPER_PAL = [rgb(c) for c in ["#000000", "#1f2a1a", "#c6e85a", "#86c63e", "#4f8f34", "#d8432e", "#f3df62", "#ffffff", "#d8f2ee", "#9cc8c4", "#3a3a24"]]
BODY_LIGHT, BODY, BODY_DARK, STRIPE, HORN_TIP, EYE_SHINE, WING, WING_VEIN, LEG = 2, 3, 4, 5, 6, 7, 8, 9, 10


def fill_polygon(img, points, color):
    """Scanline fill. Returns the (x, y) pixels it painted."""
    painted = []
    ys = [p[1] for p in points]
    for y in range(min(ys), max(ys) + 1):
        crossings = []
        for (x0, y0), (x1, y1) in zip(points, points[1:] + points[:1]):
            if y0 != y1 and min(y0, y1) <= y < max(y0, y1):
                crossings.append(x0 + (y - y0) * (x1 - x0) / (y1 - y0))
        crossings.sort()
        for a, b in zip(crossings[::2], crossings[1::2]):
            for x in range(round(a), round(b) + 1):
                img[y][x] = color
                painted.append((x, y))
    return painted


def treehopper(wing):
    """A thorn-backed treehopper in profile, facing right."""
    img = canvas(32, 32)
    wings = {
        "up": ([(16, 14), (7, 3), (3, 5), (9, 15)], [(14, 13), (6, 5)]),
        "mid": ([(15, 14), (3, 9), (1, 13), (9, 17)], [(13, 14), (3, 11)]),
        "down": ([(15, 16), (8, 16), (2, 24), (6, 27)], [(12, 17), (5, 25)]),
    }
    outline_points, (vein_a, vein_b) = wings[wing]
    fill_polygon(img, outline_points, WING)
    steps = max(abs(vein_b[0] - vein_a[0]), abs(vein_b[1] - vein_a[1]))
    for i in range(steps + 1):
        img[vein_a[1] + (vein_b[1] - vein_a[1]) * i // steps][vein_a[0] + (vein_b[0] - vein_a[0]) * i // steps] = WING_VEIN
    for points in ([(13, 22), (11, 26)], [(18, 22), (17, 26)], [(23, 21), (25, 25)]):  # legs
        (x0, y0), (x1, y1) = points
        for i in range(5):
            img[y0 + (y1 - y0) * i // 4][x0 + (x1 - x0) * i // 4] = LEG
    body = [(28, 16), (26, 11), (22, 9), (19, 2), (17, 9), (12, 12), (7, 15), (3, 20), (9, 22), (16, 23), (23, 22), (27, 20)]
    pixels = fill_polygon(img, body, BODY)
    tops, bottoms = {}, {}
    for x, y in pixels:
        tops[x] = min(tops.get(x, y), y)
        bottoms[x] = max(bottoms.get(x, y), y)
    for x, y in pixels:
        if y <= 6:
            c = HORN_TIP if y <= 4 else STRIPE
        elif 21 <= x + y <= 23 or 31 <= x + y <= 33:
            c = STRIPE
        elif y - tops[x] <= 1:
            c = BODY_LIGHT
        elif bottoms[x] - y <= 2:
            c = BODY_DARK
        else:
            c = BODY
        img[y][x] = c
    for x, y in [(24, 15), (25, 15), (24, 16), (25, 16)]:  # eye
        img[y][x] = STRIPE
    img[15][25] = EYE_SHINE
    return outline(img, 1)


def hopper_sheet():
    out = canvas(128, 32)
    for i, wing in enumerate(["mid", "up", "mid", "down"]):
        blit(out, treehopper(wing), i * 32, 0)
    return out


# ---------------------------------------------------------------------- text
FONT = {
    "A": "01110 10001 10001 11111 10001 10001 10001",
    "B": "11110 10001 10001 11110 10001 10001 11110",
    "E": "11111 10000 10000 11110 10000 10000 11111",
    "F": "11111 10000 10000 11110 10000 10000 10000",
    "G": "01111 10000 10000 10111 10001 10001 01111",
    "H": "10001 10001 10001 11111 10001 10001 10001",
    "I": "11111 00100 00100 00100 00100 00100 11111",
    "L": "10000 10000 10000 10000 10000 10000 11111",
    "M": "10001 11011 10101 10101 10001 10001 10001",
    "N": "10001 11001 10101 10011 10001 10001 10001",
    "O": "01110 10001 10001 10001 10001 10001 01110",
    "P": "11110 10001 10001 11110 10000 10000 10000",
    "R": "11110 10001 10001 11110 10100 10010 10001",
    "S": "01111 10000 10000 01110 00001 00001 11110",
    "V": "10001 10001 10001 10001 10001 01010 00100",
    "Y": "10001 10001 01010 00100 00100 00100 00100",
    " ": "00000 00000 00000 00000 00000 00000 00000",
    "0": "01110 10001 10011 10101 11001 10001 01110",
    "1": "00100 01100 00100 00100 00100 00100 01110",
    "2": "01110 10001 00001 00110 01000 10000 11111",
    "3": "11110 00001 00001 01110 00001 00001 11110",
    "4": "00010 00110 01010 10010 11111 00010 00010",
    "5": "11111 10000 11110 00001 00001 10001 01110",
    "6": "00110 01000 10000 11110 10001 10001 01110",
    "7": "11111 00001 00010 00100 01000 01000 01000",
    "8": "01110 10001 10001 01110 10001 10001 01110",
    "9": "01110 10001 10001 01111 00001 00010 01100",
}
TEXT_PAL = [rgb(c) for c in ["#000000", "#ffffff", "#ffd35a", "#e8572a", "#2a1f3a"]]
BUTTON_PAL = [rgb(c) for c in ["#000000", "#e8572a", "#ffffff", "#7a2412"]]


def put_text(img, text, x, y, color, sx=1, sy=1, gap=0):
    for ch in text:
        for gy, row in enumerate(FONT[ch].split()):
            for gx, bit in enumerate(row):
                if bit == "1":
                    for yy in range(sy):
                        for xx in range(sx):
                            img[y + gy * sy + yy][x + gx * sx + xx] = color
        x += 6 * sx + gap


def centered(img, text, y, color, sx=2, sy=2, gap=2):
    width = len(text) * (6 * sx + gap) - sx - gap
    put_text(img, text, (len(img[0]) - width) // 2, y, color, sx, sy, gap)


def banner(line1, line2, color):
    img = canvas(96, 32)
    centered(img, line1, 1, color)
    centered(img, line2, 17, color)
    return outline(img, 4)


def digit(d):
    img = canvas(16, 16)
    put_text(img, str(d), 3, 1, 1, 2, 2)
    return outline(img, 4)


def button():
    img = canvas(48, 16)
    for y in range(1, 15):
        inset = 2 if y in (1, 14) else 1 if y in (2, 13) else 0
        for x in range(inset, 48 - inset):
            img[y][x] = 3 if y == 14 or y == 13 else 1
    put_text(img, "PRESS A", 3, 4, 2)
    return img


# --------------------------------------------------------------------- sound
RATE = 10512


def clip(samples):
    return [max(-127, min(127, int(s))) for s in samples]


def sfx_wing():
    rng = random.Random(1)
    out, low = [], 0.0
    n = int(RATE * 0.11)
    for i in range(n):
        t = i / n
        low += (rng.uniform(-1, 1) - low) * (0.5 - 0.35 * t)
        tone = math.sin(2 * math.pi * (260 + 520 * t) * i / RATE)
        out.append((low * 170 + tone * 26) * math.sin(math.pi * t) ** 0.7)
    return clip(out)


def sfx_point():
    out = []
    for freq, length in ((988, 0.07), (1319, 0.15)):
        n = int(RATE * length)
        for i in range(n):
            square = 1 if (i * freq / RATE) % 1 < 0.5 else -1
            out.append(square * 62 * (1 - i / n) ** 1.3)
    return clip(out)


def sfx_hit():
    rng = random.Random(2)
    out, low = [], 0.0
    n = int(RATE * 0.2)
    for i in range(n):
        t = i / n
        low += (rng.uniform(-1, 1) - low) * 0.22
        tone = math.sin(2 * math.pi * (150 - 90 * t) * i / RATE)
        out.append((low * 260 + tone * 80) * (1 - t) ** 2)
    return clip(out)


def sfx_die():
    out, phase = [], 0.0
    n = int(RATE * 0.38)
    for i in range(n):
        t = i / n
        phase += (620 - 470 * t) / RATE
        square = 1 if phase % 1 < 0.5 else -1
        out.append(square * 58 * (1 - t))
    return clip(out)


# -------------------------------------------------------------------- output
def array(ctype, name, values, attr=" __attribute__((aligned(4)))"):
    lines = [f"const {ctype} {name}[{len(values)}]{attr} = {{"]
    for i in range(0, len(values), 16):
        lines.append("  " + ",".join(str(v) for v in values[i : i + 16]) + ",")
    lines.append("};")
    return "\n".join(lines) + "\n"


def main():
    out = sys.argv[1]
    c = ['#include "assets.h"\n']
    h = ["#ifndef ASSETS_H\n#define ASSETS_H\n"]

    def emit(name, values, ctype="unsigned short"):
        c.append(array(ctype, name, values))
        h.append(f"extern const {ctype} {name}[{len(values)}];")
        h.append(f"#define {name}Len {len(values) * (2 if ctype == 'unsigned short' else 1)}")

    bg_tiles, bg_map, bg_count = tile_map(background(), 1)
    assert bg_count <= 512, bg_count
    fg_tiles, fg_map, fg_count = tile_map(ground(), 0)
    assert fg_count <= 512, fg_count
    print(f"background {bg_count} tiles, ground {fg_count} tiles")

    emit("backgroundTiles", bg_tiles)
    emit("backgroundMap", bg_map)
    emit("backgroundPal", BG_PAL)
    emit("groundTiles", fg_tiles)
    emit("groundMap", fg_map)
    emit("groundPal", GROUND_PAL)
    emit("hopperTiles", sprite_blocks(hopper_sheet(), 32))
    emit("hopperPal", HOPPER_PAL)
    emit("columnTiles", sprite_blocks(columns(), 32))
    emit("columnPal", COLUMN_PAL)
    emit("buttonTiles", sprite_blocks(button(), 16))
    emit("buttonPal", BUTTON_PAL)
    text = sprite_blocks(banner("FLAPPY", "HOPPER", 2), 32) + sprite_blocks(banner("GAME", "OVER", 1), 32)
    for d in range(10):
        text += sprite_tiles(digit(d))
    emit("textTiles", text)
    emit("textPal", TEXT_PAL)
    for name, data in (("sfxWing", sfx_wing()), ("sfxPoint", sfx_point()), ("sfxHit", sfx_hit()), ("sfxDie", sfx_die())):
        emit(name, data, "signed char")

    h.append("\n#endif")
    open(f"{out}/assets.c", "w").write("\n".join(c))
    open(f"{out}/assets.h", "w").write("\n".join(h) + "\n")


if __name__ == "__main__":
    main()
