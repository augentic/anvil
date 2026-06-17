# All-sections baseline

Baseline exercising all four delta operations in one merge.

### Requirement: User authenticates with email and password

ID: REQ-001

Authentication via email and password.

#### Scenario: Valid credentials

- GIVEN a registered user
- WHEN they submit correct credentials
- THEN they receive a session token

### Requirement: User can log out

ID: REQ-002

Session invalidation and audit log on logout.

#### Scenario: Active session

- GIVEN an authenticated user
- WHEN they log out
- THEN the session is invalidated

#### Scenario: Audit log

- GIVEN a logout event
- WHEN logout completes
- THEN an audit-log entry records the user and timestamp

### Requirement: Refresh expired tokens

ID: REQ-004

Refresh access tokens before expiry.

#### Scenario: Near-expiry token

- GIVEN a token within 60s of expiry
- WHEN a request is made
- THEN the refresh flow runs transparently
