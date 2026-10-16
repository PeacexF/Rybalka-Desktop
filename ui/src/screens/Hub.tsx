import { useState } from "react";
import { closeGame } from "../ipc/game";
import { formatClock, formatMoney } from "../format";
import { toastError, useGame } from "../store/game";
import { Base } from "./hub/Base";
import { Diary } from "./hub/Diary";
import { SaleCard } from "./hub/SaleCard";
import { Shop } from "./hub/Shop";
import { Tackle } from "./hub/Tackle";
import { Trips } from "./hub/Trips";

type Tab = "trips" | "base" | "shop" | "tackle" | "diary";

/**
 * Between sessions. In town: passes, the full shop and tackle. On a trip:
 * the base with its spots, the local shop and tackle.
 */
export function Hub() {
  const profile = useGame((s) => s.snapshot?.profile ?? null);
  const sale = useGame((s) => s.sale);
  const onTrip = !!profile?.trip;
  // Each place remembers its own tab; arriving opens that place's main tab.
  const [picked, setPicked] = useState<{ town: Tab; trip: Tab }>({ town: "trips", trip: "base" });
  const tab = onTrip ? picked.trip : picked.town;
  const setTab = (t: Tab) => setPicked((p) => (onTrip ? { ...p, trip: t } : { ...p, town: t }));
  if (!profile) return null;

  const tabs: [Tab, string][] = onTrip
    ? [
        ["base", "База"],
        ["shop", "Лавка"],
        ["tackle", "Снасти"],
        ["diary", "Дневник"],
      ]
    : [
        ["trips", "Путёвки"],
        ["shop", "Магазин"],
        ["tackle", "Снасти"],
        ["diary", "Дневник"],
      ];
  const next = profile.nextRankXp;
  const progress = next ? (profile.xp - profile.rankXp) / (next - profile.rankXp) : 1;

  return (
    <div className="screen hub-screen">
      <div className="hub">
        <header className="hub-head">
          <div className="hub-title">
            {profile.trip ? profile.trip.locationName : "Город"}
            <span className="dim">
              День {profile.day}, {formatClock(profile.hour)}
            </span>
          </div>
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
          {tabs.map(([id, name]) => (
            <button key={id} className={tab === id ? "on" : ""} onClick={() => setTab(id)}>
              {name}
            </button>
          ))}
        </nav>
        <main className="hub-body">
          {tab === "trips" && <Trips profile={profile} onTackle={() => setTab("tackle")} />}
          {tab === "base" && <Base profile={profile} onTackle={() => setTab("tackle")} />}
          {tab === "shop" && <Shop profile={profile} local={onTrip} />}
          {tab === "tackle" && <Tackle profile={profile} />}
          {tab === "diary" && <Diary />}
        </main>
      </div>
      {sale && <SaleCard report={sale} />}
    </div>
  );
}
