---
id: specs
description: Create specification files that define WHAT the system should do
generates: specs/**/*.md
needs: [proposal]
---

First, read the proposal's **Source** section to determine the workflow:

---

**Repository path** (Source is a repository URL):

  1. Clone the source repository. Invoke `/rt:git-cloner` with
     arguments:
       `<repo-url> legacy/ true`
     This clones the repo into `legacy/<repo-name>` as a detached tree.
  2. Generate specs and design. Invoke `/spec:extract` with
     arguments:
       `legacy/<repo-name> <change-dir>`
     extract produces both `specs/` and `design.md` in a single pass.
  3. Review the generated specs for completeness and adjust if needed.
  4. Proceed to the next artifact. design.md was already produced by
     extract — the design phase will review/enrich it.

---

**Source-code path** (Source is a local path to existing code):

  1. Generate specs and design. Invoke `/spec:extract` with arguments:
       `<source-path> <change-dir>`
     extract produces both `specs/` and `design.md` in a single pass.
  2. Review the generated specs for completeness and adjust if needed.
  3. Proceed to the next artifact. design.md was already produced by
     extract — the design phase will review/enrich it.

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

  Follow the spec format conventions defined in the define skill for
  delta operations, format rules, and the MODIFIED/ADDED workflows.
