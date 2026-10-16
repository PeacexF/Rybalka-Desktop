import { useState } from "react";
import type { LocationInfo } from "../../ipc/bindings/LocationInfo";
import type { ProfileView } from "../../ipc/bindings/ProfileView";
import type { WaterKind } from "../../ipc/bindings/WaterKind";
import { buyTrip } from "../../ipc/game";
import { formatMoney, plural } from "../../format";
import { toastError, useGame } from "../../store/game";

const KIND_NAME: Record<WaterKind, string> = {
  Pond: "Пруд",
  Lake: "Озеро",
  River: "Река",
  Reservoir: "Водохранилище",
};

/** Town: buy a pass to one of the waters. */
export function Trips({ profile, onTackle }: { profile: ProfileView; onTackle: () => void }) {
  const locations = useGame((s) => s.locations);
  const ready = profile.slots.some((s) => s.rod && s.reel && s.line);

  return (
    <div className="trips">
      {!ready && (
        <div className="hub-note">
          Ни одна удочка не собрана.{" "}
          <button className="link" onClick={onTackle}>
            Собрать во вкладке «Снасти»
          </button>
        </div>
      )}
      {locations.map((loc) => (
        <TripCard key={loc.id} loc={loc} profile={profile} />
      ))}
    </div>
  );
}

function TripCard({ loc, profile }: { loc: LocationInfo; profile: ProfileView }) {
  const [days, setDays] = useState(1);
  const locked = profile.rank < loc.rank;
  const cost = loc.pricePerDay * days;
  const poor = cost > profile.money;
  const cover = loc.spots[0]?.background;

  return (
    <div className={`panel trip-card ${locked ? "locked" : ""}`}>
      {cover && <img src={`/${cover}`} alt="" />}
      <div className="trip-info">
        <div className="trip-head">
          <h2>{loc.name}</h2>
          <span className="dim">
            {KIND_NAME[loc.kind]} · {loc.spots.length}{" "}
            {plural(loc.spots.length, "место", "места", "мест")}
          </span>
        </div>
        <p>{loc.description}</p>
        <div className="dim trip-fish">{loc.fish.join(", ")}</div>
        <div className="trip-buy">
          {locked ? (
            <span className="dim">Нужен разряд «{loc.rankName}»</span>
          ) : (
            <>
              <div className="stepper">
                <span>Дней</span>
                <button disabled={days <= 1} onClick={() => setDays(days - 1)}>
                  −
                </button>
                <b>{days}</b>
                <button disabled={days >= loc.maxDays} onClick={() => setDays(days + 1)}>
                  +
                </button>
              </div>
              <span className="trip-price">
                {loc.pricePerDay === 0 ? "бесплатно" : formatMoney(cost)}
              </span>
              <button
                className="primary"
                disabled={poor}
                title={poor ? "Не хватает денег" : undefined}
                onClick={() => buyTrip(loc.id, days).catch(toastError)}
              >
                Поехать
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
