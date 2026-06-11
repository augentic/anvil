# #11 — Workspace breakout after build failure in a slot

Pins the cross-cutting workspace + breakout contract: `/spec:execute` parks on a build failure in `project-a`; the operator stays at the workspace and runs `/spec:build` standalone; the breakout body resolves the active slice's project and `chdir`s into the slot without operator intervention.

## Starting state

- `plan.yaml.lifecycle == approved`; `workspace: true`.
- `auth-rotate` is already `in-progress` from a prior `/spec:execute` pass; slice lifecycle is `refined` (refine landed before the build-failure park).
- `audit-shipper-rotate` is still `pending`.
- `.specify/workspace/project-a/` exists (materialised + branch prepared during the prior pass); `.specify/workspace/project-b/` is empty.
- A prior `/spec:execute` pass produced `stop: build-failed` for slice 1 task `task-7` and exited; the lock is released.

## Trace

1. **Operator runs `/spec:build` from the workspace.**
   - The breakout body's first action is to acquire `.specify/plan.lock` via the same snippet `/spec:execute` uses ([`../../../../../plugins/spec/references/plan-lock.md`](../../../../../plugins/spec/references/plan-lock.md)). Lock acquired at the workspace (not in any slot).
   - `specify plan next` returns slice 1 (`auth-rotate`, `in-progress`, `project: project-a`).
   - Workspace routing rule kicks in identically to the loop: save CWD = workspace; resolve `project-a` through `registry.yaml`; slot already materialised; `chdir` into `.specify/workspace/project-a/`; export `SPECIFY_PLAN_DIR=<workspace-root>`. Emit `Routing: auth-rotate → project-a (.specify/workspace/project-a/)`.
   - `/spec:build` resumes from task 7 (the failing one); operator's patch landed before the breakout; build passes; slice lifecycle transitions to `built`.
   - `/spec:build` records `PhaseOutcome { phase: build, outcome: success }`.
   - `chdir` back to workspace.
   - Plan entry stays `in-progress` (build does not write `done`; only `/spec:merge` does).
   - Lock released on shell exit.

2. **Operator runs `/spec:execute`.**
   - Lock acquired.
   - `specify plan next` returns slice 1 (still `in-progress`).
   - Slice lifecycle is `built` → loop skips `/spec:refine` and `/spec:build`; dispatches `/spec:merge`.
   - Merge succeeds; baseline + residue commits land in the slot; `specify slice merge run` stamps the entry `done` in the workspace plan through the exported plan root.
   - Next iteration: slice 2 routes into `project-b` (slot materialised); phase sequence runs end-to-end; `done`.
   - Next iteration: drained → closing hint `drained — run /spec:finalize identity-rotation`.

## Terminal state

- Both per-entry `status: done`.
- Two slots materialised; each on `specify/identity-rotation` with one baseline commit + one residue commit per slice.

## Stress test

- The breakout `/spec:build` resolved `auth-rotate → project-a` without the operator passing `--project` or `chdir`-ing manually. The skill body's routing is the same code path the loop uses.
- `.specify/plan.lock` is the same file in both runs: workspace, never per-slot. Two `/spec:build` shells cannot race on the same plan even when they target different slots.
- The breakout never advanced the plan entry to `done`; merge remains the sole writer of per-entry `done` regardless of how it was triggered.
