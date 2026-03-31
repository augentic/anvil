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
- **Vectis** (`vectis`) -- Cross-platform Crux app generation (Rust core, iOS shells, Android shells, design system)
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

Cursor's plugin cache is populated from the server when it is missing, and left alone when it already exists. The dev-plugins script exploits this by clearing the cache and repopulating it with files from your working tree. The agent then loads your local skill, rule, and reference content instead of the published versions.

#### Dev iteration loop

1. Edit skills, rules, or references in `plugins/`.
2. Run `make dev-plugins` to copy local files into the cache.
3. Restart Cursor.
4. Test in a target project.
5. Repeat from step 1.

```bash
make dev-plugins    # copy local plugins into cache
```

When finished, revert to published plugins:

```bash
make prod-plugins   # clear cache; Cursor refetches from server on restart
```

> [!NOTE]  
> Restart Cursor after running either command. A window reload is not sufficient.

#### Testing schema changes

Schemas are read from the filesystem at `/spec:init` time, not from the plugin cache. To iterate on schemas in a separate project, symlink them from this repo:

```bash
SPECIFY_REPO="path/to/augentic/specify"
ln -sf "$SPECIFY_REPO/schemas" schemas
```

Schema edits take effect immediately — no cache clear or restart needed.

#### Publishing a new plugin

New plugins added to `marketplace.json` require a one-time server-side setup:

1. Push the plugin to `main` and merge.
2. Open the Cursor plugin marketplace dashboard.
3. Refresh the marketplace (even if auto-refresh is enabled).
4. Set the new plugin to **Required**.
5. Click **Save**.
6. Restart Cursor locally to pick up the new plugin.

After this initial setup, the plugin participates in the normal dev/prod workflow above.

### Contributing

All skills follow the shared `SKILL.md` structure. Changes to generation behavior belong in the relevant skill or reference. See [CONTRIBUTING.md](CONTRIBUTING.md) for the full contribution guide, including DCO requirements and pull request procedure.

## Documentation

- [Plugin Reference](docs/plugins.md)
- [Vectis User Guide](docs/vectis.md) -- prerequisites, Crux workflow, Xcode setup, design system
- [Repository Architecture](docs/architecture.md)
- [Specify Artifact Guidance](plugins/references/specify.md)
- [Project Rule](.cursor/rules/project.mdc)
- [Agent Instructions](AGENTS.md)
- [Contribution Guide](CONTRIBUTING.md)
- [Governance](GOVERNANCE.md)
- [Code of Conduct](CODE-OF-CONDUCT.md)
- [Cursor Skills Documentation](https://cursor.com/docs/skills)
- [Cursor Plugin Reference](https://cursor.com/docs/reference/plugins)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.