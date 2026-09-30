PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS company_groups (
  id TEXT PRIMARY KEY,
  code TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1))
);

CREATE TABLE IF NOT EXISTS companies (
  id TEXT PRIMARY KEY,
  code TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  company_group_id TEXT,
  is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1)),
  FOREIGN KEY (company_group_id) REFERENCES company_groups(id)
);

CREATE TABLE IF NOT EXISTS org_units (
  id TEXT PRIMARY KEY,
  code TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  company_id TEXT NOT NULL,
  parent_id TEXT,
  unit_type TEXT NOT NULL CHECK (unit_type IN ('SECTION','DEPARTMENT','UNIT')),
  sort_order INTEGER NOT NULL DEFAULT 999,
  is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1)),
  FOREIGN KEY (company_id) REFERENCES companies(id),
  FOREIGN KEY (parent_id) REFERENCES org_units(id)
);

CREATE TABLE IF NOT EXISTS employment_types (
  id TEXT PRIMARY KEY,
  code TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL UNIQUE,
  is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1))
);

CREATE TABLE IF NOT EXISTS positions (
  id TEXT PRIMARY KEY,
  code TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  org_unit_id TEXT NOT NULL,
  parent_position_id TEXT,
  grade INTEGER,
  target_hc REAL NOT NULL DEFAULT 1 CHECK (target_hc >= 0),
  employment_type_id TEXT,
  sort_order INTEGER NOT NULL DEFAULT 999,
  is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0,1)),
  FOREIGN KEY (org_unit_id) REFERENCES org_units(id),
  FOREIGN KEY (parent_position_id) REFERENCES positions(id),
  FOREIGN KEY (employment_type_id) REFERENCES employment_types(id)
);

CREATE TABLE IF NOT EXISTS employees (
  id TEXT PRIMARY KEY,
  employee_code TEXT NOT NULL UNIQUE,
  prefix TEXT,
  full_name TEXT NOT NULL,
  nickname TEXT,
  gender TEXT,
  birth_date TEXT,
  national_id TEXT,
  nationality TEXT,
  education TEXT,
  home_company_id TEXT,
  employment_type_id TEXT,
  phone TEXT,
  email TEXT,
  address TEXT,
  emergency_contact TEXT,
  emergency_phone TEXT,
  hire_date TEXT,
  probation_end_date TEXT,
  termination_date TEXT,
  termination_reason TEXT,
  status TEXT NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('ACTIVE','INACTIVE')),
  FOREIGN KEY (home_company_id) REFERENCES companies(id),
  FOREIGN KEY (employment_type_id) REFERENCES employment_types(id)
);

CREATE TABLE IF NOT EXISTS assignments (
  id TEXT PRIMARY KEY,
  employee_id TEXT NOT NULL,
  position_id TEXT NOT NULL,
  role_type TEXT NOT NULL CHECK (role_type IN ('PRIMARY','SECONDARY','COORDINATION')),
  fte REAL,
  effective_from TEXT,
  effective_to TEXT,
  is_cancelled INTEGER NOT NULL DEFAULT 0 CHECK (is_cancelled IN (0,1)),
  FOREIGN KEY (employee_id) REFERENCES employees(id),
  FOREIGN KEY (position_id) REFERENCES positions(id),
  CHECK (fte IS NULL OR (fte > 0 AND fte <= 1.0)),
  CHECK (effective_to IS NULL OR effective_from IS NULL OR effective_to >= effective_from)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_assignment_primary_open
ON assignments(employee_id)
WHERE role_type='PRIMARY' AND effective_to IS NULL AND is_cancelled=0;

CREATE INDEX IF NOT EXISTS idx_org_units_company ON org_units(company_id, parent_id);
CREATE INDEX IF NOT EXISTS idx_positions_org_unit ON positions(org_unit_id);
CREATE INDEX IF NOT EXISTS idx_employees_company ON employees(home_company_id, status);
CREATE INDEX IF NOT EXISTS idx_assignments_employee ON assignments(employee_id, effective_from, effective_to);
CREATE INDEX IF NOT EXISTS idx_assignments_position ON assignments(position_id, effective_from, effective_to);
