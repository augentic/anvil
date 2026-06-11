# Workspace routing

Plan artifacts (`plan.yaml`, `change.md`, `discovery.md`, `.specify/plan.lock`) live at the workspace. Each project's slot lives at `.specify/workspace/<project>/` and carries its own `.specify/slices/<name>/` tree. `/spec:execute` (and breakouts) share one uniform routing rule: lock at the workspace, resolve the active slice's `project`, `chdir` into the slot for phase work, return for the next plan write.

This is the file companion to the `## Workspace routing` H2 in [`../SKILL.md`](../SKILL.md). The single-repo path skips every step here: when the active plan entry has no `project` field, phase work runs in the project root and no `workspace sync` or `chdir` happens.

## Per-iteration routing

For every iteration of the loop where `entry.project` is non-null:

1. **Save CWD.** The initiating directory is the workspace; `specify plan next`, `specify plan transition`, and the plan lock all live there. Restore to it at the end of the iteration.
2. **Resolve `entry.project` through `registry.yaml`.** Same selector preflight as every `specify workspace *` verb. Unknown names halt before any filesystem, Git, or phase side-effect.
3. **Materialise the slot if missing.** Run `specify workspace sync <project>` only for the active project — do not broad-sync. Re-check the slot's `.specify/project.yaml` exists before continuing.
4. **Prepare the branch.** Run `specify workspace prepare <project> --change <change-name>` (the helper resolves `origin/HEAD`, creates or reuses `specify/<change-name>`, and classifies dirty work against the active slice boundary).
5. **`chdir` into the slot.** Remember the returned `slot_path`; phase work runs from there. Emit `Routing: <slice> → <project> (<slot_path>)`.
6. **Export the plan root.** Set `SPECIFY_PLAN_DIR=<workspace-root>` for the duration of slot-side phase work (or pass `--plan-dir <workspace-root>` on each `specify` invocation). No slot grows its own `plan.yaml`; the override is how slot-side plan readers (`source extract`, `slice synthesize`, `slice validate`, `slice provenance`) and `slice merge`'s `done` stamp resolve the workspace's plan, and how relative `sources.<key>.path` bindings keep resolving against the workspace. Unset it on CWD restore. Source adapters still resolve slot-locally (vendored `adapters/` tree or manifest cache) — populate the slot's manifest cache when the adapter lives only at the workspace.
7. **Run the phase sequence.** `/spec:refine` → `/spec:build` → `/spec:merge` operate against the slot's `.specify/slices/<name>/` tree.
8. **Residue commit on merge success.** `specify slice merge run` commits only `.specify/specs/` and `.specify/archive/` as `specify: merge <slice>` (run with the plan root exported, it also stamps the entry `done` in the workspace plan — `slice merge` stays the sole writer of `done`). Stage every other dirty path under the slot and commit as `specify: residue <slice>`. Halt with `baseline-residue-after-merge` if either baseline tree is dirty after merge success; halt with `residue-commit-failed` if `git commit` returns non-zero.
9. **Restore CWD.** `chdir` back to the saved workspace (and unset `SPECIFY_PLAN_DIR`) before the next `specify plan next` so plan writes resolve against the correct `plan.yaml`.

## Branch-preparation failures

`specify workspace prepare` failures are pre-phase failures: never run a phase skill against an unprepared slot, never call `/spec:drop`, never transition the plan entry. Halt with the helper's diagnostic key (`origin-head-unresolved`, `dirty-unrelated-tracked`, `dirty-branch-mismatch`, `workspace-slot-missing`, `origin-mismatch`, `branch-pattern-mismatch`, `git-operation-failed`) and release the lock. The entry stays `pending` (fresh run) or `in-progress` (re-entry); the operator triages.

## Breakout routing

`/spec:refine`, `/spec:build`, and `/spec:merge` invoked standalone share the same routing. Their skill bodies:

1. Acquire `.specify/plan.lock` via the snippet in [`../../../references/plan-lock.md`](../../../references/plan-lock.md).
2. Resolve the active `in-progress` entry via `specify plan next`.
3. If `entry.project` is non-null, repeat steps 2–6 of the per-iteration routing above (including the `SPECIFY_PLAN_DIR` export) before invoking phase work.
4. Run the single phase the operator asked for.
5. Restore CWD (and unset `SPECIFY_PLAN_DIR`) before exit; release the lock on the trailing edge of the snippet.

The plan lock at the workspace is what guarantees that an operator running `/spec:build` from the workspace cannot race a background `/spec:execute` (or a sibling operator running `/spec:merge`) on the same plan. Scenario #11 in [`../../../../../evals/fixtures/skills/execute/`](../../../../../evals/fixtures/skills/execute/) pins the breakout-after-build-failure path: `/spec:execute` parks on `auth-rotate` in `project-a`, releases the lock, and the operator runs `/spec:build` from the workspace — which re-acquires the lock, resolves `auth-rotate → project-a`, `chdir`s into the slot, and resumes the failing task.

## CWD restore in the loop

Every iteration brackets the `chdir` with a save/restore. The closure is intentionally conservative: even an iteration that doesn't change CWD (single-repo entry, or workspace entry whose `project` resolution failed before step 5) still runs the restore. This keeps `specify plan next`, `specify plan transition`, and the trailing-edge lock release on the trailing iteration anchored at the workspace.
