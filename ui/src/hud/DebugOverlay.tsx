import type { SessionView } from "../ipc/bindings/SessionView";
import type { Weather } from "../ipc/bindings/Weather";
import { sendInput } from "../ipc/game";
import { useGame } from "../store/game";

const WEATHERS: Weather[] = ["Clear", "Cloudy", "Overcast", "Rain", "Wind", "Fog"];
const HOURS = [3, 5, 7, 12, 19.5, 21, 23];

export function DebugOverlay({ session }: { session: SessionView }) {
  const tick = useGame((s) => s.snapshot?.tick ?? 0);
  const rates = session.debug?.biteRates ?? [];
  return (
    <div className="debug">
      <div>
        tick {tick} · {session.locationId}/{session.spotId} · hour {session.clock.hour.toFixed(2)}
      </div>
      <div className="debug-row">
        {HOURS.map((h) => (
          <button key={h} onClick={() => sendInput({ type: "Debug", hour: h, weather: null })}>
            {h}ч
          </button>
        ))}
      </div>
      <div className="debug-row">
        {WEATHERS.map((w) => (
          <button key={w} onClick={() => sendInput({ type: "Debug", hour: null, weather: w })}>
            {w}
          </button>
        ))}
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
