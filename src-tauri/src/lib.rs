mod auth;
mod commands;
mod db;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| { db::migrate(app.handle()).map_err(std::io::Error::other)?; Ok(()) })
        .invoke_handler(tauri::generate_handler![commands::needs_setup,commands::create_first_admin,commands::login,commands::dashboard_summary])
        .run(tauri::generate_context!())
        .expect("error while running Roster Master Desktop");
}
