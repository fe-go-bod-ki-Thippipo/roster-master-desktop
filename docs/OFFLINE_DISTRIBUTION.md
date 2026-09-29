# Offline Central / Unit Distribution

Roster Master uses one codebase with two locked editions.

## Central Edition
- Initializes the Central site once.
- May create the first SYSTEM_ADMIN.
- Owns user, role, permission and data-scope administration.
- Will export signed .rmpkg packages for Unit sites.
- Keeps the package signing private key only on Central.

## Unit Edition
- Cannot create SYSTEM_ADMIN locally.
- Starts unprovisioned and must import a package issued by Central.
- Receives site identity, users (password hashes only), roles, permissions, scopes and authorized HR data.
- Verifies the Central digital signature before import.
- Cannot elevate permissions beyond the imported site/user scope.
- Will export local changes and audit records back to Central as a package.

## Package security
A package must contain a manifest with package ID, schema version, source site, target site, issued timestamp, payload hash and signing-key ID. The Central private key never ships in a Unit package. Unit clients hold only trusted public verification keys. Any changed payload or manifest must fail verification.

## Bootstrap rule
An empty database is not automatically an administrator. Central initialization must happen before first-admin creation. Unit provisioning is a separate import flow and never calls create_first_admin.
