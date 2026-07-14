# Modify REQ-001

## MODIFIED Requirements

### Requirement: User can log in

ID: REQ-001

Authentication via email/password *or* passkey.

#### Scenario: Valid credentials

- GIVEN a registered user
- WHEN they submit correct credentials
- THEN they receive a session token

#### Scenario: Passkey login

- GIVEN a registered user with a passkey
- WHEN they authenticate via passkey
- THEN they receive a session token
