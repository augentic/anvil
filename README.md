# Specify

Specify is a plugin system for [Cursor](https://cursor.com) that orchestrates spec-driven software development, providing specialist skills for structured proposal-to-implementation workflows.

Each change flows through a defined lifecycle — propose, implement, archive — with artifact validation built into the implementation step. All artifacts are version-controlled alongside your code.

## Getting Started

### Prerequisites

You will need to have the [Cursor IDE](https://cursor.com) installed with the Augentic plugin marketplace installed in Cursor (Settings > Plugins > search for `augentic`).

### Initialize a project

Initialize Specify in a project by running the `/spec:init "<schema URL>"` skill in Cursor Agent chat. The `<schema URL>` argument is used to select the schema to use for the project. 

Available schemas are:


| Schema     | URL | Use case |
| ---------- | --- | -------- |
| `omnia`    | `https://github.com/augentic/specify/schemas/omnia` | Greenfield [Omnia](https://omnia.host) development |


For example, to initialize a new Omnia project:

```text
/spec:init https://github.com/augentic/specify/schemas/omnia
```

This creates the `.specify/` directory with a `config.yaml` you can customize to describe your project's tech stack, architecture, and constraints. Schema URLs support an optional `@ref` suffix (e.g., `@v1`, `@main`) to pin a specific version.

### Work through a change

Once initialized, use the Specify workflow to propose, implement, and finalize changes:

```text
/spec:propose -> /spec:apply -> /spec:archive
```

To propose a new change:

```text
/spec:propose "Add a new feature to the user interface"
```

To migrate a TypeScript project to Omnia:

```text
/spec:propose "Migrate https://github.com/org/repo"
```

#### Commands

Core commands:

- `/spec:propose "description"` -- Generate a complete set of artifacts (proposal, specs, design, tasks) from a description of what you want to build.
- `/spec:apply` -- Validate artifacts against schema rules, then implement the tasks defined in the change artifacts.
- `/spec:archive` -- Merge delta specs into the baseline and archive the completed change.

Additional commands:

- `/spec:abandon` -- Discard a change without merging specs into baseline.
- `/spec:verify` -- Detect drift between your code and baseline specs.
- `/spec:status` -- Check artifact completion, task progress, and active changes.
- `/spec:explore` -- Think through ideas and investigate problems before or during a change.

## Plugins

Specify ships as a [Cursor plugin marketplace](https://cursor.com/docs/reference/plugins) containing four plugins. Each plugin provides specialist skills namespaced by domain. Plugins also expose [MCP](https://cursor.com/docs/mcp) tool servers for programmatic integration.

### Specify (`plugins/spec/`)

Core workflow orchestration for spec-driven development.

- **init** -- Initialize Specify in a project (one-time setup)
- **propose** -- Create a change and generate all artifacts in one step
- **apply** -- Validate artifacts and implement tasks from a Specify change
- **archive** -- Finalize and archive a completed change
- **abandon** -- Discard a change without merging specs into baseline
- **verify** -- Detect drift between code and baseline specs
- **explore** -- Thinking partner for ideas, investigation, and requirements
- **status** -- Check artifact completion, task progress, and active changes

### Omnia (`plugins/omnia/`)

Generate and review Rust WASM crates targeting the Omnia runtime.

- **crate-writer** -- Generate or update Rust crates from Specify artifacts
- **test-writer** -- Generate or update test suites from Specify artifacts and crate code
- **guest-writer** -- Generate the WASM guest wrapper
- **code-reviewer** -- Review generated code for correctness and Omnia/WASM compliance

### RT (`plugins/rt/`)

TypeScript source analysis, fixture capture, and regression testing for migrations.

- **code-analyzer** -- Derive specs and design from TypeScript source
- **git-cloner** -- Clone a source repository for analysis
- **replay-writer** -- Add regression tests from captured fixtures
- **wiretapper** -- Capture fixture data from legacy services

### Plan (`plugins/plan/`)

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

## Repository Structure

```text
specify/
├── .cursor/
│   └── rules/                    # Project guidance for agents
├── .cursor-plugin/
│   └── marketplace.json          # Multi-plugin marketplace manifest
├── plugins/
│   ├── references/               # Shared references (specify.md, agent-teams.md)
│   ├── spec/                     # Specify workflow plugin
│   │   ├── skills/               # Workflow skills (init, propose, apply, ...)
│   │   ├── references/           # Artifact templates and schema resolution
│   │   └── mcp.json              # MCP server definition
│   ├── omnia/                    # Omnia code generation plugin
│   │   ├── skills/               # Code generation skills (crate-writer, test-writer, ...)
│   │   ├── references/           # Guardrails, providers, guest wiring patterns
│   │   └── mcp.json
│   ├── rt/                       # RT migration plugin
│   │   ├── skills/               # Migration skills (code-analyzer, replay-writer, ...)
│   │   └── mcp.json
│   └── plan/                     # Plan requirements analysis plugin
│       ├── skills/               # Planning skills (epic-analyzer, sow-writer)
│       └── mcp.json
├── schemas/                      # Schema definitions (omnia, realtime)
│   ├── omnia/                    # Greenfield Rust WASM schema
│   └── realtime/                 # TypeScript migration schema (extends omnia)
└── scripts/                      # Documentation and consistency checks
```

## Development

### Validation

Run documentation and consistency checks from the repository root:

```bash
make checks
```

This executes `./scripts/checks.sh`, which requires `python3` and `bash`.

### Local plugin development

To use a local checkout of the plugins in another project (for development or testing), symlink the plugins directory. 

Be sure to replace `/path/to/augentic/specify` with the absolute path to your local clone of this repository.

**Symlink skills:**

```bash
SPECIFY_REPO="path/to/augentic/specify"

mkdir -p .cursor/skills && \
for skill in "$SPECIFY_REPO"/plugins/*/skills/*; do
    name="$(basename "$skill")"
    ln -s "$skill" ".cursor/skills/$name"
done
```

**Use the symlinked skills:**

```bash
cat > .cursor/settings.json << 'EOF'
{
    "skills": {
        "source": "local"
    }
}
EOF
echo "Created .cursor/settings.json"
```

### Contributing

All skills follow the shared `SKILL.md` structure. Changes to generation behavior belong in the relevant skill or reference. See [CONTRIBUTING.md](CONTRIBUTING.md) for the full contribution guide, including DCO requirements and pull request procedure.

## Documentation

- [Specify Artifact Guidance](plugins/references/specify.md)
- [Project Rule](.cursor/rules/project.mdc)
- [Contribution Guide](CONTRIBUTING.md)
- [Governance](GOVERNANCE.md)
- [Code of Conduct](CODE-OF-CONDUCT.md)
- [Agent Instructions](AGENTS.md)
- [Cursor Skills Documentation](https://cursor.com/docs/skills)
- [Cursor Plugin Reference](https://cursor.com/docs/reference/plugins)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.