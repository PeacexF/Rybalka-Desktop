import type { Bottom } from "../ipc/bindings/Bottom";

export const BOTTOMS: Bottom[] = ["Silt", "Sand", "Stone", "Weed", "Snag"];
/** Depth step of the R channel, metres. */
const DEPTH_STEP = 0.05;

export interface MapCell {
  depthM: number;
  bottom: Bottom;
  zone: number;
}

/** What a map-editor brush stroke does to the cells it touches. */
export type Brush =
  | { kind: "water"; depthM: number }
  | { kind: "land" }
  | { kind: "deeper"; m: number }
  | { kind: "setDepth"; m: number }
  | { kind: "bottom"; bottom: Bottom }
  | { kind: "zone"; zone: number };

/**
 * Client-side copy of a spot's water map: hover info, click validation and
 * (in dev builds) the map editor, which edits it in place.
 */
export class WaterMap {
  private constructor(
    readonly width: number,
    readonly height: number,
    readonly data: Uint8ClampedArray,
  ) {}

  static async load(url: string): Promise<WaterMap> {
    const img = new Image();
    img.src = url;
    await img.decode();
    const canvas = document.createElement("canvas");
    canvas.width = img.naturalWidth;
    canvas.height = img.naturalHeight;
    const ctx = canvas.getContext("2d", { willReadFrequently: true })!;
    ctx.drawImage(img, 0, 0);
    const { data } = ctx.getImageData(0, 0, canvas.width, canvas.height);
    return new WaterMap(canvas.width, canvas.height, data);
  }

  /** Cell at normalized image coords, or null on land. */
  sample(x: number, y: number): MapCell | null {
    if (x < 0 || x >= 1 || y < 0 || y >= 1) return null;
    const i = (Math.floor(y * this.height) * this.width + Math.floor(x * this.width)) * 4;
    if (this.data[i + 3]! < 128) return null;
    return {
      depthM: this.data[i]! * DEPTH_STEP,
      bottom: BOTTOMS[this.data[i + 1]!] ?? "Silt",
      zone: this.data[i + 2]!,
    };
  }

  /** Apply a brush in a circle of `radius` map pixels around normalized (x, y). */
  paint(x: number, y: number, radius: number, brush: Brush): void {
    const cx = x * this.width;
    const cy = y * this.height;
    const r2 = radius * radius;
    for (let py = Math.floor(cy - radius); py <= cy + radius; py++) {
      for (let px = Math.floor(cx - radius); px <= cx + radius; px++) {
        if (px < 0 || py < 0 || px >= this.width || py >= this.height) continue;
        if ((px + 0.5 - cx) ** 2 + (py + 0.5 - cy) ** 2 > r2) continue;
        const i = (py * this.width + px) * 4;
        const d = this.data;
        const wet = d[i + 3]! >= 128;
        switch (brush.kind) {
          case "water":
            if (!wet) {
              d[i] = Math.round(brush.depthM / DEPTH_STEP);
              d[i + 1] = 0;
              d[i + 2] = d[i + 2] || 1;
              d[i + 3] = 255;
            }
            break;
          case "land":
            d[i] = d[i + 1] = d[i + 2] = d[i + 3] = 0;
            break;
          case "deeper":
            if (wet) d[i] = Math.max(1, Math.min(255, d[i]! + Math.round(brush.m / DEPTH_STEP)));
            break;
          case "setDepth":
            if (wet) d[i] = Math.max(1, Math.min(255, Math.round(brush.m / DEPTH_STEP)));
            break;
          case "bottom":
            if (wet) d[i + 1] = BOTTOMS.indexOf(brush.bottom);
            break;
          case "zone":
            if (wet) d[i + 2] = brush.zone;
            break;
        }
      }
    }
  }

  /** Encode as an RGBA PNG (what `map.png` holds). */
  async toPng(): Promise<Uint8Array> {
    const canvas = document.createElement("canvas");
    canvas.width = this.width;
    canvas.height = this.height;
    const ctx = canvas.getContext("2d")!;
    // Straight (unpremultiplied) alpha must survive: land pixels are fully
    // transparent, water fully opaque, so nothing in between gets mangled.
    ctx.putImageData(new ImageData(new Uint8ClampedArray(this.data), this.width), 0, 0);
    const blob = await new Promise<Blob>((ok, err) =>
      canvas.toBlob((b) => (b ? ok(b) : err(new Error("toBlob failed"))), "image/png"),
    );
    return new Uint8Array(await blob.arrayBuffer());
  }
}
