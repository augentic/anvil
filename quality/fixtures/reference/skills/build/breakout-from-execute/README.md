# `build/breakout-from-execute/`

Pins the plan-lock re-entrancy contract from [`plugins/spec/references/plan-lock.md`](../../../../../plugins/spec/references/plan-lock.md). Stress-tests workflow §Eval scenario `#11` (workspace breakout after build failure) at the env-var-detection layer — the same logic applies whether the parent is single-repo `/spec:execute` or workspace `/spec:execute`.

## Scenario

`/spec:execute` is mid-loop, holds `.specify/plan.lock` via its own `specify plan lock -- <cmd>` wrapper, and runs `/spec:build` as a descendant — so `SPECIFY_PLAN_LOCK_HELD=1` is already in the environment. Re-entrancy is CLI-owned: a nested `specify plan lock -- <cmd>` sees the env var, skips re-acquisition, and runs the child directly rather than failing `plan-lock-busy` or deadlocking on the lock the parent already holds.

The contract is:

1. The build body still drives its phase under `specify plan lock -- <cmd>` (it does not branch on the env var itself for lock acquisition).
2. Because `SPECIFY_PLAN_LOCK_HELD=1` is inherited from the parent, the nested wrapper skips re-acquisition — it neither re-takes nor releases the parent's lock, and never reports `plan-lock-busy`.
3. The env var stays in the environment for any further descendants (brief-loaded child shells, the verify-repair loop sub-shells, `specify` invocations).
4. Otherwise the body behaves identically to the standalone path — same slice resolution, same brief execution, same success/failure transitions.

The same contract applies symmetrically to `/spec:merge` invoked under `SPECIFY_PLAN_LOCK_HELD=1` from the loop. See [`evals/fixtures/skills/merge/`](../../merge/) for the merge-side success/failure fixtures.
