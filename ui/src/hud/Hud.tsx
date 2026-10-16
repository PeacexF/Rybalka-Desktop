import type { Weather } from "../ipc/bindings/Weather";
import { sendInput } from "../ipc/game";
import { useGame } from "../store/game";
import { formatClock } from "../format";
import { CatchCard } from "./CatchCard";
import { DebugOverlay } from "./DebugOverlay";
import { FightBar } from "./FightBar";
import { Keepnet } from "./Keepnet";
import { RodBar } from "./RodBar";

export const WEATHER_NAME: Record<Weather, string> = {
  Clear: "Ясно",
  Cloudy: "Облачно",
  Overcast: "Пасмурно",
  Rain: "Дождь",
  Wind: "Ветер",
  Fog: "Туман",
};

export function Hud() {
  const session = useGame((s) => s.snapshot?.session ?? null);
  const keepnetOpen = useGame((s) => s.keepnetOpen);
  const debug = useGame((s) => s.debug);
  if (!session) return null;
  const rod = session.rods[session.activeRod];

  return (
    <>
      <div className="hud-top">
        <div className="panel place">
          <div className="place-name">{session.spotName}</div>
          <div className="place-clock">
            День {session.clock.day}, {formatClock(session.clock.hour)} ·{" "}
            {WEATHER_NAME[session.weather]}
            <button
              className={`ff-btn ${session.clock.fastForward ? "on" : ""}`}
              title="Ускорить время (или держите T)"
              onClick={() => sendInput({ type: "FastForward", on: !session.clock.fastForward })}
            >
              ▶▶
            </button>
          </div>
        </div>
        <button className="panel keepnet-btn" onClick={() => useGame.getState().toggleKeepnet()}>
          Садок · {session.keepnet.length} <kbd>S</kbd>
        </button>
      </div>

      {rod?.state.state === "Fighting" && <FightBar rod={rod} state={rod.state} />}
      <RodBar session={session} />
      {rod?.state.state === "Landed" && (
        <CatchCard index={session.activeRod} catch_={rod.state.catch} />
      )}
      {keepnetOpen && <Keepnet catches={session.keepnet} />}
      {debug && <DebugOverlay session={session} />}
    </>
  );
}
