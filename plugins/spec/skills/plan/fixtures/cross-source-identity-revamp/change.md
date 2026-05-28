# Change — identity-revamp

## Intent

Reconstruct the identity surface against the latest design notes while preserving the behavior the legacy monolith encodes today.

## Scope

Two slices, both fused by `propose` from a documentation source and a TypeScript legacy source. The propose sub-step matched `user-registration` cleanly across both sources; the `password-reset` row pairs the documentation lead with the legacy lead `account-pwd-reset` after a tentative match on intent.

## Tentative merges

`password-reset` — the documentation lead `password-reset` and the legacy lead `account-pwd-reset` describe the same operator-visible flow under different names. Both surfaced from the same product area but the legacy lead predates the rename in the design notes. Operator review at Gate 1 should confirm or split via `specrun plan amend`.
