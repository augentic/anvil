---
name: specify-drop
description: Drop a slice without merging specs into the baseline. Use when an in-progress slice must be abandoned and archived without folding its deltas into the baseline — the rollback counterpart to `merge`.
argument-hint: "[slice-name]"
---

# Drop

Drop a slice without merging its specs into the baseline.

Deterministic bookkeeping — slice selection, lifecycle transition, archive move — is delegated to the `specify` CLI. This skill drives the confirmation flow and the summary.

## Non-interactive mode

When invoked with `reason`, skip the confirmation `AskQuestion` calls in steps 1–3; proceed directly to step 4 with the supplied reason. The slice name must be provided explicitly as the positional argument. Exit code is 0 on a clean drop, non-zero only on CLI failure.

Non-interactive mode is how `/change:execute` invokes this skill during `loop`, supervised single-slice runs, and self-heal reclaim of a `failure` / `deferred` outcome (see `/change:execute` steps 11b, 12b, and §"Self-heal on startup" step 2). The driver supplies a `reason` string assembled from the upstream phase's outcome — see the verbatim-`summary` rule in [`../../references/phase-outcome-contract.md`](../../references/phase-outcome-contract.md). This skill forwards that string to `specify slice drop` verbatim, without prompting.

When working plan-driven (a `plan.yaml` exists), after `specify slice drop` succeeds the plan entry should transition to `failed` or `blocked` — `failed` for a build/test failure the human does not intend to retry automatically, `blocked` when a design question needs resolving before the entry is re-entered as `pending`:

```bash
specify change plan transition <name> failed  --reason "<short rationale>"
specify change plan transition <name> blocked --reason "<short rationale>"
```

This is an advisory note — this skill does not run the command itself. `/change:execute` will run it automatically; when driving the loop manually, the operator closes it.

## Phase outcome contract

This skill is the **drop** phase of the `/change:execute` driver loop. Apply the shared [phase outcome contract](../../references/phase-outcome-contract.md), including drop's CLI-stamped success path, non-success deltas, journal rules, plan-mutation allowlist, and verbatim-`summary` rule.

## Input

Optionally specify a slice name. If omitted, check whether it can be inferred from conversation context. If vague or ambiguous, you MUST prompt for available slices.

## Steps

1. **Select the slice**

   If a name is provided, use it. Otherwise run `specify status --format json` to enumerate active slices from the dashboard:

   - If only one entry exists, use it but confirm with the user.
   - If multiple, use the **AskQuestion tool** to let the user select.

   **IMPORTANT**: Always confirm the slice name before dropping it.

   If `reason` was supplied (non-interactive mode — see above), the slice name must be the positional argument; skip the prompting fallback and the confirmation.

2. **Check lifecycle status**

   Run `specify slice status <name> --format json` and inspect `status`:

   - `complete`: warn that the slice appears ready to merge normally — `/spec:merge` may be the intended action.
   - `merged` or `dropped`: stop and tell the user the slice is already finalized (the CLI would error with `lifecycle`, but surface it clearly before attempting).
   - Any other status: explain that dropping will discard the working slice without promoting its specs.

   If `reason` was NOT supplied, use the **AskQuestion tool** to confirm the user wants to drop the slice. In non-interactive mode skip the prompt and proceed (the CLI still enforces the terminal-status check in step 4 — a `merged` / `dropped` slice surfaces `Error::Lifecycle` there).

3. **Summarize what will happen**

   Before invoking the CLI, display a short summary:

   ```text
   ## Drop Preview: <slice-name>

   - Slice status will be set to `dropped`
   - The slice directory will move under `.specify/archive/YYYY-MM-DD-<slice-name>/`
   - No specs will be merged into `.specify/specs/`
   - Existing baseline specs remain unchanged
   ```

   If `reason` was NOT supplied, use the **AskQuestion tool** to confirm:

   - **Proceed**: drop the slice
   - **Cancel**: keep the slice as-is

   In non-interactive mode skip this confirmation too; the preview may still be printed as an informational line but the skill does not wait for input.

4. **Drop and archive**

   Run:

   ```bash
   specify slice drop <name> --reason "<user-supplied rationale>" --format json
   ```

   The CLI performs the lifecycle transition (enforcing the legal non-terminal → `dropped` edge), stamps `dropped-at`, records the optional reason in `.metadata.yaml.drop-reason`, and moves the directory under `.specify/archive/YYYY-MM-DD-<name>/`. The `archive-path` field in the JSON response names the final location.

5. **Display summary**

## Output On Success

```text
## Slice Dropped

**Slice:** <slice-name>
**Archived to:** .specify/archive/YYYY-MM-DD-<slice-name>/
**Reason:** <drop-reason>

No specs were merged into `.specify/specs/`.
The baseline remains unchanged.
```

## Guardrails

- Always confirm the slice before dropping it.
- Do not merge or rewrite any files under `.specify/specs/`.
- Warn if the slice is already `complete`, since `/spec:merge` may be the intended action.
- Stop if the slice is already finalized as `merged` or `dropped`.
- `specify slice drop` is the sole writer for `.metadata.yaml` and the archive directory on drop. See [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state).
