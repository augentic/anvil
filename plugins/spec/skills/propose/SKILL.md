---
name: propose
description: Propose a new change with all artifacts generated in one step. Use when the user wants to quickly describe what they want to build and get a complete proposal with design, specs, and tasks ready for implementation.
license: MIT
metadata:
  author: specify
  version: "2.0"
---

Propose a new change - create the change and generate all artifacts in one step.

I'll create a change with artifacts:
- proposal.md (what & why)
- specs/ (behavioral requirements)
- design.md (how)
- tasks.md (implementation steps)

When ready to implement, run /spec:apply

---

**Input**: The user's request should include a change name (kebab-case) OR a description of what they want to build.

**Steps**

1. **If no clear input provided, ask what they want to build**

   Use the **AskUserQuestion tool** (open-ended, no preset options) to ask:
   > "What change do you want to work on? Describe what you want to build or fix."

   From their description, derive a kebab-case name (e.g., "add user authentication" -> `add-user-auth`).

   **IMPORTANT**: Do NOT proceed without understanding what the user wants to build.

2. **Validate the change name**

   The name must be kebab-case: lowercase letters, digits, and hyphens only. No leading or trailing hyphens. No spaces or uppercase.

   Good: `add-dark-mode`, `fix-export-bug`, `user-auth-v2`
   Bad: `Add-Dark-Mode`, `add dark mode`, `-leading`, `trailing-`

3. **Check initialization and resolve schema**

   - Verify `.specify/config.yaml` exists. If not, tell the user to run `/spec:init` first.
   - Read `.specify/config.yaml` to get the `schema` value. Default to `omnia` if not found.
   - **Resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml` and `templates/*`.
   - Read `schema.yaml` from the resolved schema directory. This defines the artifact list, dependency graph, templates, and instructions. **All artifact knowledge comes from the schema** — do not assume fixed artifact IDs or output paths.
   - Check if `.specify/changes/<name>/` already exists. If so, ask if user wants to continue it or create a new one with a different name.

4. **Create the change directory**

   ```bash
   mkdir -p .specify/changes/<name>/specs
   ```

   Write `.specify/changes/<name>/.metadata.yaml` using the schema read from config:
   ```yaml
   schema: <schema_from_config>
   created_at: <current ISO-8601 timestamp>
   ```

5. **Read project config for context and rules**

   Read `.specify/config.yaml` to get:
   - `context`: Project background (constraints for you - do NOT include in artifact output)
   - `rules`: Per-artifact rules (constraints for you - do NOT include in artifact output)

6. **Create artifacts in dependency order**

   Use the **TodoWrite tool** to track progress through the artifacts.

   Build the dependency graph from the `requires` field of each artifact in `schema.yaml`. Topologically sort: an artifact is ready when all artifacts listed in its `requires` are complete. Artifacts with no `requires` come first; artifacts sharing the same dependency level can be created in parallel or any order.

   For each artifact (in dependency order):

   - Read any completed dependency files (the artifacts listed in `requires`) for context
   - Read the `instruction` field from `schema.yaml` for this artifact
   - Read the template from `templates/<template>` in the resolved schema directory (the `template` field in `schema.yaml` gives the filename)
   - Determine the output path from the `generates` field, relative to `.specify/changes/<name>/`:
     - Simple filename (e.g., `proposal.md`): write to `.specify/changes/<name>/<generates>`
     - Glob pattern (e.g., `specs/**/*.md`): the instruction determines how many files to create and where within the pattern
   - Create the artifact file using the template structure and following the instruction
   - Apply `context` and `rules` from config.yaml as constraints — but do NOT copy them into the file
   - Verify the file exists after writing before proceeding to next

7. **Show final status**

   After completing all artifacts, summarize:
   - Change name and location
   - List of artifacts created with brief descriptions
   - What's ready: "All artifacts created! Ready for implementation."
   - Prompt: "Run `/spec:apply` or ask me to implement to start working on the tasks."

**Guardrails**
- Create ALL artifacts defined in `schema.yaml` before declaring the change ready
- Always read dependency artifacts (from `requires`) before creating a new one
- If context is critically unclear, ask the user -- but prefer making reasonable decisions to keep momentum
- If a change with that name already exists, ask if user wants to continue it or create a new one
- Verify each artifact file exists after writing before proceeding to next
- **IMPORTANT**: `context` and `rules` from config.yaml are constraints for YOU, not content for the file. Do NOT copy `<context>`, `<rules>`, `<project_context>` blocks into any artifact.
