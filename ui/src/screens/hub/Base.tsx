import type { ProfileView } from "../../ipc/bindings/ProfileView";
import { endTrip, goFishing, sellCatch, sleep } from "../../ipc/game";
import { formatClock, formatWeight, plural } from "../../format";
import { toastError, useGame } from "../../store/game";

/** At a fishing base: pick a spot, sell the catch, sleep, go home. */
export function Base({ profile, onTackle }: { profile: ProfileView; onTackle: () => void }) {
  const trip = profile.trip!;
  const loc = useGame((s) => s.locations.find((l) => l.id === trip.location));
  const ready = profile.slots.some((s) => s.rod && s.reel && s.line);
  const night = profile.hour >= 22 || profile.hour < 4;
  const n = profile.keepnet;
  const setSale = useGame((s) => s.setSale);

  return (
    <div className="base">
      <div className="base-spots">
        {trip.over && (
          <div className="hub-note">Путёвка закончилась. Можно продать улов и ехать домой.</div>
        )}
        {!ready && !trip.over && (
          <div className="hub-note">
            Ни одна удочка не собрана.{" "}
            <button className="link" onClick={onTackle}>
              Собрать во вкладке «Снасти»
            </button>
          </div>
        )}
        <div className="spot-grid">
          {loc?.spots.map((spot) => (
            <div key={spot.id} className="spot-card">
              <img src={`/${spot.background}`} alt="" />
              <div className="spot-foot">
                <span>{spot.name}</span>
                <button
                  className="primary"
                  disabled={!ready || trip.over}
                  onClick={() => goFishing(trip.location, spot.id).catch(toastError)}
                >
                  Рыбачить
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>
      <aside className="panel base-side">
        <div>
          <div className="dim">Путёвка до</div>
          <b>
            дня {trip.endsDay}, {formatClock(trip.endsHour)}
          </b>
        </div>
        <div>
          <div className="dim">Садок</div>
          <b>
            {n === 0
              ? "пусто"
              : `${n} ${plural(n, "рыба", "рыбы", "рыб")}, ${formatWeight(profile.keepnetKg)}`}
          </b>
        </div>
        <button disabled={n === 0} onClick={() => sellCatch().then(setSale).catch(toastError)}>
          Продать улов
        </button>
        <button
          disabled={!trip.canSleep}
          title={trip.canSleep ? undefined : "Путёвка закончится раньше утра"}
          onClick={() => sleep().catch(toastError)}
        >
          {night ? "Спать до утра" : "Отдохнуть до утра"}
        </button>
        <button
          className={trip.over ? "primary" : ""}
          onClick={() =>
            endTrip()
              .then((r) => r.lines.length > 0 && setSale(r))
              .catch(toastError)
          }
        >
          {n > 0 ? "Продать улов и уехать" : "Уехать домой"}
        </button>
      </aside>
    </div>
  );
}
