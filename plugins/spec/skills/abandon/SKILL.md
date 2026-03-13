---
name: abandon
description: Abandon a change without merging specs into the baseline. Use when the user wants to discard a change that should not be archived normally.
license: MIT
---

# Abandon

Abandon a change without merging its specs into the baseline.

## Input

Optionally specify a change name. If omitted, check whether it can be inferred from conversation context. If vague or ambiguous, you MUST prompt for available changes.

## Steps

1. **Select the change**

   If a name is provided, use it. Otherwise:
   - List directories in `.specify/changes/`, skipping `archive/`, looking for dirs with `.metadata.yaml`
   - If only one active change exists, use it but confirm with the user
   - If multiple, use the **AskQuestion tool** to let the user select

   **IMPORTANT**: Always confirm the change name before abandoning it.

   Read `.specify/changes/<name>/.metadata.yaml` for the schema value and status. **Resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`.

2. **Check lifecycle status**

   Read `status` from `.metadata.yaml`:
   - If `status` is `complete`, warn that the change appears ready to archive normally
   - If `status` is `archived` or `abandoned`, stop and tell the user the change is already finalized
   - For any other status, explain that abandoning will discard the working change without promoting its specs

   Use the **AskQuestion tool** to confirm the user wants to abandon the change.

3. **Summarize what will happen**

   Before making any file changes, display a short summary:

   ```text
   ## Abandon Preview: <change-name>

   - Change status will be set to `abandoned`
   - The change directory will move under `.specify/changes/archive/`
   - No specs will be merged into `.specify/specs/`
   - Existing baseline specs remain unchanged
   ```

   Use the **AskQuestion tool** to confirm:
   - **Proceed**: abandon the change
   - **Cancel**: keep the change as-is

4. **Update metadata**

   Update `.specify/changes/<name>/.metadata.yaml`:
   - Set `status` to `abandoned`

5. **Move the change to archive**

   ```bash
   mkdir -p .specify/changes/archive
   mv .specify/changes/<name> .specify/changes/archive/YYYY-MM-DD-<name>
   ```

   Use today's date in `YYYY-MM-DD` format.

6. **Display summary**

## Output On Success

```text
## Change Abandoned

**Change:** <change-name>
**Archived to:** .specify/changes/archive/YYYY-MM-DD-<name>/

No specs were merged into `.specify/specs/`.
The baseline remains unchanged.
```

## Guardrails

- Always confirm the change before abandoning it
- Do not merge or rewrite any files under `.specify/specs/`
- Warn if the change is already `complete`, since `/spec:archive` may be the intended action
- Stop if the change is already finalized as `archived` or `abandoned`
