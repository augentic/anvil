# Specification

Four bound sources disagree; authority resolves what it can, tied
peers surface as [conflict] for the operator, and the uncovered
acceptance gap is preserved as [unknown], never guessed.

### Requirement: login.flow [conflict]

ID: REQ-001
Sources: [docs, wiki-live, code]
Status: conflict

The documentation peers disagree — magic link versus passkey — and no
higher authority resolves them; the operator must reconcile.

#### Scenario: Login selected

- **WHEN** a login method must be selected
- **THEN** operator reconciliation is required

### Requirement: session.timeout [divergence]

ID: REQ-002
Sources: [intent, docs, code]
Status: divergence

Sessions must expire after 30 minutes of inactivity.

#### Scenario: Session expires

- **WHEN** a session is inactive for 30 minutes
- **THEN** the session expires

### Requirement: session.timeout acceptance criteria [unknown]

ID: REQ-003
Sources: []
Status: unknown

No source contributed an acceptance criterion for `session.timeout`;
the gap is preserved as [unknown], never guessed.

#### Scenario: Session acceptance checked

- **WHEN** the timeout behaviour is checked
- **THEN** its acceptance criteria remain unspecified
