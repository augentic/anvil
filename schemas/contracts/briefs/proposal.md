---
id: proposal
description: Establish the interface scope for this contract change
generates: proposal.md
---

Sections:

- **Why**: What interface gap does this contract address? 1-2 sentences on the problem or opportunity.
- **Interface Scope**: Which API boundaries, endpoints, or message channels are in scope? Be specific about the interaction surface being defined.
- **Participants**: Which projects will produce and consume these contracts? List project names and their roles (producer, consumer, or both).
- **Authorship Mode**: One of:
  - **Generate from prose** — deriving machine-readable contracts from design documentation, requirements, or other prose source material
  - **Import existing contracts** — normalizing supplied OpenAPI, AsyncAPI, or JSON Schema artifacts into the platform baseline
  - **Modify existing contracts** — updating contracts already present in root `contracts/`
- **Source Material**: List the prose documents, design notes, external contract files, or baseline contract paths that define the interface. Arbitrary source paths are allowed here; build must copy or normalize any imported contract files into the change-local `contracts/` tree before verification.
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

## Authorship Mode

<!-- Generate from prose | Import existing contracts | Modify existing contracts -->

## Source Material

<!-- Prose docs, design notes, external contract files, or baseline contract paths.
For imports, list the source file paths that build should ingest. -->

## Impact

<!-- Which implementation changes depend on these contracts?
Reference plan entries or future changes that will consume the artifacts. -->
```
