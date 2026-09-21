// AudioWorklet processor for rgba.
//
// The main thread posts interleaved L/R Float32 chunks (drained from the
// emulator's APU each frame). We queue them and emit them sample-by-sample in
// process(). Latency is capped so a fast-running emulator can't build an
// unbounded backlog.

const MAX_QUEUED = 32768; // ~0.5s of stereo samples at 32768 Hz

class GbaAudioProcessor extends AudioWorkletProcessor {
  constructor() {
    super();
    this.buf = new Float32Array(0);
    this.pos = 0;
    this.port.onmessage = (e) => {
      const incoming = e.data;
      const remaining = this.buf.length - this.pos;
      const merged = new Float32Array(remaining + incoming.length);
      merged.set(this.buf.subarray(this.pos), 0);
      merged.set(incoming, remaining);
      this.buf = merged;
      this.pos = 0;
      // Drop oldest samples if we're backing up (keep latency bounded).
      const queued = this.buf.length - this.pos;
      if (queued > MAX_QUEUED) {
        this.pos = this.buf.length - MAX_QUEUED;
      }
    };
  }

  process(_inputs, outputs) {
    const out = outputs[0];
    const left = out[0];
    const right = out[1] || out[0];
    for (let i = 0; i < left.length; i++) {
      if (this.pos + 1 < this.buf.length) {
        left[i] = this.buf[this.pos++];
        right[i] = this.buf[this.pos++];
      } else {
        left[i] = 0;
        right[i] = 0;
      }
    }
    return true;
  }
}

registerProcessor('gba-audio', GbaAudioProcessor);
