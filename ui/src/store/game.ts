import { create } from "zustand";
import type { BaitInfo } from "../ipc/bindings/BaitInfo";
import type { GameEvent } from "../ipc/bindings/GameEvent";
import type { LocationInfo } from "../ipc/bindings/LocationInfo";
import type { SaleReport } from "../ipc/bindings/SaleReport";
import type { ShopItem } from "../ipc/bindings/ShopItem";
import type { Snapshot } from "../ipc/bindings/Snapshot";

export interface Toast {
  id: number;
  text: string;
  tone: "info" | "bad" | "good";
}

interface GameState {
  snapshot: Snapshot | null;
  /** `performance.now()` when the snapshot arrived (for extrapolating phase timers). */
  snapshotAt: number;
  baits: BaitInfo[];
  catalog: ShopItem[];
  locations: LocationInfo[];
  /** Result of the last trip, shown in the hub until dismissed. */
  sale: SaleReport | null;
  keepnetOpen: boolean;
  debug: boolean;
  toasts: Toast[];
  setSnapshot: (s: Snapshot) => void;
  setStatic: (d: { baits: BaitInfo[]; catalog: ShopItem[]; locations: LocationInfo[] }) => void;
  setSale: (r: SaleReport | null) => void;
  toggleKeepnet: (open?: boolean) => void;
  toggleDebug: () => void;
  toast: (text: string, tone?: Toast["tone"]) => void;
}

let nextToast = 1;
const eventListeners = new Set<(e: GameEvent) => void>();

/** Subscribe to game events (sounds, scene effects). Returns an unsubscribe function. */
export function onGameEvent(fn: (e: GameEvent) => void): () => void {
  eventListeners.add(fn);
  return () => eventListeners.delete(fn);
}

export const useGame = create<GameState>((set, get) => ({
  snapshot: null,
  snapshotAt: 0,
  baits: [],
  catalog: [],
  locations: [],
  sale: null,
  keepnetOpen: false,
  debug: false,
  toasts: [],
  setSnapshot: (snapshot) => {
    set({ snapshot, snapshotAt: performance.now() });
    for (const e of snapshot.events) for (const fn of eventListeners) fn(e);
  },
  setStatic: (d) => set(d),
  setSale: (sale) => set({ sale }),
  toggleKeepnet: (open) => set((s) => ({ keepnetOpen: open ?? !s.keepnetOpen })),
  toggleDebug: () => set((s) => ({ debug: !s.debug })),
  toast: (text, tone = "info") => {
    const id = nextToast++;
    set({ toasts: [...get().toasts, { id, text, tone }].slice(-4) });
    setTimeout(() => set({ toasts: get().toasts.filter((t) => t.id !== id) }), 2600);
  },
}));

/** Show a failed command as a toast. */
export function toastError(e: unknown): void {
  useGame.getState().toast(String(e), "bad");
}
