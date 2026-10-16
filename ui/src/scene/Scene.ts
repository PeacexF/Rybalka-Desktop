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
/** Rod butt positions per slot, as fractions of the screen width. */
const BUTT_X = [0.62, 0.76, 0.89];

/** Graphics for one rod slot: blank, line and (for float rods) the float. */
class RodGfx {
  readonly blank = new Graphics();
  readonly line = new Graphics();
  readonly float = new Container();
  readonly floatBody = new Graphics();
  readonly floatMask = new Graphics();
  /** Smoothed rig position (screen px) while fighting. */
  fightPos: Vec | null = null;
  lastNibbleRipple = 0;
  lastWake = 0;

  constructor() {
    this.floatBody.mask = this.floatMask;
    this.float.addChild(this.floatBody, this.floatMask);
    drawFloat(this.floatBody, this.floatMask);
  }
}

/**
 * Renders a fishing spot: photo background, rods, lines, floats and water effects.
 *
 * The scene works in screen pixels; game positions arrive in normalized image
 * coordinates and are mapped through the "cover" fit of the background.
 */
export class Scene {
  readonly app = new Application();
  private readonly world = new Container();
  private readonly lines = new Container();
  private readonly floats = new Container();
  private readonly blanks = new Container();
  private bg = new Sprite();
  private readonly ripples = new Graphics();
  private rods: RodGfx[] = [];
  private bgUrl = "";
  private session: SessionView | null = null;
  private snapshotAt = 0;
  private time = 0;
  private rippleList: Ripple[] = [];

  async init(host: HTMLElement): Promise<void> {
    await this.app.init({ resizeTo: host, background: "#0e1512", antialias: true });
    host.appendChild(this.app.canvas);
    this.world.addChild(this.bg, this.ripples, this.lines, this.floats, this.blanks);
    this.app.stage.addChild(this.world);
    this.app.ticker.add((t) => this.frame(t.deltaMS / 1000));
  }

  destroy(): void {
    this.app.destroy(true, { children: true });
  }

  /** Feed the latest session view (called on every snapshot). */
  update(session: SessionView | null, at: number): void {
    this.session = session;
    this.snapshotAt = at;
    if (!session) return;
    const url = `/${session.background}`;
    if (url !== this.bgUrl) {
      this.bgUrl = url;
      Assets.load<Texture>(url).then((tex) => {
        this.bg.texture = tex;
      });
    }
    while (this.rods.length < session.rods.length) {
      const g = new RodGfx();
      this.rods.push(g);
      this.lines.addChild(g.line);
      this.floats.addChild(g.float);
      this.blanks.addChild(g.blank);
    }
  }

  onEvent(e: GameEvent): void {
    const st = this.session?.rods[e.rod]?.state;
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
    this.rods.forEach((g, i) => {
      const rod = s?.rods[i];
      this.drawRod(g, i, rod ?? null, i === s?.activeRod, dt);
    });
    this.drawRipples(dt);
  }

  private drawRod(g: RodGfx, slot: number, rod: RodView | null, active: boolean, dt: number) {
    g.blank.clear();
    g.line.clear();
    g.float.visible = false;
    if (!rod) return;

    const { width, height } = this.app.screen;
    const butt = { x: width * (BUTT_X[slot] ?? 0.6), y: height * 1.08 };
    const st = rod.state;
    const since = (performance.now() - this.snapshotAt) / 1000;
    const isFloat = rod.kind === "Float";

    // Where the rig is and how it looks this frame.
    let target: Vec | null = null;
    let scale = 1;
    let sink = 0; // px below the surface (scaled)
    let lift = 0; // px above the surface while flying
    let tilt = 0;
    let tension = 0.15;
    let inAir = false;
    let tipKick = 0; // quivertip deflection, fraction of screen height (+ = pulled down)

    const restTip = { x: butt.x - height * 0.06, y: butt.y - height * 0.5 };

    switch (st.state) {
      case "Idle":
      case "Landed":
        g.fightPos = null;
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
        sink = (Math.sin(this.time * 1.7 + slot) * 0.8 + 0.8) * scale;
        if (st.lying) {
          tilt = 1.35;
          sink = 4 * scale;
        }
        tension = isFloat ? 0.15 : 0.35;
        break;
      }
      case "Bite": {
        target = this.toScreen(st.at);
        scale = this.scaleAt(st.at.y);
        const t = st.phaseT + since;
        if (st.lying) tilt = 1.35;
        tension = isFloat ? 0.25 : 0.4;
        switch (st.phase) {
          case "Nibble":
            sink = (Math.abs(Math.sin(t * 23)) * 5 + Math.sin(t * 7) * 2) * scale;
            target.x += Math.sin(t * 31) * 1.2 * scale;
            tipKick = Math.abs(Math.sin(t * 19)) * 0.012;
            if (isFloat && this.time - g.lastNibbleRipple > 0.45) {
              g.lastNibbleRipple = this.time;
              this.addRipple(target, scale, 0.35);
            }
            break;
          case "Dip":
            sink = Math.sin(Math.PI * Math.min(1, t / 0.35)) * 22 * scale;
            tipKick = Math.sin(Math.PI * Math.min(1, t / 0.35)) * 0.03;
            break;
          case "Sink":
          case "Grab":
            sink = Math.min(1, t / 0.25) * FLOAT_H * scale;
            tipKick = Math.min(1, t / 0.2) * 0.06 + Math.sin(t * 14) * 0.008;
            break;
          case "Lead":
            sink = 14 * scale;
            target.x += Math.min(t, 2.5) * 22 * scale;
            target.y -= Math.min(t, 2.5) * 3 * scale;
            tipKick = Math.min(1, t / 0.8) * 0.05;
            break;
          case "Lift":
            sink = -Math.min(1, t / 0.6) * 10 * scale;
            tilt = Math.max(0, Math.min(1, (t - 0.6) / 0.8)) * 1.35;
            // Bream lifts the feeder: the tip springs back straight.
            tipKick = -Math.min(1, t / 0.5) * 0.025;
            tension = 0.05;
            break;
        }
        break;
      }
      case "Lure": {
        target = this.toScreen(st.at);
        scale = this.scaleAt(st.at.y);
        tension = st.reeling ? 0.45 : 0.1;
        // A shallow lure on the retrieve leaves a wake.
        if (st.reeling && st.depthM < 0.6 && this.time - g.lastWake > 0.18) {
          g.lastWake = this.time;
          this.addRipple(target, scale, 0.25);
        }
        break;
      }
      case "Snagged":
        target = this.toScreen(st.at);
        scale = this.scaleAt(st.at.y);
        tension = 0.9;
        tipKick = 0.02 + Math.sin(this.time * 20) * 0.003;
        break;
      case "Fighting": {
        const p = this.toScreen(st.at);
        if (!g.fightPos) g.fightPos = { ...p };
        const k = 1 - Math.exp(-dt * 6);
        g.fightPos.x += (p.x - g.fightPos.x) * k;
        g.fightPos.y += (p.y - g.fightPos.y) * k;
        target = { ...g.fightPos };
        scale = this.scaleAt(this.toImage(target.x, target.y).y);
        sink = (18 + Math.sin(this.time * 9) * 6) * scale;
        tilt = Math.sin(this.time * 4) * 0.3;
        tension = Math.min(1.2, st.tension);
        if (st.slipping && Math.random() < dt * 6) this.addRipple(target, scale, 0.6);
        break;
      }
    }

    // Rod: aims at the rig, bends with tension; quivertip kicks on bites.
    const aim = target ?? restTip;
    // Rods stand fairly upright (as on a rod pod) and lean towards their rig.
    const dir = norm({ x: (aim.x - butt.x) * 0.45, y: -height * 0.85 });
    const len = height * (active ? 0.62 : 0.56);
    const bend = 0.08 + tension * 0.35;
    const tip = {
      x: butt.x + dir.x * len * (1 - bend * 0.3),
      y: butt.y + dir.y * len * (1 - bend) + (bend + tipKick) * height * 0.12 + tipKick * height,
    };
    const ctrl = { x: butt.x + dir.x * len * 0.55, y: butt.y + dir.y * len * 0.55 };
    const alpha = active ? 1 : 0.85;
    drawTapered(g.blank, butt, ctrl, tip, 9, 2.2, 0x2a2016, alpha);
    drawTapered(g.blank, butt, ctrl, tip, 3, 0.8, 0x6b5537, 0.6 * alpha);
    if (!isFloat) {
      // Painted quivertip: the last stretch of the blank.
      const qt = bezier(butt, ctrl, tip, 0.86);
      g.blank
        .moveTo(qt.x, qt.y)
        .lineTo(tip.x, tip.y)
        .stroke({ color: 0xf0e04a, width: 2.2, alpha, cap: "round" });
    }
    const reel = lerp2(butt, ctrl, 0.25);
    g.blank
      .circle(reel.x + 8, reel.y + 4, 11)
      .fill({ color: 0x1c1c1c, alpha })
      .stroke({ color: active ? 0xe8b84a : 0x555555, width: 2, alpha });

    if (!target) return;

    // Line: sags when slack.
    const fp = { x: target.x, y: target.y - lift };
    const sag = (1 - Math.min(1, tension)) * height * 0.05;
    const mid = lerp2(tip, fp, 0.5);
    g.line
      .moveTo(tip.x, tip.y)
      .quadraticCurveTo(mid.x, mid.y + sag, fp.x, fp.y - (inAir ? 0 : 2))
      .stroke({ color: 0xf2f2ea, width: 1, alpha: 0.55 });

    // Float (feeders only show the rig while it flies).
    if (!isFloat && !inAir && st.state !== "Fighting") return;
    g.float.visible = true;
    g.float.position.set(fp.x, fp.y);
    g.float.scale.set(isFloat ? scale : scale * 0.6);
    g.floatBody.y = sink / scale;
    g.floatBody.rotation = tilt;
    g.floatBody.mask = inAir ? null : g.floatMask;
    g.floatMask.visible = !inAir;
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

function drawFloat(g: Graphics, mask: Graphics): void {
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
  mask.clear().rect(-60, -200, 120, 200).fill(0xffffff);
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
