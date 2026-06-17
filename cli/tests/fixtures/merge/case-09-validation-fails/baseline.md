# Validation-fails baseline

Intentionally malformed — duplicate IDs, missing scenario, missing ID.

### Requirement: User can log in

ID: REQ-001

Authentication.

#### Scenario: Valid credentials

- GIVEN a registered user
- WHEN they submit correct credentials
- THEN they receive a session token

### Requirement: User can log out

ID: REQ-001

Duplicate ID above.

#### Scenario: Active session

- GIVEN an authenticated user
- WHEN they log out
- THEN the session is invalidated

### Requirement: Orphan requirement without ID line

Purposefully has no `ID:` line to exercise the missing-id check.

#### Scenario: Placeholder

- GIVEN anything
- WHEN validated
- THEN the missing-id error fires

### Requirement: Missing scenario

ID: REQ-004

No scenario heading below — exercises the missing-scenario check.
