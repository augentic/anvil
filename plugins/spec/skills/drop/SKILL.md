---
name: specify-drop
description: Drop a slice without merging specs into the baseline. Use when an in-progress slice must be abandoned and archived without folding its deltas into the baseline — the rollback counterpart to `merge`.
argument-hint: "[slice-name]"
---

# Drop

Drop a slice without merging its specs into the baseline.

Deterministic bookkeeping — slice selection, lifecycle transition, archive move — is delegated to the `specify` CLI. This skill drives the confirmation flow and the summary.

## Non-interactive mode

When invoked with `reason`, skip the confirmation `AskQuestion` calls in steps 1–3; proceed directly to step 4 with the supplied reason. The slice name must be provided explicitly as the positional argument. Exit code is 0 on a clean drop, non-zero only on CLI failure. Non-interactive mode forwards `--reason` to `specify slice drop`.

## Critical Path

1. **Select the slice**

   If a name is provided, use it. Otherwise inspect `.specify/slices/*/.metadata.yaml` directly to enumerate active slices:

   - If only one entry exists, use it but confirm with the user.
   - If multiple, use the **AskQuestion tool** to let the user select.

2. **Check lifecycle status**

   Read `.specify/slices/<name>/.metadata.yaml` and inspect `status`:

   - `built`: warn that the slice is ready for `/spec:merge`.
   - `merged` or `dropped`: stop and tell the user the slice is already finalized (the CLI would error with `lifecycle`, but surface it clearly before attempting).
   - Any other status: explain that dropping will discard the working slice without promoting its specs.

   Use the **AskQuestion tool** to confirm the user wants to drop the slice. (non-interactive: skip — see above; the CLI still enforces the terminal-status check in step 4.)

3. **Summarize what will happen**

   Before invoking the CLI, display a short summary:

   ```text
   ## Drop Preview: <slice-name>

   - Slice status will be set to `dropped`
   - The slice directory will move under `.specify/archive/YYYY-MM-DD-<slice-name>/`
   - No specs will be merged into `.specify/specs/`
   - Existing baseline specs remain unchanged
   ```

   Use the **AskQuestion tool** to confirm: **Proceed** drops the slice, **Cancel** keeps it as-is. (non-interactive: skip — see above; the preview may still be printed as an informational line.)

4. **Drop and archive**

   Run:

   ```bash
   specify slice drop <name> --reason "<user-supplied rationale>" --format json
   ```

   The CLI performs the lifecycle transition (enforcing the legal non-terminal → `dropped` edge), stamps `dropped-at`, records the optional reason in `.metadata.yaml.drop-reason`, and moves the directory under `.specify/archive/YYYY-MM-DD-<name>/`. The `archive-path` field in the JSON response names the final location.

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

- Do not merge or rewrite any files under `.specify/specs/`.
- Warn if the slice is already `built`, since `/spec:merge` may be the intended action.
- Stop if the slice is already finalized as `merged` or `dropped`.
- `specify slice drop` is the sole writer for `.metadata.yaml` and the archive directory on drop. See [shared guardrails](../../references/guardrails.md#single-writer-for-lifecycle-state).
