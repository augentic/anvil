---
name: drop
description: Drop a change without merging specs into the baseline. Use when the user wants to discard a change that should not be merged normally.
license: MIT
argument-hint: "[change-name?]"
---

# Drop

Drop a change without merging its specs into the baseline.

## Input

Optionally specify a change name. If omitted, check whether it can be inferred from conversation context. If vague or ambiguous, you MUST prompt for available changes.

## Process

1. **Select the change**

   If a name is provided, use it. Otherwise:
   - List directories in `.specify/changes/`, skipping `archive/`, looking for dirs with `.metadata.yaml`
   - If only one active change exists, use it but confirm with the user
   - If multiple, use the **AskQuestion tool** to let the user select

   **IMPORTANT**: Always confirm the change name before dropping it.

   Read `.specify/changes/<name>/.metadata.yaml` for the schema value and status. **Resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`.

2. **Check lifecycle status**

   Read `status` from `.metadata.yaml`:
   - If `status` is `complete`, warn that the change appears ready to merge normally
   - If `status` is `merged` or `dropped`, stop and tell the user the change is already finalized
   - For any other status, explain that dropping will discard the working change without promoting its specs

   Use the **AskQuestion tool** to confirm the user wants to drop the change.

3. **Summarize what will happen**

   Before making any file changes, display a short summary:

   ```text
   ## Drop Preview: <change-name>

   - Change status will be set to `dropped`
   - The change directory will move under `.specify/changes/archive/`
   - No specs will be merged into `.specify/specs/`
   - Existing baseline specs remain unchanged
   ```

   Use the **AskQuestion tool** to confirm:
   - **Proceed**: drop the change
   - **Cancel**: keep the change as-is

4. **Update metadata**

   Update `.specify/changes/<name>/.metadata.yaml`:
   - Set `status` to `dropped`
   - **Verify**: re-read `.metadata.yaml` and confirm the `status` value is exactly `dropped`. Valid lifecycle values are: `defining`, `defined`, `building`, `complete`, `merged`, `dropped`. If `status` is not one of these, correct it to `dropped`.

5. **Move the change to archive**

   ```bash
   mkdir -p .specify/changes/archive
   mv .specify/changes/<name> .specify/changes/archive/YYYY-MM-DD-<name>
   ```

   Use today's date in `YYYY-MM-DD` format.

6. **Display summary**

## Output On Success

```text
## Change Dropped

**Change:** <change-name>
**Archived to:** .specify/changes/archive/YYYY-MM-DD-<name>/

No specs were merged into `.specify/specs/`.
The baseline remains unchanged.
```

## Guardrails

- Always confirm the change before dropping it
- Do not merge or rewrite any files under `.specify/specs/`
- Warn if the change is already `complete`, since `/spec:merge` may be the intended action
- Stop if the change is already finalized as `merged` or `dropped`
- Valid lifecycle status values are: `defining`, `defined`, `building`, `complete`, `merged`, `dropped` -- use these exact strings when updating `.metadata.yaml`, no other values are permitted
