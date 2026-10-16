import type { ProfileView } from "../../ipc/bindings/ProfileView";
import { goFishing } from "../../ipc/game";
import { plural } from "../../format";
import { toastError, useGame } from "../../store/game";

export function Waters({ profile, onTackle }: { profile: ProfileView; onTackle: () => void }) {
  const locations = useGame((s) => s.locations);
  const ready = profile.slots.some((s) => s.rod && s.reel && s.line);
  const n = profile.keepnet;

  return (
    <div className="waters">
      {!ready && (
        <div className="hub-note">
          Ни одна удочка не собрана.{" "}
          <button className="link" onClick={onTackle}>
            Собрать во вкладке «Снасти»
          </button>
        </div>
      )}
      {n > 0 && (
        <div className="hub-note">
          В садке {n} {plural(n, "рыба", "рыбы", "рыб")} с прошлого раза — продадутся, когда уйдёте
          с водоёма.
        </div>
      )}
      {locations.map((loc) => (
        <section key={loc.id}>
          <h2>{loc.name}</h2>
          <div className="spot-grid">
            {loc.spots.map((spot) => (
              <div key={spot.id} className="spot-card">
                <img src={`/${spot.background}`} alt="" />
                <div className="spot-foot">
                  <span>{spot.name}</span>
                  <button
                    className="primary"
                    disabled={!ready}
                    onClick={() => goFishing(loc.id, spot.id).catch(toastError)}
                  >
                    Рыбачить
                  </button>
                </div>
              </div>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
