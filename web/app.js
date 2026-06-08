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
  feedAudio();
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
function feedAudio() {
  if (!audioOn || !audioNode) return;
  const s = gba.takeAudioF32();
  if (s.length) audioNode.port.postMessage(s, [s.buffer]);
}

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
