---
name: archive
description: Archive a completed change. Merges delta specs into baseline and moves the change to the archive. Use when the user wants to finalize a change after implementation is complete.
license: MIT
---

# Archive

Archive a completed change.

## Input

Optionally specify a change name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available changes.

## Steps

1. **Select the change**

   If a name is provided, use it. Otherwise:
   - List directories in `.specify/changes/`, skipping `archive/`, looking for dirs with `.metadata.yaml`
   - If only one active change exists, use it but confirm with the user
   - If multiple, use the **AskQuestion tool** to let the user select

   **IMPORTANT**: Always confirm the change name before archiving.

   Read `.specify/changes/<name>/.metadata.yaml` for the schema value and status. **Resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`.

   Read `schema.yaml` for artifact definitions, `spec-format` heading conventions, and `terminology.unit` (e.g., "crate" vs "capability"). Infer plural and heading forms from the unit name. Use schema terminology in summary output.

2. **Check lifecycle status**

   Read `status` from `.metadata.yaml`:
   - If `status` is not `complete`: display warning (e.g., "This change has status `<status>` — it may not be fully implemented.")
   - Use **AskQuestion tool** to confirm user wants to proceed despite the status
   - Proceed if user confirms

3. **Check artifact completion**

   For each artifact defined in `schema.yaml`, check whether it is complete:
   - If `generates` is a simple filename (e.g., `proposal.md`), check if `.specify/changes/<name>/<generates>` exists.
   - If `generates` is a glob pattern (e.g., `specs/**/*.md`), check if the directory contains at least one matching `.md` file.

   **If any artifacts are missing:**
   - Display warning listing incomplete artifacts
   - Use **AskQuestion tool** to confirm user wants to proceed
   - Proceed if user confirms

4. **Check task completion**

   Read the file tracked by `apply.tracks` (from `schema.yaml`) and count:
   - `- [ ]` lines = incomplete tasks
   - `- [x]` or `- [X]` lines = complete tasks

   **If incomplete tasks found:**
   - Display warning showing count of incomplete tasks
   - Use **AskQuestion tool** to confirm user wants to proceed
   - Proceed if user confirms

   **If no tasks file exists:** Proceed without task-related warning.

5. **Preview merge operations**

   For each subdirectory in `.specify/changes/<name>/specs/`:
   - The subdirectory name is the **capability name**
   - The file at `specs/<capability>/spec.md` is the **delta spec**
   - The baseline is at `.specify/specs/<capability>/spec.md`

   Read the `spec-format` section from `schema.yaml` for heading conventions:
   - `delta-operations.added`, `delta-operations.modified`, `delta-operations.removed`, `delta-operations.renamed` — the headings used in delta specs
   - `requirement-heading` — the heading prefix for requirement blocks (e.g., `### Requirement:`)
   - `requirement-id-prefix` — the stable requirement ID line prefix (e.g., `ID:`)

   For each capability with a delta spec, show what will happen WITHOUT performing the merge:

   ```text
   ## Archive Preview: <change-name>

   ### <capability-1>/spec.md (existing baseline)
   - REMOVING: REQ-001 — <name>
   - MODIFYING: REQ-002 — <name>
   - ADDING: REQ-003 — <name>

   ### <capability-2>/spec.md (new baseline)
   - Creating new baseline with N requirements
   ```

   **Conflict detection**: For each capability with `type: modified` in `.metadata.yaml`'s `touched_specs` (if present), check if `.specify/specs/<capability>/spec.md` has been modified since `proposed_at` (compare file modification time). If the baseline has changed since the change was proposed:
   - Warn: "The baseline for `<capability>` has been modified since this change was proposed (possibly by archiving another change)."
   - Use **AskQuestion tool**: proceed anyway, or cancel

   Use the **AskQuestion tool** to confirm:
   - **Proceed**: apply all merges
   - **Show full content**: display the complete merged baseline for each capability before writing
   - **Cancel**: abort archive

   Only proceed to the actual merge after user confirms.

6. **Merge delta specs into baseline**

   **For each capability with a delta spec**, invoke the deterministic merge tool:

   a. **Set paths**:
      - `SCHEMA` = resolved `schema.yaml` path
      - `DELTA` = `.specify/changes/<name>/specs/<capability>/spec.md`
      - `BASELINE` = `.specify/specs/<capability>/spec.md` (may not exist yet)
      - `OUTPUT` = same as `BASELINE`

   b. **If NO baseline exists** (new capability): create the `.specify/specs/<capability>/` directory.

   c. **Run the merge tool**:

      ```bash
      python3 scripts/merge-specs.py \
        --schema "$SCHEMA" \
        --delta "$DELTA" \
        --baseline "$BASELINE" \
        --output "$OUTPUT"
      ```

      If the baseline does not exist yet, omit `--baseline` and the tool creates a new baseline from the delta's ADDED section (or copies the delta verbatim when it has no delta operation headers).

   d. **Check the exit code**:
      - Exit 0: merge succeeded. Proceed to the next capability.
      - Exit 1: merge failed. Display the error messages from stderr and stop. Use the **AskQuestion tool** to let the user decide whether to fix the delta and retry, or abort the archive.

7. **Baseline coherence check**

   After all merges complete, validate every spec file that was created or updated. For each file, run:

   ```bash
   python3 scripts/merge-specs.py \
     --schema "$SCHEMA" \
     --validate ".specify/specs/<capability>/spec.md" \
     --design ".specify/changes/<name>/design.md"
   ```

   Omit `--design` if `design.md` does not exist.

   The tool checks: no duplicate IDs, no duplicate requirement names, valid heading structure (ID line after each requirement heading, ID matches pattern, at least one scenario per requirement), and no orphaned design references.

   **If any check fails** (exit code 1):
   - Display the failures from stderr
   - Use the **AskQuestion tool**:
     - **Proceed anyway**: continue to archive despite the issues
     - **Abort**: leave the change in its current directory for manual correction (the merged baseline files are already written; the user can edit them before re-running archive)

   Only proceed to step 8 after user confirms.

8. **Update metadata and move to archive**

   Update `.specify/changes/<name>/.metadata.yaml`:
   - Set `status` to `archived`

   ```bash
   mkdir -p .specify/changes/archive
   mv .specify/changes/<name> .specify/changes/archive/YYYY-MM-DD-<name>
   ```

   Use today's date in `YYYY-MM-DD` format.

9. **Display summary**

## Output On Success

```text
## Archive Complete

**Change:** <change-name>
**Archived to:** .specify/changes/archive/YYYY-MM-DD-<name>/

### Specs Merged
- <capability-1>: merged into .specify/specs/<capability-1>/spec.md
- <capability-2>: new baseline created at .specify/specs/<capability-2>/spec.md

(or "No delta specs to merge" if specs/ was empty)

All artifacts complete. All tasks complete.
```

## Guardrails

- Always confirm the change before archiving
- Warn on incomplete artifacts or tasks but don't block
- Use `scripts/merge-specs.py` for all merge and validation operations — do not perform merges inline
- If the merge tool is unavailable (e.g., `python3` not installed), fall back to manual merge following the algorithm in `references/delta-merge.md`
- If the merge tool reports errors, stop and ask the user before proceeding

For the merge algorithm and a worked example, see `references/delta-merge.md`.
