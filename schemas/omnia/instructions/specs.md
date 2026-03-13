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
  Crates section. Choose the right template based on the proposal:

  **New Crates**: use the **New Crate** template from the resolved
  schema's `templates/spec-new.md`. Use the exact kebab-case name
  from the proposal (specs/<crate>/spec.md).

  New crate guidelines:

  Structure the spec using the section heading defined in
  `schema.yaml`'s `spec_format.section_heading` (e.g., `## Handler:`).
  Each section contains:
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

  **Modified Crates**: use the **Modified Crate** template from the
  resolved schema's `templates/spec-delta.md`. Use the existing spec
  folder name from .specify/specs/<crate>/ when creating the delta
  spec at specs/<crate>/spec.md.

  Delta operations use the headings defined in `schema.yaml`'s
  `spec_format.delta_operations`:
    - **ADDED Requirements**: New behavior
    - **MODIFIED Requirements**: Changed behavior - MUST include full
      updated content.
    - **REMOVED Requirements**: Deprecated features - MUST include
      **Reason** and **Migration**.
    - **RENAMED Requirements**: Name changes only - use FROM:/TO: format

  Delta format requirements:
    - Each requirement: `### Requirement: <name>` followed by description
    - Use SHALL/MUST for normative requirements (avoid should/may)
    - Each scenario: `#### Scenario: <name>` with WHEN/THEN format
    - **CRITICAL**: In delta specs, scenarios MUST use exactly 4 hashtags
      (`####`). In full baseline specs organized under section headings,
      scenarios are one level deeper at `#####`.
    - Every requirement MUST have at least one scenario.

  MODIFIED requirements workflow:
    1. Locate the existing requirement in
      .specify/specs/<capability>/spec.md
    2. Copy the ENTIRE requirement block (from `### Requirement:`
      through all scenarios).
    3. Paste under the MODIFIED heading and edit to reflect new
      behavior.
    4. Ensure header text matches exactly (whitespace-insensitive)

  Common pitfall: Using MODIFIED with partial content loses detail at
  archive time.

  If adding new concerns without changing existing behavior, use ADDED
  instead.

  Specs should be testable - each scenario is a potential test case.
