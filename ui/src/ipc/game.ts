import { Channel, invoke } from "@tauri-apps/api/core";
import type { Snapshot } from "./bindings/Snapshot";

export function ping(): Promise<string> {
  return invoke("ping");
}

/** Start receiving per-tick snapshots from the Rust game thread. */
export async function subscribe(onSnapshot: (s: Snapshot) => void): Promise<void> {
  const channel = new Channel<Snapshot>();
  channel.onmessage = onSnapshot;
  await invoke("subscribe", { onSnapshot: channel });
}
