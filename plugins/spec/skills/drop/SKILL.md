---
name: drop
description: Drop a change without merging specs into the baseline. Use when the user wants to discard a change that should not be merged normally.
license: MIT
argument-hint: "[change-name?]"
---

# Drop

Drop a change without merging its specs into the baseline.

Deterministic bookkeeping — change selection, lifecycle transition, archive
move — is delegated to the `specify` CLI. This skill drives the confirmation
flow and the summary.

When working plan-driven (a `.specify/plan.yaml` exists), after `specify change drop` succeeds the plan entry should transition to `failed` or `blocked` per RFC-2 semantics — `failed` for a build/test failure the human does not intend to retry automatically, `blocked` when a design question needs resolving before the entry is re-entered as `pending`:

```bash
specify plan transition <name> failed  --reason "<short rationale>"
specify plan transition <name> blocked --reason "<short rationale>"
```

This is an advisory note — this skill does not run the command itself. RFC-2 Layer 2's `/spec:execute` will run it automatically; in Layer 1 the human closes the loop.

## Input

Optionally specify a change name. If omitted, check whether it can be inferred from conversation context. If vague or ambiguous, you MUST prompt for available changes.

## Steps

1. **Select the change**

   If a name is provided, use it. Otherwise run `specify status --format json` to enumerate active changes:

   - If only one entry exists, use it but confirm with the user.
   - If multiple, use the **AskQuestion tool** to let the user select.

   **IMPORTANT**: Always confirm the change name before dropping it.

2. **Check lifecycle status**

   Run `specify status <name> --format json` and inspect `status`:

   - `complete`: warn that the change appears ready to merge normally — `/spec:merge` may be the intended action.
   - `merged` or `dropped`: stop and tell the user the change is already finalized (the CLI would error with `lifecycle`, but surface it clearly before attempting).
   - Any other status: explain that dropping will discard the working change without promoting its specs.

   Use the **AskQuestion tool** to confirm the user wants to drop the change.

3. **Summarize what will happen**

   Before invoking the CLI, display a short summary:

   ```text
   ## Drop Preview: <change-name>

   - Change status will be set to `dropped`
   - The change directory will move under `.specify/archive/YYYY-MM-DD-<change-name>/`
   - No specs will be merged into `.specify/specs/`
   - Existing baseline specs remain unchanged
   ```

   Use the **AskQuestion tool** to confirm:

   - **Proceed**: drop the change
   - **Cancel**: keep the change as-is

4. **Drop and archive**

   Run:

   ```bash
   specify change drop <name> --reason "<user-supplied rationale>" --format json
   ```

   The CLI performs the lifecycle transition (enforcing the legal
   non-terminal → `dropped` edge), stamps `dropped_at`, records the
   optional reason in `.metadata.yaml.drop_reason`, and moves the
   directory under `.specify/archive/YYYY-MM-DD-<name>/`. The
   `archive_path` field in the JSON response names the final location.

5. **Display summary**

## Output On Success

```text
## Change Dropped

**Change:** <change-name>
**Archived to:** .specify/archive/YYYY-MM-DD-<change-name>/
**Reason:** <drop_reason>

No specs were merged into `.specify/specs/`.
The baseline remains unchanged.
```

## Guardrails

- Always confirm the change before dropping it.
- Do not merge or rewrite any files under `.specify/specs/`.
- Warn if the change is already `complete`, since `/spec:merge` may be the intended action.
- Stop if the change is already finalized as `merged` or `dropped`.
- Never hand-edit `.metadata.yaml` or the archive directory. `specify change drop` is the sole supported code path.
