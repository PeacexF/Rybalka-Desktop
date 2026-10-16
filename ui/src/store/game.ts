import { create } from "zustand";
import type { Snapshot } from "../ipc/bindings/Snapshot";

interface GameState {
  snapshot: Snapshot | null;
  setSnapshot: (s: Snapshot) => void;
}

export const useGame = create<GameState>((set) => ({
  snapshot: null,
  setSnapshot: (snapshot) => set({ snapshot }),
}));
