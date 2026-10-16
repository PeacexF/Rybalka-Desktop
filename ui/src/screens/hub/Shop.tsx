import { useState } from "react";
import type { ItemCategory } from "../../ipc/bindings/ItemCategory";
import type { ProfileView } from "../../ipc/bindings/ProfileView";
import { buy } from "../../ipc/game";
import { formatMoney } from "../../format";
import { toastError, useGame } from "../../store/game";

export const CATEGORY_NAME: Record<ItemCategory, string> = {
  Rod: "Удилища",
  Reel: "Катушки",
  Line: "Лески",
  Tackle: "Оснастка",
  Bait: "Наживка",
  Lure: "Приманки",
};
const CATEGORIES = Object.keys(CATEGORY_NAME) as ItemCategory[];

export function Shop({ profile }: { profile: ProfileView }) {
  const catalog = useGame((s) => s.catalog);
  const [cat, setCat] = useState<ItemCategory>("Rod");
  const items = catalog.filter((i) => i.category === cat);

  return (
    <div className="shop">
      <div className="chips">
        {CATEGORIES.map((c) => (
          <button key={c} className={c === cat ? "on" : ""} onClick={() => setCat(c)}>
            {CATEGORY_NAME[c]}
          </button>
        ))}
      </div>
      <ul className="shop-list">
        {items.map((it) => {
          const owned = profile.items[it.id] ?? 0;
          const locked = profile.rank < it.rank;
          const poor = profile.money < it.price;
          return (
            <li key={it.id} className={locked ? "locked" : ""}>
              <div>
                <div className="item-name">{it.name}</div>
                <div className="dim">{it.stats}</div>
              </div>
              <div className="item-owned dim">{owned > 0 && `есть ${owned}`}</div>
              <div className="item-price">
                {formatMoney(it.price)}
                {it.pack > 1 && <span className="dim"> / {it.pack} шт.</span>}
              </div>
              {locked ? (
                <div className="item-lock dim">с разряда «{it.rankName}»</div>
              ) : (
                <button
                  disabled={poor}
                  title={poor ? "Не хватает денег" : undefined}
                  onClick={() =>
                    buy(it.id)
                      .then(() => useGame.getState().toast(`Куплено: ${it.name}`, "good"))
                      .catch(toastError)
                  }
                >
                  Купить
                </button>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
