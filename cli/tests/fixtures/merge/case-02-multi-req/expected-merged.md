# Multi-requirement baseline

Exercises parser grouping across several requirements.

### Requirement: User can log in

ID: REQ-001

Authentication via email and password.

#### Scenario: Valid credentials

- GIVEN a registered user
- WHEN they submit correct credentials
- THEN they receive a session token

### Requirement: User can log out

ID: REQ-002

Session invalidation on logout.

#### Scenario: Active session

- GIVEN an authenticated user
- WHEN they log out
- THEN the session is invalidated

### Requirement: User can reset password

ID: REQ-003

Self-service password reset flow.

#### Scenario: Reset request

- GIVEN a registered user
- WHEN they request a reset
- THEN they receive a reset link by email
