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
  Crates section. Choose the right template based on the proposal:

  **New Crates**: use the **New Crate** template from the resolved
  schema's `templates/spec-new.md`. Use the exact kebab-case name
  from the proposal (`specs/<crate>/spec.md`).

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

  **Modified Crates**: use the **Modified Crate** template from the
  resolved schema's `templates/spec-delta.md`. Use the existing spec
  folder name from `.specify/specs/<crate>/` when creating the delta
  spec at `specs/<crate>/spec.md`.

  Delta operations use the headings defined in `schema.yaml`'s
  `spec_format.delta_operations`:
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
