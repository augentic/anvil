# Specify

Specify is a plugin system to orchestrate spec-driven software development. This repository provides the specialist skills used to power structured proposal-to-implementation workflows.

Each change flows through a defined lifecycle — define, build, merge — with artifact validation built into the implementation step. All artifacts are version-controlled alongside your code.

## Getting Started

### Prerequisites

You will need to have the [Cursor IDE](https://cursor.com) installed with the Augentic plugin marketplace installed in Cursor (Settings > Plugins > search for `Augentic`).

### Initialize a project

Initialize Specify in a project by running the `/spec:init "<schema URL>"` skill in Cursor Agent chat. The `<schema URL>` argument is used to select the schema to use for the project. 

Available schemas are:


| Schema | URL | Use case |
| ------ | --- | -------- |
| `omnia` | `https://github.com/augentic/specify/schemas/omnia` | Greenfield [Omnia](https://omnia.host) development |
| `vectis` | `https://github.com/augentic/specify/schemas/vectis` | Cross-platform [Crux](https://redbadger.github.io/crux/) apps (Rust core, iOS/Android shells) |


For example, to initialize a new Omnia project:

```text
/spec:init https://github.com/augentic/specify/schemas/omnia
```

Or to start a new cross-platform Crux app:

```text
/spec:init https://github.com/augentic/specify/schemas/vectis
```

This creates the `.specify/` directory with a `config.yaml` you can customize to describe your project's tech stack, architecture, and constraints. Schema URLs support an optional `@ref` suffix (e.g., `@v1`, `@main`) to pin a specific version.

### Work through a change

Once initialized, use the Specify workflow to define, build, and merge changes:

```text
/spec:define -> /spec:build -> /spec:merge
```

To define a new change:

```text
/spec:define "Add a new feature to the user interface"
```

To migrate a TypeScript project to Omnia:

```text
/spec:define "Migrate https://github.com/org/repo"
```

#### Commands

Core commands:

- `/spec:define "description"` -- Generate a complete set of artifacts (proposal, specs, design, tasks) from a description of what you want to build.
- `/spec:build` -- Validate artifacts against schema rules, then implement the tasks defined in the change artifacts.
- `/spec:merge` -- Merge delta specs into the baseline and archive the completed change.

Additional commands:

- `/spec:drop` -- Discard a change without merging specs into baseline.
- `/spec:verify` -- Detect drift between your code and baseline specs.
- `/spec:status` -- Check artifact completion, task progress, and active changes.
- `/spec:explore` -- Think through ideas and investigate problems before or during a change.

## Plugins

Specify ships as a Cursor plugin marketplace with five plugins:

- **Specify** (`spec`) -- Core workflow: define, build, merge, verify, explore
- **Omnia** (`omnia`) -- Rust WASM crate generation, testing, and review
- **Vectis** (`vectis`) -- Cross-platform Crux app generation (Rust core, iOS shells, design system)
- **RT** (`rt`) -- TypeScript analysis, fixture capture, and migration
- **Plan** (`plan`) -- JIRA epic analysis and SoW generation

See [docs/plugins.md](docs/plugins.md) for the full skill reference and artifact lifecycle.

## Development

### Validation

Run documentation and consistency checks from the repository root:

```bash
make checks
```

This runs `scripts/checks.ts` via [Deno](https://deno.land). Deno must be installed separately.

### Local plugin development

To test plugins locally before releasing to the marketplace (preserves namespacing and interdependencies such as `/spec:build` → `/omnia:crate-writer` or `/spec:build` → `/vectis:core-writer`):

```bash
make dev-plugins
```

To revert to Augentic marketplace plugins:

```bash
make prod-plugins
```

> [!NOTE]  
> Reload (or restart) Cursor to pick up the changes for both dev and prod plugins.

For plugins not yet published to `main`, the script mirrors them at the hash Cursor uses for remote plugins. On a fresh setup (no prior cache), run the script twice with a Cursor reload in between — the first reload populates the remote hash, and the second run mirrors unpublished plugins at that hash.

#### Testing schema changes

If testing schema changes in a separate project, schemas will need to be symlinked in to the project from this repo so they can be used locally:

```bash
SPECIFY_REPO="path/to/augentic/specify"
ln -sf "$SPECIFY_REPO/schemas" schemas
```

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