---
name: specify-merge
description: Merge a completed slice. Merges delta specs into baseline and moves the slice to the archive. Use when the user wants to finalize a slice after implementation is complete.
argument-hint: "[slice-name]"
---

# Merge

## Critical Path (Quick Reference)

1. **Select and confirm the slice** — infer or ask for the slice, then confirm before any merge operation.
2. **Check prerequisites via CLI** — run status, validate, and task progress; warn on non-`complete`, validation failures, or pending tasks before proceeding.
3. **Preview and conflict-check** — run `specify slice merge preview` and `specify slice merge conflict-check`, render operations, and surface baseline drift clearly.
4. **Get explicit confirmation** — let the user proceed, inspect full content, or cancel; record `deferred` if merge is not invoked.
5. **Apply through `merge run` only** — run `specify slice merge run <name> --format json`; never hand-merge specs, edit metadata, or move archives manually.
6. **Handle outcomes correctly** — on success, trust the CLI-stamped merge outcome; on non-zero exit, record merge `failure` via the shared [phase outcome contract](../../references/phase-outcome-contract.md).
7. **Summarize the archive** — report merged specs, created baselines, archive path, and any workspace auto-commit warning or residue note.

Merge a completed slice.

Deterministic bookkeeping — slice selection, prerequisite validation, merge operation computation, baseline conflict detection, the per-capability merge itself, baseline coherence validation, status transitions, and the archive move — is delegated to the `specify` CLI. This skill drives the agent-side work: reading the merge preview, coordinating the `AskQuestion` confirmation flow, and summarising results.

When working plan-driven (a `plan.yaml` exists), after `specify slice merge run` returns successfully the plan entry should be transitioned to `done`:

```bash
specify change plan transition <name> done
```

This is an advisory note — this skill does not run the command itself. `/change:execute` will run it automatically; in Layer 1 the human closes the loop.

## Phase outcome contract

This skill is the **merge** phase of the `/change:execute` driver loop. Apply the shared [phase outcome contract](../../references/phase-outcome-contract.md), including merge's CLI-stamped success path, non-success deltas, journal rules, plan-mutation allowlist, and verbatim-`summary` rule.

## Input

Optionally specify a slice name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available slices.

## Steps

1. **Select the slice**

   If a name is provided, use it. Otherwise run `specify status --format json` to enumerate active slices from the dashboard:

   - If only one entry exists, use it but confirm with the user.
   - If multiple, use the **AskQuestion tool** to let the user select.

   **IMPORTANT**: Always confirm the slice name before merging.

2. **Check prerequisites (status, needs, artifacts, tasks)**

   Run in order:

   ```bash
   specify slice status <name> --format json
   specify slice validate <name> --format json
   specify slice task progress <name> --format json
   ```

   Interpret:

   - **Status**: `complete` is the expected value. Warn on anything else (e.g., `building`, `defining`), and use **AskQuestion** to confirm proceeding. `specify slice merge run` will fail later if status isn't `Complete`, so this is a courtesy early exit.
   - **Validate**: if `passed` is `false`, surface the `brief-results` and `cross-checks` failures. Use **AskQuestion** to let the user proceed or abort. Failures in merge-phase needs (usually missing/incomplete artifacts) are the typical blocker.
   - **Tasks**: if `pending > 0`, warn with the count and use **AskQuestion** to confirm proceeding. If there is no tasks file, proceed without warning.

   `specify slice validate` already runs baseline coherence checks, so the explicit "Baseline coherence check" step from the previous version of this skill is folded in here.

3. **Preview the merge and check for baseline drift**

   Run:

   ```bash
   specify slice merge preview <name> --format json
   specify slice merge conflict-check <name> --format json
   ```

   Render the preview in a human-friendly summary using the `operations[]` array from `specify slice merge preview`. For each spec, operations are typed as `added`, `modified`, `removed`, `renamed`, or `created_baseline`:

   ```text
   ## Merge Preview: <slice-name>

   ### <capability-1>/spec.md (existing baseline)
   - REMOVING: REQ-001 — <name>
   - MODIFYING: REQ-002 — <name>
   - ADDING: REQ-003 — <name>

   ### <capability-2>/spec.md (new baseline)
   - CREATING baseline with N requirements
   ```

   If `specify slice merge preview` returns an empty `specs` array, report "No delta specs to merge" and stop.

   If `change merge conflict-check` returns any entries under `conflicts`, surface them clearly — each entry names the capability, the slice's `defined-at`, and the baseline's `baseline-modified-at`:

   > "The baseline for `<capability>` was modified at `<baseline-modified-at>` (after this slice was defined at `<defined-at>`). Another change may have already touched it."

   Use the **AskQuestion tool** to let the user:

   - **Proceed**: apply the merge (step 4)
   - **Show full content**: display the merged baseline that would be written (re-run `change merge preview --format json` and extract the operations list, or read each delta/baseline pair from disk)
   - **Cancel**: abort

   Only proceed after the user confirms.

4. **Apply the merge**

   Run:

   ```bash
   specify slice merge run <name> --format json
   ```

   This single call:

   - Gates on `.metadata.yaml.status == Complete` (errors with `lifecycle` if not).
   - Computes the same operations as `change merge preview`.
   - Runs baseline coherence validation on every merged output (`specify slice validate` semantics).
   - Writes each merged baseline under `.specify/specs/<capability>/spec.md`.
   - Transitions `.metadata.yaml` to `merged`, stamps `merged-at` / `completed-at`, and stamps `PhaseOutcome { phase: merge, outcome: success }`.
   - Moves `.specify/slices/<name>/` into `.specify/archive/YYYY-MM-DD-<name>/`.

   On success, the outcome is already recorded — **do not call `outcome set`** (see §Phase outcome contract above).

   **Workspace clone auto-commit.** When CWD is inside a workspace clone (`.specify/workspace/*/` with `.specify/project.yaml`), the CLI auto-commits **only** `.specify/specs/` and `.specify/archive/` with message `specify: merge <slice-name>`. Commit failure is a **warning**, not an error — the spec merge still succeeds. Any project-output residue outside those two trees is left for `/change:execute` to commit as `specify: residue <slice-name>`. Committed changes remain local until the operator explicitly runs `specify workspace push`.

   **If the call exits non-zero**: the filesystem is unchanged (baselines not written, slice dir not moved). Record the failure via `specify slice outcome set` (the slice directory still exists). Report the error and stop — do not retry until the user has edited the failing delta or addressed the lifecycle state.

5. **Display summary**

   On success, the CLI returns `merged-specs[]` with the same operation list. Render a completion summary:

## Output On Success

```text
## Merge Complete

**Slice:** <slice-name>
**Merged to:** .specify/archive/YYYY-MM-DD-<name>/

### Specs Merged
- <capability-1>: merged into .specify/specs/<capability-1>/spec.md
- <capability-2>: new baseline created at .specify/specs/<capability-2>/spec.md

(or "No delta specs to merge" if `specify slice merge preview` returned an empty `specs` array)

All artifacts complete. All tasks complete.
```

## Guardrails

- Always confirm the slice before merging.
- Validate prerequisites via the CLI before running `specify slice merge run`; warn but don't block if the user explicitly accepts.
- All spec-level operations (preview, merge, validate, conflict-check) go through the `specify` CLI. Never hand-merge delta sections or re-implement the algorithm — the CLI is the sole implementation.
- `specify slice merge run` is the sole writer for `.metadata.yaml` and the archive directory on merge — it handles the status transition and archive move atomically; on failure the filesystem is left untouched. See [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state).
- If `specify slice merge run` reports an error, stop and ask the user before retrying.

For the merge algorithm and a worked example, see `delta-merge.md`.
