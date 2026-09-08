# Specification

Four bound sources disagree; authority resolves what it can, tied peers surface as [conflict] for the operator, and the uncovered acceptance gap is preserved, never guessed.

### Requirement: login.flow [conflict]

ID: REQ-001
Sources: [docs, wiki-live, code]
Status: conflict

Note: docs (documentation, login.flow): Users sign in with a magic link.
Note: wiki-live (documentation, login.flow): Users sign in with a passkey.
Note: code (behaviour, login.flow): Users sign in with email and password.
Note: Operator reconciliation required.

#### Scenario: Login selected

- **WHEN** a login method must be selected
- **THEN** operator reconciliation is required

### Requirement: session.timeout [divergence]

ID: REQ-002
Sources: [intent, docs, code]
Status: divergence

Sessions must expire after 30 minutes of inactivity.

Note: code (behaviour, session-expiry): Sessions expire after 15 minutes of inactivity.
Note: acceptance criteria not evidenced.

#### Scenario: Session expires

- **WHEN** a session is inactive for 30 minutes
- **THEN** the session expires
