# Specify

Specify is a Rust runtime for **spec-driven software development**. This repository produces the `specify` CLI and the ultrathin Cursor skill wrappers that invoke it. Source and target adapters (Omnia, Vectis, Contracts, …) live in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters).

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

An adapter is named by a package reference (`specify:omnia@1.0.0`), the first-party shorthand (`omnia@1.0.0`), a local `.wasm` component path, or the bare development shorthand (`omnia`, resolving the project component cache or the project's own release build). Common targets:

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

## Operator surface

In Cursor, operators drive the workflow with `/spec:*` skills (ultrathin wrappers over `specify` verbs) plus the Capture plugin for migration workflows:

- **Specify** (`plugins/spec/`) — `/spec:init`, `/spec:plan`, `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/spec:finalize`; the execute loop is `specify plan execute`
- **Capture** (`plugins/capture/`) — runtime capture for migration workflows

Domain-specific code generation lives in target adapters (`omnia`, `vectis`, `contracts`), not in Cursor skills. See [Cursor operator plugins](docs/contributing/operator-plugins.md) for the marketplace layout and local preview.

## Vocabulary

Two lifecycle nouns — **slice** and **change** — appear constantly in this codebase. [AGENTS.md § Workflow nouns](AGENTS.md#workflow-nouns) is their canonical home; read it there rather than relying on restatements.

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
cargo test -p checks
```

Adapter boundary + docs/plugin link integrity. Only a Rust toolchain is required; the same tests run inside `cargo make ci`. See [Consistency Checks](docs/contributing/checks.md).

### Local iteration

- **CLI / crates** — edit under `src/` and `crates/`; run `cargo make ci`.
- **Adapters** — live in [`augentic/specify-adapters`](https://github.com/augentic/specify-adapters). Build there (`cargo make adapter omnia` or `cargo make release`) and initialise the consuming project with the component path — `/spec:init /path/to/…/omnia.wasm` — which mirrors it into the project component cache. There is no sibling-checkout probe; re-run init with the rebuilt component to pick up changes.
- **Cursor skill wrappers** — preview with `cursor-agent --plugin-dir plugins/spec` (see [Cursor operator plugins](docs/contributing/operator-plugins.md)).

### Contributing

Generation behavior belongs in guest orchestrations and target-adapter prompts, not in skill bodies. Skill wrappers stay ultrathin invoke-and-relay. See [CONTRIBUTING.md](CONTRIBUTING.md) for the contribution guide, including DCO requirements and pull request procedure.

## Documentation

- **[Developer Guide](docs/SUMMARY.md)** — tutorials, how-to guides, explanation, reference, and appendices (mdBook)
  - [Tutorials](docs/tutorials/index.md) — progressive walkthroughs from first slice to cross-repo changes
  - [How-to guides](docs/how-to/index.md) — task recipes for common operator situations
  - [Reference](docs/reference/index.md) — skills, CLI, adapters, configuration
  - [Quick reference](docs/reference/quick-reference.md) — single-page cheat sheet
- [Specify artifact guidance supplement](docs/explanation/augentic-specify-usage.md)
- [Project Rule](.cursor/rules/project.mdc)
- [Agent Instructions](AGENTS.md)
- [Contribution Guide](CONTRIBUTING.md)
- [Governance](GOVERNANCE.md)
- [Code of Conduct](CODE-OF-CONDUCT.md)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
