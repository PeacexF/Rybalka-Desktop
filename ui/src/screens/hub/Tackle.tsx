import type { Part } from "../../ipc/bindings/Part";
import type { ProfileView } from "../../ipc/bindings/ProfileView";
import type { ShopItem } from "../../ipc/bindings/ShopItem";
import type { Slot } from "../../ipc/bindings/Slot";
import { equip } from "../../ipc/game";
import { toastError, useGame } from "../../store/game";

const PART_FIELD = { Rod: "rod", Reel: "reel", Line: "line", Bait: "bait" } as const;

/** Rod slots: pick gear from the backpack for each. */
export function Tackle({ profile }: { profile: ProfileView }) {
  const catalog = useGame((s) => s.catalog);
  const owned = (cat: ShopItem["category"][]) =>
    catalog.filter((i) => cat.includes(i.category) && (profile.items[i.id] ?? 0) > 0);
  const stock = owned(["Tackle", "Bait", "Lure"]);

  return (
    <div className="tackle">
      <div className="slots">
        {profile.slots.map((slot, i) => (
          <SlotCard key={i} index={i} slot={slot} profile={profile} owned={owned} />
        ))}
      </div>
      <aside className="backpack">
        <h3>Рюкзак</h3>
        {stock.length === 0 ? (
          <div className="dim">Пусто</div>
        ) : (
          <ul>
            {stock.map((it) => (
              <li key={it.id}>
                <span>{it.name}</span>
                <span className="dim">{profile.items[it.id]}</span>
              </li>
            ))}
          </ul>
        )}
      </aside>
    </div>
  );
}

function SlotCard({
  index,
  slot,
  profile,
  owned,
}: {
  index: number;
  slot: Slot;
  profile: ProfileView;
  owned: (cat: ShopItem["category"][]) => ShopItem[];
}) {
  const catalog = useGame((s) => s.catalog);
  const rod = catalog.find((i) => i.id === slot.rod);
  const spinning = rod?.rodKind === "Spinning";
  const missing = [!slot.rod && "удилища", !slot.reel && "катушки", !slot.line && "лески"].filter(
    Boolean,
  );

  // Pieces of durable gear already on other rods.
  const usedElsewhere = (part: Part, id: string) =>
    profile.slots.filter((s, i) => i !== index && s[PART_FIELD[part]] === id).length;

  const select = (part: Part, label: string, items: ShopItem[], durable: boolean) => {
    const value = slot[PART_FIELD[part]];
    return (
      <label>
        <span className="dim">{label}</span>
        <select
          value={value ?? ""}
          onChange={(e) => equip(index, part, e.target.value || null).catch(toastError)}
        >
          <option value="">— нет —</option>
          {items.map((it) => {
            const n = profile.items[it.id] ?? 0;
            const busy = durable && it.id !== value && usedElsewhere(part, it.id) >= n;
            return (
              <option key={it.id} value={it.id} disabled={busy}>
                {it.name}
                {durable ? (busy ? " (на другой удочке)" : "") : ` · ${n}`}
              </option>
            );
          })}
          {value && !items.some((it) => it.id === value) && (
            <option value={value} disabled>
              {catalog.find((i) => i.id === value)?.name ?? value} · 0
            </option>
          )}
        </select>
      </label>
    );
  };

  return (
    <div className={`panel slot-card ${missing.length ? "" : "ready"}`}>
      <div className="slot-head">
        <span className="rod-slot">{index + 1}</span>
        <b>{rod?.name ?? "Пустой слот"}</b>
      </div>
      {select("Rod", "Удилище", owned(["Rod"]), true)}
      {select("Reel", "Катушка", owned(["Reel"]), true)}
      {select("Line", "Леска", owned(["Line"]), true)}
      {rod &&
        select(
          "Bait",
          spinning ? "Приманка" : "Наживка",
          owned([spinning ? "Lure" : "Bait"]),
          false,
        )}
      <div className={missing.length ? "dim" : "slot-ready"}>
        {missing.length ? `Не хватает: ${missing.join(", ")}` : (rod?.stats ?? "")}
      </div>
    </div>
  );
}
