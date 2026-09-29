# Initial Local Data Model

This is the bootstrap schema for the desktop application. HR domain tables will be refined before production migration.

## Security tables

### users
- id
- username
- password_hash
- display_name
- is_active
- last_login_at
- created_at
- updated_at

### roles
- id
- code
- name

### permissions
- id
- code
- name

### user_roles
- user_id
- role_id

### role_permissions
- role_id
- permission_id

### user_data_scopes
- id
- user_id
- scope_type
- company_id
- department_id

## Audit table

### audit_logs
- id
- user_id
- action
- entity_type
- entity_id
- before_json
- after_json
- occurred_at
- device_id

## HR domain direction
The desktop schema will preserve separate entities for:
- company groups
- companies
- departments / organization hierarchy
- positions
- employees
- employee assignments
- employment types

Important existing workforce rules (HC/FTE, non-overlapping assignments, immutable employee identifiers, etc.) will be migrated explicitly rather than inferred from the current UI.
