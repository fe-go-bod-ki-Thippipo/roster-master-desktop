use argon2::password_hash::PasswordHash;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use chrono::{DateTime, Duration, Utc};
use ed25519_dalek::{Signer, SigningKey};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::AppHandle;
use uuid::Uuid;

use crate::{audit, db, edition, trust};

const MAX_PACKAGE_BYTES: usize = 5 * 1024 * 1024;
const PACKAGE_FORMAT: &str = "RMPKG-PROVISION-v2";
const PACKAGE_TYPE: &str = "PROVISION";
const KEY_ENV: &str = "ALPHA";
const TARGET_EDITION: &str = "UNIT";
const CLOCK_SKEW_MINUTES: i64 = 5;

const ALLOWED_ROLES: &[&str] = &["CENTRAL_HR", "COMPANY_HR", "MANAGER", "STAFF", "VIEWER"];
const ALLOWED_PERMISSIONS: &[&str] = &[
    "dashboard.view", "employee.view", "employee.create", "employee.update", "employee.terminate",
    "position.view", "position.manage", "department.view", "department.manage",
    "assignment.view", "assignment.manage", "import.employee", "export.employee",
    "export.dashboard", "audit.view",
];

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvisionEnvelope {
    pub format: String,
    pub key_id: String,
    pub payload_b64: String,
    pub signature_b64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvisionManifest {
    pub schema_version: u32,
    pub package_type: String,
    pub key_env: String,
    pub target_edition: String,
    pub package_id: String,
    pub source_site_code: String,
    pub target_site_code: String,
    pub target_site_name: String,
    pub issued_at: String,
    pub expires_at: String,
    pub users: Vec<ProvisionUser>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvisionUser {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub display_name: String,
    pub role_code: String,
    pub permissions: Vec<String>,
    pub scopes: Vec<ProvisionScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvisionScope {
    pub scope_type: String,
    pub company_id: Option<String>,
    pub department_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProvisionInspection {
    pub package_id: String,
    pub source_site_code: String,
    pub target_site_code: String,
    pub target_site_name: String,
    pub issued_at: String,
    pub expires_at: String,
    pub key_id: String,
    pub key_env: String,
    pub user_count: usize,
    pub usernames: Vec<String>,
    pub users: Vec<ProvisionInspectionUser>,
}
#[derive(Debug, Serialize)]
pub struct ProvisionInspectionUser {
    pub username: String,
    pub role_code: String,
    pub permissions: Vec<String>,
    pub scopes: Vec<ProvisionScope>,
}

#[derive(Debug, Serialize)]
pub struct SigningKeyStatus {
    pub installed: bool,
    pub key_id: Option<String>,
    pub key_env: Option<String>,
    pub embedded_trust: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SigningKeyFile {
    key_id: String,
    env: String,
    private_key_b64: String,
    public_key_b64: String,
}

fn is_system_admin(conn: &rusqlite::Connection, user_id: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM user_roles ur JOIN roles r ON r.id=ur.role_id WHERE ur.user_id=?1 AND r.code='SYSTEM_ADMIN')",
        [user_id],
        |r| r.get::<_, i64>(0),
    ).map(|v| v == 1).map_err(|e| e.to_string())
}

fn has_permission(conn: &rusqlite::Connection, user_id: &str, permission: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM user_roles ur JOIN role_permissions rp ON rp.role_id=ur.role_id JOIN permissions p ON p.id=rp.permission_id WHERE ur.user_id=?1 AND p.code=?2)",
        params![user_id, permission],
        |r| r.get::<_, i64>(0),
    ).map(|v| v == 1).map_err(|e| e.to_string())
}

fn validate_hash(hash: &str) -> Result<(), String> {
    if !hash.starts_with("$argon2id$") {
        return Err("Package มี password hash ที่ไม่ใช่ Argon2id".into());
    }
    let parsed = PasswordHash::new(hash).map_err(|_| "Package มี password hash ที่ไม่ถูกต้อง".to_string())?;
    if parsed.algorithm.as_str() != "argon2id" {
        return Err("Package มี password hash ที่ไม่ใช่ Argon2id".into());
    }
    Ok(())
}

fn validate_scope(scope: &ProvisionScope) -> Result<(), String> {
    match scope.scope_type.as_str() {
        "GLOBAL" if scope.company_id.is_none() && scope.department_id.is_none() => Ok(()),
        "COMPANY" if scope.company_id.as_deref().is_some_and(|v| !v.trim().is_empty()) && scope.department_id.is_none() => Ok(()),
        "DEPARTMENT" if scope.department_id.as_deref().is_some_and(|v| !v.trim().is_empty()) => Ok(()),
        _ => Err("Package มี Data Scope ไม่ถูกต้อง".into()),
    }
}

fn validate_manifest(p: &ProvisionManifest, key: &trust::EmbeddedKey, expected_site_code: Option<&str>, now: DateTime<Utc>) -> Result<(), String> {
    if p.schema_version != 2 || p.package_type != PACKAGE_TYPE || p.target_edition != TARGET_EDITION {
        return Err("Package type/version/edition ไม่ถูกต้อง".into());
    }
    if p.key_env != KEY_ENV || p.key_env != key.env {
        return Err("Package key environment ไม่ถูกต้อง".into());
    }
    if p.package_id.trim().is_empty() || p.source_site_code.trim().is_empty() || p.target_site_code.trim().is_empty() || p.target_site_name.trim().is_empty() {
        return Err("Package identity ไม่สมบูรณ์".into());
    }
    if let Some(expected) = expected_site_code {
        if p.target_site_code != expected.trim() {
            return Err("Package นี้ไม่ได้ออกให้ Site ที่ระบุ".into());
        }
    }
    let issued = DateTime::parse_from_rfc3339(&p.issued_at).map_err(|_| "issued_at ไม่ถูกต้อง")?.with_timezone(&Utc);
    let expires = DateTime::parse_from_rfc3339(&p.expires_at).map_err(|_| "expires_at ไม่ถูกต้อง")?.with_timezone(&Utc);
    if issued > now + Duration::minutes(CLOCK_SKEW_MINUTES) {
        return Err("เวลาออก Package อยู่ในอนาคตเกิน clock skew".into());
    }
    if expires <= now {
        return Err("Package หมดอายุแล้ว".into());
    }
    let lifetime = expires - issued;
    if lifetime < Duration::days(1) || lifetime > Duration::days(30) {
        return Err("อายุ Package ต้องอยู่ระหว่าง 1–30 วัน".into());
    }
    if p.users.is_empty() {
        return Err("Package ต้องมีผู้ใช้อย่างน้อย 1 ราย".into());
    }
    for u in &p.users {
        if u.id.trim().is_empty() || u.username.trim().len() < 3 || u.display_name.trim().is_empty() {
            return Err("ข้อมูลผู้ใช้ใน Package ไม่สมบูรณ์".into());
        }
        if !ALLOWED_ROLES.contains(&u.role_code.as_str()) {
            return Err("Package มี Role ที่ Unit ไม่อนุญาต".into());
        }
        if u.permissions.iter().any(|x| !ALLOWED_PERMISSIONS.contains(&x.as_str())) {
            return Err("Package มี Permission ที่ Unit ไม่อนุญาต".into());
        }
        if u.scopes.is_empty() {
            return Err("Package ต้องมี Scope อย่างน้อย 1 รายการต่อผู้ใช้".into());
        }
        for scope in &u.scopes { validate_scope(scope)?; }
        validate_hash(&u.password_hash)?;
    }
    Ok(())
}

fn decode_verified(json: &str, expected_site_code: Option<&str>, now: DateTime<Utc>) -> Result<(ProvisionEnvelope, ProvisionManifest), String> {
    if json.as_bytes().len() > MAX_PACKAGE_BYTES {
        return Err("Package มีขนาดเกิน 5MB".into());
    }
    let envelope: ProvisionEnvelope = serde_json::from_str(json).map_err(|_| "Envelope ไม่ถูกต้อง".to_string())?;
    if envelope.format != PACKAGE_FORMAT {
        return Err("Package format ไม่ถูกต้อง".into());
    }
    let payload = B64.decode(&envelope.payload_b64).map_err(|_| "payload_b64 ไม่ถูกต้อง".to_string())?;
    if payload.len() > MAX_PACKAGE_BYTES {
        return Err("Payload มีขนาดเกิน 5MB".into());
    }
    let key = trust::verify_envelope(&envelope.key_id, &envelope.signature_b64, &payload)?;
    let manifest: ProvisionManifest = serde_json::from_slice(&payload).map_err(|_| "Payload JSON ไม่ถูกต้อง".to_string())?;
    validate_manifest(&manifest, &key, expected_site_code, now)?;
    Ok((envelope, manifest))
}

#[cfg(feature = "central")]
fn install_signing_key_core(c:&rusqlite::Connection,user_id:&str,key_json:&str)->Result<(),String>{
    if key_json.as_bytes().len() > 64 * 1024 { return Err("Signing key file ใหญ่ผิดปกติ".into()); }
    if !is_system_admin(c, user_id)? { return Err("เฉพาะ SYSTEM_ADMIN เท่านั้นที่ติดตั้ง Signing Key ได้".into()); }
    let file: SigningKeyFile = serde_json::from_str(key_json).map_err(|_| "ไฟล์ .rmkey ไม่ถูกต้อง".to_string())?;
    if file.env != KEY_ENV { return Err("รอบ Alpha รับเฉพาะ ALPHA signing key".into()); }
    let trusted = trust::trusted_key(&file.key_id)?;
    if trusted.env != KEY_ENV || trusted.public_key_b64 != file.public_key_b64 {
        return Err("Signing key ไม่ตรงกับ Embedded ALPHA Trust Anchor".into());
    }
    let private = B64.decode(&file.private_key_b64).map_err(|_| "Private key ไม่ถูกต้อง")?;
    let arr: [u8; 32] = private.try_into().map_err(|_| "Private key length ไม่ถูกต้อง")?;
    let sk = SigningKey::from_bytes(&arr);
    let derived = sk.verifying_key().to_bytes();
    if B64.encode(derived) != file.public_key_b64 {
        return Err("Private/Public key ไม่ใช่คู่เดียวกัน".into());
    }
    let digest = Sha256::digest(derived);
    let derived_id = digest[..8].iter().map(|b| format!("{b:02x}")).collect::<String>();
    if derived_id != file.key_id { return Err("key_id ไม่ตรงกับ SHA-256 ของ public key".into()); }

    let tx = c.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute("UPDATE central_signing_keys SET is_active=0 WHERE is_active=1", []).map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO central_signing_keys(id,key_id,key_env,private_key_b64,public_key_b64,installed_at,installed_by_user_id,is_active) VALUES(?1,?2,?3,?4,?5,?6,?7,1)
         ON CONFLICT(key_id) DO UPDATE SET key_env=excluded.key_env,private_key_b64=excluded.private_key_b64,public_key_b64=excluded.public_key_b64,installed_at=excluded.installed_at,installed_by_user_id=excluded.installed_by_user_id,is_active=1",
        params![Uuid::new_v4().to_string(), file.key_id, file.env, file.private_key_b64, file.public_key_b64, Utc::now().to_rfc3339(), user_id],
    ).map_err(|e| e.to_string())?;
    audit::write(&tx, user_id, "INSTALL", "SIGNING_KEY", &file.key_id, None, Some(r#"{"env":"ALPHA"}"#))?;
    tx.commit().map_err(|e| e.to_string())
}

#[cfg(feature = "central")]
pub fn install_signing_key(app:&AppHandle,user_id:&str,key_json:&str)->Result<(),String>{
    if !edition::is_central(app)? { return Err("ติดตั้ง Signing Key ได้เฉพาะ Central Edition".into()); }
    let c=db::open(app)?;install_signing_key_core(&c,user_id,key_json)
}
#[cfg(feature = "central")]
pub fn signing_key_status(app: &AppHandle, user_id: &str) -> Result<SigningKeyStatus, String> {
    if !edition::is_central(app)? { return Err("Signing Key Status ใช้ได้เฉพาะ Central Edition".into()); }
    let c = db::open(app)?;
    if !is_system_admin(c, user_id)? { return Err("เฉพาะ SYSTEM_ADMIN เท่านั้นที่ดู Signing Key Status ได้".into()); }
    let row = c.query_row(
        "SELECT key_id,key_env FROM central_signing_keys WHERE is_active=1 ORDER BY installed_at DESC LIMIT 1",
        [], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    ).optional().map_err(|e| e.to_string())?;
    match row {
        Some((key_id, key_env)) => Ok(SigningKeyStatus { embedded_trust: trust::trusted_key(&key_id).is_ok(), installed: true, key_id: Some(key_id), key_env: Some(key_env) }),
        None => Ok(SigningKeyStatus { installed: false, key_id: None, key_env: None, embedded_trust: false }),
    }
}

#[cfg(feature = "central")]
fn export_provision_core(c:&rusqlite::Connection,user_id:&str,source:&str,target_site_code:&str,target_site_name:&str,user_ids:Vec<String>,valid_days:i64)->Result<String,String>{
    if target_site_code.trim().is_empty() || target_site_name.trim().is_empty() || user_ids.is_empty() {
        return Err("ต้องระบุ Site ปลายทาง ชื่อ Site และผู้ใช้อย่างน้อย 1 ราย".into());
    }
    if !(1..=30).contains(&valid_days) { return Err("อายุ Package ต้องอยู่ระหว่าง 1–30 วัน".into()); }
    if !has_permission(c, user_id, "user.manage")? { return Err("ไม่มีสิทธิ์สร้าง Package".into()); }

    let mut users = Vec::new();
    let mut seen_user_ids = std::collections::HashSet::new();
    for uid in user_ids {
        if !seen_user_ids.insert(uid.clone()) { continue; }
        let (id, username, password_hash, display_name) = c.query_row(
            "SELECT id,username,password_hash,display_name FROM users WHERE id=?1 AND is_active=1",
            [&uid], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?)),
        ).map_err(|_| "ไม่พบผู้ใช้ที่ใช้งานอยู่".to_string())?;
        validate_hash(&password_hash)?;
        let roles: Vec<String> = {
            let mut s = c.prepare("SELECT r.code FROM roles r JOIN user_roles ur ON ur.role_id=r.id WHERE ur.user_id=?1 ORDER BY r.code").map_err(|e| e.to_string())?;
            let rows = s.query_map([&uid], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
            let values: Vec<String> = rows.filter_map(Result::ok).collect();
            values
        };
        if roles.len() != 1 || !ALLOWED_ROLES.contains(&roles[0].as_str()) {
            return Err("ผู้ใช้ที่จะส่งไป Unit ต้องมี Role ที่อนุญาตเพียง 1 Role และห้าม SYSTEM_ADMIN".into());
        }
        let role_code = roles[0].clone();
        let permissions: Vec<String> = {
            let mut s = c.prepare("SELECT p.code FROM permissions p JOIN role_permissions rp ON rp.permission_id=p.id JOIN roles r ON r.id=rp.role_id WHERE r.code=?1 ORDER BY p.code").map_err(|e| e.to_string())?;
            let rows = s.query_map([&role_code], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
            let values: Vec<String> = rows.filter_map(Result::ok).collect();
            values
        };
        if permissions.iter().any(|x| !ALLOWED_PERMISSIONS.contains(&x.as_str())) {
            return Err("Role มี Permission ที่ Unit allowlist ไม่อนุญาต".into());
        }
        let scopes: Vec<ProvisionScope> = {
            let mut s = c.prepare("SELECT scope_type,company_id,department_id FROM user_data_scopes WHERE user_id=?1 ORDER BY scope_type,company_id,department_id").map_err(|e| e.to_string())?;
            let rows = s.query_map([&uid], |r| Ok(ProvisionScope { scope_type: r.get(0)?, company_id: r.get(1)?, department_id: r.get(2)? })).map_err(|e| e.to_string())?;
            let values: Vec<ProvisionScope> = rows.filter_map(Result::ok).collect();
            values
        };
        if scopes.is_empty() { return Err("ผู้ใช้ที่จะส่งไป Unit ต้องมี Data Scope".into()); }
        for s in &scopes { validate_scope(s)?; }
        users.push(ProvisionUser { id, username, password_hash, display_name, role_code, permissions, scopes });
    }

    let (key_id, private_b64, key_env) = c.query_row(
        "SELECT key_id,private_key_b64,key_env FROM central_signing_keys WHERE is_active=1 ORDER BY installed_at DESC LIMIT 1",
        [], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)),
    ).map_err(|_| "Central ยังไม่ได้ติดตั้ง ALPHA Signing Key".to_string())?;
    let trusted = trust::trusted_key(&key_id)?;
    if key_env != KEY_ENV || trusted.env != KEY_ENV { return Err("Signing Key environment ไม่ถูกต้อง".into()); }

    let issued = Utc::now();
    let manifest = ProvisionManifest {
        schema_version: 2, package_type: PACKAGE_TYPE.into(), key_env: KEY_ENV.into(), target_edition: TARGET_EDITION.into(),
        package_id: Uuid::new_v4().to_string(), source_site_code: source.into(), target_site_code: target_site_code.trim().into(),
        target_site_name: target_site_name.trim().into(), issued_at: issued.to_rfc3339(),
        expires_at: (issued + Duration::days(valid_days)).to_rfc3339(), users,
    };
    let payload = serde_json::to_vec(&manifest).map_err(|e| e.to_string())?;
    let private = B64.decode(private_b64).map_err(|_| "Signing key ไม่ถูกต้อง")?;
    let arr: [u8; 32] = private.try_into().map_err(|_| "Signing key length ไม่ถูกต้อง")?;
    let sk = SigningKey::from_bytes(&arr);
    if B64.encode(sk.verifying_key().to_bytes()) != trusted.public_key_b64 { return Err("Installed Signing Key ไม่ตรงกับ Trust Anchor".into()); }
    let mut message = trust::DOMAIN_PREFIX.to_vec();
    message.extend_from_slice(&payload);
    let signature = sk.sign(&message);
    let envelope = ProvisionEnvelope { format: PACKAGE_FORMAT.into(), key_id, payload_b64: B64.encode(&payload), signature_b64: B64.encode(signature.to_bytes()) };
    let json = serde_json::to_string_pretty(&envelope).map_err(|e| e.to_string())?;
    audit::write(c, user_id, "EXPORT", "PROVISION_PACKAGE", &manifest.package_id, None, Some(&format!(r#"{{"target_site_code":"{}","users":{}}}"#, manifest.target_site_code, manifest.users.len())))?;
    Ok(json)
}

#[cfg(feature = "central")]
pub fn export_provision(app:&AppHandle,user_id:&str,target_site_code:&str,target_site_name:&str,user_ids:Vec<String>,valid_days:i64)->Result<String,String>{
    if !edition::is_central(app)? { return Err("สร้าง Package ได้เฉพาะ Central Edition".into()); }
    let c=db::open(app)?;
    let source=edition::identity(app)?.and_then(|x|x.site_code).ok_or("Central Site ยังไม่สมบูรณ์")?;
    export_provision_core(&c,user_id,&source,target_site_code,target_site_name,user_ids,valid_days)
}

pub fn inspect_provision(json: &str) -> Result<ProvisionInspection, String> {
    let (envelope, p) = decode_verified(json, None, Utc::now())?;
    Ok(ProvisionInspection {
        package_id: p.package_id, source_site_code: p.source_site_code, target_site_code: p.target_site_code,
        target_site_name: p.target_site_name, issued_at: p.issued_at, expires_at: p.expires_at,
        key_id: envelope.key_id, key_env: p.key_env, user_count: p.users.len(),
        usernames: p.users.iter().map(|u| u.username.clone()).collect(),
        users: p.users.into_iter().map(|u| ProvisionInspectionUser { username:u.username, role_code:u.role_code, permissions:u.permissions, scopes:u.scopes }).collect(),
    })
}

#[cfg(feature = "unit")]
fn import_verified_manifest(conn: &mut rusqlite::Connection, p: &ProvisionManifest) -> Result<(), String> {
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let already: i64 = tx.query_row("SELECT EXISTS(SELECT 1 FROM package_imports WHERE package_id=?1)", [&p.package_id], |r| r.get(0)).map_err(|e| e.to_string())?;
    if already == 1 { return Err("Package นี้เคยถูกนำเข้าแล้ว".into()); }
    let now = Utc::now().to_rfc3339();
    tx.execute("INSERT INTO app_identity(singleton_id,edition,site_code,site_name,initialized_at) VALUES(1,'UNIT',?1,?2,?3)", params![p.target_site_code, p.target_site_name, now]).map_err(|e| e.to_string())?;

    for u in &p.users {
        let role_id = Uuid::new_v4().to_string();
        tx.execute("INSERT OR IGNORE INTO roles(id,code,name) VALUES(?1,?2,?2)", params![role_id, u.role_code]).map_err(|e| e.to_string())?;
        let actual_role: String = tx.query_row("SELECT id FROM roles WHERE code=?1", [&u.role_code], |r| r.get(0)).map_err(|e| e.to_string())?;
        for code in &u.permissions {
            let pid = Uuid::new_v4().to_string();
            tx.execute("INSERT OR IGNORE INTO permissions(id,code,name) VALUES(?1,?2,?2)", params![pid, code]).map_err(|e| e.to_string())?;
            tx.execute("INSERT OR IGNORE INTO role_permissions(role_id,permission_id) SELECT ?1,id FROM permissions WHERE code=?2", params![actual_role, code]).map_err(|e| e.to_string())?;
        }
        tx.execute("INSERT INTO users(id,username,password_hash,display_name,is_active,created_at,updated_at) VALUES(?1,?2,?3,?4,1,?5,?5)", params![u.id, u.username, u.password_hash, u.display_name, now]).map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO user_roles(user_id,role_id) VALUES(?1,?2)", params![u.id, actual_role]).map_err(|e| e.to_string())?;
        for s in &u.scopes {
            tx.execute("INSERT INTO user_data_scopes(id,user_id,scope_type,company_id,department_id) VALUES(?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(), u.id, s.scope_type, s.company_id, s.department_id]).map_err(|e| e.to_string())?;
        }
    }
    let stored = serde_json::to_string(&p).map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO package_imports(id,package_id,source_site_code,target_site_code,package_type,manifest_json,imported_at) VALUES(?1,?2,?3,?4,'PROVISION',?5,?6)", params![Uuid::new_v4().to_string(), p.package_id, p.source_site_code, p.target_site_code, stored, now]).map_err(|e| e.to_string())?;
    tx.execute("INSERT INTO audit_logs(id,user_id,action,entity_type,entity_id,after_json,occurred_at) VALUES(?1,NULL,'IMPORT','PROVISION_PACKAGE',?2,?3,?4)", params![Uuid::new_v4().to_string(), p.package_id, format!(r#"{{"source_site_code":"{}","target_site_code":"{}","users":{}}}"#, p.source_site_code, p.target_site_code, p.users.len()), now]).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

#[cfg(feature = "unit")]
pub fn import_provision(app: &AppHandle, json: &str, expected_site_code: &str) -> Result<(), String> {
    if expected_site_code.trim().is_empty() { return Err("ต้องพิมพ์ Site Code เพื่อยืนยัน".into()); }
    if edition::identity(app)?.is_some() { return Err("เครื่องนี้ถูก Provision แล้ว".into()); }
    let (_envelope, p) = decode_verified(json, Some(expected_site_code), Utc::now())?;
    let mut conn = db::open(app)?;
    import_verified_manifest(&mut conn, &p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use argon2::{password_hash::{rand_core::OsRng as SaltRng, SaltString}, Argon2, PasswordHasher};
    use ed25519_dalek::{Signer, SigningKey};
    use rand_core::OsRng;
    use serde_json::{json, Value};

    const TEST_PRIVATE_B64:&str="AQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA=";
    const TEST_KEY_ID:&str="65b60673d6ed884b";

    fn hash()->String{Argon2::default().hash_password(b"alpha-test-password",&SaltString::generate(&mut SaltRng)).unwrap().to_string()}
    fn manifest()->ProvisionManifest{
        let now=Utc::now();
        ProvisionManifest{schema_version:2,package_type:PACKAGE_TYPE.into(),key_env:KEY_ENV.into(),target_edition:TARGET_EDITION.into(),
          package_id:Uuid::new_v4().to_string(),source_site_code:"CENTRAL".into(),target_site_code:"SITE-A".into(),target_site_name:"Site A".into(),
          issued_at:now.to_rfc3339(),expires_at:(now+Duration::days(7)).to_rfc3339(),
          users:vec![ProvisionUser{id:Uuid::new_v4().to_string(),username:"user-a".into(),password_hash:hash(),display_name:"User A".into(),
            role_code:"VIEWER".into(),permissions:vec!["dashboard.view".into()],scopes:vec![ProvisionScope{scope_type:"GLOBAL".into(),company_id:None,department_id:None}]}]}
    }
    fn test_signing_key()->SigningKey{
        let b=B64.decode(TEST_PRIVATE_B64).unwrap();let a:[u8;32]=b.try_into().unwrap();SigningKey::from_bytes(&a)
    }
    fn envelope_for_payload(payload:&[u8],key_id:&str,sk:&SigningKey)->String{
        let mut msg=trust::DOMAIN_PREFIX.to_vec();msg.extend_from_slice(payload);
        serde_json::to_string(&ProvisionEnvelope{format:PACKAGE_FORMAT.into(),key_id:key_id.into(),payload_b64:B64.encode(payload),signature_b64:B64.encode(sk.sign(&msg).to_bytes())}).unwrap()
    }
    fn signed(p:&ProvisionManifest)->String{envelope_for_payload(&serde_json::to_vec(p).unwrap(),TEST_KEY_ID,&test_signing_key())}
    fn rejected(p:&ProvisionManifest,site:Option<&str>)->bool{decode_verified(&signed(p),site,Utc::now()).is_err()}

    #[test] fn attack_01_signed_system_admin_role_rejected(){let mut p=manifest();p.users[0].role_code="SYSTEM_ADMIN".into();assert!(rejected(&p,None));}
    #[test] fn attack_02_signed_user_manage_permission_rejected(){let mut p=manifest();p.users[0].permissions.push("user.manage".into());assert!(rejected(&p,None));}
    #[test] fn attack_03_signed_plaintext_password_hash_rejected(){let mut p=manifest();p.users[0].password_hash="password123".into();assert!(rejected(&p,None));}
    #[test] fn attack_04_signed_argon2i_rejected(){let mut p=manifest();p.users[0].password_hash=p.users[0].password_hash.replacen("$argon2id$","$argon2i$",1);assert!(rejected(&p,None));}
    #[test] fn attack_05_signed_wrong_target_site_rejected(){assert!(rejected(&manifest(),Some("SITE-B")));}
    #[test] fn attack_06_signed_expired_rejected(){let mut p=manifest();p.issued_at=(Utc::now()-Duration::days(3)).to_rfc3339();p.expires_at=(Utc::now()-Duration::days(1)).to_rfc3339();assert!(rejected(&p,None));}
    #[test] fn attack_07_signed_future_clock_rejected(){let mut p=manifest();p.issued_at=(Utc::now()+Duration::hours(1)).to_rfc3339();p.expires_at=(Utc::now()+Duration::days(2)).to_rfc3339();assert!(rejected(&p,None));}
    #[test] fn attack_08_signed_over_30_days_rejected(){let mut p=manifest();p.expires_at=(Utc::now()+Duration::days(31)).to_rfc3339();assert!(rejected(&p,None));}
    #[test] fn attack_09_signed_under_1_day_rejected(){let mut p=manifest();p.expires_at=(Utc::now()+Duration::hours(12)).to_rfc3339();assert!(rejected(&p,None));}
    #[test] fn attack_10_signed_wrong_edition_rejected(){let mut p=manifest();p.target_edition="CENTRAL".into();assert!(rejected(&p,None));}
    #[test] fn attack_11_signed_wrong_environment_rejected(){let mut p=manifest();p.key_env="PRODUCTION".into();assert!(rejected(&p,None));}
    #[test] fn attack_12_signed_invalid_scope_rejected(){let mut p=manifest();p.users[0].scopes[0]=ProvisionScope{scope_type:"COMPANY".into(),company_id:None,department_id:None};assert!(rejected(&p,None));}
    #[test] fn attack_13_oversized_envelope_rejected(){let huge="x".repeat(MAX_PACKAGE_BYTES+1);assert!(decode_verified(&huge,None,Utc::now()).is_err());}
    #[test] fn attack_14_tampered_payload_with_original_signature_rejected(){
        let good=manifest();let env:ProvisionEnvelope=serde_json::from_str(&signed(&good)).unwrap();
        let mut payload=B64.decode(&env.payload_b64).unwrap();payload[0]^=1;
        let tampered=serde_json::to_string(&ProvisionEnvelope{payload_b64:B64.encode(payload),..env}).unwrap();
        assert!(decode_verified(&tampered,None,Utc::now()).is_err());
    }
    #[test] fn attacker_key_with_real_key_id_rejected(){
        let attacker=SigningKey::generate(&mut OsRng);let payload=serde_json::to_vec(&manifest()).unwrap();
        assert!(decode_verified(&envelope_for_payload(&payload,TEST_KEY_ID,&attacker),None,Utc::now()).is_err());
    }
    #[test] fn attacker_own_key_id_rejected(){
        let attacker=SigningKey::generate(&mut OsRng);let payload=serde_json::to_vec(&manifest()).unwrap();
        assert!(decode_verified(&envelope_for_payload(&payload,"attacker-key",&attacker),None,Utc::now()).is_err());
    }
    #[test] fn signed_unknown_password_field_rejected(){
        let mut v=serde_json::to_value(manifest()).unwrap();v["users"][0]["password"]=json!("plaintext");
        let payload=serde_json::to_vec(&v).unwrap();assert!(decode_verified(&envelope_for_payload(&payload,TEST_KEY_ID,&test_signing_key()),None,Utc::now()).is_err());
    }
    #[test] fn signed_unknown_manifest_field_rejected(){
        let mut v=serde_json::to_value(manifest()).unwrap();v["unexpected"]=json!(true);
        let payload=serde_json::to_vec(&v).unwrap();assert!(decode_verified(&envelope_for_payload(&payload,TEST_KEY_ID,&test_signing_key()),None,Utc::now()).is_err());
    }
    #[test] fn unknown_envelope_field_rejected(){
        let mut v:Value=serde_json::from_str(&signed(&manifest())).unwrap();v["public_key_b64"]=json!("attacker");
        assert!(decode_verified(&serde_json::to_string(&v).unwrap(),None,Utc::now()).is_err());
    }
    #[test] fn correctly_signed_test_envelope_is_accepted(){assert!(decode_verified(&signed(&manifest()),Some("SITE-A"),Utc::now()).is_ok());}
    #[cfg(feature="unit")]
    #[test] fn legacy_trusted_package_keys_cannot_supply_attacker_key(){
        let c=unit_db();let attacker=SigningKey::generate(&mut OsRng);
        let public=B64.encode(attacker.verifying_key().to_bytes());let attacker_id="attacker-db-key";
        c.execute("INSERT INTO trusted_package_keys(id,key_id,public_key,is_active,created_at) VALUES('evil',?1,?2,1,'now')",params![attacker_id,public]).unwrap();
        let payload=serde_json::to_vec(&manifest()).unwrap();
        let package=envelope_for_payload(&payload,attacker_id,&attacker);
        assert!(decode_verified(&package,None,Utc::now()).is_err());
    }


    fn migrated_db()->rusqlite::Connection{
        let c=rusqlite::Connection::open_in_memory().unwrap();
        for sql in [
            include_str!("../../database/migrations/0001_security.sql"),
            include_str!("../../database/migrations/0002_hr_domain.sql"),
            include_str!("../../database/migrations/0003_offline_distribution.sql"),
            include_str!("../../database/migrations/0004_package_signing.sql"),
            include_str!("../../database/migrations/0005_scope_constraints.sql"),
            include_str!("../../database/migrations/0006_central_signing_keys.sql"),
        ]{c.execute_batch(sql).unwrap();} c
    }
    #[cfg(feature="central")]
    fn seed_central()->rusqlite::Connection{
        let c=migrated_db();let h=hash();
        c.execute("INSERT INTO users(id,username,password_hash,display_name,is_active,created_at,updated_at) VALUES('admin','admin',?1,'Admin',1,'now','now'),('hr','hruser',?1,'HR',1,'now','now'),('viewer','viewer',?1,'Viewer',1,'now','now')",[&h]).unwrap();
        c.execute_batch("INSERT INTO roles(id,code,name) VALUES('ra','SYSTEM_ADMIN','SYSTEM_ADMIN'),('rh','CENTRAL_HR','CENTRAL_HR'),('rv','VIEWER','VIEWER');
          INSERT INTO permissions(id,code,name) VALUES('pu','user.manage','user.manage'),('pd','dashboard.view','dashboard.view');
          INSERT INTO role_permissions(role_id,permission_id) VALUES('ra','pu'),('rh','pu'),('rv','pd');
          INSERT INTO user_roles(user_id,role_id) VALUES('admin','ra'),('hr','rh'),('viewer','rv');
          INSERT INTO user_data_scopes(id,user_id,scope_type) VALUES('sv','viewer','GLOBAL');").unwrap();c
    }
    #[cfg(feature="central")]
    fn test_key_json(env:&str,private_override:Option<&str>)->String{
        let sk=test_signing_key();serde_json::json!({"key_id":TEST_KEY_ID,"env":env,
          "private_key_b64":private_override.unwrap_or(TEST_PRIVATE_B64),"public_key_b64":B64.encode(sk.verifying_key().to_bytes())}).to_string()
    }
    #[cfg(feature="central")]
    #[test] fn c1_export_without_key_rejected_and_does_not_generate_key(){
        let c=seed_central();let e=export_provision_core(&c,"admin","CENTRAL","SITE-A","Site A",vec!["viewer".into()],7).unwrap_err();
        assert!(e.contains("Signing Key"));let n:i64=c.query_row("SELECT COUNT(*) FROM central_signing_keys",[],|r|r.get(0)).unwrap();assert_eq!(n,0);
        let legacy:i64=c.query_row("SELECT COUNT(*) FROM package_signing_keys",[],|r|r.get(0)).unwrap();assert_eq!(legacy,0);
    }
    #[cfg(feature="central")]
    #[test] fn c2_non_system_admin_cannot_install_key(){let c=seed_central();assert!(install_signing_key_core(&c,"hr",&test_key_json("ALPHA",None)).is_err());}
    #[cfg(feature="central")]
    #[test] fn c3_untrusted_key_rejected(){
        let c=seed_central();let attacker=SigningKey::generate(&mut OsRng);let pb=B64.encode(attacker.verifying_key().to_bytes());
        let digest=Sha256::digest(attacker.verifying_key().to_bytes());let id=digest[..8].iter().map(|b|format!("{b:02x}")).collect::<String>();
        let k=json!({"key_id":id,"env":"ALPHA","private_key_b64":B64.encode(attacker.to_bytes()),"public_key_b64":pb}).to_string();
        assert!(install_signing_key_core(&c,"admin",&k).is_err());
    }
    #[cfg(feature="central")]
    #[test] fn c4_wrong_private_public_pair_rejected(){
        let c=seed_central();let attacker=SigningKey::generate(&mut OsRng);
        assert!(install_signing_key_core(&c,"admin",&test_key_json("ALPHA",Some(&B64.encode(attacker.to_bytes())))).is_err());
    }
    #[cfg(feature="central")]
    #[test] fn c5_production_key_rejected(){let c=seed_central();assert!(install_signing_key_core(&c,"admin",&test_key_json("PRODUCTION",None)).is_err());}
    #[cfg(feature="central")]
    #[test] fn c6_central_hr_export_rejected(){let c=seed_central();assert!(export_provision_core(&c,"hr","CENTRAL","SITE-A","Site A",vec!["viewer".into()],7).is_err());}
    #[cfg(feature="central")]
    #[test] fn c7_system_admin_cannot_be_exported(){
        let c=seed_central();install_signing_key_core(&c,"admin",&test_key_json("ALPHA",None)).unwrap();
        assert!(export_provision_core(&c,"admin","CENTRAL","SITE-A","Site A",vec!["admin".into()],7).is_err());
    }
    #[cfg(feature="central")]
    #[test] fn c8_valid_days_over_30_rejected(){let c=seed_central();assert!(export_provision_core(&c,"admin","CENTRAL","SITE-A","Site A",vec!["viewer".into()],31).is_err());}
    #[cfg(feature="central")]
    #[test] fn c9_valid_export_is_signed_deduped_and_contains_no_plaintext_password(){
        let c=seed_central();install_signing_key_core(&c,"admin",&test_key_json("ALPHA",None)).unwrap();
        let blob=export_provision_core(&c,"admin","CENTRAL","SITE-A","Site A",vec!["viewer".into(),"viewer".into()],7).unwrap();
        assert!(!blob.contains("alpha-test-password"));let (_,p)=decode_verified(&blob,Some("SITE-A"),Utc::now()).unwrap();assert_eq!(p.users.len(),1);
    }

    #[cfg(feature="unit")]
    fn unit_db()->rusqlite::Connection{migrated_db()}

    #[cfg(feature="unit")]
    #[test] fn import_signed_happy_path_persists_unit_identity_and_argon2_login_material(){
        use argon2::PasswordVerifier;
        let json=signed(&manifest());let (_,p)=decode_verified(&json,Some("SITE-A"),Utc::now()).unwrap();let mut c=unit_db();
        import_verified_manifest(&mut c,&p).unwrap();
        let edition:String=c.query_row("SELECT edition FROM app_identity WHERE singleton_id=1",[],|r|r.get(0)).unwrap();assert_eq!(edition,"UNIT");
        let stored:String=c.query_row("SELECT password_hash FROM users WHERE username='user-a'",[],|r|r.get(0)).unwrap();
        let parsed=PasswordHash::new(&stored).unwrap();assert!(Argon2::default().verify_password(b"alpha-test-password",&parsed).is_ok());
    }
    #[cfg(feature="unit")]
    #[test] fn duplicate_package_is_rejected(){
        let json=signed(&manifest());let (_,p)=decode_verified(&json,Some("SITE-A"),Utc::now()).unwrap();let mut c=unit_db();
        import_verified_manifest(&mut c,&p).unwrap();assert!(import_verified_manifest(&mut c,&p).is_err());
        let n:i64=c.query_row("SELECT COUNT(*) FROM package_imports",[],|r|r.get(0)).unwrap();assert_eq!(n,1);
    }
    #[cfg(feature="unit")]
    #[test] fn duplicate_username_rolls_back_identity(){
        let mut p=manifest();p.users[0].username="collision".into();let json=signed(&p);let (_,p)=decode_verified(&json,Some("SITE-A"),Utc::now()).unwrap();let mut c=unit_db();
        c.execute("INSERT INTO users(id,username,password_hash,display_name,is_active,created_at,updated_at) VALUES('existing','collision',?1,'Existing',1,'now','now')",[hash()]).unwrap();
        assert!(import_verified_manifest(&mut c,&p).is_err());
        let identities:i64=c.query_row("SELECT COUNT(*) FROM app_identity",[],|r|r.get(0)).unwrap();assert_eq!(identities,0);
        let imports:i64=c.query_row("SELECT COUNT(*) FROM package_imports",[],|r|r.get(0)).unwrap();assert_eq!(imports,0);
    }
}
