import init, { GbaHandle } from './pkg/rusty_gba.js';

// ---------------------------------------------------------------- helpers
const $ = (id) => document.getElementById(id);
const hex = (v, w = 8) => (v >>> 0).toString(16).padStart(w, '0');
const parseAddr = (s) => (parseInt(String(s).trim().replace(/^0x/i, ''), 16) >>> 0) || 0;

// ---------------------------------------------------------------- state
let gba = null;
let romBytes = null;
let running = false;
let rafId = 0;
let prevRegs = new Array(16).fill(0);
const bps = new Set();          // breakpoint addresses (mirrors wasm)
let memEditing = false;

// audio
let audioCtx = null, audioNode = null, audioOn = false;

// canvas
const canvas = $('screen');
const ctx = canvas.getContext('2d');
const image = ctx.createImageData(240, 160);

// audio scope (oscilloscope) — per-channel ring buffers filled from the audio
// stream every frame, independent of whether playback is enabled.
const scopeCanvas = $('scope');
const scopeCtx = scopeCanvas.getContext('2d');
const SCOPE_SAMPLES = 1024;
let scopeL = new Float32Array(SCOPE_SAMPLES);
let scopeR = new Float32Array(SCOPE_SAMPLES);
let scopePos = 0;

// fps meter (wall-clock emulated frames/sec, updated ~2x/sec)
let fpsFrames = 0, fpsLast = performance.now();

// layer isolation
const MODE_KIND = ['tiled', 'tiled', 'tiled', 'bitmap', 'bitmap', 'bitmap'];
const isolateBox = $('isolate');
const lyrBoxes = Array.from(document.querySelectorAll('.lyrbox'));

// GBA key bit order expected by GbaHandle.setKey
const KEYMAP = {
  KeyZ: 0, KeyX: 1, Space: 2, Enter: 3,
  ArrowRight: 4, ArrowLeft: 5, ArrowUp: 6, ArrowDown: 7,
  KeyS: 8, KeyA: 9,
};

// ---------------------------------------------------------------- render
function paint(buf) {
  image.data.set(buf);
  ctx.putImageData(image, 0, 0);
}

function refreshStatus() {
  $('pcval').textContent = hex(gba.pc());
  $('modeval').textContent = gba.isThumb() ? 'THUMB' : 'ARM';
  // Monotonic cycle count — visible proof the core is advancing even when the
  // PC is parked in a VBlank-wait loop at frame boundaries.
  $('cycval').textContent = 'cyc ' + Math.round(gba.cycles()).toLocaleString();
  const m = gba.videoMode();
  $('vmodeval').textContent = `MODE ${m} · ${MODE_KIND[m] || '?'}`;
  syncLayerUi();
}

function refreshRegs() {
  // NB: gba.registers() is a Uint32Array. Calling .map() on a typed array
  // returns another *typed* array (coercing our HTML strings to NaN -> 0), so
  // copy to a plain Array first or every register renders as "00000000".
  const regs = Array.from(gba.registers());
  const names = [...Array(13).keys()].map((i) => 'r' + i).concat(['sp', 'lr', 'pc']);
  $('reggrid').innerHTML = regs.map((v, i) => {
    const changed = v !== prevRegs[i] ? ' changed' : '';
    return `<div class="reg${changed}"><span class="rn">${names[i]}</span><span class="rv">${hex(v)}</span></div>`;
  }).join('');
  prevRegs = Array.from(regs);

  const c = gba.cpsr();
  const flag = (bit, ch) => `<span class="flag ${(c & (1 << bit)) ? 'on' : ''}">${ch}</span>`;
  $('flags').innerHTML =
    flag(31, 'N') + flag(30, 'Z') + flag(29, 'C') + flag(28, 'V') +
    flag(7, 'I') + flag(6, 'F') + flag(5, 'T');
  const mode = (c & 0x1f).toString(16);
  $('psr').innerHTML = `cpsr <b>${hex(c)}</b>  mode 0x${mode}`;
}

function refreshDisasm() {
  const thumb = gba.isThumb();
  const step = thumb ? 2 : 4;
  const pc = gba.pc();
  const start = (pc - step * 4) >>> 0;
  const lines = JSON.parse(gba.disassemble(start, 28, thumb));
  $('disasmBody').innerHTML = lines.map((l) => {
    const cls = (l.addr === pc ? ' pc' : '') + (bps.has(l.addr) ? ' bp' : '');
    return `<div class="dline${cls}" data-addr="${l.addr}">` +
      `<span class="gutter">●</span>` +
      `<span class="addr">${hex(l.addr)}</span>` +
      `<span class="byt">${l.bytes}</span>` +
      `<span class="txt">${l.text}</span></div>`;
  }).join('');
}

const ASCII = (b) => (b >= 0x20 && b < 0x7f) ? String.fromCharCode(b) : '.';

function refreshMem() {
  if (memEditing) return; // don't clobber an in-progress edit
  const base = parseAddr($('memAddr').value);
  const ROWS = 12, COLS = 16;
  const data = gba.readMemory(base, ROWS * COLS);
  let html = '';
  for (let r = 0; r < ROWS; r++) {
    const rowAddr = (base + r * COLS) >>> 0;
    let hexs = '', asc = '';
    for (let cidx = 0; cidx < COLS; cidx++) {
      const b = data[r * COLS + cidx];
      const a = (rowAddr + cidx) >>> 0;
      const cls = b ? 'nz' : 'zero';
      hexs += `<span class="byte ${cls}" data-addr="${a}">${hex(b, 2)}</span>` + (cidx === 7 ? ' ' : '');
      asc += ASCII(b);
    }
    html += `<div class="mrow"><span class="maddr">${hex(rowAddr)}</span>` +
      `<span class="mhex">${hexs}</span><span class="masc">${asc}</span></div>`;
  }
  $('memBody').innerHTML = html;
}

function refreshAll() {
  refreshStatus();
  refreshRegs();
  refreshDisasm();
  refreshMem();
}

// ---------------------------------------------------------------- run loop
function setRunning(on) {
  running = on;
  $('run').disabled = on || !gba;
  $('pause').disabled = !on;
  $('step').disabled = on || !gba;
  $('pcbox').classList.toggle('live', on);
  if (on) { resumeAudio(); loop(); } else { cancelAnimationFrame(rafId); }
}

function loop() {
  if (!running) return;
  const hitBp = gba.runUntilBreak();
  paint(gba.framebuffer());
  drainAudio();
  drawScope();
  tickFps();
  refreshStatus();
  refreshRegs();
  refreshDisasm();
  if (!memEditing) refreshMem();
  if (hitBp) { setRunning(false); flashBreak(); return; }
  rafId = requestAnimationFrame(loop);
}

function flashBreak() {
  const pcLine = document.querySelector('.dline.pc');
  if (pcLine) pcLine.scrollIntoView({ block: 'center' });
}

// ---------------------------------------------------------------- audio
async function ensureAudio() {
  if (audioCtx) return;
  audioCtx = new AudioContext({ sampleRate: gba.sampleRate() });
  await audioCtx.audioWorklet.addModule('./audio-processor.js');
  audioNode = new AudioWorkletNode(audioCtx, 'gba-audio', { outputChannelCount: [2] });
  audioNode.connect(audioCtx.destination);
}
function resumeAudio() { if (audioOn && audioCtx && audioCtx.state === 'suspended') audioCtx.resume(); }

// Drain the emulator's audio every frame: always copy into the scope ring (so
// the waveform shows even with playback muted), and forward to the AudioWorklet
// only when playback is on. Draining unconditionally also keeps the core's
// sample queue from growing while muted.
function drainAudio() {
  if (!gba) return;
  const s = gba.takeAudioF32(); // interleaved L,R in [-1, 1]
  if (!s.length) return;
  for (let i = 0; i + 1 < s.length; i += 2) {
    scopeL[scopePos] = s[i];
    scopeR[scopePos] = s[i + 1];
    scopePos = (scopePos + 1) % SCOPE_SAMPLES;
  }
  // Read scope samples above BEFORE transferring the buffer to the worklet.
  if (audioOn && audioNode) audioNode.port.postMessage(s, [s.buffer]);
}

function drawScope() {
  const W = scopeCanvas.width, H = scopeCanvas.height, mid = H / 2;
  scopeCtx.clearRect(0, 0, W, H);
  scopeCtx.strokeStyle = '#18202b';
  scopeCtx.lineWidth = 1;
  scopeCtx.beginPath();
  scopeCtx.moveTo(0, mid);
  scopeCtx.lineTo(W, mid);
  scopeCtx.stroke();
  const trace = (buf, color) => {
    scopeCtx.strokeStyle = color;
    scopeCtx.beginPath();
    for (let x = 0; x < W; x++) {
      // scopePos points at the oldest sample (next write slot).
      const idx = (scopePos + Math.floor((x / W) * SCOPE_SAMPLES)) % SCOPE_SAMPLES;
      const y = mid - buf[idx] * (mid - 2);
      if (x === 0) scopeCtx.moveTo(x, y); else scopeCtx.lineTo(x, y);
    }
    scopeCtx.stroke();
  };
  trace(scopeR, 'rgba(56,189,248,.6)');  // R = cyan
  trace(scopeL, 'rgba(0,255,163,.9)');   // L = green
}

function tickFps() {
  fpsFrames++;
  const now = performance.now();
  const dt = now - fpsLast;
  if (dt >= 500) {
    $('fpsval').textContent = Math.round((fpsFrames * 1000) / dt) + ' fps';
    fpsFrames = 0;
    fpsLast = now;
  }
}

// ---------------------------------------------------------------- layers
// Push the current isolation choice to the core. With "isolate" off we render
// normally (mask = -1 → respect the game's DISPCNT). With it on, only the
// checked layers draw — overriding DISPCNT, so a layer the game disabled can be
// forced visible to inspect hidden content.
function applyLayerMask() {
  if (!gba) return;
  if (!isolateBox.checked) { gba.setLayerMask(-1); return; }
  let mask = 0;
  for (const b of lyrBoxes) if (b.checked) mask |= 1 << Number(b.dataset.bit);
  gba.setLayerMask(mask);
}

// Keep the layer checkboxes in sync. While isolating they are user-editable;
// otherwise they are a live read-out of the game's DISPCNT enable bits.
function syncLayerUi() {
  const editable = isolateBox.checked;
  $('lyrset').classList.toggle('live', editable);
  for (const b of lyrBoxes) b.disabled = !editable;
  if (!editable && gba) {
    const f = gba.layerFlags();
    for (const b of lyrBoxes) b.checked = ((f >> Number(b.dataset.bit)) & 1) !== 0;
  }
}

isolateBox.addEventListener('change', () => { syncLayerUi(); applyLayerMask(); });
for (const b of lyrBoxes) b.addEventListener('change', applyLayerMask);

$('audio').addEventListener('click', async () => {
  if (!gba) return;
  audioOn = !audioOn;
  const b = $('audio');
  b.classList.toggle('on', audioOn);
  b.textContent = audioOn ? '🔊 audio' : '🔇 audio';
  if (audioOn) { await ensureAudio(); await audioCtx.resume(); }
  else if (audioCtx) { audioCtx.suspend(); }
});

// ---------------------------------------------------------------- breakpoints
$('disasmBody').addEventListener('click', (e) => {
  const line = e.target.closest('.dline');
  if (!line || !gba) return;
  const addr = Number(line.dataset.addr) >>> 0;
  if (bps.has(addr)) { bps.delete(addr); gba.clearBreakpoint(addr); }
  else { bps.add(addr); gba.setBreakpoint(addr); }
  refreshDisasm();
});

// ---------------------------------------------------------------- memory edit
$('memBody').addEventListener('click', (e) => {
  const span = e.target.closest('.byte');
  if (!span || span.classList.contains('editing')) return;
  memEditing = true;
  span.dataset.orig = span.textContent;
  span.classList.add('editing');
  span.contentEditable = 'true';
  span.focus();
  const range = document.createRange();
  range.selectNodeContents(span);
  const sel = getSelection();
  sel.removeAllRanges();
  sel.addRange(range);
});
$('memBody').addEventListener('keydown', (e) => {
  const span = e.target.closest('.byte.editing');
  if (!span) return;
  if (e.key === 'Enter') { e.preventDefault(); commitByte(span, true); }
  else if (e.key === 'Escape') { span.textContent = span.dataset.orig; commitByte(span, false); }
});
$('memBody').addEventListener('blur', (e) => {
  const span = e.target.closest && e.target.closest('.byte.editing');
  if (span) commitByte(span, true);
}, true);

function commitByte(span, write) {
  const addr = Number(span.dataset.addr) >>> 0;
  if (write) {
    const v = parseInt(span.textContent.trim(), 16);
    if (!Number.isNaN(v) && v >= 0 && v <= 255) gba.writeByte(addr, v & 0xff);
  }
  span.contentEditable = 'false';
  span.classList.remove('editing');
  memEditing = false;
  refreshMem();
  refreshDisasm();
}

$('memGo').addEventListener('click', refreshMem);
$('memAddr').addEventListener('keydown', (e) => { if (e.key === 'Enter') refreshMem(); });
$('memSp').addEventListener('click', () => {
  if (!gba) return;
  $('memAddr').value = '0x' + hex(gba.registers()[13]);
  refreshMem();
});

// ---------------------------------------------------------------- assembler
function doAssemble() {
  if (!gba) { $('asmOut').textContent = 'load a ROM first'; return null; }
  const addr = parseAddr($('asmAddr').value);
  const res = JSON.parse(gba.assemble(addr, $('asmSrc').value));
  if (res.ok) {
    $('asmOut').innerHTML = `<span class="ok">✓ ok</span>  ${res.len} bytes: ${res.bytes}`;
    refreshDisasm(); refreshMem();
  } else {
    $('asmOut').innerHTML = `<span class="err">✗ ${res.error}</span>`;
  }
  return res.ok ? addr : null;
}
$('asmBtn').addEventListener('click', doAssemble);
$('asmRun').addEventListener('click', () => {
  const addr = doAssemble();
  if (addr === null) return;
  gba.setPc(addr);
  $('memAddr').value = '0x' + hex(addr);
  refreshAll();
});

// ---------------------------------------------------------------- transport
function bootGba(bytes) {
  gba = new GbaHandle(bytes);
  // re-apply breakpoints to the fresh instance
  for (const a of bps) gba.setBreakpoint(a);
  prevRegs = new Array(16).fill(-1); // force first-frame highlight off
  prevRegs = Array.from(gba.registers());
  paint(gba.framebuffer());
  // A fresh core resets the LCD layer override; re-apply the current choice.
  applyLayerMask();
  refreshAll();
  for (const b of ['run', 'pause', 'step', 'reset', 'audio']) $(b).disabled = false;
  $('pause').disabled = true;
}

$('rom').addEventListener('change', async (e) => {
  const file = e.target.files[0];
  if (!file) return;
  romBytes = new Uint8Array(await file.arrayBuffer());
  setRunning(false);
  bootGba(romBytes);
});

$('run').addEventListener('click', () => setRunning(true));
$('pause').addEventListener('click', () => { setRunning(false); refreshAll(); });
$('step').addEventListener('click', () => {
  if (!gba) return;
  gba.step();
  paint(gba.framebuffer());
  refreshAll();
});
$('reset').addEventListener('click', () => {
  if (!romBytes) return;
  setRunning(false);
  bootGba(romBytes);
});

// ---------------------------------------------------------------- input
addEventListener('keydown', (e) => {
  if (gba && !e.target.matches('input,textarea,[contenteditable=true]') && e.code in KEYMAP) {
    gba.setKey(KEYMAP[e.code], true); e.preventDefault();
  }
});
addEventListener('keyup', (e) => {
  if (gba && e.code in KEYMAP) { gba.setKey(KEYMAP[e.code], false); }
});

// ---------------------------------------------------------------- boot
await init();
console.log('rusty-gba debugger ready — load a .gba to begin');
