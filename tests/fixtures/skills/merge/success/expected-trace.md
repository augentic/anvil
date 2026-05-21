# Expected trace — merge success

Visible side effects of `/spec:merge password-hash-rotate` on the success path:

1. `specify plan next --format json` returns the entry `name: password-hash-rotate, target: omnia, status: in-progress`.
2. `SPECIFY_PLAN_LOCK_HELD` is unset → the body acquires `.specify/plan.lock` (single-repo) via `flock(LOCK_EX | LOCK_NB)` on fd 9.
3. `.specify/slices/password-hash-rotate/.metadata.yaml` reads `status: built` → proceed.
4. `specify target resolve omnia --format json` returns the resolved manifest path; the body reads `targets/omnia/briefs/merge.md` from there.
5. The brief's pre-merge gate runs and passes: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo check --workspace`, `cargo test`, `cargo build --target wasm32-wasip2 --release --workspace` all green.
6. AskQuestion confirmation prompts the operator (interactive path) and the operator selects "Proceed".
7. `specify slice merge password-hash-rotate --format json` is called exactly once. The CLI atomically:
   - writes the merged baseline under `.specify/specs/omnia/spec.md`,
   - transitions `.metadata.yaml.status` to `merged` and stamps `merged-at`,
   - moves `.specify/slices/password-hash-rotate/` into `.specify/archive/2026-05-21-password-hash-rotate/`,
   - writes `plan.yaml.slices[0].status = done` (the only place per-entry `done` is produced).
8. The body renders the post-merge summary (archive path + merged spec list).
9. The body returns; fd 9 closes; the plan lock releases.

## Post-conditions

- `.specify/slices/password-hash-rotate/` no longer exists; it lives at `.specify/archive/2026-05-21-password-hash-rotate/`.
- `plan.yaml.slices[0].status` is `done`.
- The omnia target has no post-merge validator hook, so step 7 of the SKILL Critical Path is a no-op for this fixture; the contracts target is the canonical post-merge-validator surface.
