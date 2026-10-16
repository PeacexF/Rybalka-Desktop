import type { SaleReport } from "../../ipc/bindings/SaleReport";
import { formatMoney, formatWeight } from "../../format";
import { useGame } from "../../store/game";

export function SaleCard({ report }: { report: SaleReport }) {
  const close = () => useGame.getState().setSale(null);
  const rankName = useGame((s) => s.snapshot?.profile?.rankName);
  const rankUp = report.rankAfter > report.rankBefore;

  return (
    <div className="modal-backdrop dimmed" onClick={close}>
      <div className="panel sale-card" onClick={(e) => e.stopPropagation()}>
        <div className="menu-title">{report.lines.length ? "Улов продан" : "Садок пуст"}</div>
        {report.lines.length > 0 ? (
          <>
            <ul className="sale-lines">
              {report.lines.map((l) => (
                <li key={l.name}>
                  <span>
                    {l.name}
                    {l.count > 1 && <span className="dim"> ×{l.count}</span>}
                  </span>
                  <span className="dim">{formatWeight(l.weightKg)}</span>
                  <span>{formatMoney(l.money)}</span>
                </li>
              ))}
            </ul>
            <div className="sale-total">
              <span>+{formatMoney(report.money)}</span>
              <span className="dim">+{Math.round(report.xp)} опыта</span>
            </div>
          </>
        ) : (
          <div className="dim">Продавать нечего — в следующий раз повезёт.</div>
        )}
        {rankUp && <div className="rank-up">Новый разряд: {rankName}!</div>}
        <div className="menu-buttons">
          <button className="primary" onClick={close} autoFocus>
            Хорошо
          </button>
        </div>
      </div>
    </div>
  );
}
