# Roster Master v5.55 Migration Map

This document is derived from the v5.55 HTML prototype and records what the desktop version must preserve.

| v5.55 | Desktop |
|---|---|
| settings.companyGroups | company_groups |
| settings.companies | companies |
| settings.groups / sections / departments | org_units hierarchy |
| positions | positions |
| employees | employees |
| assignments | assignments |
| localStorage | SQLite |

## Confirmed v5.55 behavior
- Employee, Position, Department and Assignment are separate collections.
- Department codes are Dxxx and position codes are Pxxx in the prototype.
- A position has target HC and an active/inactive state.
- Employee assignment distinguishes primary and secondary positions.
- Current employee position prefers the primary assignment.
- v5.55 FTE assumes one employee = 1.0 total FTE and divides that equally by the employee's assignment count when no explicit allocation exists.
- Dashboard and workforce calculations derive HC/FTE from assignments.

## Desktop improvement without changing user-facing intent
The HTML prototype relates assignments to positions/departments by names. Desktop stores stable IDs and foreign keys so renaming an organizational unit or position does not break assignment history.

## FTE compatibility
During migration:
1. If an assignment has an explicit FTE, use it.
2. Otherwise preserve v5.55 behavior: 1 / active assignment count for that employee.
3. A later validation layer will enforce total effective FTE <= 1.00 per employee.
