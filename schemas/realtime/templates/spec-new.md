Use this template for capabilities listed under **New Capabilities** in the
proposal. This format matches what code-analyzer produces, and is what
crate-writer and test-writer expect as input.

```markdown
# <Capability Name> Specification

## Handler: <handler-name>

### Purpose

<1-2 sentence description of what this handler does>

### Requirements

#### Requirement: <Behavior Name>

The system SHALL <behavioral description>.

##### Scenario: <Happy Path>

- **WHEN** <trigger or input>
- **THEN** <expected behavior>

##### Scenario: <Error Case>

- **WHEN** <invalid input or failing condition>
- **THEN** <expected error behavior>

### Error Conditions

- <error type>: <description and trigger conditions>

### Metrics

- `<metric_name>` — type: <counter|gauge|histogram>; emitted: <when>
```

Repeat `## Handler:` sections for each handler in the capability.
