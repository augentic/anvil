# Identity user registration

## Overview

The registration endpoint accepts an email + password payload, validates the email per RFC-5322, persists the user on success, and returns a structured `400` for invalid input.

### Requirement: Registration accepts RFC-5322-valid emails

ID: REQ-001
Sources: [identity-design-notes, legacy-monolith]
Status: agreed

The registration endpoint accepts an email address that is RFC-5322 valid and rejects all others with a `400` response carrying the body `{ "error": "invalid-email" }`.

## Scenarios

Given a registration request whose email field is RFC-5322 valid, the handler persists the user (REQ-001) and returns `201` with the persisted record.

Given a registration request whose email field is not RFC-5322 valid, the handler returns `400` with the body `{ "error": "invalid-email" }` and does not persist (REQ-001).
