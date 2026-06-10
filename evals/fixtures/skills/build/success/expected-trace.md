# Expected trace — build success

Visible side effects of `/spec:build password-hash-rotate` on the success path:

1. `specify plan next --format json` returns the entry `name: password-hash-rotate, target: omnia, status: in-progress`.
2. `SPECIFY_PLAN_LOCK_HELD` is unset → the body acquires `.specify/plan.lock` (single-repo) via `flock(LOCK_EX | LOCK_NB)` on fd 9.
3. `.specify/slices/password-hash-rotate/metadata.yaml` reads `status: refined` → proceed.
4. `specify target resolve omnia --format json` returns the resolved manifest path; the body reads `adapters/targets/omnia/briefs/build.md` from there.
5. The brief's verify-repair loop converges: `cargo fmt --check`, `cargo check`, `cargo clippy -- -D warnings`, `cargo test` all pass; `adapters/targets/omnia/briefs/build.md` § Code reviewer emits `REVIEW.md` with no critical / high findings outstanding.
6. As each task completes the body marks the matching `tasks.md` checkbox via the brief's task-flip cadence.
7. `specify slice transition password-hash-rotate built --format json` is called exactly once. The CLI stamps `metadata.yaml`.
8. The body returns; fd 9 closes; the plan lock releases.

## Post-conditions

- `.specify/slices/password-hash-rotate/metadata.yaml` `status` is `built`.
- `plan.yaml.slices[0].status` is unchanged (`in-progress`); `/spec:merge` is the only writer that advances it to `done`.
- No stop hint emitted; the body's final visible output is the standard build summary.
