# Change — account-revamp

## Intent

Refresh the account service against the latest design notes: registration, password reset, and the operator-visible audit log.

## Scope

Three slices, all driven by `docs=./design-notes/account`. No legacy code source; clean propose without divergence.

## Notes

Scaffolded by `/spec:plan account-revamp source docs=./design-notes/account`. No tentative merges; no likely divergences. Operator reviews and stamps Gate 1 with `specrun plan transition account-revamp reviewed`.
