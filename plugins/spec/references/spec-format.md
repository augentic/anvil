# Spec Format

Hard-coded heading conventions used by all Specify skills for parsing and
generating requirement specs. These are not configurable per-schema.

## Requirement Blocks

- **Requirement heading**: `### Requirement:`
- **Requirement ID prefix**: `ID:`
- **Requirement ID pattern**: `^REQ-[0-9]{3}$`
- **Scenario heading**: `#### Scenario:`

A requirement block starts at a `### Requirement:` heading, includes the
immediately following `ID:` line, and continues until the next requirement
heading or `##` header or end of file.

## Delta Operations

Delta specs for modified capabilities use these top-level headings:

| Operation | Heading |
|-----------|---------|
| Added | `## ADDED Requirements` |
| Modified | `## MODIFIED Requirements` |
| Removed | `## REMOVED Requirements` |
| Renamed | `## RENAMED Requirements` |

### Delta Spec Template

For modified deliverables, the delta spec uses these sections (include only
sections that apply):

```markdown
## ADDED Requirements

### Requirement: <!-- requirement name -->
ID: REQ-<!-- next available id -->
<!-- requirement text -->

#### Scenario: <!-- scenario name -->
- **WHEN** <!-- condition -->
- **THEN** <!-- expected outcome -->

## MODIFIED Requirements

### Requirement: <!-- existing requirement name -->
ID: REQ-<!-- existing id (must match baseline) -->
<!-- full updated requirement text -->

#### Scenario: <!-- scenario name -->
- **WHEN** <!-- condition -->
- **THEN** <!-- expected outcome -->

## REMOVED Requirements

### Requirement: <!-- existing requirement name -->
ID: REQ-<!-- existing id -->
**Reason**: <!-- why this requirement is being removed -->
**Migration**: <!-- how to handle the removal -->

## RENAMED Requirements

ID: REQ-<!-- existing id -->
TO: <!-- new requirement name -->
```
