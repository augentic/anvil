# Expected trace — build success

Visible side effects of `/spec:build password-hash-rotate` on the success path:

1. `SPECIFY_PLAN_LOCK_HELD` is unset → the body drives the phase under `specify plan lock -- <cmd>` (single-repo), which takes `.specify/plan.lock` before any plan verb and exports `SPECIFY_PLAN_LOCK_HELD=1` to the child.
2. `specify plan next --format json` returns the entry `name: password-hash-rotate, target: omnia, status: in-progress` (the CLI's lock probe passes — the wrapper holds the lock).
3. `.specify/slices/password-hash-rotate/metadata.yaml` reads `status: refined` → proceed.
4. `specify slice build password-hash-rotate --phase prepare --format json` assembles + schema-validates the request and prints the handoff envelope; the body reads `adapters/targets/omnia/briefs/build.md` from its `build-brief` field.
5. The brief's verify-repair loop converges: `cargo fmt --check`, `cargo check`, `cargo clippy -- -D warnings`, `cargo test` all pass; `adapters/targets/omnia/briefs/build.md` § Code reviewer emits `REVIEW.md` with no critical / high findings outstanding. The brief writes `build/report.yaml` with `status: success`.
6. As each task completes the body marks the matching `tasks.md` checkbox via the brief's task-flip cadence.
7. `specify slice build password-hash-rotate --phase finalize --format json` is called exactly once. The CLI validates the report and stamps `metadata.yaml` to `built` (emitting `slice.build.started` / `slice.build.succeeded`).
8. The body returns; the `specify plan lock` child exits and the plan lock releases.

## Post-conditions

- `.specify/slices/password-hash-rotate/metadata.yaml` `status` is `built`.
- `plan.yaml.slices[0].status` is unchanged (`in-progress`); `/spec:merge` is the only writer that advances it to `done`.
- No stop hint emitted; the body's final visible output is the standard build summary.
