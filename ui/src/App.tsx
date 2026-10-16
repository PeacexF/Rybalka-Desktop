import { useEffect, useState } from "react";
import { baits, catalog, continueGame, locations, subscribe } from "./ipc/game";
import { Tips } from "./hud/Tips";
import { Toasts } from "./hud/Toasts";
import { Hub } from "./screens/Hub";
import { MainMenu } from "./screens/MainMenu";
import { Settings } from "./screens/Settings";
import { Water } from "./screens/Water";
import { useGame } from "./store/game";
import { initSettings, useSettings } from "./store/settings";

/**
 * Screen router. The screen follows the game state streamed from Rust:
 * a session means we're on the water, a profile without one means the hub,
 * neither means the main menu. A UI reload lands back on the same screen.
 */
export function App() {
  const [error, setError] = useState<string | null>(null);
  const screen = useGame((s) =>
    !s.snapshot ? "loading" : s.snapshot.session ? "water" : s.snapshot.profile ? "hub" : "menu",
  );

  useEffect(() => {
    initSettings();
    const key = (e: KeyboardEvent) => {
      if (e.code !== "F11") return;
      useSettings.getState().toggleFullscreen();
      e.preventDefault();
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);

  useEffect(() => {
    subscribe((s) => useGame.getState().setSnapshot(s))
      .then(() => Promise.all([baits(), catalog(), locations()]))
      .then(([baits, catalog, locations]) =>
        useGame.getState().setStatic({ baits, catalog, locations }),
      )
      // `VITE_DEV_CONTINUE=1 npm run tauri dev` skips the main menu.
      .then(() => {
        if (
          import.meta.env.DEV &&
          import.meta.env.VITE_DEV_CONTINUE &&
          !useGame.getState().snapshot?.profile
        )
          return continueGame();
      })
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <>
      {screen === "water" && <Water />}
      {screen === "hub" && <Hub />}
      {screen === "menu" && <MainMenu />}
      <div className="overlay">
        <Tips />
        <Settings />
        <Toasts />
        {error && <div className="fatal">{error}</div>}
      </div>
    </>
  );
}
