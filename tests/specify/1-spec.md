# Specification

One bound source: the mock component's minimal greeting profile,
reconciled with no disagreement and one acceptance gap.

### Requirement: greeting.behaviour

ID: REQ-001
Sources: [source]
Status: agreed

GET /greeting returns the static string 'hello'.

#### Scenario: Greeting requested

- **WHEN** `/greeting` is requested
- **THEN** the response is `hello`

### Requirement: greeting.behaviour acceptance criteria [unknown]

ID: REQ-002
Sources: []
Status: unknown

No source contributed an acceptance criterion for `greeting.behaviour`;
the gap is preserved as [unknown], never guessed.

#### Scenario: Greeting acceptance checked

- **WHEN** the greeting behaviour is checked
- **THEN** its acceptance criteria remain unspecified
