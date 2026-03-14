# Delta Merge

This document describes the merge algorithm used when archiving a change to promote delta specs into the baseline, along with a worked example.

## Merge Algorithm

The `scripts/merge-specs.py` tool implements this algorithm. The description is kept here for reference and as a fallback when the tool is unavailable.

**What is a requirement block?**
A requirement block starts at a requirement heading (as defined in `spec-format.requirement-heading`), includes the immediately following `ID:` line, and continues until the next requirement heading or the next `##` header or end of file. This includes the description text, all scenario sub-sections, and any other content within the block.

**Preserve preamble**: Any text before the first requirement heading or `##` header in the baseline is preserved as-is.

**New capability** (no baseline):

- If the delta contains no delta operation headers: copy verbatim as the new baseline
- If the delta contains delta operation headers: extract only requirement blocks from the ADDED section

**Existing capability** (baseline exists) -- apply operations in strict order:

1. **RENAMED**: For each `ID:` + `TO:` pair, find the matching requirement block by ID and update its heading. Preserve the `ID:` line.
2. **REMOVED**: For each requirement, delete the matching block by ID.
3. **MODIFIED**: For each requirement, replace the matching block by ID with the delta version.
4. **ADDED**: Append each requirement block to the end of the baseline. Error if the ID already exists.

Errors are reported for missing IDs (RENAMED/REMOVED/MODIFIED) or duplicate IDs (ADDED).

## Worked Example

Given this baseline at `.specify/specs/user-auth/spec.md`:

```markdown
### Requirement: Password login
ID: REQ-001
The system SHALL authenticate users via password.

#### Scenario: Successful login
- **WHEN** user submits valid credentials
- **THEN** session is created

### Requirement: Session timeout
ID: REQ-002
The system SHALL expire sessions after 30 minutes of inactivity.

#### Scenario: Idle timeout
- **WHEN** session is inactive for 30 minutes
- **THEN** session is invalidated
```

And this delta spec at `.specify/changes/add-oauth/specs/user-auth/spec.md`:

```markdown
## ADDED Requirements

### Requirement: OAuth login
ID: REQ-003
The system SHALL authenticate users via OAuth 2.0 providers.

#### Scenario: Google OAuth
- **WHEN** user clicks "Sign in with Google"
- **THEN** system redirects to Google OAuth and creates session on callback

## MODIFIED Requirements

### Requirement: Session timeout
ID: REQ-002
The system SHALL expire sessions after 60 minutes of inactivity.

#### Scenario: Idle timeout
- **WHEN** session is inactive for 60 minutes
- **THEN** session is invalidated

## REMOVED Requirements

### Requirement: Password login
ID: REQ-001
**Reason**: Replaced by OAuth authentication
**Migration**: Users should use OAuth providers instead
```

The merged baseline becomes:

```markdown
### Requirement: Session timeout
ID: REQ-002
The system SHALL expire sessions after 60 minutes of inactivity.

#### Scenario: Idle timeout
- **WHEN** session is inactive for 60 minutes
- **THEN** session is invalidated

### Requirement: OAuth login
ID: REQ-003
The system SHALL authenticate users via OAuth 2.0 providers.

#### Scenario: Google OAuth
- **WHEN** user clicks "Sign in with Google"
- **THEN** system redirects to Google OAuth and creates session on callback
```

(Password login was REMOVED; Session timeout was MODIFIED with new duration; OAuth login was ADDED at the end.)
