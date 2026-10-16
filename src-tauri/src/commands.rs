use rybalka_core::map::WaterMap;
use rybalka_core::sim::rod::Tackle;
use rybalka_core::{Input, Snapshot};
use tauri::State;
use tauri::ipc::Channel;

use crate::DataRoot;
use crate::game_thread::GameHandle;

/// Start streaming per-tick snapshots to the given channel.
#[tauri::command]
pub fn subscribe(
    game: State<'_, GameHandle>,
    on_snapshot: Channel<Snapshot>,
) -> Result<(), String> {
    game.subscribe(on_snapshot)
}

/// Go fishing at a spot with the starter tackle. If that spot's session is
/// already running (e.g. after a UI reload), it is kept as is.
#[tauri::command]
pub fn start_session(
    game: State<'_, GameHandle>,
    root: State<'_, DataRoot>,
    location: String,
    spot: String,
) -> Result<(), String> {
    let running = game.with_world({
        let (location, spot) = (location.clone(), spot.clone());
        move |w| {
            w.session()
                .is_some_and(|s| s.location_id == location && s.spot.id == spot)
        }
    })?;
    if running {
        return Ok(());
    }
    let map_rel = game.with_world({
        let (location, spot) = (location.clone(), spot.clone());
        move |w| {
            w.content()
                .location(&location)
                .and_then(|l| l.spot(&spot))
                .map(|s| s.map.clone())
        }
    })?;
    let map_rel = map_rel.ok_or_else(|| format!("unknown spot {location}/{spot}"))?;
    let path = root.0.join("assets").join(map_rel);
    let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let map = WaterMap::from_png(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    game.with_world(move |w| {
        w.start_session(
            &location,
            &spot,
            map,
            vec![
                Tackle::starter_float(),
                Tackle::starter_feeder(),
                Tackle::starter_spinning(),
            ],
        )
        .map_err(|e| e.to_string())
    })?
}

#[tauri::command]
pub fn input(game: State<'_, GameHandle>, input: Input) -> Result<(), String> {
    game.with_world(move |w| w.input(input).map_err(|e| e.to_string()))?
}

#[tauri::command]
pub fn set_debug(game: State<'_, GameHandle>, on: bool) -> Result<(), String> {
    game.with_world(move |w| w.debug = on)
}

#[tauri::command]
pub fn baits(game: State<'_, GameHandle>) -> Result<Vec<rybalka_core::sim::BaitInfo>, String> {
    game.with_world(|w| w.baits())
}
