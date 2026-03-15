# Plugins

Specify ships as a [Cursor plugin marketplace](https://cursor.com/docs/reference/plugins) containing four plugins. Each plugin provides specialist skills namespaced by domain. Plugins also expose [MCP](https://cursor.com/docs/mcp) tool servers for programmatic integration.

## Specify (`plugins/spec/`)

Core workflow orchestration for spec-driven development.

- **init** -- Initialize Specify in a project (one-time setup)
- **propose** -- Create a change and generate all artifacts in one step
- **apply** -- Validate artifacts and implement tasks from a Specify change
- **archive** -- Finalize and archive a completed change
- **abandon** -- Discard a change without merging specs into baseline
- **verify** -- Detect drift between code and baseline specs
- **explore** -- Thinking partner for ideas, investigation, and requirements
- **status** -- Check artifact completion, task progress, and active changes

## Omnia (`plugins/omnia/`)

Generate and review Rust WASM crates targeting the Omnia runtime.

- **crate-writer** -- Generate or update Rust crates from Specify artifacts
- **test-writer** -- Generate or update test suites from Specify artifacts and crate code
- **guest-writer** -- Generate the WASM guest wrapper
- **code-reviewer** -- Review generated code for correctness and Omnia/WASM compliance

## RT (`plugins/rt/`)

TypeScript source analysis, fixture capture, and regression testing for migrations.

- **code-analyzer** -- Derive specs and design from TypeScript source
- **git-cloner** -- Clone a source repository for analysis
- **replay-writer** -- Add regression tests from captured fixtures
- **wiretapper** -- Capture fixture data from legacy services

## Plan (`plugins/plan/`)

Requirements analysis, design enrichment, and SoW generation from JIRA.

- **epic-analyzer** -- Derive proposal, specs, and design context from JIRA epics
- **sow-writer** -- Translate Specify artifacts into client-facing SoW material

## How It Works

Specify manages changes as a set of interdependent artifacts stored in `.specify/changes/<change-name>/`:

| Artifact      | Responsibility                                       |
| ------------- | ---------------------------------------------------- |
| `proposal.md` | Why the change exists and what is in scope           |
| `spec.md`     | Behavioral requirements, scenarios, error conditions |
| `design.md`   | Domain model, APIs, integrations, configuration      |
| `tasks.md`    | Implementation sequencing                            |

The workflow is:

1. **Propose** -- Describe what you want to build. Specify generates all four artifacts from your description, optionally enriched by JIRA epics (`/plan:epic-analyzer`) or TypeScript source analysis (`/rt:code-analyzer`).
2. **Apply** -- Validate artifacts for completeness and cross-artifact consistency, then implement each task. Specialist skills (crate-writer, test-writer, guest-writer) generate code from the artifacts.
3. **Archive** -- Merge the change's specs into your project's baseline at `.specify/specs/` and move the change to the archive.

Baseline specs accumulate over time, giving future changes a foundation to build on. Use `/spec:verify` at any point to detect drift between your code and the baseline.
