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

2. **Check artifact completion**

   Check file existence for all artifacts:

   | Artifact | Complete when |
   |----------|---------------|
   | proposal | `.specify/changes/<name>/proposal.md` exists |
   | specs | `.specify/changes/<name>/specs/` contains at least one `.md` file (in any subdirectory) |
   | design | `.specify/changes/<name>/design.md` exists |
   | tasks | `.specify/changes/<name>/tasks.md` exists |

   **If any artifacts are missing:**
   - Display warning listing incomplete artifacts
   - Use **AskQuestion tool** to confirm user wants to proceed
   - Proceed if user confirms

3. **Check task completion**

   Read `tasks.md` and count:
   - `- [ ] ` lines = incomplete tasks
   - `- [x] ` or `- [X] ` lines = complete tasks

   **If incomplete tasks found:**
   - Display warning showing count of incomplete tasks
   - Use **AskQuestion tool** to confirm user wants to proceed
   - Proceed if user confirms

   **If no tasks file exists:** Proceed without task-related warning.

4. **Merge delta specs into baseline**

   For each subdirectory in `.specify/changes/<name>/specs/`:
   - The subdirectory name is the **capability name**
   - The file at `specs/<capability>/spec.md` is the **delta spec**
   - The baseline is at `.specify/specs/<capability>/spec.md`

   **For each capability with a delta spec**, perform the merge:

   a. **Read the delta spec** from `.specify/changes/<name>/specs/<capability>/spec.md`

   b. **Check if a baseline exists** at `.specify/specs/<capability>/spec.md`

   c. **If NO baseline exists** (new capability):
      - Create `.specify/specs/<capability>/` directory
      - Detect the spec format:
        - If the spec contains `## Handler:` sections (new crate format):
          copy the entire content as the new baseline directly — it is
          already in baseline format.
        - If the spec contains `## ADDED Requirements` (delta format):
          extract only the `### Requirement:` blocks and their content
          (description, scenarios), without the `## ADDED Requirements`
          header.
      - Write the result as the new baseline at `.specify/specs/<capability>/spec.md`
      - Ignore any MODIFIED/REMOVED/RENAMED sections (they don't apply to a new baseline)

   d. **If a baseline EXISTS** (existing capability):
      - Read the baseline from `.specify/specs/<capability>/spec.md`
      - Parse the delta spec to identify sections by their `## ` headers (case-insensitive matching):
        - `## ADDED Requirements`
        - `## MODIFIED Requirements`
        - `## REMOVED Requirements`
        - `## RENAMED Requirements`
      - Apply operations in **this exact order** (order matters):

      **Step 1 -- RENAMED** (must happen first so MODIFIED/REMOVED use new names):
      - Look for `FROM:` and `TO:` lines within the RENAMED section
      - For each pair, find the `### Requirement: <FROM name>` block in the baseline
      - Change its header to `### Requirement: <TO name>`
      - If the FROM name is not found in the baseline, report an error

      **Step 2 -- REMOVED**:
      - For each `### Requirement: <name>` in the REMOVED section, delete the entire matching block from the baseline (from the `### Requirement:` line through to the next `### Requirement:` line or end of file)
      - If the name is not found in the baseline, report an error

      **Step 3 -- MODIFIED**:
      - For each `### Requirement: <name>` in the MODIFIED section, find the matching block in the baseline and replace it entirely with the version from the delta (from the `### Requirement:` line through all its content)
      - If the name is not found in the baseline, report an error

      **Step 4 -- ADDED**:
      - Append each `### Requirement: <name>` block from the ADDED section to the end of the baseline

      - Write the merged result to `.specify/specs/<capability>/spec.md`

   e. **Verify the merge**: Re-read the merged baseline and confirm it looks structurally correct (has proper `### Requirement:` headers, no duplicate names, no orphaned content).

   **What is a requirement block?**
   A requirement block starts at a `### Requirement: <name>` line and includes all content until the next `### Requirement:` line or the next `## ` header or end of file. This includes the description text, all `#### Scenario:` sub-sections, and any other content within the block.

   **Preserve preamble**: Any text before the first `### Requirement:` or `## ` header in the baseline should be preserved as-is.

5. **Move the change to archive**

   ```bash
   mkdir -p .specify/changes/archive
   mv .specify/changes/<name> .specify/changes/archive/YYYY-MM-DD-<name>
   ```

   Use today's date in `YYYY-MM-DD` format.

6. **Display summary**

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
- Report errors if RENAMED/REMOVED/MODIFIED reference requirement names not found in the baseline
- After merging, verify the result by re-reading the merged file
- If the merge looks wrong, stop and ask the user before proceeding to the move step
