import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { create } from "zustand";
import { setVolumes, type Volumes } from "../audio";

export interface Settings extends Volumes {
  /** Game minutes per real minute. */
  timeSpeed: number;
  fullscreen: boolean;
}

export const TIME_SPEEDS: [number, string][] = [
  [5, "Медленно"],
  [10, "Обычно"],
  [20, "Быстро"],
  [30, "Очень быстро"],
];

const DEFAULTS: Settings = { master: 0.8, sfx: 1, ambient: 0.7, timeSpeed: 10, fullscreen: false };
const KEY = "rybalka.settings";

function load(): Settings {
  try {
    const raw = localStorage.getItem(KEY);
    return raw ? { ...DEFAULTS, ...(JSON.parse(raw) as Partial<Settings>) } : DEFAULTS;
  } catch {
    return DEFAULTS;
  }
}

function store(s: Settings): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(s));
  } catch {
    // Settings just won't survive a restart.
  }
}

/** Push settings to where they take effect: audio, the game clock, the window. */
function apply(s: Settings, prev?: Settings): void {
  setVolumes(s);
  if (s.timeSpeed !== prev?.timeSpeed)
    invoke("set_time_speed", { speed: s.timeSpeed }).catch((e) => console.warn(e));
  if (s.fullscreen !== (prev?.fullscreen ?? false))
    getCurrentWindow()
      .setFullscreen(s.fullscreen)
      .catch((e) => console.warn(e));
}

interface SettingsState {
  settings: Settings;
  open: boolean;
  setOpen: (open: boolean) => void;
  update: (patch: Partial<Settings>) => void;
  toggleFullscreen: () => void;
}

export const useSettings = create<SettingsState>((set, get) => ({
  settings: load(),
  open: false,
  setOpen: (open) => set({ open }),
  update: (patch) => {
    const prev = get().settings;
    const settings = { ...prev, ...patch };
    set({ settings });
    store(settings);
    apply(settings, prev);
  },
  toggleFullscreen: () => get().update({ fullscreen: !get().settings.fullscreen }),
}));

/** Apply saved settings at startup. */
export function initSettings(): void {
  apply(useSettings.getState().settings);
}
