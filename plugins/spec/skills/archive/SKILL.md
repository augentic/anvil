---
name: archive
description: Archive a completed change. Merges delta specs into baseline and moves the change to the archive. Use when the user wants to finalize a change after implementation is complete.
license: MIT
---

Archive a completed change.

**Input**: Optionally specify a change name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available changes.

**Steps**

1. **Select the change**

   If a name is provided, use it. Otherwise:
   - List directories in `.specify/changes/`, skipping `archive/`, looking for dirs with `.metadata.yaml`
   - If only one active change exists, use it but confirm with the user
   - If multiple, use the **AskQuestion tool** to let the user select

   **IMPORTANT**: Always confirm the change name before archiving.

   Read `.specify/changes/<name>/.metadata.yaml` for the schema value and status. **Resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`.

   Read `schema.yaml` for artifact definitions, `spec_format` heading conventions, and terminology (e.g., "Crates" vs "Capabilities"). Use schema terminology in summary output.

2. **Check lifecycle status**

   Read `status` from `.metadata.yaml`:
   - If `status` is not `complete`: display warning (e.g., "This change has status '<status>' — it may not be fully implemented.")
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
   - `- [ ] ` lines = incomplete tasks
   - `- [x] ` or `- [X] ` lines = complete tasks

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

   Read the `spec_format` section from `schema.yaml` for heading conventions:
   - `delta_operations.added`, `delta_operations.modified`, `delta_operations.removed`, `delta_operations.renamed` — the headings used in delta specs
   - `requirement_heading` — the heading prefix for requirement blocks (e.g., `### Requirement:`)

   For each capability with a delta spec, show what will happen WITHOUT performing the merge:

   ```
   ## Archive Preview: <change-name>

   ### <capability-1>/spec.md (existing baseline)
   - REMOVING: Requirement: <name>
   - MODIFYING: Requirement: <name>
   - ADDING: Requirement: <name>

   ### <capability-2>/spec.md (new baseline)
   - Creating new baseline with N requirements
   ```

   **Conflict detection**: For each capability with `type: modified` in `.metadata.yaml`'s `touched_capabilities` (if present), check if `.specify/specs/<capability>/spec.md` has been modified since `proposed_at` (compare file modification time). If the baseline has changed since the change was proposed:
   - Warn: "The baseline for `<capability>` has been modified since this change was proposed (possibly by archiving another change)."
   - Use **AskQuestion tool**: proceed anyway, or cancel

   Use the **AskQuestion tool** to confirm:
   - **Proceed**: apply all merges
   - **Show full content**: display the complete merged baseline for each capability before writing
   - **Cancel**: abort archive

   Only proceed to the actual merge after user confirms.

6. **Merge delta specs into baseline**

   **For each capability with a delta spec**, perform the merge:

   a. **Read the delta spec** from `.specify/changes/<name>/specs/<capability>/spec.md`

   b. **Check if a baseline exists** at `.specify/specs/<capability>/spec.md`

   c. **If NO baseline exists** (new capability):
      - Create `.specify/specs/<capability>/` directory
      - Check whether the spec contains any delta operation headers (the headings from `spec_format.delta_operations` in `schema.yaml`)
      - If the spec does NOT contain delta operation headers: copy the entire content as the new baseline directly — it is already in baseline format
      - If the spec DOES contain delta operation headers: extract only the requirement blocks (matching `spec_format.requirement_heading`) from the ADDED section and write them as the baseline. Ignore MODIFIED/REMOVED/RENAMED sections (they don't apply to a new baseline).
      - Write the result as the new baseline at `.specify/specs/<capability>/spec.md`

   d. **If a baseline EXISTS** (existing capability):
      - Read the baseline from `.specify/specs/<capability>/spec.md`
      - Parse the delta spec to identify sections by the headings defined in `schema.yaml`'s `spec_format.delta_operations` (case-insensitive matching)
      - Apply operations in **this exact order** (order matters):

      **Step 1 -- RENAMED** (must happen first so MODIFIED/REMOVED use new names):
      - Look for `FROM:` and `TO:` lines within the RENAMED section
      - For each pair, find the matching requirement block (using `spec_format.requirement_heading`) in the baseline
      - Change its header to use the TO name
      - If the FROM name is not found in the baseline, report an error

      **Step 2 -- REMOVED**:
      - For each requirement in the REMOVED section, delete the entire matching block from the baseline (from the requirement heading through to the next requirement heading or end of file)
      - If the name is not found in the baseline, report an error

      **Step 3 -- MODIFIED**:
      - For each requirement in the MODIFIED section, find the matching block in the baseline and replace it entirely with the version from the delta
      - If the name is not found in the baseline, report an error

      **Step 4 -- ADDED**:
      - Append each requirement block from the ADDED section to the end of the baseline

      - Write the merged result to `.specify/specs/<capability>/spec.md`

   e. **Verify the merge**: Re-read the merged baseline and confirm it looks structurally correct (has proper requirement headings, no duplicate names, no orphaned content).

   **What is a requirement block?**
   A requirement block starts at a requirement heading (as defined in `spec_format.requirement_heading`) and includes all content until the next requirement heading or the next `## ` header or end of file. This includes the description text, all scenario sub-sections, and any other content within the block.

   **Preserve preamble**: Any text before the first requirement heading or `## ` header in the baseline should be preserved as-is.

7. **Update metadata and move to archive**

   Update `.specify/changes/<name>/.metadata.yaml`:
   - Set `status` to `archived`

   ```bash
   mkdir -p .specify/changes/archive
   mv .specify/changes/<name> .specify/changes/archive/YYYY-MM-DD-<name>
   ```

   Use today's date in `YYYY-MM-DD` format.

8. **Display summary**

**Output On Success**

```
## Archive Complete

**Change:** <change-name>
**Archived to:** .specify/changes/archive/YYYY-MM-DD-<name>/

### Specs Merged
- <capability-1>: merged into .specify/specs/<capability-1>/spec.md
- <capability-2>: new baseline created at .specify/specs/<capability-2>/spec.md

(or "No delta specs to merge" if specs/ was empty)

All artifacts complete. All tasks complete.
```

**Delta Merge Example**

Given this baseline at `.specify/specs/user-auth/spec.md`:
```markdown
### Requirement: Password login
The system SHALL authenticate users via password.

#### Scenario: Successful login
- **WHEN** user submits valid credentials
- **THEN** session is created

### Requirement: Session timeout
The system SHALL expire sessions after 30 minutes of inactivity.

#### Scenario: Idle timeout
- **WHEN** session is inactive for 30 minutes
- **THEN** session is invalidated
```

And this delta spec at `.specify/changes/add-oauth/specs/user-auth/spec.md`:
```markdown
## ADDED Requirements

### Requirement: OAuth login
The system SHALL authenticate users via OAuth 2.0 providers.

#### Scenario: Google OAuth
- **WHEN** user clicks "Sign in with Google"
- **THEN** system redirects to Google OAuth and creates session on callback

## MODIFIED Requirements

### Requirement: Session timeout
The system SHALL expire sessions after 60 minutes of inactivity.

#### Scenario: Idle timeout
- **WHEN** session is inactive for 60 minutes
- **THEN** session is invalidated

## REMOVED Requirements

### Requirement: Password login
**Reason**: Replaced by OAuth authentication
**Migration**: Users should use OAuth providers instead
```

The merged baseline becomes:
```markdown
### Requirement: Session timeout
The system SHALL expire sessions after 60 minutes of inactivity.

#### Scenario: Idle timeout
- **WHEN** session is inactive for 60 minutes
- **THEN** session is invalidated

### Requirement: OAuth login
The system SHALL authenticate users via OAuth 2.0 providers.

#### Scenario: Google OAuth
- **WHEN** user clicks "Sign in with Google"
- **THEN** system redirects to Google OAuth and creates session on callback
```

(Password login was REMOVED; Session timeout was MODIFIED with new duration; OAuth login was ADDED at the end.)

**Guardrails**
- Always confirm the change before archiving
- Warn on incomplete artifacts or tasks but don't block
- Apply delta operations in strict order: RENAMED -> REMOVED -> MODIFIED -> ADDED
- Use heading conventions from `schema.yaml`'s `spec_format` — do not hard-code heading patterns
- Report errors if RENAMED/REMOVED/MODIFIED reference requirement names not found in the baseline
- After merging, verify the result by re-reading the merged file
- If the merge looks wrong, stop and ask the user before proceeding to the move step
