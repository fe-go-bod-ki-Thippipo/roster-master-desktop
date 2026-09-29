use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};
use chrono::Utc;
use rusqlite::params;
use serde::Serialize;
use tauri::{AppHandle,State};
use uuid::Uuid;
use crate::{db,session::SessionStore};

#[derive(Serialize)]
pub struct Session { pub token:String, pub username:String, pub display_name:String, pub permissions:Vec<String> }

pub fn needs_setup(app:&AppHandle)->Result<bool,String>{
 let conn=db::open(app)?;
 let count:i64=conn.query_row("SELECT COUNT(*) FROM users",[],|r|r.get(0)).map_err(|e|e.to_string())?;
 Ok(count==0)
}
pub fn create_first_admin(app:&AppHandle,username:&str,password:&str,display_name:&str)->Result<(),String>{
 if !crate::edition::is_central(app)? {return Err("สร้าง System Admin ได้เฉพาะ Central Edition".into())}
 if username.trim().len()<3 || password.len()<10 || display_name.trim().is_empty(){return Err("กรุณากำหนดชื่อผู้ใช้ ชื่อแสดง และรหัสผ่านอย่างน้อย 10 ตัวอักษร".into())}
 if !needs_setup(app)? {return Err("ระบบมีผู้ใช้งานแล้ว".into())}
 let conn=db::open(app)?;
 let salt=SaltString::generate(&mut OsRng);
 let hash=Argon2::default().hash_password(password.as_bytes(),&salt).map_err(|e|e.to_string())?.to_string();
 let user_id=Uuid::new_v4().to_string(); let role_id=Uuid::new_v4().to_string(); let now=Utc::now().to_rfc3339();
 let tx=conn.unchecked_transaction().map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO users(id,username,password_hash,display_name,is_active,created_at,updated_at) VALUES(?1,?2,?3,?4,1,?5,?5)",params![user_id,username.trim(),hash,display_name.trim(),now]).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO roles(id,code,name) VALUES(?1,'SYSTEM_ADMIN','System Administrator')",params![role_id]).map_err(|e|e.to_string())?;
 let perms=[("dashboard.view","ดู Dashboard"),("employee.view","ดูพนักงาน"),("employee.create","เพิ่มพนักงาน"),("employee.update","แก้ไขพนักงาน"),("employee.terminate","พ้นสภาพพนักงาน"),("position.view","ดูตำแหน่ง"),("position.manage","จัดการตำแหน่ง"),("department.view","ดูหน่วยงาน"),("department.manage","จัดการหน่วยงาน"),("assignment.view","ดู Assignment"),("assignment.manage","จัดการ Assignment"),("import.employee","นำเข้าพนักงาน"),("export.employee","ส่งออกพนักงาน"),("export.dashboard","ส่งออก Dashboard"),("user.manage","จัดการผู้ใช้"),("role.manage","จัดการสิทธิ์"),("audit.view","ดู Audit Log"),("settings.manage","ตั้งค่าระบบ")];
 for (code,name) in perms { let pid=Uuid::new_v4().to_string(); tx.execute("INSERT INTO permissions(id,code,name) VALUES(?1,?2,?3)",params![pid,code,name]).map_err(|e|e.to_string())?; tx.execute("INSERT INTO role_permissions(role_id,permission_id) VALUES(?1,?2)",params![role_id,pid]).map_err(|e|e.to_string())?; }
 tx.execute("INSERT INTO user_roles(user_id,role_id) VALUES(?1,?2)",params![user_id,role_id]).map_err(|e|e.to_string())?;
 tx.execute("INSERT INTO user_data_scopes(id,user_id,scope_type) VALUES(?1,?2,'GLOBAL')",params![Uuid::new_v4().to_string(),user_id]).map_err(|e|e.to_string())?;
 tx.commit().map_err(|e|e.to_string())
}
pub fn login(app:&AppHandle,store:&State<SessionStore>,username:&str,password:&str)->Result<Session,String>{
 let conn=db::open(app)?;
 let row=conn.query_row("SELECT id,username,password_hash,display_name FROM users WHERE username=?1 AND is_active=1",[username],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))).map_err(|_|"ชื่อผู้ใช้หรือรหัสผ่านไม่ถูกต้อง".to_string())?;
 let parsed=PasswordHash::new(&row.2).map_err(|_|"ข้อมูลรหัสผ่านไม่ถูกต้อง".to_string())?;
 Argon2::default().verify_password(password.as_bytes(),&parsed).map_err(|_|"ชื่อผู้ใช้หรือรหัสผ่านไม่ถูกต้อง".to_string())?;
 conn.execute("UPDATE users SET last_login_at=?1 WHERE id=?2",params![Utc::now().to_rfc3339(),row.0]).map_err(|e|e.to_string())?;
 let mut stmt=conn.prepare("SELECT DISTINCT p.code FROM permissions p JOIN role_permissions rp ON rp.permission_id=p.id JOIN user_roles ur ON ur.role_id=rp.role_id WHERE ur.user_id=?1 ORDER BY p.code").map_err(|e|e.to_string())?;
 let permissions=stmt.query_map([&row.0],|r|r.get::<_,String>(0)).map_err(|e|e.to_string())?.filter_map(Result::ok).collect();
 let token=store.issue(row.0)?; Ok(Session{token,username:row.1,display_name:row.3,permissions})
}

pub fn resolve_active(app:&AppHandle,store:&State<SessionStore>,token:&str)->Result<String,String>{let user_id=store.resolve(token)?;let c=db::open(app)?;let active:i64=c.query_row("SELECT is_active FROM users WHERE id=?1",[&user_id],|r|r.get(0)).map_err(|_|"ไม่พบบัญชีผู้ใช้")?;if active!=1{return Err("บัญชีถูกปิดใช้งาน".into())}Ok(user_id)}
pub fn logout(store:&State<SessionStore>,token:&str)->Result<(),String>{store.revoke(token)}
