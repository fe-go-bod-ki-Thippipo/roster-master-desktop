PRAGMA foreign_keys = ON;
CREATE UNIQUE INDEX IF NOT EXISTS ux_user_scope_exact ON user_data_scopes(user_id,scope_type,COALESCE(company_id,''),COALESCE(department_id,''));
CREATE INDEX IF NOT EXISTS idx_assignments_active_scope ON assignments(employee_id,effective_from,effective_to,is_cancelled);
CREATE INDEX IF NOT EXISTS idx_positions_scope_org ON positions(org_unit_id,is_active);
