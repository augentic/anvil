---
name: define
description: Define a new change with all artifacts generated in one step. Use when the user wants to quickly describe what they want to build and get a complete proposal with design, specs, and tasks ready for implementation.
license: MIT
argument-hint: "[description] [artifact-id?]"
---

# Define Skill

Define a new change - create the change and generate all artifacts in one step.

When ready to implement, run /spec:build

---

## Input

The user's request should include a change name (kebab-case) OR a description of what they want to build. Optionally, an artifact ID to regenerate a single artifact for an existing change (e.g., `/spec:define my-change design`).

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

   - Verify `.specify/project.yaml` exists. If not, tell the user to run `/spec:init` first.
   - Read `.specify/project.yaml` to get:
     - `schema`: the schema value. Default to `omnia` if not found.
     - `domain`: Project-level domain context (may be empty or a placeholder)
     - `rules`: Per-brief rule overrides (constraints for you - do NOT include in artifact output)
   - **Resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`, `briefs/*`.
   - Read `schema.yaml` from the resolved schema directory. This defines the pipeline phases and brief references. **All pipeline and brief knowledge comes from the schema** — do not assume fixed brief IDs or output paths.
   - Read `domain` from the resolved `schema.yaml` for default domain context. **Resolve effective domain**: use the project's `domain` if present and non-empty (not just a comment placeholder), otherwise fall back to the schema's `domain`. **Resolve effective rules** per brief: for each brief ID, use the project's `rules.<id>` if present and non-empty, otherwise no rules apply (rules are optional project-level overrides). These are constraints for you — do NOT include them in artifact output.

4. **Check for regenerate mode**

   If the user specified an artifact ID (e.g., `design`):

   a. Verify the change exists at `.specify/changes/<name>/`
   b. Read `.metadata.yaml` and confirm `status` is `defined` or `building`
   c. Look up the brief by `id` in `schema.yaml`'s `pipeline.define` entries
   d. Verify all briefs listed in its frontmatter `needs` exist in the change directory
   e. Read the required artifacts for context
   f. Read the brief file at the path given by the pipeline entry's `brief` field in the resolved schema directory
   g. Regenerate ONLY the specified artifact following the brief
   i. Apply `domain` and effective rules as constraints
   j. Do NOT change the `status` field
   l. Show output:

      ```markdown
      ## Artifact Regenerated

      **Change:** <name>
      **Artifact:** <generates> (regenerated)
      **Dependencies read:** <list of needs artifacts>

      The artifact has been updated. Other artifacts are unchanged.

      ```

   m. Stop — do not proceed to full define flow

5. **Create the change directory**

   - Check if `.specify/changes/<name>/` already exists. If so:
     - Read `.metadata.yaml` — if `status` is `defining`, offer to continue or restart
     - Otherwise ask if user wants to continue it or create a new one with a different name

   ```bash
   mkdir -p .specify/changes/<name>/specs
   ```

   Write `.specify/changes/<name>/.metadata.yaml`:

   ```yaml
   schema: <schema_from_config>
   status: defining
   created_at: <current ISO-8601 timestamp>
   defined_at: null
   build_started_at: null
   completed_at: null
   touched_specs: []
   ```

6. **Check for overlapping changes**

   Before creating specs, check if any other active change (in `.specify/changes/`, skipping `archive/`) also touches the same capabilities. Read each active change's `.metadata.yaml` for its `touched_specs` list. If any capability appears in both the current proposal's crates/capabilities list and another change's `touched_specs`:
   - Warn: "The capability `<name>` is also being modified by change `<other-change>`. This may cause conflicts at merge time."
   - This is informational only — do not block the proposal.

7. **Create artifacts in dependency order**

   Use the **TodoWrite tool** to track progress through the artifacts.

   Build the dependency graph from the `needs` field in each brief's YAML frontmatter. Read `pipeline.define` entries from `schema.yaml`, then for each entry read the brief file's frontmatter to get `id`, `needs`, `generates`, and `description`. Topologically sort: a brief is ready when all briefs listed in its `needs` are complete. Briefs with no `needs` come first; briefs sharing the same dependency level can be created in parallel or any order.

   For each brief (in dependency order):

   - Read any completed dependency files (the briefs listed in `needs`) for context
   - Read the brief file at the path given by the pipeline entry's `brief` field in the resolved schema directory
   - Determine the output path from the brief's frontmatter `generates` field, relative to `.specify/changes/<name>/`:
     - Simple filename (e.g., `proposal.md`): write to `.specify/changes/<name>/<generates>`
     - Glob pattern (e.g., `specs/**/*.md`): the brief determines how many files to create and where within the pattern
   - Create the artifact file following the brief, applying the format conventions below for the matching artifact type
   - Apply `domain` and effective rules as constraints — but do NOT copy them into the file
   - Verify the file exists after writing before proceeding to next

   ### Spec format conventions

   Follow the heading conventions in `references/spec-format.md` and the
   baseline/delta format in `references/specify.md` (Spec Files section).
   The instruction file provides the templates and workflow routing; these
   conventions govern the content written into those templates.

   **Delta-specific workflows (modified-crate specs):**

   MODIFIED requirements:
   1. Locate the existing requirement in `.specify/specs/<crate>/spec.md`
   2. Copy the ENTIRE requirement block (from `### Requirement:` through all scenarios), including the `ID:` line
   3. Paste under the MODIFIED heading and edit to reflect new behavior
   4. Preserve the original `ID:` value exactly

   ADDED requirements:
   1. Inspect `.specify/specs/<crate>/spec.md` for the highest existing requirement ID
   2. Assign the next sequential ID to the new requirement block
   3. Do not reuse IDs from removed requirements

   **Common pitfalls:**
   - Using MODIFIED with partial content loses detail at merge time
   - If adding new concerns without changing existing behavior, use ADDED instead

   ### Design writing guidance

   Follow the design format and decision criteria in `references/specify.md`
   (Design Document section, including "When To Create A Full Design").
   The instruction file provides the output template.

   ### Task format conventions

   Follow the task format and guidelines in `references/specify.md`
   (Tasks Document section). The instruction file provides the
   available-skills table per schema. The build phase parses checkbox
   format to track progress.

   **Skill directives (optional):** Tasks may include an HTML comment tag
   that names a specialist skill to invoke during build. The build phase
   parses these tags and delegates the task to the referenced skill instead
   of following the default build instruction.

   Format: `- [ ] X.Y Task description <!-- skill: plugin:skill-name -->`

   Tasks without a skill tag are implemented via the default build
   instruction (mode detection, verification loop, etc.). Use skill tags
   when a task maps cleanly to a single specialist skill invocation. The
   instruction file lists available skills per schema.

8. **Finalize and show status**

   Update `.specify/changes/<name>/.metadata.yaml`:
   - Set `status: defined`
   - Set `defined_at` to current ISO-8601 timestamp
   - Set `touched_specs` from the spec files created — for each subdirectory in `.specify/changes/<name>/specs/`, record an entry with `name` (the directory name) and `type` (`new` if no baseline exists at `.specify/specs/<name>/spec.md`, `modified` if one does)
   - **Verify**: re-read `.metadata.yaml` and confirm the `status` value is exactly `defined`. Valid lifecycle values are: `defining`, `defined`, `building`, `complete`, `merged`, `dropped`. If `status` is not one of these, correct it to `defined`.

   Summarize:
   - Change name and location
   - List of artifacts created with brief descriptions
   - What's ready: "All artifacts created! Ready for implementation."
   - Prompt: "Run `/spec:build` or ask me to implement to start working on the tasks."

## Guardrails

- Create all artifacts for briefs defined in `pipeline.define` before declaring the change ready
- Always read dependency artifacts (from `needs`) before creating a new one
- If context is critically unclear, ask the user -- but prefer making reasonable decisions to keep momentum
- If a change with that name already exists, check its status before deciding how to proceed
- Verify each artifact file exists after writing before proceeding to next
- **IMPORTANT**: `domain` and effective rules (project config overrides) are constraints for YOU, not content for the file. Do NOT copy `<domain>`, `<rules>`, `<project_context>` blocks into any artifact.
- Valid lifecycle status values are: `defining`, `defined`, `building`, `complete`, `merged`, `dropped` -- use these exact strings when updating `.metadata.yaml`, no other values are permitted
