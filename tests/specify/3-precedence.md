# Specification

Three bound sources disagree; authority resolves what it can and the
uncovered acceptance gap is preserved as [unknown], never guessed.

### Requirement: login.flow [divergence]

ID: REQ-001
Sources: [docs, code]
Status: divergence

Users sign in with a magic link; the observed email-and-password flow
is retained as commentary.

### Requirement: session.timeout [divergence]

ID: REQ-002
Sources: [intent, docs, code]
Status: divergence

Sessions must expire after 30 minutes of inactivity.

### Requirement: session.timeout acceptance criteria [unknown]

ID: REQ-003
Sources: []
Status: unknown

No source contributed an acceptance criterion for `session.timeout`;
the gap is preserved as [unknown], never guessed.
