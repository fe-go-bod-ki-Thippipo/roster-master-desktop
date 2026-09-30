use argon2::{Argon2,PasswordHasher};
use argon2::password_hash::{rand_core::OsRng,SaltString};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize,Serialize};
use tauri::AppHandle;
use uuid::Uuid;
use crate::{audit,db,edition};

#[derive(Serialize)] pub struct ManagedUser {pub id:String,pub username:String,pub display_name:String,pub is_active:bool,pub role_code:String,pub scope_type:String,pub company_id:Option<String>,pub department_id:Option<String>}
#[derive(Deserialize)] pub struct CreateUserInput {pub username:String,pub password:String,pub display_name:String,pub role_code:String,pub scope_type:String,pub company_id:Option<String>,pub department_id:Option<String>}

fn require_admin(app:&AppHandle,user:&str)->Result<(),String>{
 if !edition::is_central(app)?{return Err("จัดการผู้ใช้ได้เฉพาะ Central Edition".into())}
 let c=db::open(app)?; let ok:i64=c.query_row("SELECT EXISTS(SELECT 1 FROM user_roles ur JOIN role_permissions rp ON rp.role_id=ur.role_id JOIN permissions p ON p.id=rp.permission_id WHERE ur.user_id=?1 AND p.code='user.manage')",[user],|r|r.get(0)).map_err(|e|e.to_string())?;
 if ok==1{Ok(())}else{Err("ไม่มีสิทธิ์จัดการผู้ใช้".into())}
}
pub fn list(app:&AppHandle,user:&str)->Result<Vec<ManagedUser>,String>{
 require_admin(app,user)?; let c=db::open(app)?;
 let mut s=c.prepare("SELECT u.id,u.username,u.display_name,u.is_active,r.code,COALESCE(ds.scope_type,'GLOBAL'),ds.company_id,ds.department_id FROM users u JOIN user_roles ur ON ur.user_id=u.id JOIN roles r ON r.id=ur.role_id LEFT JOIN user_data_scopes ds ON ds.user_id=u.id ORDER BY u.username").map_err(|e|e.to_string())?;
 let rows=s.query_map([],|r|Ok(ManagedUser{id:r.get(0)?,username:r.get(1)?,display_name:r.get(2)?,is_active:r.get::<_,i64>(3)?==1,role_code:r.get(4)?,scope_type:r.get(5)?,company_id:r.get(6)?,department_id:r.get(7)?})).map_err(|e|e.to_string())?; let v=rows.filter_map(Result::ok).collect(); Ok(v)
}
pub fn create(app:&AppHandle,user:&str,x:CreateUserInput)->Result<(),String>{
 require_admin(app,user)?;
 if x.username.trim().len()<3||x.password.len()<10||x.display_name.trim().is_empty(){return Err("ชื่อผู้ใช้ต้องอย่างน้อย 3 ตัวอักษร และรหัสผ่านอย่างน้อย 10 ตัวอักษร".into())}
 if !["CENTRAL_HR","COMPANY_HR","MANAGER","STAFF","VIEWER"].contains(&x.role_code.as_str()){return Err("Role ไม่ถูกต้อง".into())}
 if !["GLOBAL","COMPANY","DEPARTMENT"].contains(&x.scope_type.as_str()){return Err("Data Scope ไม่ถูกต้อง".into())}
 if x.scope_type=="COMPANY"&&x.company_id.is_none(){return Err("Company Scope ต้องระบุบริษัท".into())}
 if x.scope_type=="DEPARTMENT"&&x.department_id.is_none(){return Err("Department Scope ต้องระบุหน่วยงาน".into())}
 let c=db::open(app)?; let tx=c.unchecked_transaction().map_err(|e|e.to_string())?; let now=Utc::now().to_rfc3339();
 let role_id=match tx.query_row("SELECT id FROM roles WHERE code=?1",[&x.role_code],|r|r.get::<_,String>(0)){Ok(v)=>v,Err(_)=>{
   let id=Uuid::new_v4().to_string(); tx.execute("INSERT INTO roles(id,code,name) VALUES(?1,?2,?2)",params![id,x.role_code]).map_err(|e|e.to_string())?;
   let permission_codes:Vec<&str>=match x.role_code.as_str(){
    "CENTRAL_HR"=>vec!["dashboard.view","employee.view","employee.create","employee.update","employee.terminate","position.view","position.manage","department.view","department.manage","assignment.view","assignment.manage","import.employee","export.employee","export.dashboard","audit.view"],
    "COMPANY_HR"=>vec!["dashboard.view","employee.view","employee.create","employee.update","position.view","department.view","assignment.view","assignment.manage","import.employee","export.employee"],
    "MANAGER"=>vec!["dashboard.view","employee.view","position.view","department.view","assignment.view"],
    "STAFF"=>vec!["employee.view","position.view","department.view"],
    _=>vec!["dashboard.view","employee.view","position.view","department.view","assignment.view"]};
   for code in permission_codes{tx.execute("INSERT OR IGNORE INTO role_permissions(role_id,permission_id) SELECT ?1,id FROM permissions WHERE code=?2",params![id,code]).map_err(|e|e.to_string())?;} id
 }};
 let salt=SaltString::generate(&mut OsRng); let hash=Argon2::default().hash_password(x.password.as_bytes(),&salt).map_err(|e|e.to_string())?.to_string(); let id=Uuid::new_v4().to_string();
 tx.execute("INSERT INTO users(id,username,password_hash,display_name,is_active,created_at,updated_at) VALUES(?1,?2,?3,?4,1,?5,?5)",params![id,x.username.trim(),hash,x.display_name.trim(),now]).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO user_roles(user_id,role_id) VALUES(?1,?2)",params![id,role_id]).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO user_data_scopes(id,user_id,scope_type,company_id,department_id) VALUES(?1,?2,?3,?4,?5)",params![Uuid::new_v4().to_string(),id,x.scope_type,x.company_id,x.department_id]).map_err(|e|e.to_string())?;
 audit::write(&tx,user,"CREATE","USER",&id,None,Some(&format!(r#"{{"username":"{}","role":"{}","scope":"{}"}}"#,x.username.trim(),x.role_code,x.scope_type)))?;
 tx.commit().map_err(|e|e.to_string())
}
