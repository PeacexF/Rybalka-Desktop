import type { RodView } from "../ipc/bindings/RodView";
import { sendInput } from "../ipc/game";
import { useGame } from "../store/game";

const KIND_NAME = { Float: "Поплавочная удочка", Feeder: "Фидер", Spinning: "Спиннинг" } as const;

function stateText(rod: RodView): { text: string; hint: string } {
  const st = rod.state;
  switch (st.state) {
    case "Idle":
      return { text: "Снасть в руках", hint: "ЛКМ по воде — заброс" };
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
      return { text: "Вытаскиваем снасть…", hint: "" };
    case "Landed":
      return { text: "Рыба в подсачеке", hint: "" };
  }
}

export function RodPanel({ index, rod }: { index: number; rod: RodView }) {
  const baits = useGame((s) => s.baits);
  const { text, hint } = stateText(rod);
  const st = rod.state.state;
  const canRig = st === "Idle" || st === "Waiting";

  const setDepth = (m: number) => sendInput({ type: "SetFloatDepth", rod: index, m });
  const setDrag = (kg: number) => sendInput({ type: "SetDrag", rod: index, kg });

  return (
    <div className="hud-bottom">
      <div className={`panel rod-card ${st === "Bite" ? "rod-alert" : ""}`}>
        <div className="rod-head">
          <span className="rod-slot">{index + 1}</span>
          <span className="rod-name">{KIND_NAME[rod.kind]}</span>
          <span className="rod-state">{text}</span>
        </div>
        <div className="rod-controls">
          <label>
            Наживка
            <select
              value={rod.bait}
              disabled={!canRig}
              onChange={(e) => sendInput({ type: "SetBait", rod: index, bait: e.target.value })}
            >
              {baits
                .filter((b) => b.kind === "Natural")
                .map((b) => (
                  <option key={b.id} value={b.id}>
                    {b.name}
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
          <div className="stepper">
            <span>Фрикцион</span>
            <button onClick={() => setDrag(rod.dragKg - 0.1)}>−</button>
            <b>{rod.dragKg.toFixed(1)} кг</b>
            <button onClick={() => setDrag(rod.dragKg + 0.1)}>+</button>
          </div>
          <span className="rod-line">леска {rod.lineStrengthKg.toFixed(1)} кг</span>
        </div>
        {hint && <div className="rod-hint">{hint}</div>}
      </div>
    </div>
  );
}
