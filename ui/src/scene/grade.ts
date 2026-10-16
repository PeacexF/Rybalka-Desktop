import type { Weather } from "../ipc/bindings/Weather";

/** Colour grade of the scene: brightness, RGB tint and saturation. */
export interface Grade {
  brightness: number;
  tint: [number, number, number];
  saturation: number;
}

/** Keyframes over the day (hour → grade); the photo itself is a summer evening. */
const KEYS: [number, Grade][] = [
  [0, { brightness: 0.3, tint: [0.55, 0.65, 1.0], saturation: 0.4 }],
  [3.5, { brightness: 0.32, tint: [0.6, 0.65, 0.95], saturation: 0.45 }],
  [5, { brightness: 0.62, tint: [1.08, 0.82, 0.8], saturation: 0.75 }],
  [6.5, { brightness: 0.9, tint: [1.05, 0.95, 0.88], saturation: 0.92 }],
  [9, { brightness: 1.0, tint: [1, 1, 1], saturation: 1 }],
  [17, { brightness: 1.0, tint: [1, 1, 1], saturation: 1 }],
  [19.5, { brightness: 0.92, tint: [1.1, 0.93, 0.78], saturation: 1.05 }],
  [21, { brightness: 0.6, tint: [0.95, 0.75, 0.78], saturation: 0.8 }],
  [22.5, { brightness: 0.35, tint: [0.6, 0.65, 0.95], saturation: 0.5 }],
  [24, { brightness: 0.3, tint: [0.55, 0.65, 1.0], saturation: 0.4 }],
];

const WEATHER: Record<Weather, { brightness: number; saturation: number; blue: number }> = {
  Clear: { brightness: 1.03, saturation: 1.05, blue: 0 },
  Cloudy: { brightness: 0.95, saturation: 0.9, blue: 0 },
  Overcast: { brightness: 0.86, saturation: 0.7, blue: 0.03 },
  Rain: { brightness: 0.78, saturation: 0.62, blue: 0.06 },
  Wind: { brightness: 0.97, saturation: 0.95, blue: 0.01 },
  Fog: { brightness: 0.98, saturation: 0.6, blue: 0.02 },
};

const lerp = (a: number, b: number, t: number) => a + (b - a) * t;

export function gradeAt(hour: number, weather: Weather): Grade {
  let i = KEYS.findIndex(([h]) => h > hour);
  if (i <= 0) i = 1;
  const [h0, a] = KEYS[i - 1]!;
  const [h1, b] = KEYS[i]!;
  const t = (hour - h0) / (h1 - h0);
  const w = WEATHER[weather];
  return {
    brightness: lerp(a.brightness, b.brightness, t) * w.brightness,
    tint: [
      lerp(a.tint[0], b.tint[0], t) * (1 - w.blue),
      lerp(a.tint[1], b.tint[1], t),
      lerp(a.tint[2], b.tint[2], t) * (1 + w.blue),
    ],
    saturation: lerp(a.saturation, b.saturation, t) * w.saturation,
  };
}

export function mixGrade(a: Grade, b: Grade, t: number): Grade {
  return {
    brightness: lerp(a.brightness, b.brightness, t),
    tint: [
      lerp(a.tint[0], b.tint[0], t),
      lerp(a.tint[1], b.tint[1], t),
      lerp(a.tint[2], b.tint[2], t),
    ],
    saturation: lerp(a.saturation, b.saturation, t),
  };
}

/** 4x5 colour matrix (row-major, Pixi ColorMatrixFilter layout). */
export function gradeMatrix(g: Grade): number[] {
  const s = g.saturation;
  const sr = (1 - s) * 0.3086;
  const sg = (1 - s) * 0.6094;
  const sb = (1 - s) * 0.082;
  const rows = [
    [sr + s, sg, sb],
    [sr, sg + s, sb],
    [sr, sg, sb + s],
  ];
  const m: number[] = [];
  rows.forEach((row, i) => {
    const k = g.tint[i]! * g.brightness;
    m.push(row[0]! * k, row[1]! * k, row[2]! * k, 0, 0);
  });
  m.push(0, 0, 0, 1, 0);
  return m;
}
