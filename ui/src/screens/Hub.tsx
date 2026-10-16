import { useState } from "react";
import { closeGame } from "../ipc/game";
import { formatMoney } from "../format";
import { toastError, useGame } from "../store/game";
import { SaleCard } from "./hub/SaleCard";
import { Shop } from "./hub/Shop";
import { Tackle } from "./hub/Tackle";
import { Waters } from "./hub/Waters";

const TABS = [
  ["waters", "Водоёмы"],
  ["shop", "Магазин"],
  ["tackle", "Снасти"],
] as const;
type Tab = (typeof TABS)[number][0];

/** Between trips: choose water, buy gear, assemble rods. */
export function Hub() {
  const profile = useGame((s) => s.snapshot?.profile ?? null);
  const sale = useGame((s) => s.sale);
  const [tab, setTab] = useState<Tab>("waters");
  if (!profile) return null;

  const next = profile.nextRankXp;
  const progress = next ? (profile.xp - profile.rankXp) / (next - profile.rankXp) : 1;

  return (
    <div className="screen hub-screen">
      <div className="hub">
        <header className="hub-head">
          <div className="hub-title">Рыбалка</div>
          <div className="hub-stat">
            <span className="dim">Деньги</span>
            <b>{formatMoney(profile.money)}</b>
          </div>
          <div className="hub-stat" title={next ? `${profile.xp} из ${next} опыта` : undefined}>
            <span className="dim">Разряд</span>
            <b>{profile.rankName}</b>
            <div className="xp-bar">
              <div style={{ width: `${Math.min(1, Math.max(0, progress)) * 100}%` }} />
            </div>
          </div>
          <button onClick={() => closeGame().catch(toastError)}>Главное меню</button>
        </header>
        <nav className="hub-tabs">
          {TABS.map(([id, name]) => (
            <button key={id} className={tab === id ? "on" : ""} onClick={() => setTab(id)}>
              {name}
            </button>
          ))}
        </nav>
        <main className="hub-body">
          {tab === "waters" && <Waters profile={profile} onTackle={() => setTab("tackle")} />}
          {tab === "shop" && <Shop profile={profile} />}
          {tab === "tackle" && <Tackle profile={profile} />}
        </main>
      </div>
      {sale && <SaleCard report={sale} />}
    </div>
  );
}
