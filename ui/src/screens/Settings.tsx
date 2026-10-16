import { useEffect, useState } from "react";
import { sfx } from "../audio";
import { TIME_SPEEDS, useSettings, type Settings as S } from "../store/settings";

const CONTROLS: [string, string][] = [
  ["Заброс", "ЛКМ по воде"],
  ["Подсечка", "ЛКМ при поклёвке"],
  ["Подмотка / проводка", "держать ЛКМ"],
  ["Фрикцион / скорость проводки", "колесо мыши"],
  ["Вытащить снасть, отцепить", "ПКМ или Space"],
  ["Выбрать удочку", "1 2 3"],
  ["Глубина поплавка", "[ ]"],
  ["Листать панораму", "A D, ← →, мышь у края"],
  ["Садок", "S"],
  ["Ускорить время", "держать T"],
  ["Пауза", "Esc"],
  ["Полный экран", "F11"],
];

function Volume({ label, k }: { label: string; k: "master" | "sfx" | "ambient" }) {
  const value = useSettings((s) => s.settings[k]);
  const update = useSettings((s) => s.update);
  return (
    <label className="setting">
      <span>{label}</span>
      <input
        type="range"
        min={0}
        max={1}
        step={0.05}
        value={value}
        onChange={(e) => update({ [k]: Number(e.target.value) } as Partial<S>)}
        // A short sample so the level can be judged.
        onPointerUp={() => k !== "ambient" && sfx.plop()}
      />
      <b>{Math.round(value * 100)}%</b>
    </label>
  );
}

/** Settings dialog: sound, game speed, window and the controls reference. */
export function Settings() {
  const open = useSettings((s) => s.open);
  const settings = useSettings((s) => s.settings);
  const { update, setOpen } = useSettings.getState();
  const [tab, setTab] = useState<"game" | "controls">("game");

  // Esc closes the dialog before anything underneath sees it.
  useEffect(() => {
    if (!open) return;
    const down = (e: KeyboardEvent) => {
      if (e.code !== "Escape") return;
      e.stopImmediatePropagation();
      setOpen(false);
    };
    window.addEventListener("keydown", down, true);
    return () => window.removeEventListener("keydown", down, true);
  }, [open, setOpen]);

  if (!open) return null;

  return (
    <div className="modal-backdrop dimmed">
      <div className="panel menu-card settings">
        <div className="menu-title">Настройки</div>
        <nav className="chips">
          <button className={tab === "game" ? "on" : ""} onClick={() => setTab("game")}>
            Звук и игра
          </button>
          <button className={tab === "controls" ? "on" : ""} onClick={() => setTab("controls")}>
            Управление
          </button>
        </nav>
        {tab === "game" ? (
          <div className="settings-body">
            <h3>Звук</h3>
            <Volume label="Общая громкость" k="master" />
            <Volume label="Эффекты" k="sfx" />
            <Volume label="Природа и погода" k="ambient" />
            <h3>Игра</h3>
            <label className="setting">
              <span>Скорость времени</span>
              <select
                value={settings.timeSpeed}
                onChange={(e) => update({ timeSpeed: Number(e.target.value) })}
              >
                {TIME_SPEEDS.map(([v, name]) => (
                  <option key={v} value={v}>
                    {name}: 1 мин = {v} игровых мин
                  </option>
                ))}
              </select>
            </label>
            <label className="setting">
              <span>Полный экран</span>
              <input
                type="checkbox"
                checked={settings.fullscreen}
                onChange={(e) => update({ fullscreen: e.target.checked })}
              />
              <kbd>F11</kbd>
            </label>
          </div>
        ) : (
          <table className="controls">
            <tbody>
              {CONTROLS.map(([what, keys]) => (
                <tr key={what}>
                  <td>{what}</td>
                  <td>{keys}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <div className="menu-buttons">
          <button className="primary" onClick={() => setOpen(false)}>
            Готово <kbd>Esc</kbd>
          </button>
        </div>
      </div>
    </div>
  );
}
