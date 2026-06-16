# Expected trace — breakout from `/spec:execute`

Visible side effects of `/spec:build session-cookie-harden` invoked under `SPECIFY_PLAN_LOCK_HELD=1`:

1. The body drives its phase under `specify plan lock -- <cmd>`. The wrapper sees `SPECIFY_PLAN_LOCK_HELD=1` in the environment and skips re-acquisition.
2. **No** new acquisition against `.specify/plan.lock` is attempted; the wrapper neither re-takes nor releases the parent's lock, and never reports `plan-lock-busy`.
3. `specify plan next --format json` is called as normal (the parent loop already wrote the entry's `in-progress` status; its lock probe passes against the parent-held lock).
4. The remainder of the body runs identically to the standalone path: workspace routing if `workspace: true`, slice-lifecycle refusal check, target build brief execution.
5. Sub-invocations spawned by the brief inherit `SPECIFY_PLAN_LOCK_HELD=1` from the process environment.
6. On success, `specify slice build session-cookie-harden --phase finalize --format json` runs — finalize validates the report and stamps `built` — and the body returns. On failure, the structured stop hint is emitted (see [`../failure-replay/expected-stop-hint.md`](../failure-replay/expected-stop-hint.md)).

## Negative assertion

Removing the env var (running the same input with `SPECIFY_PLAN_LOCK_HELD` unset) MUST cause the wrapper to acquire the lock at step 1; that is the standalone-breakout contract covered by [`../failure-replay/`](../failure-replay/) and [`../success/`](../success/).
