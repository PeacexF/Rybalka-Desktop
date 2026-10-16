import type { RodStateView } from "../ipc/bindings/RodStateView";
import type { RodView } from "../ipc/bindings/RodView";

type Fighting = Extract<RodStateView, { state: "Fighting" }>;

/** Tension meter: fill = line tension, marker = drag setting. */
export function FightBar({ rod, state }: { rod: RodView; state: Fighting }) {
  const max = 1.15;
  const fill = Math.min(1, state.tension / max);
  const drag = Math.min(1, rod.dragKg / rod.lineStrengthKg / max);
  const danger = state.tension > 0.85;
  return (
    <div className="fight">
      <div className="fight-row">
        <span>{state.distanceM.toFixed(1)} м</span>
        <span className={state.slipping ? "slip on" : "slip"}>
          {state.reeling ? "не крутите на рывке!" : "фрикцион трещит"}
        </span>
      </div>
      <div className="tension">
        <div
          className={`tension-fill ${danger ? "danger" : ""}`}
          style={{ width: `${fill * 100}%` }}
        />
        <div className="tension-break" style={{ left: `${(1 / max) * 100}%` }} />
        <div className="tension-drag" style={{ left: `${drag * 100}%` }} />
      </div>
    </div>
  );
}
