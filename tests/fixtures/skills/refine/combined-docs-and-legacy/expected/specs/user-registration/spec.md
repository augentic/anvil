# Identity user registration

## Overview

The registration endpoint accepts an email + password payload, validates the email per RFC-5322, persists the user on success, and returns a structured `400` for invalid input.

### Requirement: Registration accepts RFC-5322-valid emails

ID: REQ-001
Sources: [identity-design-notes, legacy-monolith]
Status: agreed

The registration endpoint accepts an email address that is RFC-5322 valid and rejects all others with a `400` response carrying the body `{ "error": "invalid-email" }`.

#### Scenario: Valid email accepted

- **WHEN** a registration request arrives with an RFC-5322-valid email
- **THEN** the handler persists the user and returns `201` with the persisted record

#### Scenario: Invalid email rejected

- **WHEN** a registration request arrives with an email that is not RFC-5322 valid
- **THEN** the handler returns `400` with the body `{ "error": "invalid-email" }` and does not persist
