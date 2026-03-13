---
name: propose
description: Propose a new change with all artifacts generated in one step. Use when the user wants to quickly describe what they want to build and get a complete proposal with design, specs, and tasks ready for implementation.
license: MIT
metadata:
  author: specify
  version: "3.0"
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
     - `rules`: Per-artifact rules (constraints for you - do NOT include in artifact output)
   - **Resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`, `instructions/*`, `templates/*`.
   - Read `schema.yaml` from the resolved schema directory. This defines the artifact list, dependency graph, and file references. **All artifact knowledge comes from the schema** — do not assume fixed artifact IDs or output paths.

4. **Check for regenerate mode**

   If the user specified an artifact ID (e.g., `design`):

   a. Verify the change exists at `.specify/changes/<name>/`
   b. Read `.metadata.yaml` and confirm `status` is `proposed` or `applying`
   c. Look up the artifact by `id` in `schema.yaml`
   d. Verify all artifacts listed in its `requires` exist in the change directory
   e. Read the required artifacts for context
   f. Read the instruction file at the path given by the artifact's `instruction` field in the resolved schema directory
   g. Read the template file(s) — for specs, use `templates.<new>` or `templates.<delta>` based on whether the crate is new or modified; for other artifacts, use `template`
   h. Regenerate ONLY the specified artifact using the instruction and template
   i. Apply `context` and `rules` from config.yaml as constraints
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
   touched_capabilities: []
   ```

6. **Check for overlapping changes**

   Before creating specs, check if any other active change (in `.specify/changes/`, skipping `archive/`) also touches the same capabilities. Read each active change's `.metadata.yaml` for its `touched_capabilities` list. If any capability appears in both the current proposal's crates/capabilities list and another change's `touched_capabilities`:
   - Warn: "The capability `<name>` is also being modified by change `<other-change>`. This may cause conflicts at archive time."
   - This is informational only — do not block the proposal.

7. **Create artifacts in dependency order**

   Use the **TodoWrite tool** to track progress through the artifacts.

   Build the dependency graph from the `requires` field of each artifact in `schema.yaml`. Topologically sort: an artifact is ready when all artifacts listed in its `requires` are complete. Artifacts with no `requires` come first; artifacts sharing the same dependency level can be created in parallel or any order.

   For each artifact (in dependency order):

   - Read any completed dependency files (the artifacts listed in `requires`) for context
   - Read the instruction file at the path given by the artifact's `instruction` field in the resolved schema directory
   - Read the template file(s) from the resolved schema directory:
     - If the artifact has a `template` field (singular): read `templates/<template>`
     - If the artifact has a `templates` field (plural, e.g., specs): read `templates/<templates.new>` for new crates/capabilities and `templates/<templates.delta>` for modified ones
   - Determine the output path from the `generates` field, relative to `.specify/changes/<name>/`:
     - Simple filename (e.g., `proposal.md`): write to `.specify/changes/<name>/<generates>`
     - Glob pattern (e.g., `specs/**/*.md`): the instruction determines how many files to create and where within the pattern
   - Create the artifact file using the template structure and following the instruction
   - Apply `context` and `rules` from config.yaml as constraints — but do NOT copy them into the file
   - If the artifact has `validate_checks` in `schema.yaml`, re-read the written file and run each structured check (see the review skill for check type definitions). If the artifact only has `validate` string rules (no `validate_checks`), verify each string rule instead. If any check fails: report which checks failed and why, attempt to fix the artifact, re-validate after fixing. If still failing after one fix attempt, warn the user and proceed.
   - Verify the file exists after writing before proceeding to next

8. **Finalize and show status**

   Update `.specify/changes/<name>/.metadata.yaml`:
   - Set `status: proposed`
   - Set `proposed_at` to current ISO-8601 timestamp
   - Set `touched_capabilities` from the spec files created — for each subdirectory in `.specify/changes/<name>/specs/`, record an entry with `name` (the directory name) and `type` (`new` if no baseline exists at `.specify/specs/<name>/spec.md`, `modified` if one does)

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
- **IMPORTANT**: `context` and `rules` from config.yaml are constraints for YOU, not content for the file. Do NOT copy `<context>`, `<rules>`, `<project_context>` blocks into any artifact.
