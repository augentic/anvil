# Artifact: Specs

**Write to**: `.specify/changes/<name>/specs/<crate>/spec.md` (one per crate)

## Template: New Crate

Use this template for crates listed under **New Crates** in the proposal.
This format matches what code-analyzer and epic-analyzer produce, and is
what crate-writer and test-writer expect as input.

```markdown
# <Crate Name> Specification

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

Repeat `## Handler:` sections for each handler in the crate.

## Template: Modified Crate

Use this template for crates listed under **Modified Crates** in the
proposal. This delta format describes changes to an existing baseline spec.

```markdown
## ADDED Requirements

### Requirement: <!-- requirement name -->
<!-- requirement text -->

#### Scenario: <!-- scenario name -->
- **WHEN** <!-- condition -->
- **THEN** <!-- expected outcome -->

## MODIFIED Requirements

### Requirement: <!-- existing requirement name (must match baseline) -->
<!-- full updated requirement text -->

#### Scenario: <!-- scenario name -->
- **WHEN** <!-- condition -->
- **THEN** <!-- expected outcome -->

## REMOVED Requirements

### Requirement: <!-- existing requirement name -->
**Reason**: <!-- why this requirement is being removed -->
**Migration**: <!-- how to handle the removal -->

## RENAMED Requirements

FROM: <!-- old requirement name -->
TO: <!-- new requirement name -->
```

## Instruction

Create specification files that define WHAT the system should do.

First, read the proposal's **Source** section to determine the workflow:

---

**RT path** (Source is a repository URL):

  1. Clone the source repository. Invoke `/rt:git-cloner` with
     arguments:
       <repo-url> legacy/ true
     This clones the repo into `legacy/<repo-name>` as a detached tree.
  2. Generate specs and design. Invoke `/rt:code-analyzer` with
     arguments:
       legacy/<repo-name> <change-dir>
     code-analyzer produces both `specs/` and `design.md` in a single
     pass.
  3. Review the generated specs for completeness and adjust if needed.
  4. Proceed to the next artifact. design.md was already produced by
     code-analyzer — the design phase will review/enrich it.

---

**Omnia path** (Source is a JIRA/ADO/Linear epic key):

  Prerequisite: The Omnia plugin must be loaded for JIRA MCP access.

  1. Generate specs and design. Invoke `/plan:epic-analyzer` with
     arguments:
       <epic-key> <change-dir>
     epic-analyzer produces `proposal.md`, `specs/`, and `design.md`.
  2. Review the generated specs for completeness and adjust if needed.
  3. Proceed to the next artifact. design.md was already produced by
     epic-analyzer — the design phase will review/enrich it.

---

**Manual path** (Source is "Manual" or absent):

  Create one spec file per crate listed in the proposal's
  Crates section.

  **Choose the right template based on the proposal**:
  - Crates listed under **New Crates**: use the **New Crate** template
    above. Organize by `## Handler:` sections. Use the exact kebab-case
    name from the proposal (specs/<crate>/spec.md).
  - Crates listed under **Modified Crates**: use the **Modified Crate**
    template above. Use the existing spec folder name from
    .specify/specs/<crate>/ when creating the delta spec at
    specs/<crate>/spec.md.

  ### New crate guidelines

  Structure the spec with one `## Handler:` section per handler.
  Each handler section contains:
  - `### Purpose` — what the handler does
  - `### Requirements` — behavioral requirements, each as
    `#### Requirement: <name>` with `##### Scenario:` entries
  - `### Error Conditions` — error types and triggers
  - `### Metrics` — (optional) metric names and types

  Format requirements:
    - Use SHALL/MUST for normative requirements (avoid should/may)
    - Each scenario: `##### Scenario: <name>` with WHEN/THEN format
    - Every requirement MUST have at least one scenario
    - Specs should be testable — each scenario is a potential test case

  ### Modified crate guidelines

  Delta operations (use ## headers):
    - **ADDED Requirements**: New behavior
    - **MODIFIED Requirements**: Changed behavior — MUST include full
      updated content.
    - **REMOVED Requirements**: Deprecated features — MUST include
      **Reason** and **Migration**.
    - **RENAMED Requirements**: Name changes only — use FROM:/TO: format

  Format requirements:
    - Each requirement: `### Requirement: <name>` followed by description
    - Use SHALL/MUST for normative requirements (avoid should/may)
    - Each scenario: `#### Scenario: <name>` with WHEN/THEN format
    - **CRITICAL**: In delta specs, scenarios MUST use exactly 4 hashtags
      (`####`). In full baseline specs organized under `## Handler:`
      sections (see references/specify.md), scenarios are one level
      deeper at `#####`. Using fewer hashtags or bullets will fail
      silently.
    - Every requirement MUST have at least one scenario.

  MODIFIED requirements workflow:
    1. Locate the existing requirement in
      .specify/specs/<capability>/spec.md
    2. Copy the ENTIRE requirement block (from `### Requirement:`
      through all scenarios).
    3. Paste under `## MODIFIED Requirements` and edit to reflect new
      behavior.
    4. Ensure header text matches exactly (whitespace-insensitive)

  Common pitfall: Using MODIFIED with partial content loses detail at
  archive time.

  If adding new concerns without changing existing behavior, use ADDED
  instead.
