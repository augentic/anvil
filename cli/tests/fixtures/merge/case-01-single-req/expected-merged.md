# Single-requirement baseline

A minimal spec with one requirement to exercise the parser.

### Requirement: User can log in

ID: REQ-001

The system supports authenticating a user by email and password.

#### Scenario: Valid credentials

- GIVEN a registered user
- WHEN they submit the correct email and password
- THEN they receive a session token

#### Scenario: Invalid credentials

- GIVEN a registered user
- WHEN they submit the wrong password
- THEN the system rejects the attempt
