/**
 * Tiny synthesized sound kit (no asset files yet): splashes, bell, reel clicks.
 * Real recordings can replace these later behind the same functions.
 */

let ctx: AudioContext | null = null;
let master: GainNode | null = null;

function ac(): { ctx: AudioContext; out: GainNode } | null {
  try {
    if (!ctx) {
      ctx = new AudioContext();
      master = ctx.createGain();
      master.gain.value = 0.5;
      master.connect(ctx.destination);
    }
    if (ctx.state === "suspended") void ctx.resume();
    return { ctx, out: master! };
  } catch {
    return null;
  }
}

function noiseBuffer(c: AudioContext, seconds: number): AudioBuffer {
  const buf = c.createBuffer(1, Math.ceil(c.sampleRate * seconds), c.sampleRate);
  const d = buf.getChannelData(0);
  for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
  return buf;
}

/** Filtered noise burst with an exponential decay. */
function burst(freq: number, q: number, dur: number, gain: number, type: BiquadFilterType) {
  const a = ac();
  if (!a) return;
  const { ctx: c, out } = a;
  const src = c.createBufferSource();
  src.buffer = noiseBuffer(c, dur);
  const f = c.createBiquadFilter();
  f.type = type;
  f.frequency.value = freq;
  f.Q.value = q;
  const g = c.createGain();
  const t = c.currentTime;
  g.gain.setValueAtTime(gain, t);
  g.gain.exponentialRampToValueAtTime(0.001, t + dur);
  src.connect(f).connect(g).connect(out);
  src.start(t);
  src.stop(t + dur);
}

function tone(freq: number, dur: number, gain: number, type: OscillatorType = "sine", delay = 0) {
  const a = ac();
  if (!a) return;
  const { ctx: c, out } = a;
  const o = c.createOscillator();
  o.type = type;
  o.frequency.value = freq;
  const g = c.createGain();
  const t = c.currentTime + delay;
  g.gain.setValueAtTime(0, t);
  g.gain.linearRampToValueAtTime(gain, t + 0.005);
  g.gain.exponentialRampToValueAtTime(0.001, t + dur);
  o.connect(g).connect(out);
  o.start(t);
  o.stop(t + dur);
}

export const sfx = {
  /** Rig hits the water. */
  plop(size = 1) {
    burst(700 / size, 1.2, 0.25 * size, 0.5, "bandpass");
    tone(220 / size, 0.12, 0.2 * size, "sine");
  },
  /** Feeder bite bell. */
  bell() {
    tone(2350, 0.9, 0.25, "sine");
    tone(3520, 0.6, 0.08, "sine");
    tone(2350, 0.7, 0.18, "sine", 0.12);
  },
  /** Soft cue for a float bite on a rod you're not looking at. */
  bloop() {
    tone(520, 0.18, 0.15, "triangle");
    tone(390, 0.22, 0.12, "triangle", 0.09);
  },
  /** One click of the reel handle / drag ratchet. */
  click(gain = 0.12) {
    burst(4200, 6, 0.025, gain, "bandpass");
  },
  splash() {
    burst(500, 0.7, 0.6, 0.6, "lowpass");
    burst(1800, 1, 0.35, 0.25, "bandpass");
  },
  snap() {
    burst(3000, 0.5, 0.08, 0.6, "highpass");
    tone(160, 0.25, 0.25, "sawtooth");
  },
};
