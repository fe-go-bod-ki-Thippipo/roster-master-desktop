use serde::Serialize;
use tauri::AppHandle;
use crate::{auth,db};

#[derive(Serialize)]
pub struct DashboardSummary { pub employees:i64, pub target_hc:f64, pub fte:f64, pub vacancies:f64 }

#[tauri::command] pub fn needs_setup(app:AppHandle)->Result<bool,String>{auth::needs_setup(&app)}
#[tauri::command] pub fn create_first_admin(app:AppHandle,username:String,password:String,display_name:String)->Result<(),String>{auth::create_first_admin(&app,&username,&password,&display_name)}
#[tauri::command] pub fn login(app:AppHandle,username:String,password:String)->Result<auth::Session,String>{auth::login(&app,&username,&password)}

#[tauri::command]
pub fn dashboard_summary(app:AppHandle)->Result<DashboardSummary,String>{
 let conn=db::open(&app)?;
 let employees=conn.query_row("SELECT COUNT(*) FROM employees WHERE status='ACTIVE'",[],|r|r.get(0)).map_err(|e|e.to_string())?;
 let target_hc:f64=conn.query_row("SELECT COALESCE(SUM(target_hc),0) FROM positions WHERE is_active=1",[],|r|r.get(0)).map_err(|e|e.to_string())?;
 let fte:f64=conn.query_row("SELECT COALESCE(SUM(COALESCE(a.fte,1.0/(SELECT COUNT(*) FROM assignments x WHERE x.employee_id=a.employee_id AND x.is_cancelled=0 AND (x.effective_from IS NULL OR x.effective_from<=date('now')) AND (x.effective_to IS NULL OR x.effective_to>=date('now'))))),0) FROM assignments a WHERE a.is_cancelled=0 AND (a.effective_from IS NULL OR a.effective_from<=date('now')) AND (a.effective_to IS NULL OR a.effective_to>=date('now'))",[],|r|r.get(0)).map_err(|e|e.to_string())?;
 Ok(DashboardSummary{employees,target_hc,fte,vacancies:(target_hc-fte).max(0.0)})
}
