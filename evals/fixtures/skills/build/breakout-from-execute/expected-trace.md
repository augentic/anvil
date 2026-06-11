# Expected trace — breakout from `/spec:execute`

Visible side effects of `/spec:build session-cookie-harden` invoked under `SPECIFY_PLAN_LOCK_HELD=1`:

1. The body reads `$SPECIFY_PLAN_LOCK_HELD` and matches `"1"`. It logs (or no-ops on) "plan lock held by parent (/spec:execute); skipping acquire".
2. **No** `flock(LOCK_EX | LOCK_NB)` call is made; **no** new fd is opened against `.specify/plan.lock`. The body never touches the lockfile on this path.
3. `specify plan next --format json` is called as normal (the parent loop already wrote the entry's `in-progress` status).
4. The remainder of the body runs identically to the standalone path: workspace routing if `workspace: true`, slice-lifecycle refusal check, target build brief execution.
5. Sub-invocations spawned by the brief inherit `SPECIFY_PLAN_LOCK_HELD=1` from the process environment.
6. On success, `specify slice build session-cookie-harden --phase finalize --format json` runs — finalize validates the report and stamps `built` — and the body returns. On failure, the structured stop hint is emitted (see [`../failure-replay/expected-stop-hint.md`](../failure-replay/expected-stop-hint.md)).

## Negative assertion

Removing the env var (running the same input with `SPECIFY_PLAN_LOCK_HELD` unset) MUST cause the body to acquire the lock at step 2; that is the standalone-breakout contract covered by [`../failure-replay/`](../failure-replay/) and [`../success/`](../success/).
