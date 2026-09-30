PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS app_identity (
  singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
  edition TEXT NOT NULL CHECK (edition IN ('CENTRAL','UNIT')),
  site_code TEXT,
  site_name TEXT,
  initialized_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS package_imports (
  id TEXT PRIMARY KEY,
  package_id TEXT NOT NULL UNIQUE,
  source_site_code TEXT NOT NULL,
  target_site_code TEXT NOT NULL,
  package_type TEXT NOT NULL,
  manifest_json TEXT NOT NULL,
  imported_at TEXT NOT NULL,
  imported_by_user_id TEXT,
  FOREIGN KEY (imported_by_user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS trusted_package_keys (
  id TEXT PRIMARY KEY,
  key_id TEXT NOT NULL UNIQUE,
  public_key TEXT NOT NULL,
  is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1)),
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_package_imports_target ON package_imports(target_site_code, imported_at);
