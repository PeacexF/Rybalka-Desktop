import type { Weather } from "./ipc/bindings/Weather";

/**
 * Tiny synthesized sound kit (no asset files yet): splashes, bell, reel clicks.
 * Real recordings can replace these later behind the same functions.
 */

export interface Volumes {
  master: number;
  sfx: number;
  ambient: number;
}

/** Overall loudness at master volume 1. */
const MASTER_GAIN = 0.5;

let ctx: AudioContext | null = null;
let buses: { master: GainNode; sfx: GainNode; ambient: GainNode } | null = null;
let volumes: Volumes = { master: 1, sfx: 1, ambient: 1 };

function applyVolumes(): void {
  if (!ctx || !buses) return;
  const t = ctx.currentTime;
  buses.master.gain.setTargetAtTime(volumes.master * MASTER_GAIN, t, 0.05);
  buses.sfx.gain.setTargetAtTime(volumes.sfx, t, 0.05);
  buses.ambient.gain.setTargetAtTime(volumes.ambient, t, 0.05);
}

/** Channel volumes, 0..1 each. Safe to call before any sound has played. */
export function setVolumes(v: Volumes): void {
  volumes = v;
  applyVolumes();
}

function context(): AudioContext | null {
  try {
    if (!ctx) {
      ctx = new AudioContext();
      const master = ctx.createGain();
      master.connect(ctx.destination);
      const sfx = ctx.createGain();
      sfx.connect(master);
      const ambient = ctx.createGain();
      ambient.connect(master);
      buses = { master, sfx, ambient };
      applyVolumes();
    }
    if (ctx.state === "suspended") void ctx.resume();
    return ctx;
  } catch {
    return null;
  }
}

function ac(): { ctx: AudioContext; out: GainNode } | null {
  const c = context();
  return c && buses ? { ctx: c, out: buses.sfx } : null;
}

/** The ambient bus (rain, birds, wind), for looped background sounds. */
function amb(): { ctx: AudioContext; out: GainNode } | null {
  const c = context();
  return c && buses ? { ctx: c, out: buses.ambient } : null;
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

/** What the ambience follows: time of day and weather on the water. */
export interface Ambience {
  hour: number;
  weather: Weather;
}

interface Bed {
  gain: GainNode;
  filter: BiquadFilterNode;
}

/** A looped, filtered noise layer on the ambient bus, silent until faded in. */
function bed(c: AudioContext, out: GainNode, type: BiquadFilterType, freq: number, q: number): Bed {
  const src = c.createBufferSource();
  src.buffer = noiseBuffer(c, 3);
  src.loop = true;
  const filter = c.createBiquadFilter();
  filter.type = type;
  filter.frequency.value = freq;
  filter.Q.value = q;
  const gain = c.createGain();
  gain.gain.value = 0;
  src.connect(filter).connect(gain).connect(out);
  src.start();
  return { gain, filter };
}

/** Short frequency sweep (a bird's note or a cricket's chirp) on the ambient bus. */
function sweep(out: GainNode, from: number, to: number, dur: number, gain: number, at: number) {
  const c = out.context;
  const o = c.createOscillator();
  o.frequency.setValueAtTime(from, at);
  o.frequency.exponentialRampToValueAtTime(to, at + dur);
  const g = c.createGain();
  g.gain.setValueAtTime(0, at);
  g.gain.linearRampToValueAtTime(gain, at + dur * 0.2);
  g.gain.exponentialRampToValueAtTime(0.001, at + dur);
  o.connect(g).connect(out);
  o.start(at);
  o.stop(at + dur);
}

/** Smooth 0..1 bump between `a` and `d`, flat at 1 from `b` to `c` (hours, may wrap midnight). */
function window4(h: number, a: number, b: number, c: number, d: number): number {
  const wrap = (x: number) => (x < a ? x + 24 : x);
  const [x, b2, c2, d2] = [wrap(h), wrap(b), wrap(c), wrap(d)];
  if (x <= b2) return (x - a) / (b2 - a);
  if (x <= c2) return 1;
  if (x <= d2) return (d2 - x) / (d2 - c2);
  return 0;
}

const BIRDS_WEATHER: Partial<Record<Weather, number>> = { Rain: 0.15, Fog: 0.5, Wind: 0.5 };
const WIND_LEVEL: Record<Weather, number> = {
  Clear: 0.15,
  Cloudy: 0.25,
  Overcast: 0.35,
  Rain: 0.5,
  Wind: 1,
  Fog: 0.05,
};
const TICK_S = 0.25;

let ambience: Ambience | null = null;
let beds: { water: Bed; wind: Bed; rain: Bed } | null = null;
let timer: ReturnType<typeof setInterval> | null = null;

/** Birds through the day, crickets at night: chances per second. */
function voices(a: Ambience): { birds: number; crickets: number } {
  const h = a.hour;
  const birds =
    (window4(h, 3.5, 5, 8.5, 11) + 0.35 * window4(h, 9, 11, 17, 21.5)) *
    (BIRDS_WEATHER[a.weather] ?? 1);
  const crickets = window4(h, 20, 22, 2, 4) * (a.weather === "Rain" ? 0.3 : 1);
  return { birds: 0.5 * birds, crickets: 1.6 * crickets };
}

function bird(out: GainNode, at: number) {
  const base = 2200 + Math.random() * 2200;
  const notes = 2 + Math.floor(Math.random() * 4);
  const up = Math.random() < 0.5;
  for (let i = 0; i < notes; i++) {
    const f = base * (1 + 0.06 * i * (up ? 1 : -1));
    sweep(out, f, f * (up ? 1.35 : 0.75), 0.07 + Math.random() * 0.05, 0.05, at + i * 0.11);
  }
}

function cricket(out: GainNode, at: number) {
  const f = 4300 + Math.random() * 500;
  for (let i = 0; i < 3; i++) sweep(out, f, f * 1.02, 0.03, 0.02, at + i * 0.045);
}

function ambienceTick() {
  const a = amb();
  if (!a || !beds) return;
  const { ctx: c, out } = a;
  const t = c.currentTime;
  const on = ambience;
  // Lapping water swells and settles; wind gusts move its pitch around.
  beds.water.gain.gain.setTargetAtTime(on ? 0.05 + Math.random() * 0.05 : 0, t, 0.8);
  beds.wind.gain.gain.setTargetAtTime(on ? 0.08 * WIND_LEVEL[on.weather] : 0, t, 1.5);
  beds.wind.filter.frequency.setTargetAtTime(250 + Math.random() * 450, t, 1.2);
  beds.rain.gain.gain.setTargetAtTime(on?.weather === "Rain" ? 0.12 : 0, t, 2);
  if (!on) return;
  const v = voices(on);
  if (Math.random() < v.birds * TICK_S) bird(out, t + Math.random() * TICK_S);
  if (Math.random() < v.crickets * TICK_S) cricket(out, t + Math.random() * TICK_S);
}

/**
 * Follow the scene with background sound: water, wind, rain, birds and
 * crickets by hour and weather. `null` fades it all out (off the water).
 */
export function setAmbience(a: Ambience | null): void {
  ambience = a;
  if (!a) return;
  if (!beds) {
    const x = amb();
    if (!x) return;
    beds = {
      water: bed(x.ctx, x.out, "lowpass", 380, 0.7),
      wind: bed(x.ctx, x.out, "bandpass", 450, 0.8),
      rain: bed(x.ctx, x.out, "lowpass", 2400, 0.7),
    };
  }
  timer ??= setInterval(ambienceTick, TICK_S * 1000);
}

/** Noise swept through a band-pass filter: rod swish, line through air. */
function swish(from: number, to: number, dur: number, gain: number) {
  const a = ac();
  if (!a) return;
  const { ctx: c, out } = a;
  const src = c.createBufferSource();
  src.buffer = noiseBuffer(c, dur);
  const f = c.createBiquadFilter();
  f.type = "bandpass";
  f.Q.value = 2;
  const t = c.currentTime;
  f.frequency.setValueAtTime(from, t);
  f.frequency.exponentialRampToValueAtTime(to, t + dur);
  const g = c.createGain();
  g.gain.setValueAtTime(0.001, t);
  g.gain.exponentialRampToValueAtTime(gain, t + dur * 0.3);
  g.gain.exponentialRampToValueAtTime(0.001, t + dur);
  src.connect(f).connect(g).connect(out);
  src.start(t);
  src.stop(t + dur);
}

export const sfx = {
  /** Rod swung for a cast or a strike. */
  whoosh(size = 1) {
    swish(600, 2600, 0.28 * size, 0.35 * size);
  },
  /** Money changes hands. */
  coins() {
    [0, 0.07, 0.15].forEach((d, i) => tone(2600 + i * 400, 0.25, 0.08, "triangle", d));
  },
  /** New record or rank: a short rising arpeggio. */
  fanfare() {
    [523, 659, 784, 1047].forEach((f, i) => tone(f, 0.5, 0.12, "triangle", i * 0.11));
  },
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
