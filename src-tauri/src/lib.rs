mod admin;
mod audit;
mod auth;
mod commands;
mod db;
mod employees;
mod edition;
mod org;
mod packages;
mod trust;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| { db::migrate(app.handle()).map_err(std::io::Error::other)?; Ok(()) })
        .invoke_handler(tauri::generate_handler![commands::app_identity,commands::initialize_central,commands::needs_setup,commands::create_first_admin,commands::login,commands::dashboard_summary,commands::list_company_groups,commands::list_companies,commands::list_org_units,commands::list_positions,commands::add_company_group,commands::add_company,commands::add_org_unit,commands::add_position,commands::list_employees,commands::add_employee,commands::list_assignments,commands::add_assignment,commands::list_managed_users,commands::create_managed_user,commands::export_provision_package,commands::import_provision_package])
        .run(tauri::generate_context!())
        .expect("error while running Roster Master Desktop");
}
