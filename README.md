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

This creates the `.specify/` directory with a `project.yaml` you can customize to describe your project's tech stack, architecture, and constraints. Schema URLs support an optional `@ref` suffix (e.g., `@v1`, `@main`) to pin a specific version.

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
- `/spec:extract` -- Extract Specify artifacts from existing source code.
- `/spec:plan` -- Author `.specify/plan.yaml` for a multi-change initiative (RFC-2 Layer 3 + RFC-3a: `/spec:analyze` discovery, optional multi-repo workspace sync, manifest scopes when tangled).
- `/spec:execute` -- Drive an initiative's `plan.yaml` through `define → build → merge` automatically (RFC-2 Layer 2; `--loop` runs until `all-done`). See [Initiative authoring + execution](#initiative-authoring--execution-plans) below for the full workflow.

### Initiative authoring + execution (plans)

A **plan** (`.specify/plan.yaml`) is an initiative's table of contents — an ordered, dependency-aware list of changes with per-entry status. It turns a sprawling effort (greenfield build, legacy migration, platform modernisation) into a series of self-contained Specify changes that accumulate in the baseline as each one merges. See [rfcs/archive/rfc-2-execution.md](rfcs/archive/rfc-2-execution.md) (status: **Implemented**) for the full design.

Three layers, independently useful, stacked top to bottom:

- **`/spec:plan <initiative-name> --source <key>=<path-or-url> ...`** authors `plan.yaml` (Layer 3). Runs `pipeline.plan` — discovery (`/spec:analyze`), optional **sync-peers** + `workspace.md` when `.specify/registry.yaml` lists multiple projects, then propose — with an interactive accept / edit / reject loop per slice. See [rfcs/rfc-3a-monoliths.md](rfcs/rfc-3a-monoliths.md).
- **`/spec:execute --loop`** drives the plan to completion (Layer 2). Picks the next eligible entry, runs `/spec:define → /spec:build → /spec:merge`, transitions status, repeats until `all-done` or `stuck`. `--dry-run` previews; a bare invocation runs one change then stops.
- **`specify initiative {init, validate, next, status, create, amend, transition, archive, lock, brief, registry, workspace}`** are the Layer 1 CLI primitives both skills shell out to. They stay available for hand-driven initiatives where automation is overkill:

  - `specify initiative init <name> [--source ...]` -- Scaffold `.specify/plan.yaml` with an empty `changes:` list.
  - `specify initiative validate` -- Structural and referential integrity checks (duplicate names, dependency cycles, unknown `depends-on` / `affects` / `sources`, at most one `in-progress`).
  - `specify initiative next` -- Report the next eligible entry (first `pending` whose `depends-on` is all `done`).
  - `specify initiative status` -- Render progress in topological order with per-status counts, the active `in-progress` entry, and any `status-reason`.
  - `specify initiative create <name>` -- Append a new entry (starts `pending`).
  - `specify initiative amend <name>` -- Edit non-status fields on an existing entry.
  - `specify initiative transition <name> <target>` -- Move an entry through the status state machine (`pending → in-progress → done`, plus `blocked`, `failed`, `skipped`).
  - `specify initiative archive` -- Move a completed `plan.yaml` (and its `.specify/plans/<name>/` working directory) to `.specify/archive/plans/<YYYYMMDD>-<name>/`.
  - `specify initiative lock {acquire, release, status}` -- Advisory `.specify/plan.lock` PID stamp held by the `/spec:execute` driver.

A typical initiative: `/spec:plan migrate-to-v2 --source monolith=/path/to/legacy` → review the authored plan with `specify initiative status` → `/spec:execute --loop` until it reports `all-done`.

## Plugins

Specify ships as a Cursor plugin marketplace with five plugins:

- **Specify** (`spec`) -- Core workflow: define, build, merge, verify, explore, extract
- **Omnia** (`omnia`) -- Rust WASM crate generation, testing, and review
- **Vectis** (`vectis`) -- Cross-platform Crux app generation (Rust core, iOS shells, Android shells, design system)
- **RT** (`rt`) -- Repository cloning, fixture capture, and regression testing for migration
- **Plan** (`plan`) -- Requirements analysis and SoW generation

See [docs/plugins.md](docs/plugins.md) for the full skill reference and artifact lifecycle.

## Installing the CLI

The `specify` binary backs every skill in the `spec` plugin. Install via (preferred order):

```bash
brew install augentic/tap/specify           # macOS + Linux (primary)
cargo install specify                       # any platform with a Rust toolchain
curl -sSfL https://specify.sh/install.sh | sh   # pre-built binary, any POSIX shell
make build                                  # local checkout, drops ./specify at repo root
```

Pin a specific version with `SPECIFY_VERSION=v0.1.0` in front of the `curl` line, or override the install location with `SPECIFY_INSTALL_DIR=/usr/local/bin`.

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

- **[Operator Guide](docs/SUMMARY.md)** -- tutorials, reference, and appendices (mdBook)
  - [Tutorials](docs/tutorials/index.md) -- progressive walkthroughs from first change to multi-repo migration
  - [Reference](docs/reference/index.md) -- skills, CLI, plugins, schemas, configuration
  - [Quick Reference](docs/appendices/quick-reference.md) -- single-page cheat sheet
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