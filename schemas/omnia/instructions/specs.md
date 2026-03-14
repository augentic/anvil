# Omnia Specs Instructions

Create specification files that define WHAT the system should do.

First, read the proposal's **Source** section to determine the workflow:

---

**RT path** (Source is a repository URL):

  1. Clone the source repository. Invoke `/rt:git-cloner` with
     arguments:
       `<repo-url> legacy/ true`
     This clones the repo into `legacy/<repo-name>` as a detached tree.
  2. Generate specs and design. Invoke `/rt:code-analyzer` with
     arguments:
       `legacy/<repo-name> <change-dir>`
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
       `<epic-key> <change-dir>`
     epic-analyzer produces `proposal.md`, `specs/`, and `design.md`.
  2. Review the generated specs for completeness and adjust if needed.
  3. Proceed to the next artifact. design.md was already produced by
     epic-analyzer — the design phase will review/enrich it.

---

**Manual path** (Source is "Manual" or absent):

  Create one spec file per crate listed in the proposal's
  Crates section.

  **New Crates**: Use the exact kebab-case name from the proposal
  (`specs/<crate>/spec.md`). Follow this structure:

  ```markdown
  # <Crate Name> Specification

  ## Purpose

  <1-2 sentence description of what this crate does>

  ### Requirement: <Behavior Name>

  ID: REQ-001

  The system SHALL <behavioral description>.

  #### Scenario: <Happy Path>

  - **WHEN** <trigger or input>
  - **THEN** <expected behavior>

  #### Scenario: <Error Case>

  - **WHEN** <invalid input or failing condition>
  - **THEN** <expected error behavior>

  ## Error Conditions

  - <error type>: <description and trigger conditions>

  ## Metrics

  - `<metric_name>` — type: <counter|gauge|histogram>; emitted: <when>
  ```

  Repeat `### Requirement:` blocks for each distinct behavior,
  incrementing `ID: REQ-XXX` for each new requirement.

  New crate guidelines:

  Structure the spec as a flat baseline document:
    - `## Purpose` — what the crate does overall
    - `### Requirement: <name>` — one block per behavioral requirement
    - `ID: REQ-XXX` — stable identifier immediately after each requirement heading
    - `#### Scenario: <name>` — one or more scenarios under each requirement
    - `## Error Conditions` — optional shared error types and triggers
    - `## Metrics` — optional metric names and types

  Format requirements:
    - Assign requirement IDs sequentially within the spec (`REQ-001`, `REQ-002`, ...)
    - Use SHALL/MUST for normative requirements (avoid should/may)
    - Each scenario: `#### Scenario: <name>` with WHEN/THEN format
    - Every requirement MUST have at least one scenario
    - Specs should be testable — each scenario is a potential test case

  **Modified Crates**: Use the existing spec folder name from
  `.specify/specs/<crate>/` when creating the delta spec at
  `specs/<crate>/spec.md`. Follow this structure:

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

  Delta operations use the headings defined in `schema.yaml`'s
  `spec-format.delta-operations`:
    - **ADDED Requirements**: New behavior with a new `ID: REQ-XXX`
    - **MODIFIED Requirements**: Changed behavior - MUST include full
      updated content and preserve the existing requirement ID.
    - **REMOVED Requirements**: Deprecated features - MUST include
      **Reason**, **Migration**, and the existing requirement ID.
    - **RENAMED Requirements**: Name changes only - use `ID:` plus `TO:` format

  Delta format requirements:
    - Each requirement block starts with `### Requirement: <name>` followed by `ID: REQ-XXX`
    - Use SHALL/MUST for normative requirements (avoid should/may)
    - Each scenario: `#### Scenario: <name>` with WHEN/THEN format
    - Every requirement MUST have at least one scenario.
    - The `ID:` line is the stable key. Heading text is display text only.

  MODIFIED requirements workflow:
    1. Locate the existing requirement in
      `.specify/specs/<crate>/spec.md`
    2. Copy the ENTIRE requirement block (from `### Requirement:`
      through all scenarios), including the `ID:` line.
    3. Paste under the MODIFIED heading and edit to reflect new
      behavior.
    4. Preserve the original `ID:` value exactly.

  ADDED requirements workflow:
    1. Inspect `.specify/specs/<crate>/spec.md` for the highest existing requirement ID
    2. Assign the next sequential ID to the new requirement block
    3. Do not reuse IDs from removed requirements

  Common pitfall: Using MODIFIED with partial content loses detail at
  archive time.

  If adding new concerns without changing existing behavior, use ADDED
  instead.

  Specs should be testable - each scenario is a potential test case.
