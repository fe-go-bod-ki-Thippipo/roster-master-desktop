use rusqlite::params;
use serde::Serialize;
use tauri::AppHandle;
use uuid::Uuid;
use crate::db;

#[derive(Serialize)] pub struct CompanyGroup { pub id:String,pub code:String,pub name:String }
#[derive(Serialize)] pub struct Company { pub id:String,pub code:String,pub name:String,pub company_group_id:Option<String> }
#[derive(Serialize)] pub struct OrgUnit { pub id:String,pub code:String,pub name:String,pub company_id:String,pub parent_id:Option<String>,pub unit_type:String }
#[derive(Serialize)] pub struct Position { pub id:String,pub code:String,pub name:String,pub org_unit_id:String,pub grade:Option<i64>,pub target_hc:f64 }

fn require(app:&AppHandle,user_id:&str,permission:&str)->Result<(),String>{
 let conn=db::open(app)?;
 let ok:i64=conn.query_row("SELECT EXISTS(SELECT 1 FROM user_roles ur JOIN role_permissions rp ON rp.role_id=ur.role_id JOIN permissions p ON p.id=rp.permission_id WHERE ur.user_id=?1 AND p.code=?2)",params![user_id,permission],|r|r.get(0)).map_err(|e|e.to_string())?;
 if ok==1 {Ok(())} else {Err("ไม่มีสิทธิ์ดำเนินการ".into())}
}
pub fn groups(app:&AppHandle,user:&str)->Result<Vec<CompanyGroup>,String>{require(app,user,"department.view")?;let c=db::open(app)?;let mut s=c.prepare("SELECT id,code,name FROM company_groups WHERE is_active=1 ORDER BY name").map_err(|e|e.to_string())?;Ok(s.query_map([],|r|Ok(CompanyGroup{id:r.get(0)?,code:r.get(1)?,name:r.get(2)?})).map_err(|e|e.to_string())?.filter_map(Result::ok).collect())}
pub fn companies(app:&AppHandle,user:&str)->Result<Vec<Company>,String>{require(app,user,"department.view")?;let c=db::open(app)?;let mut s=c.prepare("SELECT id,code,name,company_group_id FROM companies WHERE is_active=1 ORDER BY name").map_err(|e|e.to_string())?;Ok(s.query_map([],|r|Ok(Company{id:r.get(0)?,code:r.get(1)?,name:r.get(2)?,company_group_id:r.get(3)?})).map_err(|e|e.to_string())?.filter_map(Result::ok).collect())}
pub fn units(app:&AppHandle,user:&str)->Result<Vec<OrgUnit>,String>{require(app,user,"department.view")?;let c=db::open(app)?;let mut s=c.prepare("SELECT id,code,name,company_id,parent_id,unit_type FROM org_units WHERE is_active=1 ORDER BY company_id,sort_order,name").map_err(|e|e.to_string())?;Ok(s.query_map([],|r|Ok(OrgUnit{id:r.get(0)?,code:r.get(1)?,name:r.get(2)?,company_id:r.get(3)?,parent_id:r.get(4)?,unit_type:r.get(5)?})).map_err(|e|e.to_string())?.filter_map(Result::ok).collect())}
pub fn positions(app:&AppHandle,user:&str)->Result<Vec<Position>,String>{require(app,user,"position.view")?;let c=db::open(app)?;let mut s=c.prepare("SELECT id,code,name,org_unit_id,grade,target_hc FROM positions WHERE is_active=1 ORDER BY org_unit_id,sort_order,name").map_err(|e|e.to_string())?;Ok(s.query_map([],|r|Ok(Position{id:r.get(0)?,code:r.get(1)?,name:r.get(2)?,org_unit_id:r.get(3)?,grade:r.get(4)?,target_hc:r.get(5)?})).map_err(|e|e.to_string())?.filter_map(Result::ok).collect())}

pub fn add_group(app:&AppHandle,user:&str,code:&str,name:&str)->Result<(),String>{require(app,user,"department.manage")?;db::open(app)?.execute("INSERT INTO company_groups(id,code,name) VALUES(?1,?2,?3)",params![Uuid::new_v4().to_string(),code.trim(),name.trim()]).map_err(|e|e.to_string())?;Ok(())}
pub fn add_company(app:&AppHandle,user:&str,code:&str,name:&str,group_id:Option<String>)->Result<(),String>{require(app,user,"department.manage")?;db::open(app)?.execute("INSERT INTO companies(id,code,name,company_group_id) VALUES(?1,?2,?3,?4)",params![Uuid::new_v4().to_string(),code.trim(),name.trim(),group_id]).map_err(|e|e.to_string())?;Ok(())}
pub fn add_unit(app:&AppHandle,user:&str,code:&str,name:&str,company_id:&str,parent_id:Option<String>,unit_type:&str)->Result<(),String>{require(app,user,"department.manage")?;if !["SECTION","DEPARTMENT","UNIT"].contains(&unit_type){return Err("ประเภทหน่วยงานไม่ถูกต้อง".into())}db::open(app)?.execute("INSERT INTO org_units(id,code,name,company_id,parent_id,unit_type) VALUES(?1,?2,?3,?4,?5,?6)",params![Uuid::new_v4().to_string(),code.trim(),name.trim(),company_id,parent_id,unit_type]).map_err(|e|e.to_string())?;Ok(())}
pub fn add_position(app:&AppHandle,user:&str,code:&str,name:&str,org_unit_id:&str,grade:Option<i64>,target_hc:f64)->Result<(),String>{require(app,user,"position.manage")?;if target_hc<0.0{return Err("กรอบอัตราต้องไม่ติดลบ".into())}db::open(app)?.execute("INSERT INTO positions(id,code,name,org_unit_id,grade,target_hc) VALUES(?1,?2,?3,?4,?5,?6)",params![Uuid::new_v4().to_string(),code.trim(),name.trim(),org_unit_id,grade,target_hc]).map_err(|e|e.to_string())?;Ok(())}
