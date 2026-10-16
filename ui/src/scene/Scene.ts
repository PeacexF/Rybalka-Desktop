import { Application, Graphics } from "pixi.js";

/** Owns the Pixi application that renders the fishing spot. */
export class Scene {
  readonly app = new Application();
  private water = new Graphics();

  async init(host: HTMLElement): Promise<void> {
    await this.app.init({ resizeTo: host, background: "#0e1512", antialias: true });
    host.appendChild(this.app.canvas);
    this.app.stage.addChild(this.water);
    this.app.renderer.on("resize", () => this.layout());
    this.layout();
  }

  private layout(): void {
    const { width, height } = this.app.screen;
    this.water.clear();
    this.water.rect(0, 0, width, height * 0.45).fill("#9cc3d5");
    this.water.rect(0, height * 0.45, width, height * 0.55).fill("#3c6b74");
  }

  destroy(): void {
    this.app.destroy(true, { children: true });
  }
}
