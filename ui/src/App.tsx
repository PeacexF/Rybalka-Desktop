import { useEffect, useState } from "react";
import { baits, catalog, locations, subscribe } from "./ipc/game";
import { Toasts } from "./hud/Toasts";
import { Hub } from "./screens/Hub";
import { MainMenu } from "./screens/MainMenu";
import { Water } from "./screens/Water";
import { useGame } from "./store/game";

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
    subscribe((s) => useGame.getState().setSnapshot(s))
      .then(() => Promise.all([baits(), catalog(), locations()]))
      .then(([baits, catalog, locations]) =>
        useGame.getState().setStatic({ baits, catalog, locations }),
      )
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <>
      {screen === "water" && <Water />}
      {screen === "hub" && <Hub />}
      {screen === "menu" && <MainMenu />}
      <div className="overlay">
        <Toasts />
        {error && <div className="fatal">{error}</div>}
      </div>
    </>
  );
}
