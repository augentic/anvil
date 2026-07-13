# Removed-case baseline

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
