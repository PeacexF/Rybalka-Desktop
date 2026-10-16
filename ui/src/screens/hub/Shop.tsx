import { useState } from "react";
import type { ItemCategory } from "../../ipc/bindings/ItemCategory";
import type { ProfileView } from "../../ipc/bindings/ProfileView";
import { buy } from "../../ipc/game";
import { formatMoney } from "../../format";
import { toastError, useGame } from "../../store/game";
import { sfx } from "../../audio";

export const CATEGORY_NAME: Record<ItemCategory, string> = {
  Rod: "Удилища",
  Reel: "Катушки",
  Line: "Лески",
  Tackle: "Оснастка",
  Bait: "Наживка",
  Lure: "Приманки",
};
const CATEGORIES = Object.keys(CATEGORY_NAME) as ItemCategory[];

/** The town shop, or a base shop (`local`): consumables only, pricier. */
export function Shop({ profile, local }: { profile: ProfileView; local: boolean }) {
  const catalog = useGame((s) => s.catalog);
  const categories = CATEGORIES.filter(
    (c) => !local || catalog.some((i) => i.category === c && i.localPrice !== null),
  );
  const [picked, setCat] = useState<ItemCategory>(categories[0]!);
  const cat = categories.includes(picked) ? picked : categories[0]!;
  const items = catalog
    .filter((i) => i.category === cat && (!local || i.localPrice !== null))
    .map((i) => ({ ...i, price: local ? i.localPrice! : i.price }));

  return (
    <div className="shop">
      {local && (
        <div className="hub-note">Местная лавка: только расходники и подороже, чем в городе.</div>
      )}
      <div className="chips">
        {categories.map((c) => (
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
                      .then(() => {
                        sfx.coins();
                        useGame.getState().toast(`Куплено: ${it.name}`, "good");
                      })
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
