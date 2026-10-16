import { invoke } from "@tauri-apps/api/core";

/** Forward frontend errors/warnings to the Rust process log. */
export function installLogBridge(): void {
  const send = (level: string, args: unknown[]) => {
    const message = args
      .map((a) => (a instanceof Error ? `${a.message}\n${a.stack ?? ""}` : String(a)))
      .join(" ");
    invoke("log_ui", { level, message }).catch(() => {});
  };
  for (const level of ["error", "warn"] as const) {
    const orig = console[level].bind(console);
    console[level] = (...args: unknown[]) => {
      orig(...args);
      send(level, args);
    };
  }
  window.addEventListener("error", (e) => send("error", [e.error ?? e.message]));
  window.addEventListener("unhandledrejection", (e) => send("error", [e.reason]));
}
