# `build/breakout-from-execute/`

Pins the plan-lock re-entrancy contract from [`plugins/spec/references/plan-lock.md`](../../../../../plugins/spec/references/plan-lock.md). Stress-tests workflow §Acceptance scenario `#11` (workspace breakout after build failure) at the env-var-detection layer — the same logic applies whether the parent is single-repo `/spec:execute` or workspace `/spec:execute`.

## Scenario

`/spec:execute` is mid-loop, has already acquired `.specify/plan.lock` on `flock(LOCK_EX | LOCK_NB)`, and exports `SPECIFY_PLAN_LOCK_HELD=1` before invoking `/spec:build`. The build skill body MUST detect the env var and skip lock acquisition; otherwise re-entrant `flock(LOCK_EX | LOCK_NB)` would error and abort the loop.

The skill body MUST:

1. Read `$SPECIFY_PLAN_LOCK_HELD`. When equal to `"1"`, log "plan lock held by parent (/spec:execute); skipping acquire" and proceed to slice resolution.
2. Not open fd 9 against `.specify/plan.lock`. Re-entrant acquisition aborts the loop.
3. Inherit `SPECIFY_PLAN_LOCK_HELD=1` to any sub-invocations it spawns (the same brief-loaded child shells, the verify-repair loop sub-shells, `specify` invocations).
4. Otherwise behave identically to the standalone path — same slice resolution, same brief execution, same success/failure transitions.

The same contract applies symmetrically to `/spec:merge` invoked under `SPECIFY_PLAN_LOCK_HELD=1` from the loop. See [`acceptance/examples/skills/merge/`](../../merge/) for the merge-side success/failure fixtures.
