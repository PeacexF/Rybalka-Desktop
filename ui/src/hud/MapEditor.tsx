import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
import type { Bottom } from "../ipc/bindings/Bottom";
import type { View } from "../ipc/bindings/View";
import type { Scene } from "../scene/Scene";
import { BOTTOMS, type Brush, type WaterMap } from "../scene/WaterMap";
import { toastError, useGame } from "../store/game";

/**
 * Dev-only water map editor (F4 on the water): paint water/land, depth,
 * bottom and zones over the photo, try a perspective calibration, save
 * `map.png` back into the repo.
 *
 * `map.png` is normally generated from `map.ron` by `gen_map`; regenerating
 * overwrites hand edits, so use this for touch-ups and calibration.
 */

const BOTTOM_NAME: Record<Bottom, string> = {
  Silt: "Ил",
  Sand: "Песок",
  Stone: "Камень",
  Weed: "Трава",
  Snag: "Коряжник",
};
const BOTTOM_RGB: Record<Bottom, [number, number, number]> = {
  Silt: [120, 90, 60],
  Sand: [230, 200, 120],
  Stone: [150, 150, 160],
  Weed: [80, 190, 80],
  Snag: [200, 70, 40],
};
const ZONE_RGB: [number, number, number][] = [
  [255, 255, 255],
  [70, 140, 255],
  [255, 200, 40],
  [60, 220, 120],
  [240, 80, 200],
  [255, 120, 40],
  [140, 90, 255],
  [40, 220, 230],
];

type Layer = "depth" | "bottom" | "zone";
type Tool = "water" | "land" | "deeper" | "shallower" | "setDepth" | "bottom" | "zone";

function colour(map: WaterMap, layer: Layer, i: number): [number, number, number] {
  const d = map.data;
  if (layer === "bottom") return BOTTOM_RGB[BOTTOMS[d[i + 1]!] ?? "Silt"];
  if (layer === "zone") return ZONE_RGB[d[i + 2]! % ZONE_RGB.length]!;
  // Depth: light cyan (shallow) → deep navy at 10 m.
  const t = Math.min(1, (d[i]! * 0.05) / 10);
  return [Math.round(180 * (1 - t)), Math.round(240 - 200 * t), Math.round(255 - 135 * t)];
}

export function MapEditor({
  scene,
  map,
  onClose,
}: {
  scene: Scene;
  map: WaterMap;
  onClose: () => void;
}) {
  const session = useGame((s) => s.snapshot?.session);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [layer, setLayer] = useState<Layer>("depth");
  const [tool, setTool] = useState<Tool>("deeper");
  const [radius, setRadius] = useState(8);
  const [depth, setDepth] = useState(2);
  const [bottom, setBottom] = useState<Bottom>("Weed");
  const [zone, setZone] = useState(1);
  const [opacity, setOpacity] = useState(0.55);
  const [view, setView] = useState<View | null>(session?.view ?? null);
  const [hover, setHover] = useState<string>("");
  const [dirty, setDirty] = useState(false);
  const painting = useRef(false);
  const version = useRef(0);

  // Redraw the tinted map over the photo every frame (it follows panning).
  useEffect(() => {
    const off = document.createElement("canvas");
    off.width = map.width;
    off.height = map.height;
    const octx = off.getContext("2d")!;
    const img = octx.createImageData(map.width, map.height);
    let drawn = -1;
    let raf = 0;
    const draw = () => {
      raf = requestAnimationFrame(draw);
      const c = canvasRef.current;
      if (!c) return;
      if (c.width !== window.innerWidth || c.height !== window.innerHeight) {
        c.width = window.innerWidth;
        c.height = window.innerHeight;
      }
      if (drawn !== version.current) {
        drawn = version.current;
        for (let i = 0; i < map.data.length; i += 4) {
          const wet = map.data[i + 3]! >= 128;
          const [r, g, b] = wet ? colour(map, layer, i) : [0, 0, 0];
          img.data[i] = r;
          img.data[i + 1] = g;
          img.data[i + 2] = b;
          img.data[i + 3] = wet ? 255 : 0;
        }
        octx.putImageData(img, 0, 0);
      }
      const ctx = c.getContext("2d")!;
      ctx.clearRect(0, 0, c.width, c.height);
      const f = scene.fitRect();
      ctx.globalAlpha = opacity;
      ctx.imageSmoothingEnabled = false;
      ctx.drawImage(off, f.ox, f.oy, f.w, f.h);
      if (view) {
        // Calibration guides: horizon and the 10/20/40 m lines.
        ctx.globalAlpha = 0.9;
        ctx.strokeStyle = "#ff5050";
        ctx.font = "12px ui-monospace, monospace";
        ctx.fillStyle = "#ffdddd";
        const line = (y: number, label: string) => {
          const sy = f.oy + y * f.h;
          ctx.beginPath();
          ctx.moveTo(0, sy);
          ctx.lineTo(c.width, sy);
          ctx.stroke();
          ctx.fillText(label, 8, sy - 3);
        };
        ctx.setLineDash([6, 6]);
        line(view.horizonY, "горизонт");
        for (const d of [10, 20, 40]) {
          const y = view.horizonY + (view.nearM * (view.shoreY - view.horizonY)) / d;
          if (y < 1) line(y, `${d} м`);
        }
        ctx.setLineDash([]);
      }
    };
    draw();
    return () => cancelAnimationFrame(raf);
  }, [map, scene, layer, opacity, view]);

  const brush = (): Brush => {
    switch (tool) {
      case "water":
        return { kind: "water", depthM: depth };
      case "land":
        return { kind: "land" };
      case "deeper":
        return { kind: "deeper", m: 0.25 };
      case "shallower":
        return { kind: "deeper", m: -0.25 };
      case "setDepth":
        return { kind: "setDepth", m: depth };
      case "bottom":
        return { kind: "bottom", bottom };
      case "zone":
        return { kind: "zone", zone };
    }
  };

  const at = (e: React.PointerEvent) => scene.toImage(e.clientX, e.clientY);
  const paint = (e: React.PointerEvent) => {
    const p = at(e);
    map.paint(p.x, p.y, radius, brush());
    version.current++;
    setDirty(true);
  };

  const onMove = (e: React.PointerEvent) => {
    if (painting.current) paint(e);
    const p = at(e);
    const c = map.sample(p.x, p.y);
    const dist =
      view && p.y > view.horizonY
        ? (view.nearM * (view.shoreY - view.horizonY)) / (p.y - view.horizonY)
        : Infinity;
    setHover(
      `x ${p.x.toFixed(3)} y ${p.y.toFixed(3)} · ` +
        (c ? `${c.depthM.toFixed(2)} м · ${BOTTOM_NAME[c.bottom]} · зона ${c.zone}` : "суша") +
        (Number.isFinite(dist) ? ` · ${dist.toFixed(1)} м` : ""),
    );
  };

  const changeView = (patch: Partial<View>) => {
    if (!view) return;
    const v = { ...view, ...patch };
    setView(v);
    invoke("dev_set_view", { view: v }).catch(toastError);
  };

  const save = async () => {
    try {
      const png = await map.toPng();
      const path = await invoke<string>("dev_save_map", { png: Array.from(png) });
      setDirty(false);
      useGame.getState().toast(`Карта сохранена: ${path}`, "good");
    } catch (e) {
      toastError(e);
    }
  };

  const ron = view
    ? `view: (horizonY: ${view.horizonY.toFixed(3)}, shoreY: ${view.shoreY.toFixed(3)}, nearM: ${view.nearM.toFixed(1)}, fov: ${view.fov.toFixed(2)}),`
    : "";

  return (
    <>
      <canvas
        ref={canvasRef}
        className="map-editor-canvas"
        onPointerDown={(e) => {
          if (e.button !== 0) return;
          painting.current = true;
          (e.target as Element).setPointerCapture(e.pointerId);
          paint(e);
        }}
        onPointerUp={() => (painting.current = false)}
        onPointerMove={onMove}
        onContextMenu={(e) => e.preventDefault()}
      />
      <div className="panel map-editor">
        <div className="map-editor-head">
          <b>Редактор карты</b>
          <span className="dim">{session?.spotId}</span>
          <button onClick={onClose}>✕</button>
        </div>
        <label>
          Слой
          <select value={layer} onChange={(e) => setLayer(e.target.value as Layer)}>
            <option value="depth">Глубина</option>
            <option value="bottom">Дно</option>
            <option value="zone">Зоны</option>
          </select>
        </label>
        <label>
          Кисть
          <select value={tool} onChange={(e) => setTool(e.target.value as Tool)}>
            <option value="deeper">Глубже +0.25 м</option>
            <option value="shallower">Мельче −0.25 м</option>
            <option value="setDepth">Задать глубину</option>
            <option value="water">Вода</option>
            <option value="land">Суша</option>
            <option value="bottom">Дно</option>
            <option value="zone">Зона</option>
          </select>
        </label>
        {(tool === "setDepth" || tool === "water") && (
          <label>
            Глубина {depth.toFixed(2)} м
            <input
              type="range"
              min={0.1}
              max={12}
              step={0.05}
              value={depth}
              onChange={(e) => setDepth(Number(e.target.value))}
            />
          </label>
        )}
        {tool === "bottom" && (
          <label>
            Тип дна
            <select value={bottom} onChange={(e) => setBottom(e.target.value as Bottom)}>
              {BOTTOMS.map((b) => (
                <option key={b} value={b}>
                  {BOTTOM_NAME[b]}
                </option>
              ))}
            </select>
          </label>
        )}
        {tool === "zone" && (
          <label>
            Зона
            <select value={zone} onChange={(e) => setZone(Number(e.target.value))}>
              {(session ? [1, 2, 3, 4, 5, 6, 7] : []).map((z) => (
                <option key={z} value={z}>
                  {z}
                </option>
              ))}
            </select>
          </label>
        )}
        <label>
          Радиус {radius}px
          <input
            type="range"
            min={1}
            max={40}
            value={radius}
            onChange={(e) => setRadius(Number(e.target.value))}
          />
        </label>
        <label>
          Прозрачность
          <input
            type="range"
            min={0}
            max={1}
            step={0.05}
            value={opacity}
            onChange={(e) => setOpacity(Number(e.target.value))}
          />
        </label>
        {view && (
          <fieldset>
            <legend>Калибровка</legend>
            {(
              [
                ["horizonY", 0, 0.95, 0.0025],
                ["shoreY", 0.3, 1, 0.0025],
                ["nearM", 0.5, 10, 0.1],
                ["fov", 0.6, 2.5, 0.01],
              ] as const
            ).map(([k, min, max, step]) => (
              <label key={k}>
                {k} {view[k].toFixed(k === "nearM" ? 1 : 3)}
                <input
                  type="range"
                  min={min}
                  max={max}
                  step={step}
                  value={view[k]}
                  onChange={(e) => changeView({ [k]: Number(e.target.value) })}
                />
              </label>
            ))}
            <code className="map-editor-ron">{ron}</code>
            <button onClick={() => navigator.clipboard.writeText(ron).catch(toastError)}>
              Копировать для .ron
            </button>
          </fieldset>
        )}
        <div className="dim map-editor-hover">{hover}</div>
        <button className="primary" disabled={!dirty} onClick={save}>
          Сохранить map.png
        </button>
      </div>
    </>
  );
}
