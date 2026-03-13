# Omnia New Spec Template

Use this template for crates listed under **New Crates** in the proposal.
This format matches what code-analyzer and epic-analyzer produce, and is
what crate-writer and test-writer expect as input.

```markdown
# <Crate Name> Specification

## Purpose

<1-2 sentence description of what this crate does>

### Requirement: <Behavior Name>

ID: REQ-001

The system SHALL <behavioral description>.

#### Scenario: <Happy Path>

- **WHEN** <trigger or input>
- **THEN** <expected behavior>

#### Scenario: <Error Case>

- **WHEN** <invalid input or failing condition>
- **THEN** <expected error behavior>

## Error Conditions

- <error type>: <description and trigger conditions>

## Metrics

- `<metric_name>` — type: <counter|gauge|histogram>; emitted: <when>
```

Repeat `### Requirement:` blocks for each distinct behavior in the crate, incrementing the `ID: REQ-XXX` line for each new requirement.
