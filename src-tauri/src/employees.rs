use chrono::{Duration,NaiveDate};
use rusqlite::params;
use serde::{Deserialize,Serialize};
use tauri::AppHandle;
use uuid::Uuid;
use crate::{audit,db};

#[derive(Serialize)] pub struct Employee {pub id:String,pub employee_code:String,pub prefix:Option<String>,pub full_name:String,pub nickname:Option<String>,pub gender:Option<String>,pub birth_date:Option<String>,pub national_id:Option<String>,pub nationality:Option<String>,pub education:Option<String>,pub home_company_id:Option<String>,pub employment_type_id:Option<String>,pub phone:Option<String>,pub email:Option<String>,pub address:Option<String>,pub emergency_contact:Option<String>,pub emergency_phone:Option<String>,pub hire_date:Option<String>,pub probation_end_date:Option<String>,pub termination_date:Option<String>,pub termination_reason:Option<String>,pub status:String}
#[derive(Deserialize)] pub struct EmployeeInput {pub employee_code:String,pub prefix:Option<String>,pub full_name:String,pub nickname:Option<String>,pub gender:Option<String>,pub birth_date:Option<String>,pub national_id:Option<String>,pub nationality:Option<String>,pub education:Option<String>,pub home_company_id:Option<String>,pub employment_type_id:Option<String>,pub phone:Option<String>,pub email:Option<String>,pub address:Option<String>,pub emergency_contact:Option<String>,pub emergency_phone:Option<String>,pub hire_date:Option<String>,pub probation_end_date:Option<String>,pub termination_date:Option<String>,pub termination_reason:Option<String>,pub status:String}
#[derive(Serialize)] pub struct Assignment {pub id:String,pub employee_id:String,pub position_id:String,pub role_type:String,pub fte:Option<f64>,pub effective_from:Option<String>,pub effective_to:Option<String>}

fn require(app:&AppHandle,user:&str,p:&str)->Result<(),String>{let c=db::open(app)?;let ok:i64=c.query_row("SELECT EXISTS(SELECT 1 FROM user_roles ur JOIN role_permissions rp ON rp.role_id=ur.role_id JOIN permissions x ON x.id=rp.permission_id WHERE ur.user_id=?1 AND x.code=?2)",params![user,p],|r|r.get(0)).map_err(|e|e.to_string())?;if ok==1{Ok(())}else{Err("ไม่มีสิทธิ์ดำเนินการ".into())}}
pub fn list(app:&AppHandle,user:&str)->Result<Vec<Employee>,String>{require(app,user,"employee.view")?;let c=db::open(app)?;let mut s=c.prepare("SELECT id,employee_code,prefix,full_name,nickname,gender,birth_date,national_id,nationality,education,home_company_id,employment_type_id,phone,email,address,emergency_contact,emergency_phone,hire_date,probation_end_date,termination_date,termination_reason,status FROM employees ORDER BY employee_code").map_err(|e|e.to_string())?;let rows=s.query_map([],|r|Ok(Employee{id:r.get(0)?,employee_code:r.get(1)?,prefix:r.get(2)?,full_name:r.get(3)?,nickname:r.get(4)?,gender:r.get(5)?,birth_date:r.get(6)?,national_id:r.get(7)?,nationality:r.get(8)?,education:r.get(9)?,home_company_id:r.get(10)?,employment_type_id:r.get(11)?,phone:r.get(12)?,email:r.get(13)?,address:r.get(14)?,emergency_contact:r.get(15)?,emergency_phone:r.get(16)?,hire_date:r.get(17)?,probation_end_date:r.get(18)?,termination_date:r.get(19)?,termination_reason:r.get(20)?,status:r.get(21)?})).map_err(|e|e.to_string())?; let v=rows.filter_map(Result::ok).collect(); Ok(v)}
pub fn add(app:&AppHandle,user:&str,x:EmployeeInput)->Result<(),String>{
 require(app,user,"employee.create")?;
 if x.employee_code.trim().is_empty()||x.full_name.trim().is_empty(){return Err("รหัสพนักงานและชื่อ-นามสกุลห้ามว่าง".into())}
 let probation=match (&x.probation_end_date,&x.hire_date){(Some(v),_) if !v.is_empty()=>Some(v.clone()),(_,Some(h))=>NaiveDate::parse_from_str(h,"%Y-%m-%d").ok().map(|d|(d+Duration::days(90)).format("%Y-%m-%d").to_string()),_=>None};
 let id=Uuid::new_v4().to_string(); let c=db::open(app)?;
 c.execute("INSERT INTO employees(id,employee_code,prefix,full_name,nickname,gender,birth_date,national_id,nationality,education,home_company_id,employment_type_id,phone,email,address,emergency_contact,emergency_phone,hire_date,probation_end_date,termination_date,termination_reason,status) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)",params![id,x.employee_code.trim(),x.prefix,x.full_name.trim(),x.nickname,x.gender,x.birth_date,x.national_id,x.nationality,x.education,x.home_company_id,x.employment_type_id,x.phone,x.email,x.address,x.emergency_contact,x.emergency_phone,x.hire_date,probation,x.termination_date,x.termination_reason,x.status]).map_err(|e|e.to_string())?;
 audit::write(&c,user,"CREATE","EMPLOYEE",&id,None,Some(&format!(r#"{{"employee_code":"{}"}}"#,x.employee_code.trim())))?; Ok(())
}
pub fn assignments(app:&AppHandle,user:&str)->Result<Vec<Assignment>,String>{require(app,user,"assignment.view")?;let c=db::open(app)?;let mut s=c.prepare("SELECT id,employee_id,position_id,role_type,fte,effective_from,effective_to FROM assignments WHERE is_cancelled=0 ORDER BY employee_id,role_type").map_err(|e|e.to_string())?;let rows=s.query_map([],|r|Ok(Assignment{id:r.get(0)?,employee_id:r.get(1)?,position_id:r.get(2)?,role_type:r.get(3)?,fte:r.get(4)?,effective_from:r.get(5)?,effective_to:r.get(6)?})).map_err(|e|e.to_string())?; let v=rows.filter_map(Result::ok).collect(); Ok(v)}

fn overlaps(a_from:&str,a_to:Option<&str>,b_from:Option<&str>,b_to:Option<&str>)->bool{
 let a_end=a_to.unwrap_or("9999-12-31"); let b_start=b_from.unwrap_or("0001-01-01"); let b_end=b_to.unwrap_or("9999-12-31");
 a_from<=b_end && b_start<=a_end
}
pub fn add_assignment(app:&AppHandle,user:&str,employee_id:&str,position_id:&str,role_type:&str,fte:Option<f64>,effective_from:Option<String>,effective_to:Option<String>)->Result<(),String>{
 require(app,user,"assignment.manage")?;
 if !["PRIMARY","SECONDARY","COORDINATION"].contains(&role_type){return Err("ประเภท Assignment ไม่ถูกต้อง".into())}
 let from=effective_from.unwrap_or_else(||chrono::Local::now().date_naive().format("%Y-%m-%d").to_string());
 if let Some(ref to)=effective_to {if to<&from{return Err("วันที่สิ้นสุดต้องไม่ก่อนวันที่เริ่ม".into())}}
 let c=db::open(app)?;
 let mut stmt=c.prepare("SELECT role_type,fte,effective_from,effective_to FROM assignments WHERE employee_id=?1 AND is_cancelled=0").map_err(|e|e.to_string())?;
 let rows=stmt.query_map(params![employee_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<f64>>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,Option<String>>(3)?))).map_err(|e|e.to_string())?;
 let mut explicit_total=fte.unwrap_or(0.0);
 for row in rows.filter_map(Result::ok){
   if overlaps(&from,effective_to.as_deref(),row.2.as_deref(),row.3.as_deref()){
     if role_type=="PRIMARY" && row.0=="PRIMARY"{return Err("พนักงานมีตำแหน่งหลักในช่วงวันที่นี้แล้ว".into())}
     explicit_total+=row.1.unwrap_or(0.0);
   }
 }
 if fte.is_some() && explicit_total>1.000001{return Err("FTE รวมในช่วงวันที่เดียวกันต้องไม่เกิน 1.00".into())}
 let id=Uuid::new_v4().to_string();
 c.execute("INSERT INTO assignments(id,employee_id,position_id,role_type,fte,effective_from,effective_to) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![id,employee_id,position_id,role_type,fte,from,effective_to]).map_err(|e|e.to_string())?;
 audit::write(&c,user,"CREATE","ASSIGNMENT",&id,None,Some(&format!(r#"{{"employee_id":"{}","position_id":"{}","role_type":"{}"}}"#,employee_id,position_id,role_type)))?; Ok(())
}
