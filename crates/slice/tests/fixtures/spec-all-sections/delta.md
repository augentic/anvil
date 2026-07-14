# All-sections delta

## RENAMED Requirements

ID: REQ-001
TO: User authenticates with email and password

## REMOVED Requirements

### Requirement: Legacy SSO path

ID: REQ-003

Deprecated in favour of OIDC.

## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Refresh expired tokens

ID: REQ-004

Refresh access tokens before expiry.

#### Scenario: Near-expiry token

- GIVEN a token within 60s of expiry
- WHEN a request is made
- THEN the refresh flow runs transparently
