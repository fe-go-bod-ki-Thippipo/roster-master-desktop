use rusqlite::{params,Connection};
#[derive(Clone)] pub struct AuthContext {pub user_id:String,pub global:bool,pub companies:Vec<String>,pub departments:Vec<String>}
pub fn context(conn:&Connection,user_id:&str)->Result<AuthContext,String>{
 let mut s=conn.prepare("SELECT scope_type,company_id,department_id FROM user_data_scopes WHERE user_id=?1").map_err(|e|e.to_string())?;
 let rows=s.query_map([user_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,Option<String>>(2)?))).map_err(|e|e.to_string())?;
 let mut x=AuthContext{user_id:user_id.into(),global:false,companies:vec![],departments:vec![]};
 for row in rows.filter_map(Result::ok){match row.0.as_str(){"GLOBAL"=>x.global=true,"COMPANY"=>if let Some(v)=row.1{x.companies.push(v)},"DEPARTMENT"=>if let Some(v)=row.2{x.departments.push(v)},_=>{}}} Ok(x)
}
pub fn can_company(ctx:&AuthContext,company_id:&str)->bool{ctx.global||ctx.companies.iter().any(|x|x==company_id)}
pub fn can_department(conn:&Connection,ctx:&AuthContext,department_id:&str)->Result<bool,String>{
 if ctx.global||ctx.departments.iter().any(|x|x==department_id){return Ok(true)}
 let company:String=conn.query_row("SELECT company_id FROM org_units WHERE id=?1",[department_id],|r|r.get(0)).map_err(|_|"ไม่พบหน่วยงาน")?;Ok(can_company(ctx,&company))
}
pub fn can_employee(conn:&Connection,ctx:&AuthContext,employee_id:&str)->Result<bool,String>{
 if ctx.global{return Ok(true)}
 let home:Option<String>=conn.query_row("SELECT home_company_id FROM employees WHERE id=?1",[employee_id],|r|r.get(0)).map_err(|_|"ไม่พบพนักงาน")?;
 if home.as_deref().is_some_and(|x|can_company(ctx,x)){return Ok(true)}
 let mut s=conn.prepare("SELECT DISTINCT ou.company_id,ou.id FROM assignments a JOIN positions p ON p.id=a.position_id JOIN org_units ou ON ou.id=p.org_unit_id WHERE a.employee_id=?1 AND a.is_cancelled=0 AND (a.effective_from IS NULL OR a.effective_from<=date('now')) AND (a.effective_to IS NULL OR a.effective_to>=date('now'))").map_err(|e|e.to_string())?;
 let rows=s.query_map(params![employee_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).map_err(|e|e.to_string())?;
 for row in rows.filter_map(Result::ok){if can_company(ctx,&row.0)||ctx.departments.iter().any(|d|d==&row.1){return Ok(true)}} Ok(false)
}
