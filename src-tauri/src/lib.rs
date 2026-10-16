mod commands;
mod game_thread;
mod save;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use game_thread::GameHandle;
use tauri::{Manager, RunEvent};

/// Directory containing `content/` and `assets/`.
pub struct DataRoot(pub PathBuf);

fn data_root(app: &tauri::App) -> PathBuf {
    if cfg!(debug_assertions) {
        // Dev builds read straight from the repo so content edits apply on restart.
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
    } else {
        app.path().resource_dir().expect("no resource dir")
    }
}

/// Open the save store; on failure the game still runs, just without saves.
fn open_store(app: &tauri::App) -> (Option<save::Store>, Option<String>) {
    let path = match app.path().app_data_dir() {
        Ok(dir) => dir.join("save.sqlite"),
        Err(e) => return (None, Some(e.to_string())),
    };
    match save::Store::open(&path) {
        Ok(s) => (Some(s), None),
        Err(e) => {
            eprintln!("{}: {e}", path.display());
            (None, Some(e.to_string()))
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let root = data_root(app);
            let content = rybalka_core::content::load_dir(&root.join("content"))?;
            let seed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(1, |d| d.as_nanos() as u64);
            let (store, save_error) = open_store(app);
            let store: game_thread::SharedStore = Arc::new(Mutex::new(store));
            app.manage(GameHandle::spawn(Arc::new(content), seed, store.clone()));
            app.manage(store);
            app.manage(commands::SaveStatus(save_error));
            app.manage(DataRoot(root));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::subscribe,
            commands::menu_state,
            commands::new_game,
            commands::continue_game,
            commands::close_game,
            commands::exit_app,
            commands::catalog,
            commands::locations,
            commands::buy,
            commands::equip,
            commands::go_fishing,
            commands::leave_water,
            commands::input,
            commands::set_debug,
            commands::baits,
            commands::log_ui,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                app.state::<GameHandle>().flush();
            }
        });
}
