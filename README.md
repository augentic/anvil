# Specify

Specify turns a described change into versioned specs and generated code. You plan one or more slices, approve the plan, then each slice runs `refine → build → merge` with validation built into the implementation step.

This repository contains the `specify` CLI and the Cursor `/spec:*` skill wrappers that invoke it. Source and target adapters live in [augentic/specify-adapters](https://github.com/augentic/specify-adapters).

## Using Specify (operators)

### Install

In Cursor, open Settings → Plugins, search for **Augentic**, install the marketplace, and restart Cursor. This provides the `/spec:`* skills.

The skills shell out to the `specify` CLI; `/spec:init` installs or refreshes it for you. Or install it yourself:

```bash
cargo install --git https://github.com/augentic/specify --locked
```

No Rust toolchain? Use a platform archive from [GitHub Releases](https://github.com/augentic/specify/releases). All install routes and adapter tooling: [Prerequisites](docs/orientation/prerequisites.md).

### First change

Each step below shows the skill first, then the CLI command it wraps — every skill is a thin wrapper over one `specify` verb, so the CLI form is always available as a fallback.

**1. Initialize the project.** In Cursor Agent chat, in a fresh or disposable repository:

```text
/spec:init contracts@0.5.0
```

```bash
specify init contracts@0.5.0
```

The exact adapter pin downloads the published Contracts target adapter and scaffolds `.specify/`.

**2. Plan a change** from a one-line intent:

```text
/spec:plan first-contract source intent=intent@0.5.0:value:"Author an HTTP API contract for a health endpoint that returns status and version."
```

```bash
specify plan author first-contract --source intent=intent@0.5.0:value:"Author an HTTP API contract for a health endpoint that returns status and version."
```

Specify surveys the source, writes `change.md`, `discovery.md`, and `plan.yaml`, then stops at the operator review gate. Inspect those files before continuing.

**3. Approve and execute.** The skill asks for your explicit Gate 1 approval, stamps it, then drives the loop:

```text
/spec:execute
```

```bash
specify plan transition first-contract approved   # Gate 1: operator approval
specify plan execute                              # refine → build → merge per slice
```

Execution refines the slice into durable artifacts, asks the Contracts adapter to build the contract files, validates them, and merges the resulting specs. Inspect `contracts/` and `.specify/specs/`.

**4. Finalize.** Commit and publish the completed changes through your normal Git workflow, then close the change:

```text
/spec:finalize first-contract
```

```bash
specify plan status     # must be`drained`
specify plan archive
```

Guided walkthrough of every artifact and transition: [quick-start tutorial](docs/tutorials/quick-start.md). Command lookup: [Quick Reference](docs/reference/quick-reference.md).

### Breakout skills

When the execute loop parks, or you want to drive one slice by hand:


| Skill          | CLI equivalent            |
| -------------- | ------------------------- |
| `/spec:refine` | `specify slice refine`    |
| `/spec:build`  | `specify slice build`     |
| `/spec:merge`  | `specify slice merge run` |
| `/spec:drop`   | `specify slice drop`      |


Code generation lives in target adapters, not in Cursor skills. Vocabulary: [AGENTS.md § Workflow nouns](AGENTS.md#workflow-nouns); longer read: [Core concepts](docs/explanation/concepts.md).

## Adapters


| Target      | Use case                                                      |
| ----------- | ------------------------------------------------------------- |
| `omnia`     | [Omnia](https://omnia.host) Rust WASM services                |
| `vectis`    | Cross-platform [Crux](https://redbadger.github.io/crux/) apps |
| `contracts` | API/interface contract work                                   |


Source adapters turn intent, documentation, legacy TypeScript, screenshots, or runtime captures into Evidence. Target adapters consume the resulting specs and build implementation outputs. Adapter source and contribution guidance: [augentic/specify-adapters](https://github.com/augentic/specify-adapters).

## Developing Specify (contributors)

The repository root is a Rust workspace producing the `specify` binary. The default contributor loop is self-contained and model-free:

```bash
make test    # native integration suite
make check   # format, lint, tests, doctests, and docs
make ci      # full pre-commit gate, including vet and deny
```

Use `cargo make eval` only for changes to engine prompts or answer schemas; it requires model credentials. `cargo make specify -- ARGS` is the native mock-catalog lab shim, not the installed operator CLI.

Preview the working-tree Cursor skills against a local CLI:

```bash
cursor-agent --plugin-dir plugins/spec
```

Start with the [developer loop](docs/contributing/dev-loop.md), then [Cursor operator plugins](docs/contributing/operator-plugins.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

## Documentation

- [Developer Guide](docs/SUMMARY.md) — tutorials, how-tos, explanation, and reference (mdBook)
- [Quick-start tutorial](docs/tutorials/quick-start.md) — guided first change
- [Quick reference](docs/reference/quick-reference.md) — single-page cheat sheet
- [What is Specify?](docs/orientation/index.md) — orientation and prerequisites
- [AGENTS.md](AGENTS.md) — repository agent instructions
- [CONTRIBUTING.md](CONTRIBUTING.md) · [GOVERNANCE.md](GOVERNANCE.md) · [Code of Conduct](CODE-OF-CONDUCT.md)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.