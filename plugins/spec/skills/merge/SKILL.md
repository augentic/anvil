---
name: specify-merge
description: Merge a completed change. Merges delta specs into baseline and moves the change to the archive. Use when the user wants to finalize a change after implementation is complete.
argument-hint: "[change-name]"
---

# Merge

Merge a completed change.

Deterministic bookkeeping — change selection, prerequisite validation, merge operation computation, baseline conflict detection, the per-capability merge itself, baseline coherence validation, status transitions, and the archive move — is delegated to the `specify` CLI. This skill drives the agent-side work: reading the merge preview, coordinating the `AskQuestion` confirmation flow, and summarising results.

When working plan-driven (a `.specify/plan.yaml` exists), after `specify change merge run` returns successfully the plan entry should be transitioned to `done`:

```bash
specify plan transition <name> done
```

This is an advisory note — this skill does not run the command itself. `/spec:execute` will run it automatically; in Layer 1 the human closes the loop.

## Phase outcome contract

This skill is the **merge** phase of the `/spec:execute` driver loop.
The shared phase contract — outcome values, journal kinds, plan-mutation rules,
the verbatim-`summary` rule, and the success/failure/deferred semantics — is
authored once at [`../../references/phase-outcome-contract.md`](../../references/phase-outcome-contract.md).

This phase's outcome-specific deltas:

- `success` — baseline merge applied, lifecycle transitioned to `merged`, archive moved. **Uniquely CLI-stamped** — `specify change merge run` writes the success outcome atomically with the lifecycle transition before archiving; skills MUST NOT call `outcome set` on this path (see the reference for the rationale).
- `failure` — `specify change merge run` exited non-zero (filesystem unchanged); record skill-side via `outcome set ... merge failure ...`.
- `deferred` — `specify change merge run` was never invoked (user declined the preview, conflict-check needs human arbitration, lifecycle ≠ `Complete`); record skill-side via `outcome set ... merge deferred ...`.

## Input

Optionally specify a change name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available changes.

## Steps

1. **Select the change**

   If a name is provided, use it. Otherwise run `specify status --format json` to enumerate active changes from the dashboard:

   - If only one entry exists, use it but confirm with the user.
   - If multiple, use the **AskQuestion tool** to let the user select.

   **IMPORTANT**: Always confirm the change name before merging.

2. **Check prerequisites (status, needs, artifacts, tasks)**

   Run in order:

   ```bash
   specify change status <name> --format json
   specify change validate <name> --format json
   specify change task progress <name> --format json
   ```

   Interpret:

   - **Status**: `complete` is the expected value. Warn on anything else (e.g., `building`, `defining`), and use **AskQuestion** to confirm proceeding. `change merge run` will fail later if status isn't `Complete`, so this is a courtesy early exit.
   - **Validate**: if `passed` is `false`, surface the `brief-results` and `cross-checks` failures. Use **AskQuestion** to let the user proceed or abort. Failures in merge-phase needs (usually missing/incomplete artifacts) are the typical blocker.
   - **Tasks**: if `pending > 0`, warn with the count and use **AskQuestion** to confirm proceeding. If there is no tasks file, proceed without warning.

   `specify change validate` already runs baseline coherence checks, so the explicit "Baseline coherence check" step from the previous version of this skill is folded in here.

3. **Preview the merge and check for baseline drift**

   Run:

   ```bash
   specify change merge preview <name> --format json
   specify change merge conflict-check <name> --format json
   ```

   Render the preview in a human-friendly summary using the `operations[]` array from `change merge preview`. For each spec, operations are typed as `added`, `modified`, `removed`, `renamed`, or `created_baseline`:

   ```text
   ## Merge Preview: <change-name>

   ### <capability-1>/spec.md (existing baseline)
   - REMOVING: REQ-001 — <name>
   - MODIFYING: REQ-002 — <name>
   - ADDING: REQ-003 — <name>

   ### <capability-2>/spec.md (new baseline)
   - CREATING baseline with N requirements
   ```

   If `change merge preview` returns an empty `specs` array, report "No delta specs to merge" and stop.

   If `change merge conflict-check` returns any entries under `conflicts`, surface them clearly — each entry names the capability, the change's `defined-at`, and the baseline's `baseline-modified-at`:

   > "The baseline for `<capability>` was modified at `<baseline-modified-at>` (after this change was defined at `<defined-at>`). Another change may have already touched it."

   Use the **AskQuestion tool** to let the user:

   - **Proceed**: apply the merge (step 4)
   - **Show full content**: display the merged baseline that would be written (re-run `change merge preview --format json` and extract the operations list, or read each delta/baseline pair from disk)
   - **Cancel**: abort

   Only proceed after the user confirms.

4. **Apply the merge**

   Run:

   ```bash
   specify change merge run <name> --format json
   ```

   This single call:

   - Gates on `.metadata.yaml.status == Complete` (errors with `lifecycle` if not).
   - Computes the same operations as `change merge preview`.
   - Runs baseline coherence validation on every merged output (`specify change validate` semantics).
   - Writes each merged baseline under `.specify/specs/<capability>/spec.md`.
   - Transitions `.metadata.yaml` to `merged`, stamps `merged-at` / `completed-at`, and stamps `PhaseOutcome { phase: merge, outcome: success }`.
   - Moves `.specify/changes/<name>/` into `.specify/archive/YYYY-MM-DD-<name>/`.

   On success, the outcome is already recorded — **do not call `outcome set`** (see §Phase outcome contract above).

   **Workspace clone auto-commit.** When CWD is inside a workspace clone (`.specify/workspace/*/` with `.specify/project.yaml`), the CLI auto-commits `.specify/specs/` and `.specify/archive/` with message `specify: merge <change-name>`. Commit failure is a **warning**, not an error — the spec merge still succeeds. Committed changes remain local until the operator explicitly runs `specify workspace push`.

   **If the call exits non-zero**: the filesystem is unchanged (baselines not written, change dir not moved). Record the failure via `specify change outcome set` (the change directory still exists). Report the error and stop — do not retry until the user has edited the failing delta or addressed the lifecycle state.

5. **Display summary**

   On success, the CLI returns `merged-specs[]` with the same operation list. Render a completion summary:

## Output On Success

```text
## Merge Complete

**Change:** <change-name>
**Merged to:** .specify/archive/YYYY-MM-DD-<name>/

### Specs Merged
- <capability-1>: merged into .specify/specs/<capability-1>/spec.md
- <capability-2>: new baseline created at .specify/specs/<capability-2>/spec.md

(or "No delta specs to merge" if `change merge preview` returned an empty `specs` array)

All artifacts complete. All tasks complete.
```

## Guardrails

- Always confirm the change before merging.
- Validate prerequisites via the CLI before running `specify change merge run`; warn but don't block if the user explicitly accepts.
- All spec-level operations (preview, merge, validate, conflict-check) go through the `specify` CLI. Never hand-merge delta sections or re-implement the algorithm — the CLI is the sole implementation.
- Never hand-edit `.metadata.yaml` or the archive directory. `specify change merge run` handles the status transition and archive move atomically; on failure the filesystem is left untouched.
- If `specify change merge run` reports an error, stop and ask the user before retrying.

For the merge algorithm and a worked example, see `delta-merge.md`.
