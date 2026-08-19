# Specification

Reconciled from the bound sources under authority precedence
(`intent > documentation > behaviour`); disagreements stay inline.

### Requirement: login.flow [divergence]

ID: REQ-001
Sources: [mock-docs, mock-code]
Status: divergence

Users sign in with an email address and password (`mock-docs`,
documentation — winner). The observed handler validates credentials and
issues a session token (`mock-code`, behaviour) — documentation outranks
behaviour; the observed shape is commentary.

#### Scenario: Lockout

- **WHEN** five consecutive sign-in attempts fail
- **THEN** the account locks for fifteen minutes

### Requirement: session.timeout [divergence]

ID: REQ-002
Sources: [mock-intent, mock-docs, mock-code]
Status: divergence

Sessions must expire after 30 minutes of inactivity (`mock-intent`,
operator directive — winner). Documentation now states a 45-minute
expiry (`mock-docs`); observed behaviour expires sessions after 15
minutes (`mock-code`) — intent outranks both.

### Requirement: session.timeout acceptance criteria [unknown]

ID: REQ-003
Sources: []
Status: unknown

No source contributed an acceptance criterion for `session.timeout`;
the gap is preserved as [unknown], never guessed.
