---
name: merge
description: Merge a completed change. Invokes `specify merge` to apply delta specs to the baseline and archive the change. Use when the user wants to finalize a change after implementation is complete.
license: MIT
argument-hint: "[change-name?]"
---

# Merge

## Prerequisites

**If `specify` is not on PATH:** stop and instruct the user to install the
CLI via `brew install specify` (preferred), `cargo install specify`, or
the release script at https://specify.sh/install, then re-run. Do not
attempt a prose fallback — validation rules have diverged past the point
where the agent can reliably reproduce them.

Merge a completed change.

## Input

Optionally specify a change name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available changes.

## Steps

1. **Select the change**

   If a name is provided, use it. Otherwise:
   - List directories in `.specify/changes/`, skipping `archive/`, looking for dirs with `.metadata.yaml`
   - If only one active change exists, use it but confirm with the user
   - If multiple, use the **AskQuestion tool** to let the user select

   **IMPORTANT**: Always confirm the change name before merging.

   Set `$CHANGE_DIR = .specify/changes/<name>`.

2. **Pre-merge sanity checks** (warn but don't block)

   Read `$CHANGE_DIR/.metadata.yaml` for `status` and `touched_specs`.

   - **Lifecycle**: If `status` is not `complete`, warn ("this change has status `<status>` — it may not be fully implemented") and use **AskQuestion** to confirm proceed.
   - **Task progress**: run `specify task progress "$CHANGE_DIR" --format json`. If `pending > 0`, warn and use **AskQuestion** to confirm proceed.
   - **Baseline conflict**: for each capability with `type: modified` in `touched_specs`, check whether `.specify/specs/<capability>/spec.md` has been modified since `defined_at`. If so, warn ("the baseline for `<capability>` has been modified since this change was defined, possibly by merging another change") and use **AskQuestion** to confirm proceed.

   Only proceed past this step after the user confirms.

3. **Merge the change**

   ```bash
   specify merge "$CHANGE_DIR" --format json
   ```

   The CLI parses every delta spec, runs the coherence check across every merged baseline, writes the merged baseline files under `.specify/specs/`, flips `.metadata.yaml.status` to `merged`, and moves the change directory to `.specify/archive/YYYY-MM-DD-<name>/`. The operation is transactional — on any failure before commit, the filesystem is left untouched.

   Exit codes:
   - `0`: merge succeeded. Parse the JSON response and render the `merged_specs` list.
   - `1`: merge failed (coherence check, parse error, I/O). Display the JSON `error`/`message`, then use the **AskQuestion tool** to let the user fix the delta and retry, or abort.
   - `2`: validation failed.

4. **Display summary**

## Output On Success

Render the `merged_specs` array from the CLI's JSON response:

```text
## Merge Complete

**Change:** <change-name>
**Merged to:** .specify/archive/YYYY-MM-DD-<name>/

### Specs Merged
- <capability-1>: +N added, M modified, -K removed  (from `operations`)
- <capability-2>: created baseline with N requirement(s)

All artifacts complete. All tasks complete.
```

If `merged_specs` is empty, report "No delta specs to merge".

## Guardrails

- Always confirm the change before merging
- Warn but don't block on incomplete tasks or lifecycle status that isn't `complete`
- Use `specify merge` for all merge and coherence work — never attempt a prose or scripted fallback
- If the CLI exits non-zero, stop and ask the user before proceeding
- Valid lifecycle status values are: `defining`, `defined`, `building`, `complete`, `merged`, `dropped` — use these exact strings, no other values are permitted

> Implements RFC-1 Phase 1 — the CLI handles deterministic operations.
