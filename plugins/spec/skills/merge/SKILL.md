---
name: merge
description: Merge a completed change. Merges delta specs into baseline and moves the change to the archive. Use when the user wants to finalize a change after implementation is complete.
license: MIT
argument-hint: "[change-name?]"
---

# Merge

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

   Read `.specify/changes/<name>/.metadata.yaml` for the schema value and status. **Resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`, `briefs/merge.md`.

   Read `schema.yaml` for pipeline definitions. Read the merge brief's frontmatter for `needs`. Read `references/spec-format.md` for heading conventions.

2. **Validate merge needs**

   Read the merge brief's frontmatter `needs` field (e.g., `[build]`). For each needed ID, verify the condition is met:
   - For `build`: check that `.metadata.yaml` status is `complete` or that all tasks are done (all checkboxes marked `[x]` or `[X]`)

   **If any needs are not met:**
   - Display warning listing which needs are unsatisfied and why
   - Use **AskQuestion tool** to let the user choose to proceed or abort
   - Proceed only if user confirms

3. **Check lifecycle status**

   Read `status` from `.metadata.yaml`:
   - If `status` is not `complete`: display warning (e.g., "This change has status `<status>` — it may not be fully implemented.")
   - Use **AskQuestion tool** to confirm user wants to proceed despite the status
   - Proceed if user confirms

4. **Check artifact completion**

   For each brief in `pipeline.define` entries from `schema.yaml`, read the brief's frontmatter `generates` field and check whether it is complete:
   - If `generates` is a simple filename (e.g., `proposal.md`), check if `.specify/changes/<name>/<generates>` exists.
   - If `generates` is a glob pattern (e.g., `specs/**/*.md`), check if the directory contains at least one matching `.md` file.

   **If any artifacts are missing:**
   - Display warning listing incomplete artifacts
   - Use **AskQuestion tool** to confirm user wants to proceed
   - Proceed if user confirms

5. **Check task completion**

   Read the build brief (from `pipeline.build` entry) and check its frontmatter `tracks` field to determine which file tracks build progress. Read that file and count:
   - `- [ ]` lines = incomplete tasks
   - `- [x]` or `- [X]` lines = complete tasks

   **If incomplete tasks found:**
   - Display warning showing count of incomplete tasks
   - Use **AskQuestion tool** to confirm user wants to proceed
   - Proceed if user confirms

   **If no tasks file exists:** Proceed without task-related warning.

6. **Preview merge operations**

   For each subdirectory in `.specify/changes/<name>/specs/`:
   - The subdirectory name is the **capability name**
   - The file at `specs/<capability>/spec.md` is the **delta spec**
   - The baseline is at `.specify/specs/<capability>/spec.md`

   Read `references/spec-format.md` for heading conventions (requirement headings, ID prefix, delta operation headings).

   For each capability with a delta spec, show what will happen WITHOUT performing the merge:

   ```text
   ## Merge Preview: <change-name>

   ### <capability-1>/spec.md (existing baseline)
   - REMOVING: REQ-001 — <name>
   - MODIFYING: REQ-002 — <name>
   - ADDING: REQ-003 — <name>

   ### <capability-2>/spec.md (new baseline)
   - Creating new baseline with N requirements
   ```

   **Conflict detection**: For each capability with `type: modified` in `.metadata.yaml`'s `touched_specs` (if present), check if `.specify/specs/<capability>/spec.md` has been modified since `defined_at` (compare file modification time). If the baseline has changed since the change was defined:
   - Warn: "The baseline for `<capability>` has been modified since this change was defined (possibly by merging another change)."
   - Use **AskQuestion tool**: proceed anyway, or cancel

   Use the **AskQuestion tool** to confirm:
   - **Proceed**: apply all merges
   - **Show full content**: display the complete merged baseline for each capability before writing
   - **Cancel**: abort merge

   Only proceed to the actual merge after user confirms.

7. **Merge delta specs into baseline**

   **For each capability with a delta spec**, invoke the deterministic merge tool:

   a. **Set paths**:
      - `SCHEMA` = resolved `schema.yaml` path
      - `DELTA` = `.specify/changes/<name>/specs/<capability>/spec.md`
      - `BASELINE` = `.specify/specs/<capability>/spec.md` (may not exist yet)
      - `OUTPUT` = same as `BASELINE`

   b. **If NO baseline exists** (new capability): create the `.specify/specs/<capability>/` directory.

   c. **Run the merge tool** (co-located at `scripts/merge-specs.py` relative to this skill):

      ```bash
      python3 scripts/merge-specs.py \
        --delta "$DELTA" \
        --baseline "$BASELINE" \
        --output "$OUTPUT"
      ```

      If the baseline does not exist yet, omit `--baseline` and the tool creates a new baseline from the delta's ADDED section (or copies the delta verbatim when it has no delta operation headers).

   d. **Check the exit code**:
      - Exit 0: merge succeeded. Proceed to the next capability.
      - Exit 1: merge failed. Display the error messages from stderr and stop. Use the **AskQuestion tool** to let the user decide whether to fix the delta and retry, or abort the merge.

8. **Baseline coherence check**

   After all merges complete, validate every spec file that was created or updated. For each file, run:

   ```bash
   python3 scripts/merge-specs.py \
     --validate ".specify/specs/<capability>/spec.md" \
     --design ".specify/changes/<name>/design.md"
   ```

   Omit `--design` if `design.md` does not exist.

   The tool checks: no duplicate IDs, no duplicate requirement names, valid heading structure (ID line after each requirement heading, ID matches pattern, at least one scenario per requirement), and no orphaned design references.

   **If any check fails** (exit code 1):
   - Display the failures from stderr
   - Use the **AskQuestion tool**:
     - **Proceed anyway**: continue to merge despite the issues
     - **Abort**: leave the change in its current directory for manual correction (the merged baseline files are already written; the user can edit them before re-running merge)

   Only proceed to step 9 after user confirms.

9. **Update metadata and move to archive**

   Update `.specify/changes/<name>/.metadata.yaml`:
   - Set `status` to `merged`
   - **Verify**: re-read `.metadata.yaml` and confirm the `status` value is exactly `merged`. Valid lifecycle values are: `defining`, `defined`, `building`, `complete`, `merged`, `dropped`. If `status` is not one of these, correct it to `merged`.

   ```bash
   mkdir -p .specify/changes/archive
   mv .specify/changes/<name> .specify/changes/archive/YYYY-MM-DD-<name>
   ```

   Use today's date in `YYYY-MM-DD` format.

10. **Display summary**

## Output On Success

```text
## Merge Complete

**Change:** <change-name>
**Merged to:** .specify/changes/archive/YYYY-MM-DD-<name>/

### Specs Merged
- <capability-1>: merged into .specify/specs/<capability-1>/spec.md
- <capability-2>: new baseline created at .specify/specs/<capability-2>/spec.md

(or "No delta specs to merge" if specs/ was empty)

All artifacts complete. All tasks complete.
```

## Guardrails

- Always confirm the change before merging
- Validate merge needs before proceeding — warn but don't block if needs are unmet
- Warn on incomplete artifacts or tasks but don't block
- Use `scripts/merge-specs.py` for all merge and validation operations — do not perform merges inline
- If the merge tool is unavailable (e.g., `python3` not installed), fall back to manual merge following the algorithm in `delta-merge.md`
- If the merge tool reports errors, stop and ask the user before proceeding
- Valid lifecycle status values are: `defining`, `defined`, `building`, `complete`, `merged`, `dropped` -- use these exact strings when updating `.metadata.yaml`, no other values are permitted

For the merge algorithm and a worked example, see `delta-merge.md`.
