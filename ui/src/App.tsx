import { useEffect, useRef } from "react";
import { subscribe } from "./ipc/game";
import { Scene } from "./scene/Scene";
import { useGame } from "./store/game";

export function App() {
  const stageRef = useRef<HTMLDivElement>(null);
  const snapshot = useGame((s) => s.snapshot);

  useEffect(() => {
    const host = stageRef.current;
    if (!host) return;
    const scene = new Scene();
    let disposed = false;
    scene.init(host).then(() => {
      if (disposed) scene.destroy();
    });
    return () => {
      disposed = true;
      if (scene.app.renderer) scene.destroy();
    };
  }, []);

  useEffect(() => {
    subscribe(useGame.getState().setSnapshot).catch(console.error);
  }, []);

  return (
    <>
      <div className="stage" ref={stageRef} />
      <div className="overlay">
        <div style={{ padding: 12, fontFamily: "monospace" }}>
          tick {snapshot?.tick ?? "—"} · t={snapshot?.timeS.toFixed(1) ?? "—"}s
        </div>
      </div>
    </>
  );
}
