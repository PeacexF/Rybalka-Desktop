import { useEffect, useState } from "react";
import type { Diary as DiaryData } from "../../ipc/bindings/Diary";
import type { Weather } from "../../ipc/bindings/Weather";
import { diary } from "../../ipc/game";
import { formatGameTime, formatMoney, formatWeight, plural } from "../../format";
import { WEATHER_NAME } from "../../hud/Hud";
import { KIND_NAME } from "../../hud/RodBar";
import { toastError } from "../../store/game";

type Section = "records" | "trips" | "catches";

/** Personal records, trip diary and the catch log. */
export function Diary() {
  const [data, setData] = useState<DiaryData | null>(null);
  const [section, setSection] = useState<Section>("records");

  useEffect(() => {
    diary().then(setData).catch(toastError);
  }, []);
  if (!data) return <div className="dim">Загружаем дневник…</div>;

  const caught = data.records.filter((r) => r.record).length;

  return (
    <div className="diary">
      <div className="chips">
        {(
          [
            ["records", `Рекорды · ${caught} из ${data.records.length}`],
            ["trips", `Поездки · ${data.trips.length}`],
            ["catches", "Уловы"],
          ] as const
        ).map(([id, name]) => (
          <button key={id} className={section === id ? "on" : ""} onClick={() => setSection(id)}>
            {name}
          </button>
        ))}
      </div>
      {section === "records" && <Records data={data} />}
      {section === "trips" && <Trips data={data} />}
      {section === "catches" && <Catches data={data} />}
    </div>
  );
}

function Records({ data }: { data: DiaryData }) {
  return (
    <table className="diary-table">
      <thead>
        <tr>
          <th>Вид</th>
          <th>Рекорд</th>
          <th>Когда и где</th>
          <th>Снасть</th>
          <th>Поймано</th>
        </tr>
      </thead>
      <tbody>
        {data.records.map(({ fishId, name, trophyKg, record: r }) => (
          <tr key={fishId} className={r ? "" : "unknown"}>
            <td>
              <b>{r ? name : "???"}</b>
              <div className="dim">трофей от {formatWeight(trophyKg)}</div>
            </td>
            <td className="num">
              {r ? formatWeight(r.weightKg) : "—"}
              {r && r.weightKg >= trophyKg && <span className="badge">★</span>}
            </td>
            <td>
              {r && (
                <>
                  {formatGameTime(r.atMin)}
                  <div className="dim">
                    {r.location} · {r.spot}
                  </div>
                </>
              )}
            </td>
            <td>
              {r && (
                <>
                  {KIND_NAME[r.rod]}
                  <div className="dim">{r.bait}</div>
                </>
              )}
            </td>
            <td className="num">
              {r ? r.count : 0}
              {r && r.trophies > 0 && (
                <div className="dim">
                  {r.trophies} {plural(r.trophies, "трофей", "трофея", "трофеев")}
                </div>
              )}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

const weathers = (ws: Weather[]) => ws.map((w) => WEATHER_NAME[w].toLowerCase()).join(", ");

function Trips({ data }: { data: DiaryData }) {
  if (data.trips.length === 0)
    return <div className="dim">Поездок пока не было — купите путёвку в городе.</div>;
  return (
    <ul className="diary-trips">
      {data.trips.map((t, i) => (
        <li key={i} className="panel">
          <div className="trip-head">
            <b>{t.location}</b>
            <span className="dim">
              {formatGameTime(t.startedMin)} — {formatGameTime(t.endedMin)}
            </span>
          </div>
          <div>
            {t.fish} {plural(t.fish, "рыба", "рыбы", "рыб")}, {formatWeight(t.kg)} ·{" "}
            {formatMoney(t.earned)}
            {t.bestName && (
              <span className="dim">
                {" "}
                · лучшая: {t.bestName}, {formatWeight(t.bestKg)}
              </span>
            )}
          </div>
          {t.weathers.length > 0 && <div className="dim">Погода: {weathers(t.weathers)}</div>}
        </li>
      ))}
    </ul>
  );
}

function Catches({ data }: { data: DiaryData }) {
  if (data.catches.length === 0) return <div className="dim">Ещё ничего не поймано.</div>;
  return (
    <table className="diary-table">
      <tbody>
        {data.catches.map((c, i) => (
          <tr key={i}>
            <td className="dim">{formatGameTime(c.atMin)}</td>
            <td>
              <b>{c.fishName}</b>
              {c.record && <span className="badge">рекорд</span>}
            </td>
            <td className="num">
              {formatWeight(c.weightKg)}
              {c.trophy && <span className="badge">★</span>}
            </td>
            <td className="dim">
              {c.location} · {c.spot}
            </td>
            <td className="dim">
              {KIND_NAME[c.rod]}, {c.bait.toLowerCase()} · {WEATHER_NAME[c.weather].toLowerCase()}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
