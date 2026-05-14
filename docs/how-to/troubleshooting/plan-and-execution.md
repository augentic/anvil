# Plan and execution issues

Use this page when `/change:execute` or one of the `specify change plan` verbs refuses or halts -- locks held, dirty workspace slots, plans stuck, or registry amendments required.

## Contents

- [Prerequisites](#prerequisites)
- [Lock held by another process](#lock-held-by-another-process)
- [Self-heal on startup](#self-heal-on-startup)
- [Workspace slot missing](#workspace-slot-missing)
- [`origin-head-unresolved`](#origin-head-unresolved)
- [Dirty workspace slot before execution](#dirty-workspace-slot-before-execution)
- [Execution stuck](#execution-stuck)
- [Registry amendment required](#registry-amendment-required)
- [Phase failure during execution](#phase-failure-during-execution)
- [Plan doctor diagnostics](#plan-doctor-diagnostics)

## Prerequisites

- An active multi-slice change (a `plan.yaml` exists at the repo root).
- The change name and the diagnostic or status reason printed by the executor or `specify change plan status`.

## Lock held by another process

**Symptom:** `/change:execute` reports that `.specify/plan.lock` is held.

**Cause:** Another `/change:execute` session is running, or a previous session crashed without releasing the lock.

**Resolution:**
1. Check the lock: `specify change plan lock status`
2. If the PID is not running, release it: `specify change plan lock release`
3. If another session is running, wait for it to finish.

## Self-heal on startup

**Symptom:** `/change:execute` reports a self-heal operation when starting.

**Cause:** A previous execution run crashed or was killed mid-slice, leaving an `in-progress` entry.

**Resolution:** Self-heal is automatic. The driver resolves the stale entry:
- If the slice completed successfully, it transitions the entry to `done`.
- If the slice is in a broken state, it transitions to `failed` or `blocked`.

For multi-repo entries with `project`, self-heal looks at `.specify/slices/<name>/.metadata.yaml` under the target project's workspace clone, not the initiating repo. If the workspace slot is missing, execution halts (see "Workspace slot missing" below).

If self-heal itself fails, manually resolve:
1. Check the stale slice: `specify slice status <name>`
2. Complete or drop it manually.
3. Transition the plan entry: `specify change plan transition <name> done|failed`

## Workspace slot missing

**Symptom:** `/change:execute` halts with a diagnostic pointing at `specify workspace sync` for a target project.

**Cause:** A plan entry has a `project` field targeting a registry project whose workspace slot is not materialised (`.specify/workspace/<project>/` does not exist or is incomplete).

**Resolution:**
1. Run `specify workspace sync <project>` to materialise the missing selected slot, or `specify workspace sync` to materialise all registry projects.
2. Re-run `/change:execute`.

## `origin-head-unresolved`

**Symptom:** `/change:execute` refuses before define/build/merge and reports `origin-head-unresolved` for a remote-backed workspace slot.

**Cause:** Branch preparation could not resolve `origin/HEAD` after fetching. Specify will not guess the default branch because `specify/<change-name>` must be prepared from the repository's remote default before any execution mutation.

**Resolution:**
1. Inspect the slot: `specify workspace status <project>`.
2. In the workspace slot, verify the remote default branch exists on the server and that `origin` points at the registry URL.
3. Fix the remote default branch in the forge, or repair the clone with `git remote set-head origin -a` after the remote is correct.
4. Re-run `/change:execute`.

## Dirty workspace slot before execution

**Symptom:** `/change:execute` refuses during branch preparation with a dirty-work diagnostic such as `dirty-unrelated-tracked` or `dirty-branch-mismatch`.

**Cause:** The target workspace slot has tracked modifications that are outside the active slice boundary, or resume-safe tracked modifications are present while the slot is not already on `specify/<change-name>`. The executor refuses to check out or mutate over unrelated work.

**Resolution:**
1. Inspect the slot: `specify workspace status <project>`.
2. Commit, stash, or discard unrelated local work in that slot.
3. If the work belongs to the active change, check out the exact `specify/<change-name>` branch first or let a clean `/change:execute` prepare it.
4. Re-run `/change:execute`.

## Execution stuck

**Symptom:** `/change:execute loop` exits with `stuck`.

**Cause:** No `pending` entry has all dependencies satisfied. Typically because a dependency is `failed` or `blocked`, or a structural problem in the plan (cycle, unreachable entry) is preventing progress.

**Resolution:**
1. **First triage step:** run `specify change plan validate` -- it surfaces every structural problem (cycles, orphan sources, stale clones, unreachable entries) alongside the base shape rules. See [Plan health diagnostics](#plan-health-diagnostics) below.
2. Check plan status: `specify change plan status`
3. Identify the blocking entries.
4. Options:
   - Fix and retry the failed entry: `specify change plan transition <name> pending` then `/change:execute`
   - Skip it: `specify change plan transition <name> skipped`
   - Remove the dependency: `specify change plan amend <downstream> --depends-on <updated-list>`

## Registry amendment required

**Symptom:** `/change:execute loop` halts with a `registry-amendment-required` outcome on the offending slice. The slice is transitioned to `blocked` and the proposal payload is written to its `journal.yaml`.

**Cause:** A phase skill (typically `/spec:extract` or a build brief) discovered that the slice targets a capability that does not fit any existing registry project, and proposed a new project. The framework never auto-modifies the registry.

**Resolution:** Follow the canonical recovery sequence:

```bash
specify slice journal show <slice>             # read the proposal payload
specify registry add <proposed-name> \
    --url <proposed-url> \
    --capability <proposed-capability> \
    --description "<proposed-description>"
specify workspace sync                          # bootstrap the new slot
specify change plan amend <slice> --project <proposed-name>
specify change plan transition <slice> pending
# re-run /change:execute
```

For the full how-to, see [Recover from registry-amendment-required](../recover-from-registry-amendment.md).

## Phase failure during execution

**Symptom:** A plan entry transitions to `failed` during `/change:execute`.

**Cause:** The define, build, or merge phase failed for this slice.

**Resolution:**
1. The slice was automatically dropped by the driver.
2. Review the failure in the journal: check `.specify/slices/<name>/journal.yaml` (if it exists before archiving).
3. To retry: reset the plan entry to `pending` and re-run `/change:execute`.

## Plan health diagnostics

`specify change plan validate` is the first triage step when `/change:execute loop` reports `stuck`. It runs the base shape rules, then layers four health diagnostics.

### `cycle-in-depends-on`

**Symptom:** `specify change plan validate` reports `cycle-in-depends-on` with the cycle path (e.g. `["a", "b", "a"]`).

**Cause:** Two or more plan entries form a `depends-on` cycle. `next_eligible` silently skips cycles at runtime, so the executor reports `stuck`; this diagnostic is the only place where the cycle structure is surfaced.

**Resolution:** Break the cycle with `specify change plan amend <name> --depends-on <updated-list>` on one of the entries on the cycle path, then re-run validate.

### `orphan-source-key`

**Symptom:** `specify change plan validate` reports `orphan-source-key` (warning) for a key declared in the top-level `sources:` map but referenced by no entry.

**Cause:** A `--source <key>=<path>` was supplied at plan time but no proposed slice ended up using it (rejected during the propose loop, or scope changed).

**Resolution:** Either reference the key from an entry's `sources:` list (`specify change plan amend <name> --sources <key>`) or drop the declaration via a hand-edit of `plan.yaml`. Warnings are non-fatal; the loop will proceed.

### `stale-workspace-clone`

**Symptom:** `specify change plan validate` reports `stale-workspace-clone` (warning) with reason `signature-changed` (URL or capability diverged) or `missing-sync-stamp` (no stamp file and no readable git remote).

**Cause:** The workspace clone's signature has drifted from the registry, typically because `registry.yaml` was edited after the clone was first materialised.

**Resolution:** `specify workspace sync` to refresh the clone. The verb is idempotent.

### `unreachable-entry`

**Symptom:** `specify change plan validate` reports `unreachable-entry` for a pending entry whose dependency closure is rooted in a `failed` or `skipped` predecessor.

**Cause:** The entry's `depends-on` list (transitively) names an entry that can never become `done` (it is in a terminal non-success state).

**Resolution:** Two paths.

- **Reset the predecessor:** `specify change plan transition <pred> pending` (after fixing the underlying issue) and re-run `/change:execute`.
- **Drop the leaf:** `specify change plan transition <entry> skipped --reason "<reason>"` to remove the entry from the dependency frontier.
