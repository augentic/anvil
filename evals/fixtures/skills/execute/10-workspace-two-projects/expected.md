# #10 — Workspace `/spec:execute` across two projects

Pins the workspace-mode routing contract: plan artifacts (including `.specify/plan.lock`) live at the workspace; phase work runs in materialised project slots; residue commits land in the slot; CWD restores before every plan write.

## Starting state

- `plan.yaml.lifecycle == approved`; `workspace: true`.
- Two slices, both `pending`: `api-platform-v2-upgrade` → `backend`; `worker-platform-v2-upgrade` → `mobile`.
- `.specify/workspace/` is empty at run start (no slots materialised yet).
- `registry.yaml` declares both projects.

## Trace

1. **Lock acquired at the workspace.** `.specify/plan.lock` (anchored to `<workspace>/.specify/`, not any slot).

2. **First iteration — `api-platform-v2-upgrade`.**
   - `specify plan status` names the next eligible entry (`refine api-platform-v2-upgrade`, `project: backend`); `specify plan next` promotes it to `in-progress` (the CLI's lock probe passes — the workspace lock is held).
   - Workspace routing per [`../../../../../plugins/spec/skills/execute/references/workspace-routing.md`](../../../../../plugins/spec/skills/execute/references/workspace-routing.md):
     1. Save CWD = workspace.
     2. Resolve `backend` through `registry.yaml`.
     3. `.specify/workspace/backend/` is missing → `specify workspace sync backend` materialises the slot.
     4. `specify workspace prepare backend --change platform-rollout` creates `specify/platform-rollout` from `origin/HEAD`.
     5. `chdir` into `.specify/workspace/backend/`; emit `Routing: api-platform-v2-upgrade → backend (.specify/workspace/backend/)`.
     6. Export `SPECIFY_PLAN_DIR=<workspace-root>` so slot-side plan readers resolve the workspace's `plan.yaml` (the slot has none).
   - Phase sequence: `/spec:refine` → `/spec:build` → `/spec:merge`.
   - `specify slice merge run` commits `.specify/specs/` + `.specify/archive/` as `specify: merge api-platform-v2-upgrade` and — through the exported plan root — stamps the entry `done` in the workspace's `plan.yaml` (merge stays the sole writer of `done`).
   - Residue check: `crates/api/` and `migrations/` are dirty; staged and committed as `specify: residue api-platform-v2-upgrade`.
   - `chdir` back to workspace; unset `SPECIFY_PLAN_DIR`.

3. **Second iteration — `worker-platform-v2-upgrade`.**
   - `specify plan status` → `specify plan next` returns `project: mobile`.
   - `.specify/workspace/mobile/` materialised; branch prepared; `chdir` into the slot; plan root exported.
   - Phase sequence runs; merge stamps `done` through the exported plan root.
   - Residue check: only `crates/worker/` is dirty; committed as `specify: residue worker-platform-v2-upgrade`.
   - `chdir` back to workspace; export unset.

4. **Third iteration — drained.** `specify plan status` reports `drained` — no `pending` / `in-progress` entries remain.

## Terminal state

- Both per-entry `status: done`.
- Closing hint: `drained — run /spec:finalize platform-rollout`.
- Two slot directories materialised under `.specify/workspace/`, each on `specify/platform-rollout` with merge + residue commits.

## Stress test

- The plan lock at the workspace is held continuously through both iterations; an attempted second `/spec:execute` from anywhere under the workspace tree exits with `plan-lock-busy holder-pid=<pid>` — and a session that skipped the snippet is refused by the CLI itself (`plan-lock-not-held` on `plan next` / slot-side `slice merge run` through the exported plan root).
- CWD save/restore brackets every iteration: `specify plan status`, `specify plan next`, and `specify plan transition` always resolve against the workspace's `plan.yaml`, never against a slot's.
- Residue commits use the exact message format `specify: residue <slice>`; baseline commits use `specify: merge <slice>`. Two distinct commits per slice in workspace mode.
- Selected materialisation only — `/spec:execute` never broad-syncs every registered project, only the active slice's.
