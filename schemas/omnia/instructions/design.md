> **Output location**: `.specify/changes/<change-name>/`
> All artifacts for this change are written under `.specify/changes/<change-name>/`.
> The `generates` path in schema.yaml is relative to this directory.
> Do NOT use `openspec/`, `temp/`, or any other directory convention.

Create the design document to explain HOW to implement the change.

## Output Structure

```markdown
## Context

<!-- Source, purpose, and background for this change -->

## Domain Model

<!-- Entity and type definitions with field names, types, wire names, and optionality -->

## API Contracts

<!-- Endpoints with method, path, request/response shapes, errors -->

## External Services

<!-- Name, type (API, table store, cache, message broker), authentication -->

## Constants & Configuration

<!-- All config keys with descriptions and defaults -->

## Business Logic

<!-- Per-handler tagged pseudocode ([domain], [infrastructure], [mechanical]) -->

## Publication & Timing Patterns

<!-- Topics, message shapes, timing, partition keys -->

## Implementation Constraints

<!-- Platform or runtime constraints relevant to generation -->

## Source Capabilities Summary

<!-- Checklist of required provider traits -->

## Dependencies

<!-- External packages or services this change depends on -->

## Risks / Open Questions

<!-- Known risks, trade-offs, and unresolved decisions -->

## Notes

<!-- Additional observations or considerations -->
```
