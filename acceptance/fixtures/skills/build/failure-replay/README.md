# `build/failure-replay/`

Pins the build-failure stop hint contract from [`plugins/spec/skills/build/SKILL.md`](../../../../../plugins/spec/skills/build/SKILL.md) §Stop hint contract. Stress-tests workflow §Acceptance scenario `#9` from the `/spec:build`-only angle (the loop-side variant lives at [`acceptance/fixtures/skills/execute/09-build-failure-recovery/`](../../execute/09-build-failure-recovery/)).

## Scenario

`/spec:build session-cookie-harden` runs against a slice already at `status: refined`. The omnia target build brief reaches the verify-repair loop's `cargo test` step on iteration 1; one regression test (`session_cookie_secure_flag_set`) fails because the production handler dropped `.secure(true)`.

The skill body MUST:

1. Not call `specify slice transition session-cookie-harden built` — the slice stays at `refined`.
2. Not write to `plan.yaml` — the plan entry stays `in-progress`.
3. Emit the structured stop hint with `failing-task: cargo test` (or the more specific test name when available) and `log-path` pointing at the brief's captured stderr.
4. Release the plan lock on exit (the `flock`-bound fd 9 closes when the body returns).
