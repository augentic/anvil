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
