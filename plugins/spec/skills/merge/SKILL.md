---
name: merge
description: Merge a completed change. Merges delta specs into baseline and moves the change to the archive. Use when the user wants to finalize a change after implementation is complete.
license: MIT
argument-hint: "[change-name?]"
---

# Merge

Merge a completed change.

Deterministic bookkeeping — change selection, prerequisite validation, merge
operation computation, baseline conflict detection, the per-capability merge
itself, baseline coherence validation, status transitions, and the archive
move — is delegated to the `specify` CLI. This skill drives the agent-side
work: reading the merge preview, coordinating the `AskQuestion` confirmation
flow, and summarising results.

When working plan-driven (a `.specify/plan.yaml` exists), after `specify merge` returns successfully the plan entry should be transitioned to `done`:

```bash
specify initiative transition <name> done
```

This is an advisory note — this skill does not run the command itself. RFC-2 Layer 2's `/spec:execute` will run it automatically; in Layer 1 the human closes the loop.

> See `rfcs/archive/rfc-2-execution.md` §"Execution Model Overview" and
> `rfcs/assets/execution.png` for where this skill sits in the
> `/spec:execute` driver loop.

## Phase outcome contract (RFC-2 §"Phase Outcome Contract")

This skill is the **merge** phase of the `/spec:execute` driver loop.
Before returning control to the caller, always record the phase's outcome
via:

```bash
specify change phase-outcome <name> merge <outcome> --summary "..." [--context "..."]
```

where `<outcome>` is exactly one of:

- `success`  — `specify merge` completed, every delta was applied to
  the baseline, and the change directory has been moved to
  `.specify/archive/YYYY-MM-DD-<name>/`.
- `failure`  — `specify merge` exited non-zero for a non-recoverable
  reason (baseline coherence check failed even after the user declined
  to retry, filesystem error, etc.). Use `--summary` to name the
  failing capability and the load-bearing stderr line; use `--context`
  for verbatim detail.
- `deferred` — human judgement is needed (baseline drift surfaced by
  `spec conflict-check` that requires human arbitration, the user
  declined to confirm the merge preview, or the lifecycle status
  disagrees with the expected `Complete`). Use `--summary` to name the
  question.

`/spec:execute` reads `.specify/changes/<name>/.metadata.yaml:outcome`
on return and translates the outcome into a plan transition
(`done` / `failed` / `blocked`). If the field is missing or malformed,
`/spec:execute` treats the phase as `deferred` and stops for triage —
do not skip the CLI call. This `phase-outcome` invocation is the
**last action** the skill takes before returning control, and it must
happen whether or not `specify merge` itself ran (e.g. a user-declined
preview still returns `deferred`).

## Journal entries during the run (RFC-2 §"Question Recording")

Whenever the skill encounters a situation the human should see — a
genuine question, a repair attempt that failed, or a notable recovery —
append to `.specify/changes/<name>/journal.yaml` **during** the run,
not just at the end:

```bash
specify change journal-append <name> merge <kind> --summary "..." [--context "..."]
```

Kinds:

- `question` — baseline drift detected by `spec conflict-check`, the
  user was asked to confirm proceeding, or anything that might produce
  a `deferred` outcome at the end of the phase. Write one entry per
  question so the human sees the full trail when triaging.
- `failure` — `specify merge` returned an error, or a validation step
  surfaced a problem that blocked the merge. Write one entry per
  failure; the final `phase-outcome` summary rolls up only the
  load-bearing one, but auditors still see every attempt.
- `recovery` — a self-heal / recovery step happened. (Typically written
  by `/spec:execute` itself; phases rarely need to append this kind.)

`journal.yaml` is a pure append-only audit log; `/spec:execute` never
consumes it as a signalling channel. The `outcome` field in
`.metadata.yaml` is the only state `/spec:execute` reads on phase
return.

## Mutating the plan mid-run (RFC-2 §"Phase Boundary → Rule 2")

Phases may shell out to `specify initiative create` / `specify initiative amend`
mid-run when they discover something structural about the initiative.
Both commands write `.specify/plan.yaml` synchronously — the new or
updated entry is visible to every subsequent `/spec:execute` iteration.

Allowed:

- `specify initiative create <new-name> --description "...modifies <current-name>..."`
  when, for example, baseline conflict-check surfaces a neighbouring
  change that must land before this one can merge cleanly.
- `specify initiative amend <current-name> --depends-on <newly-needed>` when
  the phase discovers a dependency on another plan entry (e.g. a
  sibling change that should merge first). `amend` may target the
  currently-active entry — non-`status` fields on an `in-progress`
  entry are fair game.

Forbidden:

- Writing `status` through `amend`. The `PlanChangePatch` type has no
  `status` field — this is a type-system guarantee. Status transitions
  are `/spec:execute`'s sole prerogative via `specify initiative transition`.
- Hand-editing `.specify/plan.yaml` or
  `.specify/changes/<name>/.metadata.yaml`. Always route through the
  CLI so the single-writer invariant in RFC-2 §"Plan Mutation and
  Crash Safety" holds.

## Input

Optionally specify a change name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available changes.

## Steps

1. **Select the change**

   If a name is provided, use it. Otherwise run `specify status --format json` to enumerate active changes:

   - If only one entry exists, use it but confirm with the user.
   - If multiple, use the **AskQuestion tool** to let the user select.

   **IMPORTANT**: Always confirm the change name before merging.

2. **Check prerequisites (status, needs, artifacts, tasks)**

   Run in order:

   ```bash
   specify status <name> --format json
   specify validate .specify/changes/<name> --format json
   specify task progress .specify/changes/<name> --format json
   ```

   Interpret:

   - **Status**: `complete` is the expected value. Warn on anything else (e.g., `building`, `defining`), and use **AskQuestion** to confirm proceeding. `merge` will fail later if status isn't `Complete`, so this is a courtesy early exit.
   - **Validate**: if `passed` is `false`, surface the `brief-results` and `cross-checks` failures. Use **AskQuestion** to let the user proceed or abort. Failures in merge-phase needs (usually missing/incomplete artifacts) are the typical blocker.
   - **Tasks**: if `pending > 0`, warn with the count and use **AskQuestion** to confirm proceeding. If there is no tasks file, proceed without warning.

   `specify validate` already runs baseline coherence checks, so the explicit "Baseline coherence check" step from the previous version of this skill is folded in here.

3. **Preview the merge and check for baseline drift**

   Run:

   ```bash
   specify spec preview .specify/changes/<name> --format json
   specify spec conflict-check .specify/changes/<name> --format json
   ```

   Render the preview in a human-friendly summary using the `operations[]` array from `spec preview`. For each spec, operations are typed as `added`, `modified`, `removed`, `renamed`, or `created_baseline`:

   ```text
   ## Merge Preview: <change-name>

   ### <capability-1>/spec.md (existing baseline)
   - REMOVING: REQ-001 — <name>
   - MODIFYING: REQ-002 — <name>
   - ADDING: REQ-003 — <name>

   ### <capability-2>/spec.md (new baseline)
   - CREATING baseline with N requirements
   ```

   If `spec preview` returns an empty `specs` array, report "No delta specs to merge" and stop.

   If `spec conflict-check` returns any entries under `conflicts`, surface them clearly — each entry names the capability, the change's `defined-at`, and the baseline's `baseline-modified-at`:

   > "The baseline for `<capability>` was modified at `<baseline-modified-at>` (after this change was defined at `<defined-at>`). Another change may have already touched it."

   Use the **AskQuestion tool** to let the user:

   - **Proceed**: apply the merge (step 4)
   - **Show full content**: display the merged baseline that would be written (re-run `spec preview --format json` and extract the operations list, or read each delta/baseline pair from disk)
   - **Cancel**: abort

   Only proceed after the user confirms.

4. **Apply the merge**

   Run:

   ```bash
   specify merge .specify/changes/<name> --format json
   ```

   This single call:

   - Gates on `.metadata.yaml.status == Complete` (errors with `lifecycle` if not).
   - Computes the same operations as `spec preview`.
   - Runs baseline coherence validation on every merged output (`specify validate` semantics).
   - Writes each merged baseline under `.specify/specs/<capability>/spec.md`.
   - Transitions `.metadata.yaml` to `merged` and stamps `merged-at` / `completed-at`.
   - Moves `.specify/changes/<name>/` into `.specify/archive/YYYY-MM-DD-<name>/`.

   **If the call exits non-zero**: the filesystem is unchanged (baselines not written, change dir not moved). Report the error and stop — do not retry until the user has edited the failing delta or addressed the lifecycle state.

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

(or "No delta specs to merge" if `spec preview` returned an empty `specs` array)

All artifacts complete. All tasks complete.
```

## Guardrails

- Always confirm the change before merging.
- Validate prerequisites via the CLI before running `specify merge`; warn but don't block if the user explicitly accepts.
- All spec-level operations (preview, merge, validate, conflict-check) go through the `specify` CLI. Never hand-merge delta sections or re-implement the algorithm — the CLI is the sole implementation.
- Never hand-edit `.metadata.yaml` or the archive directory. `specify merge` handles the status transition and archive move atomically; on failure the filesystem is left untouched.
- If `specify merge` reports an error, stop and ask the user before retrying.

For the merge algorithm and a worked example, see `delta-merge.md`.
