# Specify

Specify is a plugin system to orchestrate spec-driven software development. This repository provides the specialist skills used to power structured proposal-to-implementation workflows.

Each slice flows through a defined lifecycle — refine, build, merge — with artifact validation built into the implementation step. All artifacts are version-controlled alongside your code.

## Getting Started

Read the [Developer Guide](docs/index.md) in this order:

1. [What is Specify?](docs/orientation/index.md)
2. [Prerequisites](docs/orientation/prerequisites.md)
3. [Quick Start](docs/tutorials/quick-start.md)
4. [Your First Slice](docs/tutorials/first-change.md)

Initialize a project in Cursor Agent chat with a adapter:

```text
/spec:init https://github.com/augentic/specify/adapters/targets/omnia
```

Common targets:

| Target | URL | Use case |
| ---------- | --- | -------- |
| `omnia` | `https://github.com/augentic/specify/adapters/targets/omnia` | [Omnia](https://omnia.host) Rust WASM services |
| `vectis` | `https://github.com/augentic/specify/adapters/targets/vectis` | Cross-platform [Crux](https://redbadger.github.io/crux/) apps |
| `contracts` | `https://github.com/augentic/specify/adapters/targets/contracts` | API/interface contract work |

Then work through a slice:

```text
/spec:plan "Add a new feature"
specify plan transition <name> reviewed
/spec:execute
```

`/spec:plan` authors `change.md` + `plan.yaml`, the operator stamps `reviewed` with `specify plan transition <name> reviewed`, `/spec:execute` drives each planned slice through the per-slice `refine → build → merge` loop, and `/spec:finalize` pushes branches and archives the change once every PR has merged. Cross-repo work adds `registry.yaml`, `.specify/workspace/`, `specify workspace push`, and operator-owned PR merge. See the [Quick Reference](docs/reference/quick-reference.md) for command lookup.

## Plugins

Specify ships as a Cursor plugin marketplace with seven plugins:

- **Specify** (`spec`) -- End-to-end workflow: `/spec:init`, `/spec:plan`, `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/spec:execute`, `/spec:finalize`. Plan authoring, execution driving, and finalization all live in the same plugin in 2.0.
- **Omnia** (`omnia`) -- Rust WASM crate generation, testing, and review
- **Vectis** (`vectis`) -- Cross-platform Crux app generation (Rust core, iOS shells, Android shells, design system)
- **Contract** (`contract`) -- API contract generation, validation, and import
- **RT** (`rt`) -- Repository cloning, fixture capture, and regression testing for migration
- **Client** (`client`) -- Client-facing deliverables (Statements of Work, proposals, pricing summaries) generated from Specify artifacts

See the [Developer Guide](docs/reference/plugins/index.md) for the full skill reference and artifact lifecycle.

## Vocabulary cheat sheet

Two lifecycle nouns appear constantly in this codebase. RFC-25 §Vocabulary locked their meaning:

- **Slice** — the single unit that flows through the fixed `refine → build → merge` loop. Each slice has its own proposal, spec, design, tasks, and merge step. Lives at `.specify/slices/<name>/`. Driven by `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop` and the `specify slice *` CLI verbs.
- **Change** — the operator-defined umbrella that coordinates one or more slices through `change.md` + `plan.yaml`. Driven by `/spec:plan`, `/spec:execute`, `/spec:finalize` and the `specify plan *` CLI verbs. `change` is on-disk vocabulary in 2.0, not a slash-command namespace.

## Installing the CLI

The `specify` binary backs every workflow skill. `/spec:init` can bootstrap a missing CLI after confirmation; for manual setup use:

```bash
brew install augentic/tap/specify
```

See [Prerequisites](docs/orientation/prerequisites.md) for all install paths and adapter-specific tooling.

## Development

### Validation

Run documentation and consistency checks from the repository root:

```bash
make checks
```

This runs `scripts/checks.ts` via [Deno](https://deno.land). Deno must be installed separately.

### Local plugin development

Cursor's plugin cache is populated from the server when it is missing, and left alone when it already exists. The local plugin script exploits this by clearing the cache and repopulating it with files from your working tree. The agent then loads your local skill, rule, and reference content instead of the published versions.

#### Dev iteration loop

1. Edit skills, rules, or references in `plugins/`.
2. Run `make use-local-plugins` to copy local files into the cache.
3. Restart Cursor.
4. Test in a target project.
5. Repeat from step 1.

```bash
make use-local-plugins    # copy local plugins into cache
```

When finished, revert to published plugins:

```bash
make use-team-plugins   # clear cache; Cursor refetches from server on restart
```

> [!NOTE]  
> Restart Cursor after running either command. A window reload is not sufficient.

#### Testing adapter changes

Adapters are read from the filesystem at `/spec:init` time, not from the plugin cache. To iterate on adapters in a separate project, symlink them from this repo:

```bash
SPECIFY_REPO="path/to/augentic/specify"
ln -sf "$SPECIFY_REPO/adapters" adapters
```

Adapter edits take effect immediately — no cache clear or restart needed.

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

- **[Developer Guide](docs/SUMMARY.md)** -- tutorials, how-to guides, reference, and appendices (mdBook)
  - [Tutorials](docs/tutorials/index.md) -- progressive walkthroughs from first slice to multi-repo migration
  - [Reference](docs/reference/index.md) -- skills, CLI, plugins, adapters, configuration
  - [Quick Reference](docs/reference/quick-reference.md) -- single-page cheat sheet
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