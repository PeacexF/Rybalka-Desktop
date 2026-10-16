import type { SessionView } from "../ipc/bindings/SessionView";
import { useGame } from "../store/game";

export function DebugOverlay({ session }: { session: SessionView }) {
  const tick = useGame((s) => s.snapshot?.tick ?? 0);
  const rates = session.debug?.biteRates ?? [];
  return (
    <div className="debug">
      <div>
        tick {tick} · {session.locationId}/{session.spotId} · hour {session.clock.hour.toFixed(2)}
      </div>
      {rates.map((r, i) => (
        <div key={i}>
          rod {i + 1}:{" "}
          {r.length === 0
            ? "—"
            : r.map(([name, perMin]) => `${name} ${perMin.toFixed(2)}/мин`).join(" · ")}
        </div>
      ))}
    </div>
  );
}
