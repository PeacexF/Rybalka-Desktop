import type { Catch } from "../ipc/bindings/Catch";
import { formatClock, formatWeight, plural } from "../format";
import { useGame } from "../store/game";

export function Keepnet({ catches }: { catches: Catch[] }) {
  const total = catches.reduce((s, c) => s + c.weightKg, 0);
  return (
    <div className="panel keepnet">
      <div className="keepnet-head">
        <b>Садок</b>
        <span>
          {catches.length} {plural(catches.length, "рыба", "рыбы", "рыб")} · {formatWeight(total)}
        </span>
        <button onClick={() => useGame.getState().toggleKeepnet(false)}>✕</button>
      </div>
      {catches.length === 0 ? (
        <div className="keepnet-empty">Пока пусто</div>
      ) : (
        <ul>
          {[...catches].reverse().map((c) => (
            <li key={c.id}>
              <span>
                {c.name}
                {c.trophy && " ★"}
              </span>
              <span>{formatWeight(c.weightKg)}</span>
              <span className="dim">{formatClock((c.atMin / 60) % 24)}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
