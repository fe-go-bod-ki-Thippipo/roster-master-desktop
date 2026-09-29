use rusqlite::Connection;
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

const SECURITY: &str = include_str!("../../database/migrations/0001_security.sql");
const HR_DOMAIN: &str = include_str!("../../database/migrations/0002_hr_domain.sql");

pub fn database_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("roster-master.db"))
}

pub fn open(app: &AppHandle) -> Result<Connection, String> {
    let path = database_path(app)?;
    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;").map_err(|e| e.to_string())?;
    Ok(conn)
}

pub fn migrate(app: &AppHandle) -> Result<(), String> {
    let conn = open(app)?;
    conn.execute_batch(SECURITY).map_err(|e| e.to_string())?;
    conn.execute_batch(HR_DOMAIN).map_err(|e| e.to_string())?;
    Ok(())
}
