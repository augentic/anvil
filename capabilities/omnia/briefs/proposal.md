---
id: proposal
description: Establish why this change is needed
generates: proposal.md
---

Sections:

- **Why**: 1-2 sentences on the problem or opportunity. What problem does this solve? Why now?
- **Source**: One line describing where the change's material comes from — a code path, repository URL, documentation artefact (e.g. an epic key), or **Manual** for greenfield / handwritten changes. The line is descriptive; the pipeline is driven by the plan entry's `sources:` list, which `/change:execute` forwards to `/spec:define` as `--source <key>=<path-or-url>` flags. **Manual** means no `--source` flag is supplied and the specs brief's manual branch runs — see `capabilities/omnia/briefs/specs.md`.
- **What Changes**: Bullet list of changes. Be specific about new capabilities, modifications, or removals. Mark breaking changes with **BREAKING**.
- **Crates**: Identify which specs will be created or modified:
  - **New Crates**: List crates being introduced. Each becomes a new `specs/<name>/spec.md`. Use kebab-case names (e.g., `user-auth`, `data-export`).
  - **Modified Crates**: List existing crates whose REQUIREMENTS are changing. Only include if spec-level behavior changes (not just implementation details). Each needs a delta spec file. Check `.specify/specs/` for existing spec names. Leave empty if no requirement changes.

  (For source-driven runs, the final crate set emerges from `/spec:extract`; the Crates section is the operator's intent, not a final contract.)
- **Impact**: Affected code, APIs, dependencies, or systems.

IMPORTANT: The Crates section creates the contract between proposal and specs phases. For manual changes, research existing specs before filling this in — each crate listed will need a corresponding spec file.

Keep it concise (1-2 pages). Focus on the "why" not the "how" - implementation details belong in design.md.

This is the foundation - specs, design, and tasks all build on this.

## Output Structure

```markdown
## Why

<!-- Explain the motivation for this change. What problem does this solve? -->

## Source

<!-- One line: code path / URL / doc artefact / "Manual". Descriptive only;
     the plan entry's `sources:` list drives the pipeline (forwarded as
     --source flags to /spec:define). "Manual" = greenfield / handwritten. -->

## What Changes

<!-- Describe what will change. Be specific about new capabilities or modifications. -->

## Crates

### New Crates

<!-- List crates being introduced. Each becomes a new specs/<name>/spec.md.
Use kebab-case names (e.g., user-auth, data-export). -->

### Modified Crates

<!-- List existing crates whose REQUIREMENTS are changing.
Use existing spec folder names from .specify/specs/.
Leave empty if no requirement changes. -->

## Impact

<!-- Affected code, APIs, dependencies, systems.
Call out risks such as cross-service contract changes, breaking changes,
complexity concerns -->
```
