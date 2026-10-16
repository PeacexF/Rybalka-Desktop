import { create } from "zustand";
import type { BaitInfo } from "../ipc/bindings/BaitInfo";
import type { GameEvent } from "../ipc/bindings/GameEvent";
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
  keepnetOpen: boolean;
  debug: boolean;
  toasts: Toast[];
  setSnapshot: (s: Snapshot) => void;
  setBaits: (b: BaitInfo[]) => void;
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
  keepnetOpen: false,
  debug: false,
  toasts: [],
  setSnapshot: (snapshot) => {
    set({ snapshot, snapshotAt: performance.now() });
    for (const e of snapshot.events) for (const fn of eventListeners) fn(e);
  },
  setBaits: (baits) => set({ baits }),
  toggleKeepnet: (open) => set((s) => ({ keepnetOpen: open ?? !s.keepnetOpen })),
  toggleDebug: () => set((s) => ({ debug: !s.debug })),
  toast: (text, tone = "info") => {
    const id = nextToast++;
    set({ toasts: [...get().toasts, { id, text, tone }].slice(-4) });
    setTimeout(() => set({ toasts: get().toasts.filter((t) => t.id !== id) }), 2600);
  },
}));
