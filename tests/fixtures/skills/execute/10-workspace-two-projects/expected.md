# #10 — Workspace `/spec:execute` across two projects

Pins the workspace-mode routing contract: plan artifacts (including `.specify/plan.lock`) live at the workspace; phase work runs in materialised project slots; residue commits land in the slot; CWD restores before every plan write.

## Starting state

- `plan.yaml.lifecycle == approved`; `workspace: true`.
- Two slices, both `pending`: `api-platform-v2-upgrade` → `project-a`; `worker-platform-v2-upgrade` → `project-b`.
- `.specify/workspace/` is empty at run start (no slots materialised yet).
- `registry.yaml` declares both projects.

## Trace

1. **Lock acquired at the workspace.** `.specify/plan.lock` (anchored to `<workspace>/.specify/`, not any slot).

2. **First iteration — `api-platform-v2-upgrade`.**
   - `specrun plan next` promotes the entry to `in-progress` and returns `project: project-a`.
   - Workspace routing per [`../../../../../plugins/spec/skills/execute/references/workspace-routing.md`](../../../../../plugins/spec/skills/execute/references/workspace-routing.md):
     1. Save CWD = workspace.
     2. Resolve `project-a` through `registry.yaml`.
     3. `.specify/workspace/project-a/` is missing → `specrun workspace sync project-a` materialises the slot.
     4. `specrun workspace prepare project-a --change platform-rollout` creates `specify/platform-rollout` from `origin/HEAD`.
     5. `chdir` into `.specify/workspace/project-a/`; emit `Routing: api-platform-v2-upgrade → project-a (.specify/workspace/project-a/)`.
   - Phase sequence: `/spec:refine` → `/spec:build` → `/spec:merge`.
   - `specrun slice merge run` commits `.specify/specs/` + `.specify/archive/` as `specify: merge api-platform-v2-upgrade`.
   - Residue check: `crates/api/` and `migrations/` are dirty; staged and committed as `specify: residue api-platform-v2-upgrade`.
   - `chdir` back to workspace.
   - `specrun plan transition api-platform-v2-upgrade done` (driven by `/spec:merge`).

3. **Second iteration — `worker-platform-v2-upgrade`.**
   - `specrun plan next` returns `project: project-b`.
   - `.specify/workspace/project-b/` materialised; branch prepared; `chdir` into the slot.
   - Phase sequence runs.
   - Residue check: only `crates/worker/` is dirty; committed as `specify: residue worker-platform-v2-upgrade`.
   - `chdir` back to workspace; `done` transition.

4. **Third iteration — drained.** `specrun plan next` reports no `pending` / `in-progress` entries.

## Terminal state

- Both per-entry `status: done`.
- Closing hint: `drained — run /spec:finalize platform-rollout`.
- Two slot directories materialised under `.specify/workspace/`, each on `specify/platform-rollout` with merge + residue commits.

## Stress test

- The plan lock at the workspace is held continuously through both iterations; an attempted second `/spec:execute` from anywhere under the workspace tree exits with `plan-lock-busy holder-pid=<pid>`.
- CWD save/restore brackets every iteration: `specrun plan next` and `specrun plan transition` always resolve against the workspace's `plan.yaml`, never against a slot's.
- Residue commits use the exact message format `specify: residue <slice>`; baseline commits use `specify: merge <slice>`. Two distinct commits per slice in workspace mode.
- Selected materialisation only — `/spec:execute` never broad-syncs every registered project, only the active slice's.
