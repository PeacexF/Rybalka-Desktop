import { useEffect, useRef, useState } from "react";
import type { GameEvent } from "./ipc/bindings/GameEvent";
import { baits, sendInput, setDebug, startSession, subscribe } from "./ipc/game";
import { sfx } from "./audio";
import { Hud } from "./hud/Hud";
import { Scene } from "./scene/Scene";
import { WaterMap } from "./scene/WaterMap";
import { onGameEvent, useGame } from "./store/game";

const EVENT_TOASTS: Partial<Record<GameEvent["type"], [string, "info" | "bad" | "good"]>> = {
  CastOnLand: ["Заброс на берег — снасть вернулась", "bad"],
  Missed: ["Мимо! Пустая подсечка", "info"],
  FishLeft: ["Рыба отошла", "info"],
  LineBroke: ["Обрыв лески!", "bad"],
  Escaped: ["Сход!", "bad"],
  Snagged: ["Зацеп! ПКМ — попробовать отцепить", "bad"],
  Unsnagged: ["Отцепилась!", "good"],
};

function playEventSound(e: GameEvent): void {
  const s = useGame.getState().snapshot?.session;
  const rod = s?.rods[e.rod];
  switch (e.type) {
    case "CastLanded":
      sfx.plop(rod?.kind === "Feeder" ? 1.4 : 1);
      break;
    case "Bite":
      if (rod?.kind === "Feeder") sfx.bell();
      else if (s && e.rod !== s.activeRod) sfx.bloop();
      break;
    case "Hooked":
    case "Landed":
      sfx.splash();
      break;
    case "LineBroke":
      sfx.snap();
      break;
    case "Snagged":
      sfx.click(0.4);
      break;
  }
}

interface Hover {
  x: number;
  y: number;
  depthM: number;
  distanceM: number;
  reachable: boolean;
}

export function App() {
  const stageRef = useRef<HTMLDivElement>(null);
  const sceneRef = useRef<Scene | null>(null);
  const mapRef = useRef<WaterMap | null>(null);
  const [hover, setHover] = useState<Hover | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Scene lifecycle.
  useEffect(() => {
    const host = stageRef.current;
    if (!host) return;
    const scene = new Scene();
    let disposed = false;
    scene.init(host).then(() => {
      if (disposed) scene.destroy();
      else sceneRef.current = scene;
    });
    return () => {
      disposed = true;
      if (sceneRef.current) {
        sceneRef.current.destroy();
        sceneRef.current = null;
      }
    };
  }, []);

  // Game connection: subscribe, start the session, load the water map.
  useEffect(() => {
    let mapUrl = "";
    const off = onGameEvent((e) => {
      sceneRef.current?.onEvent(e);
      playEventSound(e);
      const t = EVENT_TOASTS[e.type];
      if (t) useGame.getState().toast(t[0], t[1]);
    });
    subscribe((s) => {
      useGame.getState().setSnapshot(s);
      sceneRef.current?.update(s.session, performance.now());
      const url = s.session ? `/${s.session.map}` : "";
      if (url && url !== mapUrl) {
        mapUrl = url;
        WaterMap.load(url).then((m) => (mapRef.current = m));
      }
    })
      .then(() => baits())
      .then((b) => useGame.getState().setBaits(b))
      .then(() => startSession("pond", "willow"))
      .catch((e) => setError(String(e)));
    return off;
  }, []);

  // Reel clicks and drag ratchet while fighting.
  useEffect(() => {
    let acc = 0;
    const id = setInterval(() => {
      const s = useGame.getState().snapshot?.session;
      const st = s?.rods[s.activeRod]?.state;
      acc += 1;
      if (st?.state === "Lure" && st.reeling && acc % 3 === 0) sfx.click(0.05);
      if (st?.state !== "Fighting") return;
      if (st.slipping) sfx.click(0.18);
      else if (st.reeling && acc % 2 === 0) sfx.click(0.07);
    }, 45);
    return () => clearInterval(id);
  }, []);

  // Keyboard.
  useEffect(() => {
    const active = () => {
      const s = useGame.getState().snapshot?.session;
      return s ? { index: s.activeRod, rod: s.rods[s.activeRod] } : null;
    };
    const down = (e: KeyboardEvent) => {
      const a = active();
      if (e.repeat && e.code !== "BracketLeft" && e.code !== "BracketRight") return;
      switch (e.code) {
        case "Space":
          if (a) sendInput({ type: "Retrieve", rod: a.index });
          e.preventDefault();
          break;
        case "KeyT":
          sendInput({ type: "FastForward", on: true });
          break;
        case "Digit1":
        case "Digit2":
        case "Digit3": {
          // Selecting a rod that has a bite strikes right away (as in RR).
          const i = Number(e.code.slice(5)) - 1;
          const st = useGame.getState().snapshot?.session?.rods[i]?.state.state;
          sendInput({ type: "SelectRod", rod: i });
          if (st === "Bite") sendInput({ type: "Strike", rod: i });
          break;
        }
        case "KeyS":
          useGame.getState().toggleKeepnet();
          break;
        case "Escape":
          useGame.getState().toggleKeepnet(false);
          break;
        case "F3": {
          useGame.getState().toggleDebug();
          setDebug(useGame.getState().debug);
          e.preventDefault();
          break;
        }
        case "BracketLeft":
        case "BracketRight":
          if (a?.rod)
            sendInput({
              type: "SetFloatDepth",
              rod: a.index,
              m: a.rod.floatDepthM + (e.code === "BracketRight" ? 0.1 : -0.1),
            });
          break;
      }
    };
    const up = (e: KeyboardEvent) => {
      if (e.code === "KeyT") sendInput({ type: "FastForward", on: false });
    };
    const blur = () => {
      const a = active();
      if (a) sendInput({ type: "Reel", rod: a.index, on: false });
      sendInput({ type: "FastForward", on: false });
    };
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    window.addEventListener("blur", blur);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
      window.removeEventListener("blur", blur);
    };
  }, []);

  const activeRod = () => {
    const s = useGame.getState().snapshot?.session;
    return s ? { index: s.activeRod, rod: s.rods[s.activeRod]!, session: s } : null;
  };

  const onPointerDown = (e: React.PointerEvent) => {
    const a = activeRod();
    const scene = sceneRef.current;
    if (!a || !scene) return;
    if (e.button === 2) {
      sendInput({ type: "Retrieve", rod: a.index });
      return;
    }
    if (e.button !== 0) return;
    switch (a.rod.state.state) {
      case "Idle": {
        const p = scene.toImage(e.clientX, e.clientY);
        if (mapRef.current && !mapRef.current.sample(p.x, p.y)) {
          useGame.getState().toast("Туда не забросить — там нет воды");
          return;
        }
        sendInput({ type: "Cast", rod: a.index, x: p.x, y: p.y });
        break;
      }
      case "Waiting":
      case "Bite":
        sendInput({ type: "Strike", rod: a.index });
        break;
      case "Fighting":
      case "Lure":
        sendInput({ type: "Reel", rod: a.index, on: true });
        (e.target as Element).setPointerCapture(e.pointerId);
        break;
    }
  };

  const onPointerUp = (e: React.PointerEvent) => {
    const a = activeRod();
    if (a && e.button === 0) sendInput({ type: "Reel", rod: a.index, on: false });
  };

  const onWheel = (e: React.WheelEvent) => {
    const a = activeRod();
    if (!a) return;
    const up = e.deltaY < 0;
    if (a.rod.state.state === "Lure") {
      const mps = a.rod.retrieveMps + (up ? 0.05 : -0.05);
      sendInput({ type: "SetRetrieveSpeed", rod: a.index, mps });
    } else {
      sendInput({ type: "SetDrag", rod: a.index, kg: a.rod.dragKg + (up ? 0.1 : -0.1) });
    }
  };

  const onPointerMove = (e: React.PointerEvent) => {
    const scene = sceneRef.current;
    const map = mapRef.current;
    const a = activeRod();
    if (!scene || !map || !a) return;
    const p = scene.toImage(e.clientX, e.clientY);
    const cell = map.sample(p.x, p.y);
    if (!cell) return setHover(null);
    const v = a.session.view;
    const distanceM = (v.nearM * (v.shoreY - v.horizonY)) / Math.max(1e-3, p.y - v.horizonY);
    setHover({
      x: e.clientX,
      y: e.clientY,
      depthM: cell.depthM,
      distanceM,
      reachable: distanceM <= a.rod.maxCastM,
    });
  };

  return (
    <>
      <div
        className="stage"
        ref={stageRef}
        onPointerDown={onPointerDown}
        onPointerUp={onPointerUp}
        onPointerMove={onPointerMove}
        onPointerLeave={() => setHover(null)}
        onWheel={onWheel}
        onContextMenu={(e) => e.preventDefault()}
      />
      <div className="overlay">
        <Hud />
        {hover && (
          <div
            className={`cursor-info ${hover.reachable ? "" : "far"}`}
            style={{ left: hover.x + 16, top: hover.y + 12 }}
          >
            {hover.depthM.toFixed(1)} м · {Math.round(hover.distanceM)} м
            {!hover.reachable && " (не добросить)"}
          </div>
        )}
        {error && <div className="fatal">{error}</div>}
      </div>
    </>
  );
}
