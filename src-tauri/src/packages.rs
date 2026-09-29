use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize,Serialize};
use tauri::AppHandle;
use uuid::Uuid;
use crate::{db,edition};

#[derive(Serialize,Deserialize)]
pub struct ProvisionManifest {pub package_id:String,pub schema_version:u32,pub source_site_code:String,pub target_site_code:String,pub target_site_name:String,pub issued_at:String,pub users:Vec<ProvisionUser>}
#[derive(Serialize,Deserialize)]
pub struct ProvisionUser {pub id:String,pub username:String,pub password_hash:String,pub display_name:String,pub role_code:String,pub permissions:Vec<String>,pub scope_type:String,pub company_id:Option<String>,pub department_id:Option<String>}

pub fn export_provision(app:&AppHandle,user_id:&str,target_site_code:&str,target_site_name:&str,user_ids:Vec<String>)->Result<String,String>{
 if !edition::is_central(app)?{return Err("สร้าง Package ได้เฉพาะ Central Edition".into())}
 if target_site_code.trim().is_empty()||user_ids.is_empty(){return Err("ต้องระบุ Site ปลายทางและผู้ใช้อย่างน้อย 1 ราย".into())}
 let c=db::open(app)?; let ok:i64=c.query_row("SELECT EXISTS(SELECT 1 FROM user_roles ur JOIN role_permissions rp ON rp.role_id=ur.role_id JOIN permissions p ON p.id=rp.permission_id WHERE ur.user_id=?1 AND p.code='user.manage')",[user_id],|r|r.get(0)).map_err(|e|e.to_string())?; if ok!=1{return Err("ไม่มีสิทธิ์สร้าง Package".into())}
 let source=edition::identity(app)?.and_then(|x|x.site_code).ok_or("Central Site ยังไม่สมบูรณ์")?; let mut users=Vec::new();
 for uid in user_ids{
  let (id,username,password_hash,display_name,role_code,scope_type,company_id,department_id)=c.query_row("SELECT u.id,u.username,u.password_hash,u.display_name,r.code,COALESCE(ds.scope_type,'GLOBAL'),ds.company_id,ds.department_id FROM users u JOIN user_roles ur ON ur.user_id=u.id JOIN roles r ON r.id=ur.role_id LEFT JOIN user_data_scopes ds ON ds.user_id=u.id WHERE u.id=?1 AND u.is_active=1",[&uid],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?))).map_err(|e|e.to_string())?;
  if role_code=="SYSTEM_ADMIN"{return Err("ห้ามส่ง SYSTEM_ADMIN ไป Unit".into())}
  let mut ps=c.prepare("SELECT p.code FROM permissions p JOIN role_permissions rp ON rp.permission_id=p.id JOIN roles r ON r.id=rp.role_id WHERE r.code=?1 ORDER BY p.code").map_err(|e|e.to_string())?;
  let permissions=ps.query_map([&role_code],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?.filter_map(Result::ok).collect();
  users.push(ProvisionUser{id,username,password_hash,display_name,role_code,permissions,scope_type,company_id,department_id});
 }
 serde_json::to_string_pretty(&ProvisionManifest{package_id:Uuid::new_v4().to_string(),schema_version:1,source_site_code:source,target_site_code:target_site_code.trim().into(),target_site_name:target_site_name.trim().into(),issued_at:Utc::now().to_rfc3339(),users}).map_err(|e|e.to_string())
}

pub fn import_provision(app:&AppHandle,json:&str)->Result<(),String>{
 if edition::identity(app)?.is_some(){return Err("เครื่องนี้ถูก Provision แล้ว".into())}
 let p:ProvisionManifest=serde_json::from_str(json).map_err(|_|"Package ไม่ถูกต้อง".to_string())?;
 if p.schema_version!=1||p.users.is_empty(){return Err("Package version หรือข้อมูลผู้ใช้ไม่ถูกต้อง".into())}
 let c=db::open(app)?; let tx=c.unchecked_transaction().map_err(|e|e.to_string())?; let now=Utc::now().to_rfc3339();
 tx.execute("INSERT INTO app_identity(singleton_id,edition,site_code,site_name,initialized_at) VALUES(1,'UNIT',?1,?2,?3)",params![p.target_site_code,p.target_site_name,now]).map_err(|e|e.to_string())?;
 for u in p.users{
  if u.role_code=="SYSTEM_ADMIN"{return Err("Unit Package ห้ามมี SYSTEM_ADMIN".into())}
  let role_id=Uuid::new_v4().to_string(); tx.execute("INSERT OR IGNORE INTO roles(id,code,name) VALUES(?1,?2,?2)",params![role_id,u.role_code]).map_err(|e|e.to_string())?;
  let actual_role:String=tx.query_row("SELECT id FROM roles WHERE code=?1",[&u.role_code],|r|r.get(0)).map_err(|e|e.to_string())?;
  for code in u.permissions{let pid=Uuid::new_v4().to_string();tx.execute("INSERT OR IGNORE INTO permissions(id,code,name) VALUES(?1,?2,?2)",params![pid,code]).map_err(|e|e.to_string())?;tx.execute("INSERT OR IGNORE INTO role_permissions(role_id,permission_id) SELECT ?1,id FROM permissions WHERE code=?2",params![actual_role,code]).map_err(|e|e.to_string())?;}
  tx.execute("INSERT INTO users(id,username,password_hash,display_name,is_active,created_at,updated_at) VALUES(?1,?2,?3,?4,1,?5,?5)",params![u.id,u.username,u.password_hash,u.display_name,now]).map_err(|e|e.to_string())?;
  tx.execute("INSERT INTO user_roles(user_id,role_id) VALUES(?1,?2)",params![u.id,actual_role]).map_err(|e|e.to_string())?;
  tx.execute("INSERT INTO user_data_scopes(id,user_id,scope_type,company_id,department_id) VALUES(?1,?2,?3,?4,?5)",params![Uuid::new_v4().to_string(),u.id,u.scope_type,u.company_id,u.department_id]).map_err(|e|e.to_string())?;
 }
 tx.execute("INSERT INTO package_imports(id,package_id,source_site_code,target_site_code,package_type,manifest_json,imported_at) VALUES(?1,?2,?3,?4,'PROVISION',?5,?6)",params![Uuid::new_v4().to_string(),p.package_id,p.source_site_code,p.target_site_code,json,now]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())
}
