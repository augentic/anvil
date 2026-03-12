---
name: archive
description: Archive a completed change. Use when the user wants to finalize and archive a change after implementation is complete.
license: MIT
compatibility: Requires specify CLI.
metadata:
  author: specify
  version: "1.1"
  generatedBy: "1.2.0"
---

Archive a completed change.

**Input**: Optionally specify a change name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available changes.

**Steps**

1. **If no change name provided, prompt for selection**

   Run `specify list --json` to get available changes. Use the **AskUserQuestion tool** to let the user select.

   Show only active changes (not already archived).
   Include the schema used for each change if available.

   **IMPORTANT**: Do NOT guess or auto-select a change. Always let the user choose.

2. **Check artifact completion status**

   Run `specify status "<name>" --json` to check artifact completion.

   Parse the JSON to understand:
   - `schemaName`: The workflow being used
   - `artifacts`: List of artifacts with their status (`done` or other)

   **If any artifacts are not `done`:**
   - Display warning listing incomplete artifacts
   - Use **AskUserQuestion tool** to confirm user wants to proceed
   - Proceed if user confirms

3. **Check task completion status**

   Read the tasks file (typically `tasks.md`) to check for incomplete tasks.

   Count tasks marked with `- [ ]` (incomplete) vs `- [x]` (complete).

   **If incomplete tasks found:**
   - Display warning showing count of incomplete tasks
   - Use **AskUserQuestion tool** to confirm user wants to proceed
   - Proceed if user confirms

   **If no tasks file exists:** Proceed without task-related warning.

4. **Run the archive**

   ```bash
   specify archive "<name>"
   ```

   The CLI handles delta spec sync and moving the change to the archive
   directory. If the CLI prompts for sync confirmation, relay the prompt
   to the user via **AskUserQuestion tool**.

   If the CLI is not available or the command fails, fall back to manual
   archival:
   ```bash
   mkdir -p .specify/changes/archive
   mv .specify/changes/<name> .specify/changes/archive/YYYY-MM-DD-<name>
   ```

5. **Display summary**

   Show archive completion summary including:
   - Change name
   - Schema that was used
   - Archive location
   - Whether specs were synced (if applicable)
   - Note about any warnings (incomplete artifacts/tasks)

**Output On Success**

```
## Archive Complete

**Change:** <change-name>
**Schema:** <schema-name>
**Archived to:** .specify/changes/archive/YYYY-MM-DD-<name>/
**Specs:** Synced to main specs (or "No delta specs" or "Sync skipped")

All artifacts complete. All tasks complete.
```

**Guardrails**
- Always prompt for change selection if not provided
- Use artifact graph (specify status --json) for completion checking
- Don't block archive on warnings - just inform and confirm
- Show clear summary of what happened
