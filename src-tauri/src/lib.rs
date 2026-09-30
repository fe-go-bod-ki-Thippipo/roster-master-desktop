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
        .setup(|app| {
            db::migrate(app.handle()).map_err(std::io::Error::other)?;
            edition::ensure_build_matches_identity(app.handle()).map_err(std::io::Error::other)?;
            Ok(())
        });

    #[cfg(feature="central")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::app_identity,commands::build_edition,commands::inspect_provision_package,
        commands::initialize_central,commands::needs_setup,commands::create_first_admin,
        commands::login,commands::logout,commands::dashboard_summary,
        commands::list_company_groups,commands::list_companies,commands::list_org_units,commands::list_positions,
        commands::add_company_group,commands::add_company,commands::add_org_unit,commands::add_position,
        commands::list_employees,commands::add_employee,commands::list_assignments,commands::add_assignment,
        commands::list_managed_users,commands::create_managed_user,
        commands::install_signing_key,commands::signing_key_status,commands::export_provision_package
    ]);

    #[cfg(feature="unit")]
    let builder = builder.invoke_handler(tauri::generate_handler![
        commands::app_identity,commands::build_edition,commands::inspect_provision_package,
        commands::needs_setup,commands::login,commands::logout,
        commands::dashboard_summary,commands::list_company_groups,commands::list_companies,
        commands::list_org_units,commands::list_positions,commands::list_employees,
        commands::list_assignments,commands::import_provision_package
    ]);

    builder.run(tauri::generate_context!())
        .expect("error while running Roster Master Desktop");
}


#[cfg(test)]
mod ipc_isolation_tests {
    const SOURCE:&str=include_str!("lib.rs");
    fn central_handler_block()->&'static str{
        let a=SOURCE.find("#[cfg(feature=\"central\")]\n    let builder").unwrap();
        let b=SOURCE[a..].find("#[cfg(feature=\"unit\")]").map(|x|a+x).unwrap();
        &SOURCE[a..b]
    }
    fn unit_handler_block()->&'static str{
        let a=SOURCE.find("#[cfg(feature=\"unit\")]\n    let builder").unwrap();
        let b=SOURCE[a..].find("builder.run").map(|x|a+x).unwrap();
        &SOURCE[a..b]
    }
    #[test] fn ipc_command_sets_are_edition_isolated(){
        let central=central_handler_block();let unit=unit_handler_block();
        for cmd in ["initialize_central","create_first_admin","create_managed_user","list_managed_users","install_signing_key","signing_key_status","export_provision_package"]{
            assert!(central.contains(cmd),"{cmd} missing from Central IPC");assert!(!unit.contains(cmd),"{cmd} leaked into Unit IPC");
        }
        assert!(unit.contains("import_provision_package"));assert!(!central.contains("import_provision_package"));
        for cmd in ["inspect_provision_package","build_edition"]{assert!(central.contains(cmd));assert!(unit.contains(cmd));}
    }
}
