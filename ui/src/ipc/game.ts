import { Channel, invoke } from "@tauri-apps/api/core";
import type { BaitInfo } from "./bindings/BaitInfo";
import type { Input } from "./bindings/Input";
import type { LocationInfo } from "./bindings/LocationInfo";
import type { MenuState } from "./bindings/MenuState";
import type { Part } from "./bindings/Part";
import type { SaleReport } from "./bindings/SaleReport";
import type { ShopItem } from "./bindings/ShopItem";
import type { Snapshot } from "./bindings/Snapshot";

/** Start receiving per-tick snapshots from the Rust game thread. */
export async function subscribe(onSnapshot: (s: Snapshot) => void): Promise<void> {
  const channel = new Channel<Snapshot>();
  channel.onmessage = onSnapshot;
  await invoke("subscribe", { onSnapshot: channel });
}

export function sendInput(input: Input): void {
  invoke("input", { input }).catch((e) => console.warn("input rejected:", input, e));
}

export function setDebug(on: boolean): Promise<void> {
  return invoke("set_debug", { on });
}

export function baits(): Promise<BaitInfo[]> {
  return invoke("baits");
}

export function catalog(): Promise<ShopItem[]> {
  return invoke("catalog");
}

export function locations(): Promise<LocationInfo[]> {
  return invoke("locations");
}

export function menuState(): Promise<MenuState> {
  return invoke("menu_state");
}

export function newGame(): Promise<void> {
  return invoke("new_game");
}

export function continueGame(): Promise<void> {
  return invoke("continue_game");
}

/** Save and return to the main menu. */
export function closeGame(): Promise<void> {
  return invoke("close_game");
}

export function exitApp(): Promise<void> {
  return invoke("exit_app");
}

export function buy(id: string, packs = 1): Promise<void> {
  return invoke("buy", { id, packs });
}

export function equip(slot: number, part: Part, id: string | null): Promise<void> {
  return invoke("equip", { slot, part, id });
}

export function goFishing(location: string, spot: string): Promise<void> {
  return invoke("go_fishing", { location, spot });
}

/** Leave the water; the keepnet is sold. */
export function leaveWater(): Promise<SaleReport> {
  return invoke("leave_water");
}
