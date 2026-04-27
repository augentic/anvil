---
id: design
description: Create the design document to explain HOW to implement the change
generates: design.md
needs: [proposal, contracts]
---

## Output Structure

```markdown
## Context

<!-- Source, purpose, and background for this change -->

## Domain Model

<!-- Entity and type definitions with field names, types, wire names, and optionality -->

## API Contracts

<!-- When `.specify/contracts/http/` exists: reference the OpenAPI specifications
     there rather than re-describing endpoint shapes. Add implementation-level notes
     not captured in the contract: auth schemes, rate limits, caching, versioning strategy.
     
     When no baseline contracts exist: endpoints with method, path,
     request/response shapes, errors (the existing behavior). -->

## External Services

<!-- Name, type (API, table store, cache, message broker), authentication -->

## Constants & Configuration

<!-- All config keys with descriptions and defaults -->

## Business Logic

<!-- Per-handler tagged pseudocode ([domain], [infrastructure], [mechanical]) -->

## Publication & Timing Patterns

<!-- When `.specify/contracts/messages/` exists: reference the AsyncAPI specifications
     there rather than re-describing message shapes. Add implementation-level notes
     not captured in the contract: ordering guarantees, retry policies, DLQ strategy.
     
     When no baseline contracts exist: topics, message shapes, timing,
     partition keys (the existing behavior). -->

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
