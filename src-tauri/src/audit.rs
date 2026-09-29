use chrono::Utc;
use rusqlite::{params,Connection};
use uuid::Uuid;

pub fn write(conn:&Connection,user_id:&str,action:&str,entity_type:&str,entity_id:&str,before:Option<&str>,after:Option<&str>)->Result<(),String>{
 conn.execute("INSERT INTO audit_logs(id,user_id,action,entity_type,entity_id,before_json,after_json,occurred_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
 params![Uuid::new_v4().to_string(),user_id,action,entity_type,entity_id,before,after,Utc::now().to_rfc3339()]).map_err(|e|e.to_string())?;
 Ok(())
}
