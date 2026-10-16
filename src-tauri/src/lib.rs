mod commands;
mod game_thread;

use game_thread::GameHandle;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(GameHandle::spawn())
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::subscribe,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
