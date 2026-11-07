// VRAM tile and palette viewer for the debugger page. It only reads through
// GbaHandle.readMemory, so it shows what the emulated program has stored.

const VRAM = 0x06000000;
const PALETTE = 0x05000000;
const OAM = 0x07000000;
const DISPCNT = 0x04000000;
const BG0CNT = 0x04000008;

const REGIONS = {
  bg: { base: VRAM, bytes: 0x10000, palette: PALETTE, palBase: 0 },
  obj: { base: VRAM + 0x10000, bytes: 0x8000, palette: PALETTE + 0x200, palBase: 256 },
};
const REFRESH_FRAMES = 10; // VRAM is 96 KB, so don't reread it every frame

// [width, height] in tiles, indexed by OAM shape then size.
const OBJ_SIZES = [
  [[1, 1], [2, 2], [4, 4], [8, 8]],
  [[2, 1], [4, 1], [4, 2], [8, 4]],
  [[1, 2], [1, 4], [2, 4], [4, 8]],
];

const hex = (v, w) => (v >>> 0).toString(16).padStart(w, '0');
const u16 = (b, i) => b[i] | (b[i + 1] << 8);

function toRgba(c) {
  const up = (v) => (v << 3) | (v >> 2);
  const r = up(c & 31), g = up((c >> 5) & 31), b = up((c >> 10) & 31);
  return (0xff000000 | (b << 16) | (g << 8) | r) >>> 0;
}

export function initVramViewer(getGba, openInMemory) {
  const $ = (id) => document.getElementById(id);
  const tilesCanvas = $('vramTiles');
  const palCanvas = $('vramPal');
  const info = $('vramInfo');
  const regionSel = $('vramRegion');
  const depthSel = $('vramDepth');
  const bankSel = $('vramBank');
  const widthSel = $('vramWidth');

  for (let i = 0; i < 16; i++) bankSel.add(new Option(`pal ${i}`, String(i)));

  let lastGba = null;
  let frames = 0;
  let visible = true;
  let palette = new Uint8Array(0x400);
  // Bank last seen for each 4bpp tile. It persists because hidden sprites and
  // screens that aren't showing still have tiles in VRAM.
  let seenBanks = { bg: new Int8Array(2048).fill(-1), obj: new Int8Array(1024).fill(-1) };

  new IntersectionObserver((entries) => {
    visible = entries[entries.length - 1].isIntersecting;
    if (visible) refresh();
  }).observe($('vramPane'));

  const region = () => REGIONS[regionSel.value];
  const bitsPerPixel = () => Number(depthSel.value);
  const tileBytes = () => bitsPerPixel() * 8;
  const tileCount = () => region().bytes / tileBytes();
  const tilesPerRow = () => Number(widthSel.value);

  function noteObjBanks(gba) {
    const oam = gba.readMemory(OAM, 0x400);
    const twoD = !(u16(gba.readMemory(DISPCNT, 2), 0) & 0x40);
    // Walk backwards so that object 0, which draws on top, wins any overlap.
    for (let i = 127; i >= 0; i--) {
      const a0 = u16(oam, i * 8), a1 = u16(oam, i * 8 + 2), a2 = u16(oam, i * 8 + 4);
      const shape = a0 >> 14;
      if (shape === 3 || a0 & 0x2000) continue; // prohibited shape, or 8bpp
      const [w, h] = OBJ_SIZES[shape][a1 >> 14];
      const first = a2 & 0x3ff;
      for (let y = 0; y < h; y++) {
        for (let x = 0; x < w; x++) {
          const tile = twoD ? first + y * 32 + x : first + y * w + x;
          if (tile < 1024) seenBanks.obj[tile] = a2 >> 12;
        }
      }
    }
  }

  function noteBgBanks(gba) {
    const dispcnt = u16(gba.readMemory(DISPCNT, 2), 0);
    const mode = dispcnt & 7;
    if (mode > 1) return; // modes 2+ have no 16-color text backgrounds
    const count = mode === 1 ? 2 : 4;
    const io = gba.readMemory(BG0CNT, 8);
    for (let bg = 0; bg < count; bg++) {
      if (!(dispcnt & (0x100 << bg))) continue;
      const cnt = u16(io, bg * 2);
      if (cnt & 0x80) continue; // 8bpp
      const firstTile = ((cnt >> 2) & 3) * 512;
      const block = (cnt >> 8) & 31;
      const blocks = [1, 2, 2, 4][cnt >> 14];
      const map = gba.readMemory(VRAM + block * 0x800, blocks * 0x800);
      for (let i = 0; i + 1 < map.length; i += 2) {
        const entry = u16(map, i);
        const tile = firstTile + (entry & 0x3ff);
        if (tile < 2048) seenBanks.bg[tile] = entry >> 12;
      }
    }
  }

  function drawTiles(gba) {
    const r = region();
    const data = gba.readMemory(r.base, r.bytes);
    const bpp = bitsPerPixel();
    const count = tileCount();
    const columns = tilesPerRow();
    const rows = Math.ceil(count / columns);
    const w = columns * 8, h = rows * 8;
    if (tilesCanvas.width !== w || tilesCanvas.height !== h) {
      tilesCanvas.width = w;
      tilesCanvas.height = h;
    }

    const colors = new Uint32Array(256);
    for (let i = 0; i < 256; i++) colors[i] = toRgba(u16(palette, (r.palBase + i) * 2));

    const banks = seenBanks[regionSel.value];
    const fixed = bankSel.value === 'auto' ? -1 : Number(bankSel.value);
    const ctx = tilesCanvas.getContext('2d');
    const image = ctx.createImageData(w, h);
    const px = new Uint32Array(image.data.buffer);
    const size = tileBytes();

    for (let t = 0; t < count; t++) {
      const ox = (t % columns) * 8, oy = Math.floor(t / columns) * 8;
      const start = t * size;
      const bank = (fixed >= 0 ? fixed : Math.max(banks[t], 0)) * 16;
      for (let y = 0; y < 8; y++) {
        for (let x = 0; x < 8; x++) {
          let index;
          if (bpp === 8) {
            index = data[start + y * 8 + x];
            if (index) px[(oy + y) * w + ox + x] = colors[index];
          } else {
            const b = data[start + y * 4 + (x >> 1)];
            index = x & 1 ? b >> 4 : b & 15;
            if (index) px[(oy + y) * w + ox + x] = colors[bank + index];
          }
        }
      }
    }
    ctx.putImageData(image, 0, 0);
  }

  function drawPalette() {
    const ctx = palCanvas.getContext('2d');
    const image = ctx.createImageData(33, 16);
    const px = new Uint32Array(image.data.buffer);
    for (let i = 0; i < 256; i++) {
      const y = i >> 4, x = i & 15;
      px[y * 33 + x] = toRgba(u16(palette, i * 2));
      px[y * 33 + 17 + x] = toRgba(u16(palette, (256 + i) * 2));
    }
    ctx.putImageData(image, 0, 0);
  }

  function refresh() {
    const gba = getGba();
    if (!gba) return;
    if (gba !== lastGba) {
      seenBanks.bg.fill(-1);
      seenBanks.obj.fill(-1);
      lastGba = gba;
    }
    palette = gba.readMemory(PALETTE, 0x400);
    noteObjBanks(gba);
    noteBgBanks(gba);
    drawTiles(gba);
    drawPalette();
  }

  function tick() {
    if (!visible || ++frames < REFRESH_FRAMES) return;
    frames = 0;
    refresh();
  }

  function tileAt(e) {
    const rect = tilesCanvas.getBoundingClientRect();
    const columns = tilesPerRow();
    const col = Math.floor(((e.clientX - rect.left) / rect.width) * columns);
    const row = Math.floor(((e.clientY - rect.top) / rect.height) * (tilesCanvas.height / 8));
    const tile = row * columns + col;
    return col >= 0 && col < columns && tile >= 0 && tile < tileCount() ? tile : -1;
  }

  tilesCanvas.addEventListener('mousemove', (e) => {
    const t = tileAt(e);
    if (t < 0) return;
    const addr = region().base + t * tileBytes();
    const seen = seenBanks[regionSel.value][t];
    const bank = bitsPerPixel() === 4 && seen >= 0 ? ` · pal ${seen}` : '';
    const block = Math.floor((addr - VRAM) / 0x4000);
    info.innerHTML = `tile <b>${t}</b> · <b>0x${hex(addr, 8)}</b> · charblock ${block}${bank}`;
  });
  tilesCanvas.addEventListener('click', (e) => {
    const t = tileAt(e);
    if (t >= 0) openInMemory(region().base + t * tileBytes());
  });

  function paletteEntry(e) {
    const rect = palCanvas.getBoundingClientRect();
    const x = Math.floor(((e.clientX - rect.left) / rect.width) * 33);
    const y = Math.floor(((e.clientY - rect.top) / rect.height) * 16);
    if (x === 16 || x < 0 || x > 32 || y < 0 || y > 15) return null;
    const isObj = x > 16;
    return { isObj, bank: y, index: y * 16 + (isObj ? x - 17 : x) };
  }

  palCanvas.addEventListener('mousemove', (e) => {
    const p = paletteEntry(e);
    if (!p) return;
    const offset = (p.isObj ? 256 + p.index : p.index) * 2;
    const c = u16(palette, offset);
    info.innerHTML = `${p.isObj ? 'OBJ' : 'BG'} pal ${p.bank} · color ${p.index & 15} · ` +
      `<b>0x${hex(PALETTE + offset, 8)}</b> · <b>${hex(c, 4)}</b> ` +
      `(r${c & 31} g${(c >> 5) & 31} b${(c >> 10) & 31})`;
  });
  palCanvas.addEventListener('click', (e) => {
    const p = paletteEntry(e);
    if (!p) return;
    regionSel.value = p.isObj ? 'obj' : 'bg';
    depthSel.value = '4';
    bankSel.value = String(p.bank);
    refresh();
  });

  for (const sel of [regionSel, depthSel, bankSel, widthSel]) sel.addEventListener('change', refresh);

  return { refresh, tick };
}
