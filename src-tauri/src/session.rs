use chrono::{DateTime,Duration,Utc};
use std::{collections::HashMap,sync::Mutex};
use uuid::Uuid;

const IDLE_MINUTES:i64=30;
#[derive(Clone)] pub struct SessionContext {pub user_id:String,pub last_seen:DateTime<Utc>}
#[derive(Default)] pub struct SessionStore(pub Mutex<HashMap<String,SessionContext>>);

impl SessionStore{
 pub fn issue(&self,user_id:String)->Result<String,String>{let token=Uuid::new_v4().to_string();self.0.lock().map_err(|_|"Session store unavailable")?.insert(token.clone(),SessionContext{user_id,last_seen:Utc::now()});Ok(token)}
 pub fn resolve(&self,token:&str)->Result<String,String>{let mut map=self.0.lock().map_err(|_|"Session store unavailable")?;let s=map.get_mut(token).ok_or("Session ไม่ถูกต้องหรือหมดอายุ")?;if Utc::now()-s.last_seen>Duration::minutes(IDLE_MINUTES){map.remove(token);return Err("Session หมดอายุ กรุณาเข้าสู่ระบบใหม่".into())}s.last_seen=Utc::now();Ok(s.user_id.clone())}
 pub fn revoke(&self,token:&str)->Result<(),String>{self.0.lock().map_err(|_|"Session store unavailable")?.remove(token);Ok(())}
}
