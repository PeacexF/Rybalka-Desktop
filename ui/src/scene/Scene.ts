import { Application, Assets, Container, Graphics, Sprite, type Texture } from "pixi.js";
import type { GameEvent } from "../ipc/bindings/GameEvent";
import type { Point } from "../ipc/bindings/Point";
import type { RodView } from "../ipc/bindings/RodView";
import type { SessionView } from "../ipc/bindings/SessionView";
import type { View } from "../ipc/bindings/View";

interface Vec {
  x: number;
  y: number;
}

interface Ripple {
  x: number;
  y: number;
  scale: number;
  t: number;
  strength: number;
}

/** Visual height of the float at scale 1, px. */
const FLOAT_H = 56;

/**
 * Renders a fishing spot: photo background, rod, line, float and water effects.
 *
 * The scene works in screen pixels; game positions arrive in normalized image
 * coordinates and are mapped through the "cover" fit of the background.
 */
export class Scene {
  readonly app = new Application();
  private readonly world = new Container();
  private bg = new Sprite();
  private readonly ripples = new Graphics();
  private readonly line = new Graphics();
  private readonly rod = new Graphics();
  private readonly float = new Container();
  private readonly floatBody = new Graphics();
  private readonly floatMask = new Graphics();
  private bgUrl = "";
  private session: SessionView | null = null;
  private snapshotAt = 0;
  private time = 0;
  private rippleList: Ripple[] = [];
  /** Smoothed float position (screen px) while fighting. */
  private fightPos: Vec | null = null;
  private lastNibbleRipple = 0;

  async init(host: HTMLElement): Promise<void> {
    await this.app.init({ resizeTo: host, background: "#0e1512", antialias: true });
    host.appendChild(this.app.canvas);
    this.floatBody.mask = this.floatMask;
    this.float.addChild(this.floatBody, this.floatMask);
    this.world.addChild(this.bg, this.ripples, this.line, this.float, this.rod);
    this.app.stage.addChild(this.world);
    this.drawFloat();
    this.app.ticker.add((t) => this.frame(t.deltaMS / 1000));
  }

  destroy(): void {
    this.app.destroy(true, { children: true });
  }

  /** Feed the latest session view (called on every snapshot). */
  update(session: SessionView | null, at: number): void {
    this.session = session;
    this.snapshotAt = at;
    if (session) {
      const url = `/${session.background}`;
      if (url !== this.bgUrl) {
        this.bgUrl = url;
        Assets.load<Texture>(url).then((tex) => {
          this.bg.texture = tex;
        });
      }
    }
  }

  onEvent(e: GameEvent): void {
    const s = this.session;
    if (!s) return;
    const rod = s.rods[e.rod];
    const st = rod?.state;
    const at = st && "at" in st ? st.at : st?.state === "Casting" ? st.to : null;
    if (!at) return;
    const p = this.toScreen(at);
    const scale = this.scaleAt(at.y);
    switch (e.type) {
      case "CastLanded":
        this.addRipple(p, scale, 1.2);
        break;
      case "Bite":
      case "Hooked":
        this.addRipple(p, scale, 0.8);
        break;
    }
  }

  // ---- coordinates -------------------------------------------------------

  private fit(): { ox: number; oy: number; w: number; h: number } {
    const { width, height } = this.app.screen;
    const tex = this.bg.texture;
    const iw = tex.width > 1 ? tex.width : 1920;
    const ih = tex.height > 1 ? tex.height : 1080;
    const k = Math.max(width / iw, height / ih);
    const w = iw * k;
    const h = ih * k;
    return { ox: (width - w) / 2, oy: (height - h) / 2, w, h };
  }

  toScreen(p: Point): Vec {
    const f = this.fit();
    return { x: f.ox + p.x * f.w, y: f.oy + p.y * f.h };
  }

  /** Screen px → normalized image coords. */
  toImage(sx: number, sy: number): Point {
    const f = this.fit();
    return { x: (sx - f.ox) / f.w, y: (sy - f.oy) / f.h };
  }

  /** Apparent size multiplier at image `y` (1.0 at the near shore). */
  private scaleAt(y: number): number {
    const v: View | undefined = this.session?.view;
    if (!v) return 1;
    const k = (y - v.horizonY) / (v.shoreY - v.horizonY);
    return Math.max(0.12, Math.min(1.2, k)) * (this.app.screen.height / 900);
  }

  // ---- rendering ----------------------------------------------------------

  private frame(dt: number): void {
    this.time += dt;
    const f = this.fit();
    this.bg.position.set(f.ox, f.oy);
    this.bg.width = f.w;
    this.bg.height = f.h;

    const s = this.session;
    const rod = s?.rods[s.activeRod];
    this.drawRodAndFloat(rod ?? null, dt);
    this.drawRipples(dt);
  }

  private rodButt(): Vec {
    const { width, height } = this.app.screen;
    return { x: width * 0.64, y: height * 1.08 };
  }

  private drawRodAndFloat(rod: RodView | null, dt: number): void {
    this.rod.clear();
    this.line.clear();
    this.float.visible = false;
    if (!rod) return;

    const { height } = this.app.screen;
    const butt = this.rodButt();
    const st = rod.state;
    const since = (performance.now() - this.snapshotAt) / 1000;

    // Where the float is and how it looks this frame.
    let target: Vec | null = null;
    let scale = 1;
    let sink = 0; // px below the surface (scaled)
    let lift = 0; // px above the surface while flying
    let tilt = 0;
    let tension = 0.15;
    let inAir = false;

    const restTip = { x: butt.x - height * 0.06, y: butt.y - height * 0.5 };

    switch (st.state) {
      case "Idle":
      case "Landed":
        this.fightPos = null;
        break;
      case "Casting": {
        const to = this.toScreen(st.to);
        const p = Math.min(1, st.progress + since * 0.5);
        target = { x: restTip.x + (to.x - restTip.x) * p, y: restTip.y + (to.y - restTip.y) * p };
        lift = Math.sin(Math.PI * p) * height * 0.22;
        scale = this.scaleAt(st.to.y) * (1 + (1 - p) * 0.6);
        inAir = true;
        tension = 0;
        break;
      }
      case "Retrieving": {
        const from = this.toScreen(st.from);
        const p = Math.min(1, st.progress + since);
        target = { x: from.x + (restTip.x - from.x) * p, y: from.y + (restTip.y - from.y) * p };
        lift = Math.sin(Math.PI * p) * height * 0.08;
        scale = this.scaleAt(st.from.y) * (1 + p * 0.5);
        inAir = p > 0.1;
        tension = 0.5;
        break;
      }
      case "Waiting": {
        target = this.toScreen(st.at);
        scale = this.scaleAt(st.at.y);
        sink = (Math.sin(this.time * 1.7) * 0.8 + 0.8) * scale;
        if (st.lying) {
          tilt = 1.35;
          sink = 4 * scale;
        }
        break;
      }
      case "Bite": {
        target = this.toScreen(st.at);
        scale = this.scaleAt(st.at.y);
        const t = st.phaseT + since;
        if (st.lying) tilt = 1.35;
        switch (st.phase) {
          case "Nibble":
            sink = (Math.abs(Math.sin(t * 23)) * 5 + Math.sin(t * 7) * 2) * scale;
            target.x += Math.sin(t * 31) * 1.2 * scale;
            if (this.time - this.lastNibbleRipple > 0.45) {
              this.lastNibbleRipple = this.time;
              this.addRipple(target, scale, 0.35);
            }
            break;
          case "Dip":
            sink = Math.sin(Math.PI * Math.min(1, t / 0.35)) * 22 * scale;
            break;
          case "Sink":
          case "Grab":
            sink = Math.min(1, t / 0.25) * FLOAT_H * scale;
            break;
          case "Lead":
            sink = 14 * scale;
            target.x += Math.min(t, 2.5) * 22 * scale;
            target.y -= Math.min(t, 2.5) * 3 * scale;
            break;
          case "Lift":
            sink = -Math.min(1, t / 0.6) * 10 * scale;
            tilt = Math.max(0, Math.min(1, (t - 0.6) / 0.8)) * 1.35;
            break;
        }
        tension = 0.25;
        break;
      }
      case "Fighting": {
        const p = this.toScreen(st.at);
        if (!this.fightPos) this.fightPos = { ...p };
        const k = 1 - Math.exp(-dt * 6);
        this.fightPos.x += (p.x - this.fightPos.x) * k;
        this.fightPos.y += (p.y - this.fightPos.y) * k;
        target = { ...this.fightPos };
        scale = this.scaleAt(this.toImage(target.x, target.y).y);
        sink = (18 + Math.sin(this.time * 9) * 6) * scale;
        tilt = Math.sin(this.time * 4) * 0.3;
        tension = Math.min(1.2, st.tension);
        if (st.slipping && Math.random() < dt * 6) this.addRipple(target, scale, 0.6);
        break;
      }
    }

    // Rod: aims at the float, bends with tension.
    const aim = target ?? restTip;
    const dir = norm({ x: aim.x - butt.x, y: Math.min(aim.y - butt.y, -height * 0.2) });
    const len = height * 0.62;
    const bend = 0.08 + tension * 0.35;
    const tip = {
      x: butt.x + dir.x * len * (1 - bend * 0.3),
      y: butt.y + dir.y * len * (1 - bend) + bend * height * 0.12,
    };
    const ctrl = { x: butt.x + dir.x * len * 0.55, y: butt.y + dir.y * len * 0.55 };
    drawTapered(this.rod, butt, ctrl, tip, 9, 2.2, 0x2a2016);
    drawTapered(this.rod, butt, ctrl, tip, 3, 0.8, 0x6b5537, 0.6);
    // Reel.
    const reel = lerp2(butt, ctrl, 0.25);
    this.rod
      .circle(reel.x + 8, reel.y + 4, 11)
      .fill({ color: 0x1c1c1c })
      .stroke({ color: 0x555555, width: 2 });

    if (!target) return;

    // Line: sags when slack.
    const fp = { x: target.x, y: target.y - lift };
    const sag = (1 - Math.min(1, tension)) * height * 0.05;
    const mid = lerp2(tip, fp, 0.5);
    this.line
      .moveTo(tip.x, tip.y)
      .quadraticCurveTo(mid.x, mid.y + sag, fp.x, fp.y - (inAir ? 0 : 2))
      .stroke({ color: 0xf2f2ea, width: 1, alpha: 0.55 });

    // Float.
    this.float.visible = true;
    this.float.position.set(fp.x, fp.y);
    this.float.scale.set(scale);
    this.floatBody.y = sink / scale;
    this.floatBody.rotation = tilt;
    this.floatMask.visible = !inAir;
    this.floatBody.mask = inAir ? null : this.floatMask;
  }

  private drawFloat(): void {
    const g = this.floatBody;
    g.clear();
    // Antenna.
    g.roundRect(-2.2, -FLOAT_H, 4.4, FLOAT_H * 0.45, 2).fill(0xe2402a);
    g.rect(-2.2, -FLOAT_H * 0.62, 4.4, 6).fill(0x1a1a1a);
    // Body: red top, white middle.
    g.ellipse(0, -10, 8, 16).fill(0xf3efe4);
    g.ellipse(0, -18, 7, 9).fill(0xe2402a);
    // Keel (below water, hidden by the mask unless lying/flying).
    g.rect(-1.5, 4, 3, 20).fill(0x8a7a5a);
    // Highlight.
    g.ellipse(-3, -14, 2, 6).fill({ color: 0xffffff, alpha: 0.5 });
    // Mask: everything above the waterline.
    this.floatMask.clear().rect(-60, -200, 120, 200).fill(0xffffff);
  }

  private addRipple(p: Vec, scale: number, strength: number): void {
    this.rippleList.push({ x: p.x, y: p.y, scale, t: 0, strength });
  }

  private drawRipples(dt: number): void {
    const g = this.ripples;
    g.clear();
    this.rippleList = this.rippleList.filter((r) => r.t < 2.2);
    for (const r of this.rippleList) {
      r.t += dt;
      for (let i = 0; i < 3; i++) {
        const t = r.t - i * 0.25;
        if (t <= 0) continue;
        const rx = (6 + t * 38 * r.strength) * r.scale;
        const alpha = Math.max(0, 0.5 * r.strength * (1 - t / 2));
        g.ellipse(r.x, r.y, rx, rx * 0.28).stroke({ color: 0xffffff, width: 1.2, alpha });
      }
    }
  }
}

function norm(v: Vec): Vec {
  const l = Math.hypot(v.x, v.y) || 1;
  return { x: v.x / l, y: v.y / l };
}

function lerp2(a: Vec, b: Vec, t: number): Vec {
  return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
}

function bezier(a: Vec, c: Vec, b: Vec, t: number): Vec {
  const u = 1 - t;
  return {
    x: u * u * a.x + 2 * u * t * c.x + t * t * b.x,
    y: u * u * a.y + 2 * u * t * c.y + t * t * b.y,
  };
}

/** Quadratic curve drawn as segments of decreasing width (a rod blank). */
function drawTapered(
  g: Graphics,
  a: Vec,
  c: Vec,
  b: Vec,
  w0: number,
  w1: number,
  color: number,
  alpha = 1,
): void {
  const n = 16;
  let prev = a;
  for (let i = 1; i <= n; i++) {
    const p = bezier(a, c, b, i / n);
    const w = w0 + (w1 - w0) * (i / n);
    g.moveTo(prev.x, prev.y).lineTo(p.x, p.y).stroke({ color, width: w, alpha, cap: "round" });
    prev = p;
  }
}
