import type { RodView } from "../ipc/bindings/RodView";
import type { TackleKind } from "../ipc/bindings/TackleKind";
import type { SessionView } from "../ipc/bindings/SessionView";
import { sendInput } from "../ipc/game";
import { useGame } from "../store/game";

export const KIND_NAME = { Float: "Поплавочная", Feeder: "Фидер", Spinning: "Спиннинг" } as const;

/** Terminal tackle a cast uses, per rod kind (mirrors `profile::rig_parts`). */
const RIG: Record<RodView["kind"], [TackleKind, string][]> = {
  Float: [
    ["Hook", "крючки"],
    ["Float", "поплавки"],
  ],
  Feeder: [
    ["Hook", "крючки"],
    ["Feeder", "кормушки"],
    ["Groundbait", "прикормка"],
  ],
  Spinning: [],
};

/** Units of each kind of terminal tackle left. */
function useStock(): (kind: TackleKind) => number {
  const items = useGame((s) => s.snapshot?.profile?.items);
  const catalog = useGame((s) => s.catalog);
  return (kind) =>
    catalog.filter((i) => i.tackleKind === kind).reduce((n, i) => n + (items?.[i.id] ?? 0), 0);
}

function stateText(rod: RodView): { text: string; hint: string } {
  const st = rod.state;
  switch (st.state) {
    case "Idle":
      return { text: "В руках", hint: "ЛКМ по воде — заброс · 1/2/3 — выбрать удочку" };
    case "Casting":
      return { text: "Заброс…", hint: "" };
    case "Waiting":
      return {
        text: st.lying ? "Поплавок лёг — спуск больше глубины" : "Ждём поклёвку",
        hint: "ЛКМ — подсечка · ПКМ / Пробел — вытащить",
      };
    case "Bite":
      return { text: "Поклёвка!", hint: "ЛКМ — подсечка" };
    case "Fighting":
      return { text: "Вываживание", hint: "Держите ЛКМ — подмотка · колесо — фрикцион" };
    case "Retrieving":
      return { text: "Вытаскиваем…", hint: "" };
    case "Landed":
      return { text: "Рыба в подсачеке", hint: "" };
    case "Lure":
      return {
        text: `Проводка · ${Math.round(st.distanceM)} м · приманка ${st.depthM.toFixed(1)} из ${st.bottomM.toFixed(1)} м`,
        hint: "Держите ЛКМ — подмотка, отпустите — пауза · колесо — скорость · ПКМ — смотать",
      };
    case "Snagged":
      return { text: "Зацеп!", hint: "ПКМ / Пробел — дёрнуть (может оборвать)" };
  }
}

export function RodBar({ session }: { session: SessionView }) {
  return (
    <div className="hud-bottom">
      {session.rods.map((rod, i) =>
        i === session.activeRod ? (
          <RodCard key={i} index={i} rod={rod} />
        ) : (
          <RodChip key={i} index={i} rod={rod} />
        ),
      )}
    </div>
  );
}

function RodChip({ index, rod }: { index: number; rod: RodView }) {
  const alert = ["Bite", "Landed", "Snagged"].includes(rod.state.state);
  return (
    <button
      className={`panel rod-chip ${alert ? "rod-alert" : ""}`}
      onClick={() => sendInput({ type: "SelectRod", rod: index })}
    >
      <span className="rod-slot">{index + 1}</span>
      <span className="rod-name">{KIND_NAME[rod.kind]}</span>
      <span className="rod-state">{stateText(rod).text}</span>
    </button>
  );
}

function RodCard({ index, rod }: { index: number; rod: RodView }) {
  const baits = useGame((s) => s.baits);
  const items = useGame((s) => s.snapshot?.profile?.items);
  const stock = useStock();
  const { text, hint } = stateText(rod);
  const st = rod.state.state;
  const canRig = st === "Idle" || st === "Waiting";
  const wantLure = rod.kind === "Spinning";

  const setDepth = (m: number) => sendInput({ type: "SetFloatDepth", rod: index, m });
  const setDrag = (kg: number) => sendInput({ type: "SetDrag", rod: index, kg });
  const setSpeed = (mps: number) => sendInput({ type: "SetRetrieveSpeed", rod: index, mps });

  return (
    <div className={`panel rod-card ${st === "Bite" || st === "Snagged" ? "rod-alert" : ""}`}>
      <div className="rod-head">
        <span className="rod-slot">{index + 1}</span>
        <span className="rod-name">{KIND_NAME[rod.kind]}</span>
        <span className="rod-state">{text}</span>
      </div>
      <div className="rod-controls">
        <label>
          {wantLure ? "Приманка" : "Наживка"}
          <select
            value={rod.bait}
            disabled={!canRig}
            onChange={(e) => sendInput({ type: "SetBait", rod: index, bait: e.target.value })}
          >
            {baits
              .filter((b) => (b.kind === "Lure") === wantLure)
              .filter((b) => !items || (items[b.id] ?? 0) > 0 || b.id === rod.bait)
              .map((b) => (
                <option key={b.id} value={b.id}>
                  {b.name}
                  {items && ` · ${items[b.id] ?? 0}`}
                </option>
              ))}
          </select>
        </label>
        {rod.kind === "Float" && (
          <div className="stepper">
            <span>Спуск</span>
            <button disabled={!canRig} onClick={() => setDepth(rod.floatDepthM - 0.1)}>
              −
            </button>
            <b>{rod.floatDepthM.toFixed(2)} м</b>
            <button disabled={!canRig} onClick={() => setDepth(rod.floatDepthM + 0.1)}>
              +
            </button>
          </div>
        )}
        {rod.kind === "Spinning" && (
          <div className="stepper">
            <span>Проводка</span>
            <button onClick={() => setSpeed(rod.retrieveMps - 0.05)}>−</button>
            <b>{rod.retrieveMps.toFixed(2)} м/с</b>
            <button onClick={() => setSpeed(rod.retrieveMps + 0.05)}>+</button>
          </div>
        )}
        <div className="stepper">
          <span>Фрикцион</span>
          <button onClick={() => setDrag(rod.dragKg - 0.1)}>−</button>
          <b>{rod.dragKg.toFixed(1)} кг</b>
          <button onClick={() => setDrag(rod.dragKg + 0.1)}>+</button>
        </div>
        <span className="rod-line">
          леска {rod.lineStrengthKg.toFixed(1)} кг · до {rod.maxCastM} м
          {items &&
            RIG[rod.kind].map(([kind, name]) => (
              <span key={kind} className={stock(kind) === 0 ? "out" : ""}>
                {" "}
                · {name} {stock(kind)}
              </span>
            ))}
        </span>
      </div>
      {hint && <div className="rod-hint">{hint}</div>}
    </div>
  );
}
