mod commands;
mod game_thread;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use game_thread::GameHandle;
use tauri::Manager;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let root = data_root(app);
            let content = rybalka_core::content::load_dir(&root.join("content"))?;
            let seed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(1, |d| d.as_nanos() as u64);
            app.manage(GameHandle::spawn(Arc::new(content), seed));
            app.manage(DataRoot(root));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::subscribe,
            commands::start_session,
            commands::input,
            commands::set_debug,
            commands::baits,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
