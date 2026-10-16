import { Channel, invoke } from "@tauri-apps/api/core";
import type { BaitInfo } from "./bindings/BaitInfo";
import type { Input } from "./bindings/Input";
import type { Snapshot } from "./bindings/Snapshot";

/** Start receiving per-tick snapshots from the Rust game thread. */
export async function subscribe(onSnapshot: (s: Snapshot) => void): Promise<void> {
  const channel = new Channel<Snapshot>();
  channel.onmessage = onSnapshot;
  await invoke("subscribe", { onSnapshot: channel });
}

export function startSession(location: string, spot: string): Promise<void> {
  return invoke("start_session", { location, spot });
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
