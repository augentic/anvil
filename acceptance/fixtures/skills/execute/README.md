# `/spec:execute` worked-example fixtures

Static fixtures pinning the four `/spec:execute` acceptance scenarios (see the [lifecycle scenario catalog](../../../lifecycle/README.md) and [`docs/reference/lifecycle.md`](../../../../docs/reference/lifecycle.md)):

| Scenario | Fixture directory | What it pins |
|---|---|---|
| #8 — step-through breakout mid-execute | [`08-breakout-mid-execute/`](08-breakout-mid-execute/) | Operator cancels execute on slice 2, runs `/spec:build` directly, re-invokes `/spec:execute`; the loop resumes on slice 2 without flags. |
| #9 — build failure park + recovery | [`09-build-failure-recovery/`](09-build-failure-recovery/) | A slice's `cargo test` fails; `/spec:execute` stops with task id + log; operator patches; re-runs `/spec:execute` and the loop resumes from the failed task. |
| #10 — workspace across two projects | [`10-workspace-two-projects/`](10-workspace-two-projects/) | Plan with slices targeting `project-a` and `project-b`; `/spec:execute` materialises each slot, prepares the branch, runs the phase sequence, commits residue, restores CWD, repeats. |
| #11 — workspace breakout after build failure | [`11-workspace-breakout-after-build/`](11-workspace-breakout-after-build/) | `/spec:execute` parks on `auth-rotate` in `project-a`; operator runs `/spec:build` from the workspace; the breakout resolves the active slice's project and `chdir`s into the slot without operator intervention. |

Each fixture directory contains:

- `input/plan.yaml` — the plan as it would exist on disk when `/spec:execute` runs. Always carries `lifecycle: approved`; per-entry `status` mirrors the scenario's starting state.
- `expected.md` — narrative of the active slice, the stop reason (or drained exit), and the operator's next command.

The fixtures are shape-only: they pin what the skill body must produce given a known plan state. The CLI-side acceptance harness replays them against the current lifecycle contract once `/spec:refine`, `/spec:build` + `/spec:merge`, and `/spec:finalize` have canonical envelopes.
