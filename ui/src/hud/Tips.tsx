import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Snapshot } from "../ipc/bindings/Snapshot";
import { toastError, useGame } from "../store/game";

/** Tip id that turns all tips off (matches `profile::ALL_TIPS`). */
const ALL = "all";

const TEXT: Record<string, string> = {
  cast: "Кликните по воде, чтобы забросить. У курсора видно глубину и дистанцию; если подсказка красная, туда не добросить.",
  wait: "Ждите поклёвку. Когда поплавок уйдёт под воду или поведёт в сторону, кликните, чтобы подсечь. Если подсечь на мелкой дрожи, будет пустая подсечка.",
  wait_feeder:
    "Донка: следите за вершинкой в углу и слушайте колокольчик. Когда вершинку резко согнёт, кликайте.",
  lure: "Держите ЛКМ, и приманка пойдёт к берегу. Колесо мыши меняет скорость проводки. Паузы тоже ловят.",
  fight:
    "Держите ЛКМ, чтобы подматывать. Полоса показывает натяжение: у красной черты отпустите, иначе будет обрыв. Колесом мыши настраивается фрикцион.",
  landed: "Рыба ваша! Положите её в садок, а продать улов можно на базе.",
  rods: "Можно держать до трёх удочек. Соберите их на базе во вкладке «Снасти» и переключайтесь клавишами 1 2 3.",
  time: "Клёв зависит от времени суток и погоды: лучше всего утром и вечером. Чтобы ускорить время, держите T. По Esc откроется пауза, оттуда можно уйти на базу.",
  town: "Это город. Купите путёвку на «Пруд у деревни», она бесплатная, и поезжайте на базу.",
  base: "Вы на базе. Выберите место ловли: удочка уже собрана. Здесь же есть лавка и продажа улова, а ещё можно поспать до утра.",
  sell: "Продайте улов. Деньги нужны на снасти, а опыт открывает новые разряды и водоёмы.",
};

/** Id of the first tip that fits what the player is doing right now, in tutorial order. */
function tipFor(s: Snapshot): string | null {
  const p = s.profile;
  if (!p) return null;
  const session = s.session;
  let fits: [boolean, string][];
  if (session) {
    const rod = session.rods[session.activeRod];
    const st = rod?.state.state;
    const caught = session.keepnet.length;
    fits = [
      [st === "Idle" && caught === 0, "cast"],
      [(st === "Waiting" || st === "Bite") && rod?.kind === "Float", "wait"],
      [(st === "Waiting" || st === "Bite") && rod?.kind === "Feeder", "wait_feeder"],
      [st === "Lure", "lure"],
      [st === "Fighting", "fight"],
      [st === "Landed", "landed"],
      [st === "Idle" && caught >= 1, "rods"],
      [caught >= 3, "time"],
    ];
  } else if (!p.trip) {
    fits = [[true, "town"]];
  } else {
    fits = [
      [p.keepnet === 0, "base"],
      [p.keepnet > 0, "sell"],
    ];
  }
  const seen = new Set(p.tipsSeen);
  if (seen.has(ALL)) return null;
  return fits.find(([when, id]) => when && !seen.has(id))?.[1] ?? null;
}

/** One-line tutorial hints for the first trip; closed tips are kept in the save. */
export function Tips() {
  const tip = useGame((s) => (s.snapshot ? tipFor(s.snapshot) : null));
  const onWater = useGame((s) => !!s.snapshot?.session);
  const paused = useGame((s) => s.snapshot?.session?.paused ?? false);
  // Hide right away, before the next snapshot confirms it.
  const [closed, setClosed] = useState<string[]>([]);
  if (!tip || paused || closed.includes(tip) || closed.includes(ALL)) return null;

  const dismiss = (id: string) => {
    setClosed((c) => [...c, id]);
    invoke("dismiss_tip", { id }).catch(toastError);
  };

  return (
    <div className={`panel tip ${onWater ? "on-water" : ""}`}>
      <div>{TEXT[tip]}</div>
      <div className="tip-actions">
        <button className="link" onClick={() => dismiss(ALL)}>
          Больше не подсказывать
        </button>
        <button className="primary" onClick={() => dismiss(tip)}>
          Понятно
        </button>
      </div>
    </div>
  );
}
