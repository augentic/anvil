---
name: propose
description: Propose a new change with all artifacts generated in one step. Use when the user wants to quickly describe what they want to build and get a complete proposal with design, specs, and tasks ready for implementation.
license: MIT
---

# Propose Skill

Propose a new change - create the change and generate all artifacts in one step.

When ready to implement, run /spec:apply

---

## Input

The user's request should include a change name (kebab-case) OR a description of what they want to build. Optionally, an artifact ID to regenerate a single artifact for an existing change (e.g., `/spec:propose my-change design`).

## Steps

1. **If no clear input provided, ask what they want to build**

   Ask the user in normal chat:
   > "What change do you want to work on? Describe what you want to build or fix."

   From their description, derive a kebab-case name (e.g., "add user authentication" -> `add-user-auth`).

   **IMPORTANT**: Do NOT proceed without understanding what the user wants to build.

2. **Validate the change name**

   The name must be kebab-case: lowercase letters, digits, and hyphens only. No leading or trailing hyphens. No spaces or uppercase.

   Good: `add-dark-mode`, `fix-export-bug`, `user-auth-v2`
   Bad: `Add-Dark-Mode`, `add dark mode`, `-leading`, `trailing-`

3. **Check initialization, resolve schema, and read config**

   - Verify `.specify/config.yaml` exists. If not, tell the user to run `/spec:init` first.
   - Read `.specify/config.yaml` to get:
     - `schema`: the schema value. Default to `omnia` if not found.
     - `context`: Project background (constraints for you - do NOT include in artifact output)
     - `rules`: Per-artifact rule overrides (constraints for you - do NOT include in artifact output)
   - **Resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`, `config.yaml`, `instructions/*`.
   - Read `schema.yaml` from the resolved schema directory. This defines the artifact list, dependency graph, and file references. **All artifact knowledge comes from the schema** — do not assume fixed artifact IDs or output paths.
   - Read `config.yaml` from the resolved schema directory for default `rules`. **Resolve effective rules** per artifact: for each artifact ID, use the project's `rules.<id>` if present and non-empty, otherwise fall back to the schema's `rules.<id>`. These effective rules are constraints for you — do NOT include them in artifact output.

4. **Check for regenerate mode**

   If the user specified an artifact ID (e.g., `design`):

   a. Verify the change exists at `.specify/changes/<name>/`
   b. Read `.metadata.yaml` and confirm `status` is `proposed` or `applying`
   c. Look up the artifact by `id` in `schema.yaml`
   d. Verify all artifacts listed in its `requires` exist in the change directory
   e. Read the required artifacts for context
   f. Read the instruction file at the path given by the artifact's `instruction` field in the resolved schema directory
   g. Regenerate ONLY the specified artifact following the instruction
   i. Apply `context` and effective rules as constraints
   j. Run validators if `validate` rules are defined for this artifact (see step 6)
   k. Do NOT change the `status` field
   l. Show output:

      ```markdown
      ## Artifact Regenerated

      **Change:** <name>
      **Artifact:** <generates> (regenerated)
      **Dependencies read:** <list of requires artifacts>

      The artifact has been updated. Other artifacts are unchanged.

      ```

   m. Stop — do not proceed to full propose flow

5. **Create the change directory**

   - Check if `.specify/changes/<name>/` already exists. If so:
     - Read `.metadata.yaml` — if `status` is `proposing`, offer to continue or restart
     - Otherwise ask if user wants to continue it or create a new one with a different name

   ```bash
   mkdir -p .specify/changes/<name>/specs
   ```

   Write `.specify/changes/<name>/.metadata.yaml`:

   ```yaml
   schema: <schema_from_config>
   status: proposing
   created_at: <current ISO-8601 timestamp>
   proposed_at: null
   apply_started_at: null
   completed_at: null
   touched_specs: []
   ```

6. **Check for overlapping changes**

   Before creating specs, check if any other active change (in `.specify/changes/`, skipping `archive/`) also touches the same capabilities. Read each active change's `.metadata.yaml` for its `touched_specs` list. If any capability appears in both the current proposal's crates/capabilities list and another change's `touched_specs`:
   - Warn: "The capability `<name>` is also being modified by change `<other-change>`. This may cause conflicts at archive time."
   - This is informational only — do not block the proposal.

7. **Create artifacts in dependency order**

   Use the **TodoWrite tool** to track progress through the artifacts.

   Build the dependency graph from the `requires` field of each artifact in `schema.yaml`. Topologically sort: an artifact is ready when all artifacts listed in its `requires` are complete. Artifacts with no `requires` come first; artifacts sharing the same dependency level can be created in parallel or any order.

   For each artifact (in dependency order):

   - Read any completed dependency files (the artifacts listed in `requires`) for context
   - Read the instruction file at the path given by the artifact's `instruction` field in the resolved schema directory
   - Determine the output path from the `generates` field, relative to `.specify/changes/<name>/`:
     - Simple filename (e.g., `proposal.md`): write to `.specify/changes/<name>/<generates>`
     - Glob pattern (e.g., `specs/**/*.md`): the instruction determines how many files to create and where within the pattern
   - Create the artifact file following the instruction, applying the format conventions below for the matching artifact type
   - Apply `context` and effective rules as constraints — but do NOT copy them into the file
   - If the artifact has `validate` rules in `schema.yaml`, re-read the written file and verify each rule. If any fail: report which rules failed and why, attempt to fix the artifact, re-validate after fixing. If still failing after one fix attempt, warn the user and proceed.
   - Verify the file exists after writing before proceeding to next

   ### Spec format conventions

   These rules apply to all spec files regardless of schema. The instruction
   file provides the templates and workflow routing; these conventions govern
   the content written into those templates.

   **New-crate specs (baseline format):**

   - Structure as a flat baseline document:
     `## Purpose` → `### Requirement:` blocks → `## Error Conditions` → `## Metrics`
   - Assign requirement IDs sequentially within the spec (`REQ-001`, `REQ-002`, ...)
   - Use SHALL/MUST for normative requirements (avoid should/may)
   - Each scenario: `#### Scenario: <name>` with WHEN/THEN format
   - Every requirement MUST have at least one scenario
   - Specs should be testable — each scenario is a potential test case

   **Modified-crate specs (delta format):**

   Delta operations use the headings defined in `references/spec-format.md`:
   - **ADDED Requirements**: new behavior with a new `ID: REQ-XXX`
   - **MODIFIED Requirements**: changed behavior — MUST include full updated content and preserve the existing requirement ID
   - **REMOVED Requirements**: deprecated features — MUST include **Reason**, **Migration**, and the existing requirement ID
   - **RENAMED Requirements**: name changes only — use `ID:` plus `TO:` format

   Format rules (apply to both new and delta):
   - Each requirement block starts with `### Requirement: <name>` followed by `ID: REQ-XXX`
   - The `ID:` line is the stable key. Heading text is display text only.
   - Use SHALL/MUST for normative requirements (avoid should/may)
   - Each scenario: `#### Scenario: <name>` with WHEN/THEN format
   - Every requirement MUST have at least one scenario

   **MODIFIED requirements workflow:**
   1. Locate the existing requirement in `.specify/specs/<crate>/spec.md`
   2. Copy the ENTIRE requirement block (from `### Requirement:` through all scenarios), including the `ID:` line
   3. Paste under the MODIFIED heading and edit to reflect new behavior
   4. Preserve the original `ID:` value exactly

   **ADDED requirements workflow:**
   1. Inspect `.specify/specs/<crate>/spec.md` for the highest existing requirement ID
   2. Assign the next sequential ID to the new requirement block
   3. Do not reuse IDs from removed requirements

   **Common pitfalls:**
   - Using MODIFIED with partial content loses detail at archive time
   - If adding new concerns without changing existing behavior, use ADDED instead

   ### Design writing guidance

   These rules apply when generating design.md regardless of schema. The
   instruction file provides the output template; this guidance governs
   when and how to fill it.

   Create a full design if any of the following apply:
   - Cross-cutting change (multiple services/modules) or new architectural pattern
   - New external dependency or significant data model changes
   - Security, performance, or migration complexity
   - Ambiguity that benefits from technical decisions before coding

   If none apply, create a minimal design.md noting that a full design is
   not warranted and referencing the proposal and specs.

   For multi-crate changes, structure the document with crate-specific
   sections (`## Crate: <crate-name>`) each containing the relevant
   subsections.

   Focus on the technical shape needed for implementation. Reference the
   proposal for motivation and specs for behavioral requirements. Use
   mermaid diagrams for entity relationships and flows.

   ### Task format conventions

   These rules apply when generating tasks.md regardless of schema. The
   instruction file provides the available-skills table; this guidance
   governs the task structure.

   **IMPORTANT: Follow the format below exactly.** The apply phase parses
   checkbox format to track progress. Tasks not using `- [ ]` won't be
   tracked.

   Guidelines:
   - Group related tasks under `##` numbered headings
   - Each task MUST be a checkbox: `- [ ] X.Y Task description`
   - Tasks should be small enough to complete in one session
   - Order tasks by dependency (what must be done first?)

   Example:

   ```markdown
   ## 1. Setup
   - [ ] 1.1 Create new module structure
   - [ ] 1.2 Add dependencies to Cargo.toml

   ## 2. Core Implementation
   - [ ] 2.1 Implement data export function
   - [ ] 2.2 Add CSV formatting utilities
   ```

   Reference specs for what needs to be built, design for how to build it.
   Each task should be verifiable — you know when it's done.

   **Skill directives (optional):** Tasks may include an HTML comment tag
   that names a specialist skill to invoke during apply. The apply phase
   parses these tags and delegates the task to the referenced skill instead
   of following the default apply instruction.

   Format: `- [ ] X.Y Task description <!-- skill: plugin:skill-name -->`

   Tasks without a skill tag are implemented via the default apply
   instruction (mode detection, verification loop, etc.). Use skill tags
   when a task maps cleanly to a single specialist skill invocation. The
   instruction file lists available skills per schema.

8. **Finalize and show status**

   Update `.specify/changes/<name>/.metadata.yaml`:
   - Set `status: proposed`
   - Set `proposed_at` to current ISO-8601 timestamp
   - Set `touched_specs` from the spec files created — for each subdirectory in `.specify/changes/<name>/specs/`, record an entry with `name` (the directory name) and `type` (`new` if no baseline exists at `.specify/specs/<name>/spec.md`, `modified` if one does)

   Summarize:
   - Change name and location
   - List of artifacts created with brief descriptions
   - What's ready: "All artifacts created! Ready for implementation."
   - Prompt: "Run `/spec:apply` or ask me to implement to start working on the tasks."

## Guardrails

- Create ALL artifacts defined in `schema.yaml` before declaring the change ready
- Always read dependency artifacts (from `requires`) before creating a new one
- If context is critically unclear, ask the user -- but prefer making reasonable decisions to keep momentum
- If a change with that name already exists, check its status before deciding how to proceed
- Verify each artifact file exists after writing before proceeding to next
- **IMPORTANT**: `context` and effective rules (project config with schema defaults as fallback) are constraints for YOU, not content for the file. Do NOT copy `<context>`, `<rules>`, `<project_context>` blocks into any artifact.
