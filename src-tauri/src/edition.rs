use chrono::Utc;
use rusqlite::params;
use serde::Serialize;
use tauri::AppHandle;
use crate::db;

#[derive(Serialize)]
pub struct AppIdentity { pub edition:String, pub site_code:Option<String>, pub site_name:Option<String> }

pub fn build_edition() -> &'static str {
    #[cfg(feature="central")] { "CENTRAL" }
    #[cfg(feature="unit")] { "UNIT" }
}

pub fn identity(app:&AppHandle)->Result<Option<AppIdentity>,String>{
 let c=db::open(app)?;
 let mut s=c.prepare("SELECT edition,site_code,site_name FROM app_identity WHERE singleton_id=1").map_err(|e|e.to_string())?;
 let mut rows=s.query([]).map_err(|e|e.to_string())?;
 match rows.next().map_err(|e|e.to_string())? {
  Some(r)=>Ok(Some(AppIdentity{edition:r.get(0).map_err(|e|e.to_string())?,site_code:r.get(1).map_err(|e|e.to_string())?,site_name:r.get(2).map_err(|e|e.to_string())?})),
  None=>Ok(None)
 }
}

fn validate_identity_edition(identity_edition:&str,build:&str)->Result<(),String>{
 if identity_edition!=build{return Err(format!("ฐานข้อมูล Edition {} ไม่ตรงกับโปรแกรม {} build",identity_edition,build))} Ok(())
}
pub fn ensure_build_matches_identity(app:&AppHandle)->Result<(),String>{
 if let Some(i)=identity(app)? {validate_identity_edition(&i.edition,build_edition())?;} Ok(())
}

pub fn initialize_central(app:&AppHandle,site_code:&str,site_name:&str)->Result<(),String>{
 if build_edition()!="CENTRAL"{return Err("Unit build ไม่สามารถกำหนด Central identity".into())}
 if identity(app)?.is_some(){return Err("เครื่องนี้ถูกกำหนด Edition แล้ว".into())}
 if site_code.trim().is_empty()||site_name.trim().is_empty(){return Err("กรุณาระบุรหัสและชื่อ Site".into())}
 db::open(app)?.execute("INSERT INTO app_identity(singleton_id,edition,site_code,site_name,initialized_at) VALUES(1,'CENTRAL',?1,?2,?3)",params![site_code.trim(),site_name.trim(),Utc::now().to_rfc3339()]).map_err(|e|e.to_string())?;
 Ok(())
}

pub fn is_central(app:&AppHandle)->Result<bool,String>{Ok(identity(app)?.map(|x|x.edition=="CENTRAL").unwrap_or(false))}


#[cfg(test)]
mod tests{
 use super::*;
 #[test] fn edition_mismatch_is_rejected(){
   assert!(validate_identity_edition("CENTRAL","UNIT").is_err());
   assert!(validate_identity_edition("UNIT","CENTRAL").is_err());
   assert!(validate_identity_edition("UNIT","UNIT").is_ok());
   assert!(validate_identity_edition("CENTRAL","CENTRAL").is_ok());
 }
}
