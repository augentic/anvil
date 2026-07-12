# Login Specification

## Purpose

Issue session tokens for authenticated users.

### Requirement: Issue session token

ID: REQ-001

The system SHALL issue a signed session token when a valid username and
password are presented.

#### Scenario: Valid credentials

- **WHEN** a user submits matching credentials
- **THEN** a session token is returned
