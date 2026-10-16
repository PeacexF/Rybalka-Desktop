use rybalka_core::Snapshot;
use tauri::State;
use tauri::ipc::Channel;

use crate::game_thread::{Command, GameHandle};

#[tauri::command]
pub fn ping() -> &'static str {
    "pong"
}

/// Start streaming per-tick snapshots to the given channel.
#[tauri::command]
pub fn subscribe(
    game: State<'_, GameHandle>,
    on_snapshot: Channel<Snapshot>,
) -> Result<(), String> {
    game.send(Command::Subscribe(on_snapshot))
}
