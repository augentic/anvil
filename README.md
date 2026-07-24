# Specify

Specify turns a described change into versioned specs and generated code. You plan one or more slices, approve the plan, then each slice runs `refine → build → merge` with validation built into the implementation step.

This repository contains the `specify` CLI and the Cursor `/spec:*` skill wrappers that invoke it. Source and target adapters live in `[augentic/specify-adapters](https://github.com/augentic/specify-adapters)`.

## Quick start

### 1. Install

In Cursor, open Settings → Plugins, search for **Augentic**, install the marketplace, then restart Cursor. This installs the Specify `/spec:`* skills.

The skills use the `specify` CLI. `/spec:init` can install a missing CLI after confirmation, or install it yourself:

```bash
cargo install --git https://github.com/augentic/specify --locked
specify --version
```

Platform archives are also available from [GitHub Releases](https://github.com/augentic/specify/releases).

### 2. Initialize a project

Open a fresh or disposable repository in Cursor Agent chat and run:

```text
/spec:init contracts@0.5.0
```

The exact adapter pin downloads the published Contracts target adapter automatically and scaffolds `.specify/`.

### 3. Plan a change

Run an intent-only change using the published Intent source adapter:

```text
/spec:plan first-contract source intent=intent@0.5.0:value:"Author an HTTP API contract for a health endpoint that returns status and version."
```

Specify surveys the source, writes `change.md`, `discovery.md`, and `plan.yaml`, then stops at the operator review gate. Inspect those files before continuing.

### 4. Approve and execute

Run the commands printed by `/spec:plan`:

```bash
specify plan transition first-contract approved
specify plan execute
```

Execution refines the slice into durable artifacts, asks the Contracts adapter to build the contract files, validates them, and merges the resulting specs. Inspect `contracts/` and `.specify/specs/`.

Commit and publish the completed repository changes through your normal Git workflow. Then close the change in Cursor:

```text
/spec:finalize first-contract
```

For a guided explanation of every artifact and transition, follow the [full quick-start tutorial](docs/tutorials/quick-start.md). Command lookup: [Quick Reference](docs/reference/quick-reference.md).

## How Specify works

1. `/spec:plan` surveys bound sources and writes `change.md` + `plan.yaml`.
2. You approve with `specify plan transition <name> approved`.
3. `specify plan execute` drives each slice through refine, build, and merge.
4. `/spec:finalize` archives the change after you publish and finish your repo workflow.

Breakout skills (`/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`) drive a single slice by hand. Code generation lives in target adapters, not in Cursor skills. Vocabulary: [AGENTS.md § Workflow nouns](AGENTS.md#workflow-nouns); longer read: [Core concepts](docs/explanation/concepts.md).

## Adapters


| Target      | Use case                                                      |
| ----------- | ------------------------------------------------------------- |
| `omnia`     | [Omnia](https://omnia.host) Rust WASM services                |
| `vectis`    | Cross-platform [Crux](https://redbadger.github.io/crux/) apps |
| `contracts` | API/interface contract work                                   |


Source adapters turn intent, documentation, legacy TypeScript, screenshots, or runtime captures into Evidence. Target adapters consume the resulting specs and build implementation outputs. Adapter source, prompt-development examples, and contribution guidance live in `[augentic/specify-adapters](https://github.com/augentic/specify-adapters)`.

## Develop Specify

The default contributor loop is self-contained and model-free:

```bash
make test    # native integration suite
make check   # format, lint, tests, doctests, and docs
make ci      # full pre-commit gate, including vet and deny
```

Use `cargo make eval` only for changes to engine prompts or answer schemas; it requires model credentials. `cargo make specify -- ARGS` is the native mock-catalog lab shim, not the installed operator CLI.

Preview the working-tree Cursor skills against a local CLI with:

```bash
cursor-agent --plugin-dir plugins/spec
```

See the [developer loop](docs/contributing/dev-loop.md), [Cursor operator plugins](docs/contributing/operator-plugins.md), and [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## Documentation

- [Developer Guide](docs/SUMMARY.md) — tutorials, how-tos, explanation, and reference (mdBook)
- [Quick-start tutorial](docs/tutorials/quick-start.md) — guided first change
- [Quick reference](docs/reference/quick-reference.md) — single-page cheat sheet
- [What is Specify?](docs/orientation/index.md) — orientation and prerequisites
- [AGENTS.md](AGENTS.md) — repository agent instructions
- [CONTRIBUTING.md](CONTRIBUTING.md) · [GOVERNANCE.md](GOVERNANCE.md) · [Code of Conduct](CODE-OF-CONDUCT.md)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.