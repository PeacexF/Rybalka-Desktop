import { useEffect } from "react";
import type { Catch } from "../ipc/bindings/Catch";
import { sendInput } from "../ipc/game";
import { formatWeight } from "../format";

export function CatchCard({ index, catch_ }: { index: number; catch_: Catch }) {
  const keep = () => sendInput({ type: "KeepCatch", rod: index });
  const release = () => sendInput({ type: "ReleaseCatch", rod: index });

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.code === "Enter") keep();
      if (e.code === "Backspace" || e.code === "KeyR") release();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  return (
    <div className="modal-backdrop">
      <div className="panel catch-card">
        {catch_.trophy && <div className="trophy">Трофей!</div>}
        <div className="catch-name">{catch_.name}</div>
        <div className="catch-weight">{formatWeight(catch_.weightKg)}</div>
        <div className="catch-actions">
          <button className="primary" onClick={keep}>
            В садок <kbd>Enter</kbd>
          </button>
          <button onClick={release}>
            Отпустить <kbd>R</kbd>
          </button>
        </div>
      </div>
    </div>
  );
}
