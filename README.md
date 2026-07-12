# Specify

Specify is a plugin system to orchestrate spec-driven software development. This repository provides the specialist skills used to power structured proposal-to-implementation workflows.

Each slice flows through a defined lifecycle — refine, build, merge — with artifact validation built into the implementation step. All artifacts are version-controlled alongside your code.

## Getting Started

Read the [Developer Guide](docs/index.md) in this order:

1. [What is Specify?](docs/orientation/index.md)
2. [Prerequisites](docs/orientation/prerequisites.md)
3. [Quick start tutorial](docs/tutorials/quick-start.md)
4. [Core concepts](docs/explanation/concepts.md)

Initialize a project in Cursor Agent chat with an adapter:

```text
/spec:init omnia@1.0.0
```

An adapter is named by a package reference (`specify:omnia@1.0.0`), the first-party shorthand (`omnia@1.0.0`), or the bare development shorthand (`omnia`, resolving a sibling checkout's release build). Common targets:

| Target      | Use case                                                      |
| ----------- | ------------------------------------------------------------- |
| `omnia`     | [Omnia](https://omnia.host) Rust WASM services                |
| `vectis`    | Cross-platform [Crux](https://redbadger.github.io/crux/) apps |
| `contracts` | API/interface contract work                                   |

Then work through a slice:

```text
/spec:plan "Add a new feature"
specify plan transition <name> approved
specify plan execute
```

`/spec:plan` authors `change.md` + `plan.yaml`, the operator stamps `approved` with `specify plan transition <name> approved`, `specify plan execute` drives each planned slice through the per-slice `refine → build → merge` loop, and `/spec:finalize` archives the change after the operator has published branches and completed the required repository workflow. Cross-repo work adds `registry.yaml`, top-level `workspace/` slots, and operator-owned slot materialization and branch publication. See the [Quick Reference](docs/reference/quick-reference.md) for command lookup.

## Plugins

Specify ships as a Cursor plugin marketplace with three plugins:

- **Specify** (`spec`) -- End-to-end workflow: `/spec:init`, `/spec:plan`, `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/spec:finalize` — each an ultrathin wrapper over one `specify` verb; the execute loop is the CLI verb `specify plan execute`.
- **Capture** (`capture`) -- Runtime capture for migration workflows
- **Client** (`client`) -- Client-facing deliverables (Statements of Work, proposals, pricing summaries) generated from Specify artifacts

Domain-specific code generation lives in target adapters (`omnia`, `vectis`, `contracts`), not Cursor plugins.

See the [Developer Guide](docs/reference/plugins/index.md) for the full skill reference and artifact lifecycle.

## Vocabulary

Two lifecycle nouns — **slice** and **change** — appear constantly in this codebase. [AGENTS.md §Workflow nouns](AGENTS.md#workflow-nouns) is their canonical home; read it there rather than relying on restatements.

## Installing the CLI

The `specify` binary backs every workflow skill. `/spec:init` can bootstrap a missing CLI after confirmation; for manual setup use:

```bash
cargo install --git https://github.com/augentic/specify   # from source
```

or download the platform archive from the [GitHub Releases page](https://github.com/augentic/specify/releases) and verify it against its `.sha256` companion (see [docs/release.md](docs/release.md)). A Homebrew tap is planned but not yet published.

Once installed, keep the binary current through the same install channel: rerun `cargo install`, upgrade through your package manager, or replace the downloaded release binary. `specify init --upgrade` updates an existing project's Specify pin and preservation-safe scaffold; it does not self-update the CLI binary.

`specify completions <shell>` writes a completion script to stdout for any clap-supported shell (`bash`, `elvish`, `fish`, `powershell`, `zsh`), generated from the live clap surface so it stays in sync with every verb the binary exposes.

See [Prerequisites](docs/orientation/prerequisites.md) for all install paths and adapter-specific tooling.

## Development

### Validation

Run documentation and consistency checks from the repository root:

```bash
cargo test --test framework
```

The framework checks are plain cargo tests over the prose and manifest surfaces. Only a Rust toolchain is required; the same tests run inside the full `cargo make ci` gate. See [Consistency Checks](docs/contributing/checks.md) for the full check model.

### Local plugin development

Cursor Agent can load working-tree plugins directly with `--plugin-dir`. The agent then uses local skill, rule, and reference content instead of the published versions.

#### Dev iteration loop

1. Edit skills, rules, or references in `plugins/`.
2. Start Cursor Agent with the plugin directory.
3. Test in a target project.
4. Repeat from step 1.

```bash
cursor-agent --plugin-dir plugins/spec
```

Pass `--plugin-dir` once per local plugin when testing more than one. Omit it to use the published plugins.

#### Testing adapter changes

Adapters live in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters). For local iteration, run `cargo make release` there and initialise the consuming project with the bare development shorthand (`/spec:init omnia`) — a bare name resolves the sibling checkout's release build at `target/wasm32-wasip2/release/<name>.wasm`. Rebuild the component to pick up changes; no cache clear or restart is needed.

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

- **[Developer Guide](docs/SUMMARY.md)** — tutorials, how-to guides, explanation, reference, and appendices (mdBook)
  - [Tutorials](docs/tutorials/index.md) — progressive walkthroughs from first slice to cross-repo changes
  - [How-to guides](docs/how-to/index.md) — task recipes for common operator situations
  - [Reference](docs/reference/index.md) — skills, CLI, plugins, adapters, configuration
  - [Quick reference](docs/reference/quick-reference.md) — single-page cheat sheet
- [Specify artifact guidance supplement](docs/explanation/augentic-specify-usage.md)
- [Project Rule](.cursor/rules/project.mdc)
- [Agent Instructions](AGENTS.md)
- [Contribution Guide](CONTRIBUTING.md)
- [Governance](GOVERNANCE.md)
- [Code of Conduct](CODE-OF-CONDUCT.md)
- [Cursor Skills Documentation](https://cursor.com/docs/skills)
- [Cursor Plugin Reference](https://cursor.com/docs/reference/plugins)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.