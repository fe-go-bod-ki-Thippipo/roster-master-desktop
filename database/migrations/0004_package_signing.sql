PRAGMA foreign_keys = ON;
CREATE TABLE IF NOT EXISTS package_signing_keys (
  id TEXT PRIMARY KEY,
  key_id TEXT NOT NULL UNIQUE,
  private_key_b64 TEXT NOT NULL,
  public_key_b64 TEXT NOT NULL,
  is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1)),
  created_at TEXT NOT NULL
);
