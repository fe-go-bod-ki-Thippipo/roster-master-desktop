use serde::Serialize;
use tauri::{AppHandle,State};
use crate::{auth,db,edition,session::{LoginLimiter,SessionStore}};

#[derive(Serialize)]
pub struct DashboardSummary { pub employees:i64, pub target_hc:f64, pub fte:f64, pub vacancies:f64 }

#[tauri::command] pub fn app_identity(app:AppHandle)->Result<Option<edition::AppIdentity>,String>{edition::identity(&app)}
#[tauri::command] pub fn initialize_central(app:AppHandle,site_code:String,site_name:String)->Result<(),String>{edition::initialize_central(&app,&site_code,&site_name)}
#[tauri::command] pub fn needs_setup(app:AppHandle)->Result<bool,String>{auth::needs_setup(&app)}
#[tauri::command] pub fn create_first_admin(app:AppHandle,username:String,password:String,display_name:String)->Result<(),String>{auth::create_first_admin(&app,&username,&password,&display_name)}
#[tauri::command] pub fn login(app:AppHandle,store:State<SessionStore>,limiter:State<LoginLimiter>,username:String,password:String)->Result<auth::Session,String>{auth::login(&app,&store,&limiter,&username,&password)}
#[tauri::command] pub fn logout(store:State<SessionStore>,token:String)->Result<(),String>{auth::logout(&store,&token)}

#[tauri::command]
pub fn dashboard_summary(app:AppHandle,store:State<SessionStore>,token:String)->Result<DashboardSummary,String>{
 let user_id=auth::resolve_active(&app,&store,&token)?; let pc=db::open(&app)?; let allowed:i64=pc.query_row("SELECT EXISTS(SELECT 1 FROM user_roles ur JOIN role_permissions rp ON rp.role_id=ur.role_id JOIN permissions p ON p.id=rp.permission_id WHERE ur.user_id=?1 AND p.code='dashboard.view')",[&user_id],|r|r.get(0)).map_err(|e|e.to_string())?; if allowed!=1{return Err("ไม่มีสิทธิ์ดู Dashboard".into())}
 let employees=crate::employees::list(&app,&user_id)?.into_iter().filter(|e|e.status=="ACTIVE").count() as i64;
 let positions=crate::org::positions(&app,&user_id)?; let target_hc:f64=positions.iter().map(|p|p.target_hc).sum();
 let visible_positions:std::collections::HashSet<String>=positions.into_iter().map(|p|p.id).collect();
 let assignments=crate::employees::assignments(&app,&user_id)?; let active:Vec<_>=assignments.into_iter().filter(|a|visible_positions.contains(&a.position_id)&&a.effective_from.as_deref().map(|d|d<=chrono::Local::now().date_naive().format("%Y-%m-%d").to_string().as_str()).unwrap_or(true)&&a.effective_to.as_deref().map(|d|d>=chrono::Local::now().date_naive().format("%Y-%m-%d").to_string().as_str()).unwrap_or(true)).collect();
 let fte:f64=active.iter().map(|a|a.fte.unwrap_or(0.0)).sum();
 Ok(DashboardSummary{employees,target_hc,fte,vacancies:(target_hc-fte).max(0.0)})
}

#[tauri::command] pub fn list_company_groups(app:AppHandle,store:State<SessionStore>,token:String)->Result<Vec<crate::org::CompanyGroup>,String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::org::groups(&app,&user_id)}
#[tauri::command] pub fn list_companies(app:AppHandle,store:State<SessionStore>,token:String)->Result<Vec<crate::org::Company>,String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::org::companies(&app,&user_id)}
#[tauri::command] pub fn list_org_units(app:AppHandle,store:State<SessionStore>,token:String)->Result<Vec<crate::org::OrgUnit>,String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::org::units(&app,&user_id)}
#[tauri::command] pub fn list_positions(app:AppHandle,store:State<SessionStore>,token:String)->Result<Vec<crate::org::Position>,String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::org::positions(&app,&user_id)}
#[tauri::command] pub fn add_company_group(app:AppHandle,store:State<SessionStore>,token:String,code:String,name:String)->Result<(),String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::org::add_group(&app,&user_id,&code,&name)}
#[tauri::command] pub fn add_company(app:AppHandle,store:State<SessionStore>,token:String,code:String,name:String,company_group_id:Option<String>)->Result<(),String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::org::add_company(&app,&user_id,&code,&name,company_group_id)}
#[tauri::command] pub fn add_org_unit(app:AppHandle,store:State<SessionStore>,token:String,code:String,name:String,company_id:String,parent_id:Option<String>,unit_type:String)->Result<(),String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::org::add_unit(&app,&user_id,&code,&name,&company_id,parent_id,&unit_type)}
#[tauri::command] pub fn add_position(app:AppHandle,store:State<SessionStore>,token:String,code:String,name:String,org_unit_id:String,grade:Option<i64>,target_hc:f64)->Result<(),String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::org::add_position(&app,&user_id,&code,&name,&org_unit_id,grade,target_hc)}

#[tauri::command] pub fn list_employees(app:AppHandle,store:State<SessionStore>,token:String)->Result<Vec<crate::employees::Employee>,String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::employees::list(&app,&user_id)}
#[tauri::command] pub fn add_employee(app:AppHandle,store:State<SessionStore>,token:String,input:crate::employees::EmployeeInput)->Result<(),String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::employees::add(&app,&user_id,input)}
#[tauri::command] pub fn list_assignments(app:AppHandle,store:State<SessionStore>,token:String)->Result<Vec<crate::employees::Assignment>,String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::employees::assignments(&app,&user_id)}
#[tauri::command] pub fn add_assignment(app:AppHandle,store:State<SessionStore>,token:String,employee_id:String,position_id:String,role_type:String,fte:Option<f64>,effective_from:Option<String>,effective_to:Option<String>)->Result<(),String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::employees::add_assignment(&app,&user_id,&employee_id,&position_id,&role_type,fte,effective_from,effective_to)}

#[tauri::command] pub fn list_managed_users(app:AppHandle,store:State<SessionStore>,token:String)->Result<Vec<crate::admin::ManagedUser>,String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::admin::list(&app,&user_id)}
#[tauri::command] pub fn create_managed_user(app:AppHandle,store:State<SessionStore>,token:String,input:crate::admin::CreateUserInput)->Result<(),String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::admin::create(&app,&user_id,input)}
#[tauri::command] pub fn export_provision_package(app:AppHandle,store:State<SessionStore>,token:String,target_site_code:String,target_site_name:String,user_ids:Vec<String>)->Result<String,String>{let user_id=auth::resolve_active(&app,&store,&token)?;crate::packages::export_provision(&app,&user_id,&target_site_code,&target_site_name,user_ids)}
#[tauri::command] pub fn import_provision_package(app:AppHandle,package_json:String)->Result<(),String>{crate::packages::import_provision(&app,&package_json)}
