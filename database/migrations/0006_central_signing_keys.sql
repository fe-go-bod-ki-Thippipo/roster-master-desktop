PRAGMA foreign_keys = ON;

-- Alpha signing material is Central-only runtime state. The legacy
-- package_signing_keys table from 0004 is intentionally no longer used.
CREATE TABLE IF NOT EXISTS central_signing_keys (
  id TEXT PRIMARY KEY,
  key_id TEXT NOT NULL UNIQUE,
  key_env TEXT NOT NULL CHECK (key_env IN ('ALPHA','PRODUCTION')),
  private_key_b64 TEXT NOT NULL,
  public_key_b64 TEXT NOT NULL,
  installed_at TEXT NOT NULL,
  installed_by_user_id TEXT NOT NULL,
  is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1)),
  FOREIGN KEY (installed_by_user_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_central_signing_keys_active
  ON central_signing_keys(is_active, key_env, installed_at);
