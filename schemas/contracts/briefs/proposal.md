---
id: proposal
description: Establish the interface scope for this contract change
generates: proposal.md
---

Sections:

- **Why**: What interface gap does this contract address? 1-2 sentences on the problem or opportunity.
- **Interface Scope**: Which API boundaries, endpoints, or message channels are in scope? Be specific about the interaction surface being defined.
- **Participants**: Which projects will produce and consume these contracts? List project names and their roles (producer, consumer, or both).
- **Authorship Pattern**: One of:
  - **Contract-first** — defining a new interface before any implementation exists
  - **Contract-given** — importing an external or legacy API contract into the baseline
  - **Modification** — updating existing contracts in `.specify/contracts/`
- **Impact**: Which implementation changes depend on these contracts? Reference plan entries or future changes that will consume the contract artifacts.

Keep it concise (1 page). Focus on what interfaces are being defined, not how they will be implemented — implementation details belong in the consuming schema's design.md.

## Output Structure

```markdown
## Why

<!-- What interface gap does this contract address? -->

## Interface Scope

<!-- Which API boundaries, endpoints, or message channels are in scope?
Be specific about the interaction surface being defined. -->

## Participants

<!-- Which projects will produce and consume these contracts?
List project names and their roles (producer, consumer, or both). -->

## Authorship Pattern

<!-- Contract-first | Contract-given | Modification -->

## Impact

<!-- Which implementation changes depend on these contracts?
Reference plan entries or future changes that will consume the artifacts. -->
```
