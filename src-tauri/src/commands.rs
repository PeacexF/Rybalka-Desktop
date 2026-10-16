use rybalka_core::content::View;
use rybalka_core::map::WaterMap;
use rybalka_core::profile::{Part, Profile, SaleReport};
use rybalka_core::sim::{BaitInfo, LocationInfo, MenuState, ShopItem};
use rybalka_core::{Input, Snapshot, World};
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

use crate::DataRoot;
use crate::game_thread::{GameHandle, SharedStore};

/// Start streaming per-tick snapshots to the given channel.
#[tauri::command]
pub fn subscribe(
    game: State<'_, GameHandle>,
    on_snapshot: Channel<Snapshot>,
) -> Result<(), String> {
    game.subscribe(on_snapshot)
}

#[tauri::command]
pub fn menu_state(store: State<'_, SharedStore>, error: State<'_, SaveStatus>) -> MenuState {
    let guard = store.lock().unwrap_or_else(|e| e.into_inner());
    MenuState {
        has_save: guard
            .as_ref()
            .is_some_and(|s| s.has_save().unwrap_or(false)),
        save_error: error.0.clone(),
    }
}

/// Why the save store could not be opened, if it couldn't.
pub struct SaveStatus(pub Option<String>);

/// Start over with the starter kit (overwrites the save).
#[tauri::command]
pub fn new_game(game: State<'_, GameHandle>) -> Result<(), String> {
    game.with_world(|w| {
        let p = Profile::starter(w.content());
        w.set_profile(Some(p));
    })?;
    game.flush();
    Ok(())
}

/// Load the save; if the player was on the water, go straight back there.
#[tauri::command]
pub fn continue_game(
    game: State<'_, GameHandle>,
    store: State<'_, SharedStore>,
    root: State<'_, DataRoot>,
) -> Result<(), String> {
    let loaded = {
        let guard = store.lock().unwrap_or_else(|e| e.into_inner());
        let s = guard.as_ref().ok_or("сохранения недоступны")?;
        s.load().map_err(|e| e.to_string())?
    };
    let mut p = loaded.ok_or("сохранения нет")?;
    let at = game.with_world(move |w| {
        p.sanitize(w.content());
        let at = p.at.clone();
        w.set_profile(Some(p));
        at
    })?;
    if let Some(at) = at {
        // A missing map or tackle shouldn't block loading: stay in the hub.
        if let Err(e) = go_fishing(game, root, at.location, at.spot) {
            eprintln!("could not resume fishing: {e}");
        }
    }
    Ok(())
}

/// Save and go back to the main menu.
#[tauri::command]
pub fn close_game(game: State<'_, GameHandle>) -> Result<(), String> {
    game.with_world(World::sync_profile)?;
    game.flush();
    game.with_world(|w| w.set_profile(None))
}

#[tauri::command]
pub fn exit_app(app: AppHandle) {
    // The exit handler in `lib.rs` flushes the save.
    app.exit(0);
}

#[tauri::command]
pub fn catalog(game: State<'_, GameHandle>) -> Result<Vec<ShopItem>, String> {
    game.with_world(|w| w.catalog())
}

#[tauri::command]
pub fn locations(game: State<'_, GameHandle>) -> Result<Vec<LocationInfo>, String> {
    game.with_world(|w| w.locations())
}

#[tauri::command]
pub fn buy(game: State<'_, GameHandle>, id: String, packs: u32) -> Result<(), String> {
    let res = game.with_world(move |w| {
        w.update_profile(|p, c| p.buy(c, &id, packs).map_err(|e| e.to_string()))
            .map_err(|e| e.to_string())?
    })?;
    game.flush();
    res
}

#[tauri::command]
pub fn equip(
    game: State<'_, GameHandle>,
    slot: usize,
    part: Part,
    id: Option<String>,
) -> Result<(), String> {
    game.with_world(move |w| {
        w.update_profile(|p, c| {
            p.equip(c, slot, part, id.as_deref())
                .map_err(|e| e.to_string())
        })
        .map_err(|e| e.to_string())?
    })?
}

/// Go fishing at a spot with the rods assembled in the profile.
#[tauri::command]
pub fn go_fishing(
    game: State<'_, GameHandle>,
    root: State<'_, DataRoot>,
    location: String,
    spot: String,
) -> Result<(), String> {
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
        w.go_fishing(&location, &spot, map)
            .map_err(|e| e.to_string())?;
        dev_start_override(w);
        Ok(())
    })?
}

/// Leave the water for the base (the catch stays in the keepnet).
#[tauri::command]
pub fn leave_water(game: State<'_, GameHandle>) -> Result<(), String> {
    let res = game.with_world(|w| w.leave_water().map_err(|e| e.to_string()))?;
    game.flush();
    res
}

/// Buy a pass and travel to a water.
#[tauri::command]
pub fn buy_trip(game: State<'_, GameHandle>, location: String, days: u32) -> Result<(), String> {
    let res = game.with_world(move |w| {
        w.update_profile(|p, c| p.buy_trip(c, &location, days).map_err(|e| e.to_string()))
            .map_err(|e| e.to_string())?
    })?;
    game.flush();
    res
}

/// Sleep at the base until the next morning.
#[tauri::command]
pub fn sleep(game: State<'_, GameHandle>) -> Result<(), String> {
    game.with_world(|w| {
        w.update_profile(|p, _| p.sleep().map_err(|e| e.to_string()))
            .map_err(|e| e.to_string())?
    })?
}

/// Sell the keepnet at the base.
#[tauri::command]
pub fn sell_catch(game: State<'_, GameHandle>) -> Result<SaleReport, String> {
    let res = game.with_world(|w| w.sell_catch().map_err(|e| e.to_string()))?;
    game.flush();
    res
}

/// Go back to town; the keepnet is sold.
#[tauri::command]
pub fn end_trip(game: State<'_, GameHandle>) -> Result<SaleReport, String> {
    let res = game.with_world(|w| w.end_trip().map_err(|e| e.to_string()))?;
    game.flush();
    res
}

/// `RYBALKA_DEV_START="21.5,Rain"` starts sessions at that hour/weather
/// (for screenshots and balance checks).
fn dev_start_override(w: &mut World) {
    let Ok(spec) = std::env::var("RYBALKA_DEV_START") else {
        return;
    };
    let mut parts = spec.split(',');
    let hour = parts.next().and_then(|h| h.trim().parse::<f64>().ok());
    let weather = parts
        .next()
        .and_then(|w| serde_json::from_value(serde_json::Value::String(w.trim().into())).ok());
    let debug = w.debug;
    w.debug = true;
    let _ = w.input(Input::Debug { hour, weather });
    w.debug = debug;
}

/// Map editor (dev builds): write the edited `map.png` of the current spot
/// into the repo's `assets/` and use it right away.
#[tauri::command]
pub fn dev_save_map(
    game: State<'_, GameHandle>,
    root: State<'_, DataRoot>,
    png: Vec<u8>,
) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("редактор карт работает только в dev-сборке".into());
    }
    let map = WaterMap::from_png(&png).map_err(|e| e.to_string())?;
    let rel = game.with_world(|w| w.session().map(|s| s.spot.map.clone()))?;
    let rel = rel.ok_or("не на водоёме")?;
    let path = root.0.join("assets").join(&rel);
    std::fs::write(&path, &png).map_err(|e| format!("{}: {e}", path.display()))?;
    let path = path.canonicalize().unwrap_or(path);
    game.with_world(move |w| {
        w.dev_replace_map(Some(map), None)
            .map_err(|e| e.to_string())
    })??;
    Ok(path.display().to_string())
}

/// Map editor: try a perspective calibration on the running session.
#[tauri::command]
pub fn dev_set_view(game: State<'_, GameHandle>, view: View) -> Result<(), String> {
    game.with_world(move |w| {
        w.dev_replace_map(None, Some(view))
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
pub fn baits(game: State<'_, GameHandle>) -> Result<Vec<BaitInfo>, String> {
    game.with_world(|w| w.baits())
}

/// Frontend errors and warnings, forwarded to stderr (release builds have no devtools).
#[tauri::command]
pub fn log_ui(level: String, message: String) {
    eprintln!("[ui:{level}] {message}");
}
