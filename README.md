# Specify

Specify is a plugin system for [Cursor](https://cursor.com) that orchestrates spec-driven software development, providing specialist skills for structured proposal-to-implementation workflows.

Each change flows through a defined lifecycle — propose, implement, archive — with artifact validation built into the implementation step. All artifacts are version-controlled alongside your code.

## Getting Started

### Prerequisites

You will need to have the [Cursor IDE](https://cursor.com) installed with the Augentic plugin marketplace installed in Cursor (Settings > Plugins > search for `Augentic`).

### Initialize a project

Initialize Specify in a project by running the `/spec:init "<schema URL>"` skill in Cursor Agent chat. The `<schema URL>` argument is used to select the schema to use for the project. 

Available schemas are:


| Schema  | URL                                                 | Use case                                           |
| ------- | --------------------------------------------------- | -------------------------------------------------- |
| `omnia` | `https://github.com/augentic/specify/schemas/omnia` | Greenfield [Omnia](https://omnia.host) development |


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

Specify ships as a Cursor plugin marketplace with four plugins:

- **Specify** (`spec`) -- Core workflow: propose, apply, archive, verify, explore
- **Omnia** (`omnia`) -- Rust WASM crate generation, testing, and review
- **RT** (`rt`) -- TypeScript analysis, fixture capture, and migration
- **Plan** (`plan`) -- JIRA epic analysis and SoW generation

See [docs/plugins.md](docs/plugins.md) for the full skill reference and artifact lifecycle.

## Development

### Validation

Run documentation and consistency checks from the repository root:

```bash
make checks
```

This executes `./scripts/checks.sh`, which requires `python3` and `bash`.

### Local plugin development

To test plugins locally before releasing to the marketplace (preserves namespacing and interdependencies such as `/spec:apply` → `/omnia:crate-writer`):

```bash
make dev-plugins
```

To revert to Augentic marketplace plugins:

```bash
make prod-plugins
```

> [!NOTE]  
> Reload (or restart) Cursor to pick up the changes for both dev and prod plugins.

#### Testing schema changes

If testing schema changes in a separate project, schemas will need to be manually copied (or symlinked) to the project from this repo.

### Contributing

All skills follow the shared `SKILL.md` structure. Changes to generation behavior belong in the relevant skill or reference. See [CONTRIBUTING.md](CONTRIBUTING.md) for the full contribution guide, including DCO requirements and pull request procedure.

## Documentation

- [Plugin Reference](docs/plugins.md)
- [Repository Architecture](docs/architecture.md)
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