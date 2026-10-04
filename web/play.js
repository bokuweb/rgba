import init, { GbaHandle } from './pkg/rgba.js';

// Play mode: an HTML/CSS front-view handheld around the emulator, with touch, keyboard and
// gamepad input and 59.73 Hz frame pacing. The debugger (index.html) shares the
// same wasm core and ROM library; the mode switch carries `?rom=` across.

// ---------------------------------------------------------------- helpers
const $ = (id) => document.getElementById(id);
const fmtSize = (n) => n >= 1 << 20 ? `${(n / (1 << 20)).toFixed(1)} MB` : `${Math.ceil(n / 1024)} KB`;
const store = {
  get(k) { try { return localStorage.getItem(k); } catch { return null; } },
  set(k, v) { try { localStorage.setItem(k, v); } catch { /* private mode */ } },
};

// ---------------------------------------------------------------- state
let gba = null;
let romBytes = null;
let romId = null;
let paused = false;
// The POWER switch on the page; off stops the core, on cold-boots the ROM again.
let powered = $('power').checked;
const shell = $('gba');

const canvas = $('screen');
const ctx = canvas.getContext('2d');
const image = ctx.createImageData(240, 160);

// GBA hardware frame rate: 16.78 MHz / 280896 cycles per frame.
const FRAME_MS = 1000 / 59.7275;

// ---------------------------------------------------------------- input
// Key bits match GbaHandle.setKey: 0=A 1=B 2=Select 3=Start 4=Right 5=Left
// 6=Up 7=Down 8=R 9=L. Keyboard, touch pointers and gamepads each keep their
// own set; the union is diffed against what the core last saw.
const KEYMAP = {
  KeyZ: 0, KeyX: 1, Space: 2, Enter: 3,
  ArrowRight: 4, ArrowLeft: 5, ArrowUp: 6, ArrowDown: 7,
  KeyS: 8, KeyA: 9,
};
const kbKeys = new Set();
const padKeys = new Set();
const pointerKeys = new Map(); // pointerId -> Set<bit>
const current = new Array(10).fill(false);
// Keys pressed since the last emulated frame. Their release is held back until
// a frame has run, so a tap shorter than 16 ms still reaches the game.
const fresh = new Set();
const keyEls = [...Array(10).keys()].map((i) => document.querySelectorAll(`[data-key="${i}"]`));

function syncKeys() {
  const want = new Set([...kbKeys, ...padKeys]);
  for (const s of pointerKeys.values()) for (const k of s) want.add(k);
  for (let i = 0; i < 10; i++) {
    const on = want.has(i);
    if (on === current[i] || (!on && fresh.has(i))) continue;
    current[i] = on;
    if (on) fresh.add(i);
    if (gba) gba.setKey(i, on);
    for (const el of keyEls[i]) el.classList.toggle('pressed', on);
  }
}

addEventListener('keydown', (e) => {
  if (e.target instanceof Element && e.target.matches('input,select,textarea')) return;
  unlockAudio();
  if (e.code === 'KeyP' && !e.repeat) { togglePause(); return; }
  if (e.code in KEYMAP) { kbKeys.add(KEYMAP[e.code]); syncKeys(); e.preventDefault(); }
});
addEventListener('keyup', (e) => {
  if (e.code in KEYMAP) { kbKeys.delete(KEYMAP[e.code]); syncKeys(); }
});
addEventListener('blur', () => { kbKeys.clear(); pointerKeys.clear(); fresh.clear(); syncKeys(); });

// Touch / mouse: hit-test the point every move so a thumb can slide between
// buttons. The d-pad is resolved by angle from its centre (8 directions, so
// diagonals work) instead of by which arm is under the finger.
const DPAD_SECTORS = [[4], [4, 7], [7], [7, 5], [5], [5, 6], [6], [6, 4]]; // from +x, clockwise (y down)
function keysAt(x, y) {
  const el = document.elementFromPoint(x, y);
  if (!el || !shell.contains(el)) return [];
  const dpad = el.closest('.dpad');
  if (dpad) {
    const r = dpad.getBoundingClientRect();
    const dx = x - (r.left + r.width / 2), dy = y - (r.top + r.height / 2);
    if (Math.hypot(dx, dy) < r.width * 0.1) return [];
    const sector = (Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) + 8) % 8;
    return DPAD_SECTORS[sector];
  }
  const btn = el.closest('[data-key]');
  return btn ? [Number(btn.dataset.key)] : [];
}
function trackPointer(e) {
  pointerKeys.set(e.pointerId, new Set(keysAt(e.clientX, e.clientY)));
  syncKeys();
}
shell.addEventListener('pointerdown', (e) => {
  e.preventDefault();
  unlockAudio();
  try { shell.setPointerCapture(e.pointerId); } catch { /* pointer already gone */ }
  trackPointer(e);
});
shell.addEventListener('pointermove', (e) => { if (pointerKeys.has(e.pointerId)) trackPointer(e); });
for (const t of ['pointerup', 'pointercancel', 'lostpointercapture']) {
  shell.addEventListener(t, (e) => { pointerKeys.delete(e.pointerId); syncKeys(); });
}
shell.addEventListener('contextmenu', (e) => e.preventDefault());

// Standard-mapping gamepads: Nintendo layout (right face = A, bottom = B).
const PAD_BUTTONS = { 1: 0, 0: 1, 8: 2, 9: 3, 15: 4, 14: 5, 12: 6, 13: 7, 5: 8, 4: 9, 7: 8, 6: 9 };
function pollGamepads() {
  if (!navigator.getGamepads) return;
  padKeys.clear();
  for (const gp of navigator.getGamepads()) {
    if (!gp) continue;
    for (const [idx, bit] of Object.entries(PAD_BUTTONS)) if (gp.buttons[idx]?.pressed) padKeys.add(bit);
    const [ax = 0, ay = 0] = gp.axes;
    if (ax > 0.5) padKeys.add(4); else if (ax < -0.5) padKeys.add(5);
    if (ay < -0.5) padKeys.add(6); else if (ay > 0.5) padKeys.add(7);
  }
  syncKeys();
}

// ---------------------------------------------------------------- audio
// Browsers only start an AudioContext after a user gesture, so sound is armed
// on the first tap / key press and the button just mutes / unmutes.
let audioCtx = null, audioNode = null, audioStarting = null;
let soundOn = store.get('rgba.sound') !== 'off';
let gestured = false;

function renderSoundBtn() {
  $('sound').classList.toggle('on', soundOn);
  $('sound').textContent = soundOn ? 'SOUND ON' : 'SOUND OFF';
}
async function ensureAudio() {
  if (audioCtx || !gba) return;
  if (!audioStarting) {
    audioStarting = (async () => {
      const ac = new AudioContext({ sampleRate: gba.sampleRate() });
      await ac.audioWorklet.addModule('./audio-processor.js');
      audioNode = new AudioWorkletNode(ac, 'gba-audio', { outputChannelCount: [2] });
      audioNode.connect(ac.destination);
      audioCtx = ac;
    })();
  }
  await audioStarting;
}
async function applySound() {
  if (soundOn && gestured && !paused && powered && gba) {
    await ensureAudio();
    if (audioCtx.state !== 'running') audioCtx.resume();
  } else if (audioCtx && audioCtx.state === 'running') {
    audioCtx.suspend();
  }
}
function unlockAudio() {
  if (gestured) return;
  gestured = true;
  applySound();
}
$('sound').addEventListener('click', () => {
  soundOn = !soundOn;
  store.set('rgba.sound', soundOn ? 'on' : 'off');
  renderSoundBtn();
  gestured = true;
  applySound();
});
renderSoundBtn();

// Always drain so the core's sample queue never grows while muted.
function drainAudio() {
  const s = gba.takeAudioF32();
  if (s.length && audioNode && soundOn && audioCtx?.state === 'running') {
    audioNode.port.postMessage(s, [s.buffer]);
  }
}

// ---------------------------------------------------------------- run loop
let acc = 0, last = 0;
function loop(t) {
  requestAnimationFrame(loop);
  pollGamepads();
  const dt = Math.min(t - last, 100); // clamp after tab switches / hitches
  last = t;
  if (!gba || paused || !powered) return;
  acc += dt;
  let fb = null, n = 0;
  // Fixed-step at the GBA's own rate so 120/144 Hz displays don't speed it up.
  while (acc >= FRAME_MS && n < 4) {
    fb = gba.runFrame();
    drainAudio();
    if (fresh.size) { fresh.clear(); syncKeys(); }
    acc -= FRAME_MS;
    n++;
  }
  if (acc > FRAME_MS * 4) acc = 0;
  if (fb) { image.data.set(fb); ctx.putImageData(image, 0, 0); }
}
requestAnimationFrame((t) => { last = t; loop(t); });

function setPaused(p) {
  paused = p;
  shell.classList.toggle('paused', p);
  $('pause').textContent = p ? 'RESUME' : 'PAUSE';
  $('pause').classList.toggle('on', p);
  showOverlay(p ? 'PAUSED' : '');
  applySound();
}
function showOverlay(text) {
  const ov = $('overlay');
  ov.hidden = !text;
  if (text) ov.firstElementChild.textContent = text;
}
function togglePause() { if (gba) setPaused(!paused); }
$('pause').addEventListener('click', togglePause);
$('reset').addEventListener('click', () => { if (romBytes) boot(romBytes); });
document.addEventListener('visibilitychange', () => {
  if (document.hidden && gba && !paused) setPaused(true);
});

$('full').addEventListener('click', () => {
  if (document.fullscreenElement) document.exitFullscreen();
  else $('stage').requestFullscreen?.().catch(() => {});
});

$('power').addEventListener('change', (e) => {
  powered = e.target.checked;
  if (powered && romBytes) boot(romBytes);
  else applySound();
});

// ---------------------------------------------------------------- shell color
// The colour radios drive the device CSS via :has(); just remember the choice.
const COLORS = ['indigo', 'platinum', 'glacier'];
const savedColor = store.get('rgba.color');
if (COLORS.includes(savedColor)) $(savedColor).checked = true;
for (const c of COLORS) $(c).addEventListener('change', () => store.set('rgba.color', c));

// ---------------------------------------------------------------- boot / ROMs
function setStatus(html) { $('romStatus').innerHTML = html; }

function boot(bytes) {
  gba = new GbaHandle(bytes);
  for (let i = 0; i < 10; i++) if (current[i]) gba.setKey(i, true);
  acc = 0;
  shell.classList.add('power');
  for (const b of ['pause', 'reset']) $(b).disabled = false;
  setPaused(false);
}

// Keep the mode switch pointing at the same ROM.
function syncModeLink() {
  $('modeDebug').href = romId ? `./index.html?rom=${encodeURIComponent(romId)}` : './index.html';
}

$('rom').addEventListener('change', async (e) => {
  const file = e.target.files[0];
  if (!file) return;
  romBytes = new Uint8Array(await file.arrayBuffer());
  romId = null;
  $('romSel').value = '';
  history.replaceState(null, '', location.pathname);
  syncModeLink();
  setStatus(`${file.name} · ${fmtSize(file.size)}`);
  boot(romBytes);
});

let library = [];
async function loadLibrary() {
  try {
    const res = await fetch('./roms.json');
    if (!res.ok) return;
    const manifest = await res.json();
    const sel = $('romSel');
    for (const g of manifest.groups) {
      const grp = document.createElement('optgroup');
      grp.label = g.name;
      for (const r of g.roms) {
        library.push(r);
        const o = document.createElement('option');
        o.value = r.id;
        o.textContent = r.size ? `${r.title} (${fmtSize(r.size)})` : r.title;
        grp.appendChild(o);
      }
      sel.appendChild(grp);
    }
  } catch (e) {
    console.warn('roms.json unavailable', e);
  }
}

async function loadLibraryRom(id, { deepLink = true } = {}) {
  const r = library.find((x) => x.id === id);
  if (!r) return;
  const sel = $('romSel');
  sel.disabled = true;
  showOverlay('LOADING');
  setStatus(`fetching ${r.title}…`);
  try {
    const res = await fetch(`./roms/${r.file}`);
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    romBytes = new Uint8Array(await res.arrayBuffer());
    romId = id;
    setStatus([
      `<span class="lic">${r.title}</span>`,
      r.author ? `by ${r.author}` : '',
      r.event || '',
      r.license,
      r.source ? `<a href="${r.source}" target="_blank" rel="noopener">source</a>` : '',
    ].filter(Boolean).join(' · '));
    boot(romBytes);
    history.replaceState(null, '', deepLink ? `?rom=${encodeURIComponent(id)}` : location.pathname);
    syncModeLink();
  } catch (e) {
    showOverlay('NO CARTRIDGE');
    setStatus(`<span class="err">failed to load ${r.title}: ${e.message}</span>`);
  } finally {
    sel.disabled = false;
  }
}
$('romSel').addEventListener('change', (e) => {
  if (e.target.value) { loadLibraryRom(e.target.value); e.target.blur(); }
});

await init();
await loadLibrary();

const wanted = new URLSearchParams(location.search).get('rom');
const fallback = library.find((r) => r.default);
if (wanted && library.some((r) => r.id === wanted)) {
  $('romSel').value = wanted;
  loadLibraryRom(wanted);
} else if (fallback) {
  $('romSel').value = fallback.id;
  loadLibraryRom(fallback.id, { deepLink: false });
} else {
  showOverlay('NO CARTRIDGE');
}
