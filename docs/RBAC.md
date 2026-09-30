# Authentication and Authorization

## Access model
Authorization uses two dimensions:

1. **Role permission** — what actions the account may perform.
2. **Data scope** — which company / organizational records the account may access.

## Initial roles
| Role | Intended scope |
|---|---|
| SYSTEM_ADMIN | System configuration, users, roles, all HR data |
| CENTRAL_HR | HR operations across all authorized companies |
| COMPANY_HR | HR operations for assigned company scope |
| MANAGER | View / limited actions for assigned organizational scope |
| STAFF | Assigned operational functions only |
| VIEWER | Read-only dashboards and reports |

Roles are templates. Final permission checks must use explicit permissions rather than hard-coded role names.

## Initial permissions
- dashboard.view
- employee.view
- employee.create
- employee.update
- employee.terminate
- position.view
- position.manage
- department.view
- department.manage
- assignment.view
- assignment.manage
- import.employee
- export.employee
- export.dashboard
- user.manage
- role.manage
- audit.view
- settings.manage

## Data scope
A user may receive one or more scopes:
- GLOBAL
- COMPANY
- DEPARTMENT

Future versions may add finer org scopes.

## Security rules
- Store password hashes only; never store plaintext passwords.
- Session permissions are evaluated by the backend/application layer.
- UI visibility is convenience only and is not an authorization boundary.
- Sensitive write operations create audit events.
- Disabled users cannot authenticate.
- Permission changes invalidate or refresh cached authorization state.
