Create the proposal document that establishes WHY this change is needed.

Sections:
- **Why**: 1-2 sentences on the problem or opportunity. What problem does
  this solve? Why now?
- **Source**: Identify where the requirements come from. Pick ONE:
  - **Repository**: URL of the repository to migrate (e.g.,
    `https://github.com/org/repo`). This triggers the RT workflow
    — the specs phase will clone the repo and run code-analyzer.
  - **Epic**: JIRA/ADO/Linear epic key (e.g., `ATR-7102`). This triggers
    the Omnia workflow — the specs phase will run epic-analyzer.
  - **Manual**: Requirements are described directly in this proposal.
    This is the default workflow — specs and design are written by hand.
- **What Changes**: Bullet list of changes. Be specific about new
  capabilities, modifications, or removals. Mark breaking changes with
  **BREAKING**.
- **Crates**: Identify which specs will be created or modified:
  - **New Crates**: List crates being introduced. Each
    becomes a new `specs/<name>/spec.md`. Use kebab-case names 
    (e.g., `user-auth`, `data-export`).
  - **Modified Crates**: List existing crates whose 
    REQUIREMENTS are changing. Only include if spec-level behavior
    changes (not just implementation details). Each needs a delta spec
    file. Check `.specify/specs/` for existing spec names. Leave empty if
    no requirement changes.
  - For **Repository** or **Epic** sources, crates will be
    determined by the analyzer skill. List expected crates if
    known, but analyzer output takes precedence.
- **Impact**: Affected code, APIs, dependencies, or systems.

IMPORTANT: The Crates section creates the contract between proposal
and specs phases. For manual sources, research existing specs before
filling this in — each crate listed will need a corresponding spec
file. For repository or epic sources, the analyzer discovers crates
automatically.

Keep it concise (1-2 pages). Focus on the "why" not the "how" - 
implementation details belong in design.md.

This is the foundation - specs, design, and tasks all build on this.
