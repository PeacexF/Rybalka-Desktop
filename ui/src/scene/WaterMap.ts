/** Client-side copy of a spot's water map, for hover info and click validation. */
export class WaterMap {
  private constructor(
    private readonly width: number,
    private readonly height: number,
    private readonly data: Uint8ClampedArray,
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

  /** Depth (m) and zone at normalized image coords, or null on land. */
  sample(x: number, y: number): { depthM: number; zone: number } | null {
    if (x < 0 || x >= 1 || y < 0 || y >= 1) return null;
    const i = (Math.floor(y * this.height) * this.width + Math.floor(x * this.width)) * 4;
    if (this.data[i + 3]! < 128) return null;
    return { depthM: this.data[i]! * 0.05, zone: this.data[i + 2]! };
  }
}
