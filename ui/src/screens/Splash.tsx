/** Shown until the first snapshot arrives from the game thread. */
export function Splash() {
  return (
    <div className="splash">
      <div className="splash-float">
        <svg viewBox="0 0 40 120" width="20" height="60" aria-hidden>
          <rect x="18" y="0" width="4" height="34" rx="2" fill="#e0573c" />
          <ellipse cx="20" cy="52" rx="11" ry="20" fill="#ece9dc" />
          <rect x="18" y="68" width="4" height="40" rx="2" fill="#a9ab9a" />
        </svg>
      </div>
      <div className="splash-ripple" />
      <div className="splash-title">Рыбалка</div>
    </div>
  );
}
