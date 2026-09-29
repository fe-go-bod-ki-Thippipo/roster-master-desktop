#[cfg(all(feature="central",feature="unit"))]
compile_error!("central and unit features are mutually exclusive");
#[cfg(not(any(feature="central",feature="unit")))]
compile_error!("build exactly one edition feature: central or unit");
mod admin;
mod audit;
mod auth;
mod authz;
mod commands;
mod db;
mod employees;
mod edition;
mod org;
mod packages;
mod session;
mod trust;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .manage(session::SessionStore::default())
        .manage(session::LoginLimiter::default())
        .setup(|app| { db::migrate(app.handle()).map_err(std::io::Error::other)?; Ok(()) });

    #[cfg(feature="central")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::app_identity,commands::initialize_central,commands::needs_setup,
        commands::create_first_admin,commands::login,commands::logout,commands::dashboard_summary,
        commands::list_company_groups,commands::list_companies,commands::list_org_units,commands::list_positions,
        commands::add_company_group,commands::add_company,commands::add_org_unit,commands::add_position,
        commands::list_employees,commands::add_employee,commands::list_assignments,commands::add_assignment,
        commands::list_managed_users,commands::create_managed_user,commands::export_provision_package
    ]);

    #[cfg(feature="unit")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::app_identity,commands::needs_setup,commands::login,commands::logout,
        commands::dashboard_summary,commands::list_company_groups,commands::list_companies,
        commands::list_org_units,commands::list_positions,commands::list_employees,
        commands::list_assignments,commands::import_provision_package
    ]);

    builder.run(tauri::generate_context!())
        .expect("error while running Roster Master Desktop");
}
