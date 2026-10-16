import { sendInput } from "../ipc/game";
import { formatWeight, plural } from "../format";
import { useGame } from "../store/game";

export function PauseMenu({ onLeave }: { onLeave: () => void }) {
  const session = useGame((s) => s.snapshot?.session ?? null);
  if (!session?.paused) return null;
  const n = session.keepnet.length;
  const kg = session.keepnet.reduce((s, c) => s + c.weightKg, 0);

  return (
    <div className="modal-backdrop dimmed">
      <div className="panel menu-card">
        <div className="menu-title">Пауза</div>
        <div className="dim">
          {session.spotName} · в садке{" "}
          {n === 0 ? "пусто" : `${n} ${plural(n, "рыба", "рыбы", "рыб")}, ${formatWeight(kg)}`}
        </div>
        <div className="menu-buttons">
          <button className="primary" onClick={() => sendInput({ type: "Pause", on: false })}>
            Продолжить <kbd>Esc</kbd>
          </button>
          <button onClick={onLeave}>
            {n > 0 ? "Уйти с водоёма и продать улов" : "Уйти с водоёма"}
          </button>
        </div>
      </div>
    </div>
  );
}
