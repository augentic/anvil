# `merge/success/`

Pins the happy-path output of `/spec:merge` against an `omnia` slice and the per-entry `done` write that only `specrun slice merge` produces.

## Scenario

`/spec:merge password-hash-rotate` runs against a slice already at `status: built` (after the matching `build/success/` fixture). The omnia pre-merge gate (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo check --workspace`, `cargo test`, `cargo build --target wasm32-wasip2 --release --workspace`) passes. The CLI delta-merge applies cleanly — no baseline overlap with sibling slices.

The skill body MUST:

1. Resolve the slice via `specrun plan next` (or validate the supplied `[slice-name]` arg matches it).
2. Acquire the plan lock when invoked standalone (`SPECIFY_PLAN_LOCK_HELD` unset).
3. Read and execute `adapters/targets/omnia/briefs/merge.md`'s pre-merge gate.
4. Run the AskQuestion confirmation when interactive (skip when `SPECIFY_PLAN_LOCK_HELD=1`).
5. Run `specrun slice merge password-hash-rotate --format json` exactly once. The CLI atomically:
   - applies the delta merge against `.specify/specs/`,
   - transitions `.metadata.yaml.status` to `merged`,
   - moves `.specify/slices/password-hash-rotate/` into `.specify/archive/YYYY-MM-DD-password-hash-rotate/`,
   - stamps `plan.yaml.slices[<slice>].status = done` (the sole writer of per-entry `done`).
6. Return the merge summary (archive path + merged spec list) to the caller.
