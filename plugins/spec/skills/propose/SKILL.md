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

3. **Check initialization and existing changes**

   - Verify `.specify/config.yaml` exists. If not, tell the user to run `/spec:init` first.
   - Read `.specify/config.yaml` to determine the project schema (look for `schema: <name>`). Default to `omnia` if not found.
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

   The artifact dependency graph is:
   ```
   proposal (no dependencies)
      |
      +---> specs (requires: proposal)
      |
      +---> design (requires: proposal)
               |
               +---> tasks (requires: specs + design)
   ```

   Create artifacts in this order: **proposal** -> **specs** + **design** -> **tasks**

   For each artifact:
   - Read any completed dependency files for context
   - Create the artifact file using the template and instruction below
   - Apply `context` and `rules` from config.yaml as constraints -- but do NOT copy them into the file
   - Verify the file exists after writing before proceeding to next

   ---

   ### Artifact: proposal

   **Write to**: `.specify/changes/<name>/proposal.md`

   **Template**:
   ```markdown
   ## Why

   ## Source

   ## What Changes

   ## Capabilities

   ### New Capabilities

   ### Modified Capabilities

   ## Impact
   ```

   **Instruction**:
   Create the proposal document that establishes WHY this change is needed.

   Sections:
   - **Why**: 1-2 sentences on the problem or opportunity. What problem does this solve? Why now?
   - **Source**: Identify where the requirements come from. Pick ONE based on the project schema:

     **For 'omnia' schema:**
     - **Epic**: JIRA/ADO/Linear epic key (e.g., `ATR-7102`). This triggers the Plan workflow -- the specs phase will run epic-analyzer.
     - **Manual**: Requirements are described directly in this proposal. This is the default workflow -- specs and design are written by hand.

     **For 'realtime' schema:**
     - **Repository**: URL of the repository to migrate (e.g., `https://github.com/org/repo`). This triggers the RT workflow -- the specs phase will clone the repo and run code-analyzer.
     - **Manual**: Requirements are described directly in this proposal. This is the default workflow -- specs and design are written by hand.
   - **What Changes**: Bullet list of changes. Be specific about new capabilities, modifications, or removals. Mark breaking changes with **BREAKING**.
   - **Capabilities**: Identify which specs will be created or modified:
     - **New Capabilities**: List capabilities being introduced. Each becomes a new `specs/<name>/spec.md`. Use kebab-case names (e.g., `user-auth`, `data-export`).
     - **Modified Capabilities**: List existing capabilities whose REQUIREMENTS are changing. Only include if spec-level behavior changes (not just implementation details). Each needs a delta spec file. Check `.specify/specs/` for existing spec names. Leave empty if no requirement changes.
     - For **Repository** or **Epic** sources, capabilities will be determined by the analyzer skill. List expected capabilities if known, but analyzer output takes precedence.
   - **Impact**: Affected code, APIs, dependencies, or systems.

   IMPORTANT: The Capabilities section creates the contract between proposal and specs phases. For manual sources, research existing specs before filling this in -- each capability listed will need a corresponding spec file. For repository or epic sources, the analyzer discovers capabilities automatically.

   Keep it concise (1-2 pages). Focus on the "why" not the "how" -- implementation details belong in design.md.

   ---

   ### Artifact: specs

   **Write to**: `.specify/changes/<name>/specs/<capability>/spec.md` (one per capability)

   **Template** (for delta specs):
   ```markdown
   ## ADDED Requirements

   ### Requirement: <!-- requirement name -->
   <!-- requirement text -->

   #### Scenario: <!-- scenario name -->
   - **WHEN** <!-- condition -->
   - **THEN** <!-- expected outcome -->
   ```

   **Instruction**:
   Create specification files that define WHAT the system should do.

   First, read the proposal's **Source** section to determine the workflow:

   **RT path** (Source is a repository URL):
   1. Clone the source repository. Invoke `/rt:git-cloner` with arguments: `<repo-url> legacy/ true`
   2. Generate specs and design. Invoke `/rt:code-analyzer` with arguments: `legacy/<repo-name> <change-dir>`
   3. Review the generated specs for completeness and adjust if needed.
   4. Proceed to the next artifact. design.md was already produced by code-analyzer -- the design phase will review/enrich it.

   **Plan path** (Source is a JIRA/ADO/Linear epic key):
   1. Generate specs and design. Invoke `/plan:epic-analyzer` with arguments: `<epic-key> <change-dir>`
   2. Review the generated specs for completeness and adjust if needed.
   3. Proceed to the next artifact. design.md was already produced by epic-analyzer -- the design phase will review/enrich it.

   **Manual path** (Source is "Manual" or absent):
   Create one spec file per capability listed in the proposal's Capabilities section.

   Guidelines:
   - New capabilities: use the exact kebab-case name from the proposal (`specs/<capability>/spec.md`).
   - Modified capabilities: use the existing spec folder name from `.specify/specs/<capability>/` when creating the delta spec at `specs/<capability>/spec.md`.

   Delta operations (use `##` headers):
   - **ADDED Requirements**: New capabilities
   - **MODIFIED Requirements**: Changed behavior -- MUST include full updated content.
   - **REMOVED Requirements**: Deprecated features -- MUST include **Reason** and **Migration**.
   - **RENAMED Requirements**: Name changes only -- use `FROM:`/`TO:` format

   Format requirements:
   - Each requirement: `### Requirement: <name>` followed by description
   - Use SHALL/MUST for normative requirements (avoid should/may)
   - Each scenario: `#### Scenario: <name>` with WHEN/THEN format
   - **CRITICAL**: In delta specs, scenarios MUST use exactly 4 hashtags (`####`). In full baseline specs organized under `## Handler:` sections, scenarios are one level deeper at `#####`. Using fewer hashtags or bullets will fail silently.
   - Every requirement MUST have at least one scenario.

   MODIFIED requirements workflow:
   1. Locate the existing requirement in `.specify/specs/<capability>/spec.md`
   2. Copy the ENTIRE requirement block (from `### Requirement:` through all scenarios).
   3. Paste under `## MODIFIED Requirements` and edit to reflect new behavior.
   4. Ensure header text matches exactly (whitespace-insensitive)

   Specs should be testable -- each scenario is a potential test case.

   ---

   ### Artifact: design

   **Write to**: `.specify/changes/<name>/design.md`

   **Template**:
   ```markdown
   ## Context

   ## Domain Model

   ## API Contracts

   ## External Services

   ## Constants & Configuration

   ## Business Logic

   ## Publication & Timing Patterns

   ## Implementation Constraints

   ## Source Capabilities Summary

   ## Dependencies

   ## Risks / Open Questions

   ## Notes
   ```

   **Instruction**:
   Create the design document to explain HOW to implement the change.

   Create full design if any of the following apply:
   - Cross-cutting change (multiple services/modules) or new architectural pattern
   - New external dependency or significant data model changes
   - Security, performance, or migration complexity
   - Ambiguity that benefits from technical decisions before coding

   If none of the above apply, create a minimal design.md noting that a full design is not warranted and referencing the proposal and specs.

   Required sections (see template):
   - **Context**: Source, purpose, background, and current state
   - **Domain Model**: Entity and type definitions with field names, types, wire names, and optionality
   - **API Contracts**: Endpoints with method, path, request/response shapes, and error responses
   - **External Services**: Name, type (API, table store, cache, message broker), authentication method
   - **Constants & Configuration**: All config keys with descriptions and defaults
   - **Business Logic**: Per-handler tagged pseudocode using `[domain]`, `[infrastructure]`, `[mechanical]` tags. Include required provider traits per handler.
   - **Publication & Timing Patterns**: Topics, message shapes, timing
   - **Implementation Constraints**: Platform or runtime constraints
   - **Source Capabilities Summary**: Checklist of required capabilities
   - **Dependencies**: External packages or services
   - **Risks / Open Questions**: Known risks and unresolved decisions
   - **Notes**: Additional observations

   Focus on the technical shape needed for implementation. Reference the proposal for motivation and specs for behavioral requirements. Use mermaid diagrams for entity relationships and flows.

   ---

   ### Artifact: tasks

   **Write to**: `.specify/changes/<name>/tasks.md`

   **Template**:
   ```markdown
   ## 1. <!-- Task Group Name -->

   - [ ] 1.1 <!-- Task description -->
   - [ ] 1.2 <!-- Task description -->

   ## 2. <!-- Task Group Name -->

   - [ ] 2.1 <!-- Task description -->
   - [ ] 2.2 <!-- Task description -->
   ```

   **Instruction**:
   Create the task list that breaks down the implementation work.

   **IMPORTANT: Follow the template exactly.** The apply phase parses checkbox format to track progress. Tasks not using `- [ ]` won't be tracked.

   Guidelines:
   - Group related tasks under `##` numbered headings
   - Each task MUST be a checkbox: `- [ ] X.Y Task description`
   - Tasks should be small enough to complete in one session
   - Order tasks by dependency (what must be done first?)

   Reference specs for what needs to be built, design for how to build it. Each task should be verifiable -- you know when it's done.

7. **Show final status**

   After completing all artifacts, summarize:
   - Change name and location
   - List of artifacts created with brief descriptions
   - What's ready: "All artifacts created! Ready for implementation."
   - Prompt: "Run `/spec:apply` or ask me to implement to start working on the tasks."

**Guardrails**
- Create ALL artifacts needed for implementation (proposal, specs, design, tasks)
- Always read dependency artifacts before creating a new one
- If context is critically unclear, ask the user -- but prefer making reasonable decisions to keep momentum
- If a change with that name already exists, ask if user wants to continue it or create a new one
- Verify each artifact file exists after writing before proceeding to next
- **IMPORTANT**: `context` and `rules` from config.yaml are constraints for YOU, not content for the file. Do NOT copy `<context>`, `<rules>`, `<project_context>` blocks into any artifact.
