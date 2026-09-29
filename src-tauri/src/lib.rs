mod auth;
mod commands;
mod db;
mod employees;
mod org;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| { db::migrate(app.handle()).map_err(std::io::Error::other)?; Ok(()) })
        .invoke_handler(tauri::generate_handler![commands::needs_setup,commands::create_first_admin,commands::login,commands::dashboard_summary,commands::list_company_groups,commands::list_companies,commands::list_org_units,commands::list_positions,commands::add_company_group,commands::add_company,commands::add_org_unit,commands::add_position,commands::list_employees,commands::add_employee,commands::list_assignments,commands::add_assignment])
        .run(tauri::generate_context!())
        .expect("error while running Roster Master Desktop");
}
