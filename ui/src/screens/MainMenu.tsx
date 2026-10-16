import { useEffect, useState } from "react";
import type { MenuState } from "../ipc/bindings/MenuState";
import { continueGame, exitApp, menuState, newGame } from "../ipc/game";
import { toastError } from "../store/game";

export function MainMenu() {
  const [state, setState] = useState<MenuState | null>(null);
  const [confirmNew, setConfirmNew] = useState(false);

  useEffect(() => {
    menuState().then(setState).catch(toastError);
  }, []);

  const start = () => {
    if (state?.hasSave && !confirmNew) return setConfirmNew(true);
    newGame().catch(toastError);
  };

  return (
    <div className="screen menu-screen">
      <div className="menu-hero">
        <h1>Рыбалка</h1>
        <div className="dim">тихий пруд, поплавок и утренний клёв</div>
      </div>
      <div className="panel menu-card">
        {confirmNew ? (
          <>
            <div>Начать заново? Текущее сохранение будет перезаписано.</div>
            <div className="menu-buttons">
              <button className="danger" onClick={start}>
                Да, начать заново
              </button>
              <button onClick={() => setConfirmNew(false)}>Отмена</button>
            </div>
          </>
        ) : (
          <div className="menu-buttons">
            {state?.hasSave && (
              <button className="primary" onClick={() => continueGame().catch(toastError)}>
                Продолжить
              </button>
            )}
            <button className={state?.hasSave ? "" : "primary"} onClick={start}>
              Новая игра
            </button>
            <button onClick={() => exitApp().catch(toastError)}>Выход</button>
          </div>
        )}
        {state?.saveError && (
          <div className="menu-warn">Сохранения не работают: {state.saveError}</div>
        )}
      </div>
    </div>
  );
}
