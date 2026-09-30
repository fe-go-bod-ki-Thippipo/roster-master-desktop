# Roster Master Desktop Architecture

## Objective
Build a Windows desktop HR workforce application that remains usable without internet and can later synchronize with a central service.

## Initial stack
- Desktop shell: Tauri
- Frontend: React + TypeScript
- Local database: SQLite
- Data access: Rust commands / repository layer
- Authentication: local credential verification for offline use
- Authorization: RBAC + data-scope rules
- Audit: append-only audit log
- Packaging: Windows installer / executable

## Runtime model
```text
Desktop UI
  |
  v
Application Services
  |---- Auth / Session
  |---- Permission Engine
  |---- HR Domain Services
  |---- Import / Export
  |
  v
SQLite
  |
  +---- Users / Roles / Permissions
  +---- Companies / Org Structure
  +---- Employees / Positions / Assignments
  +---- Audit Log
  +---- Sync Metadata (future)
```

## Offline-first rules
1. Core HR functions must work without internet.
2. Authentication must support authorized offline sign-in.
3. Permissions must be cached locally and enforced by the application layer, not only hidden in the UI.
4. Future online sync must never be required to open the local application.
5. AI features are optional online capabilities and must not block HR operations.

## Migration source
Roster Master v5.55 is the reference for existing UI behavior and proven business logic. Migration will be incremental; the HTML prototype will not be embedded as the permanent production architecture.

## Data-scope decision (PO confirmed)
Employee visibility uses the mixed rule: the home company can always see its employee; a company with an active assignment can see the employee within the assignment-related scope. An assignment does not change the employee's home company. Backend authorization must enforce this rule independently of UI filtering.
