# Specification

One bound source: the mock component's minimal greeting profile, reconciled with no disagreement and one acceptance gap.

### Requirement: greeting.behaviour [unknown]

ID: REQ-001
Sources: [source]
Status: unknown

GET /greeting returns the static string 'hello'.

Note: acceptance criteria not evidenced.

#### Scenario: Greeting requested

- **WHEN** `/greeting` is requested
- **THEN** the response is `hello`
