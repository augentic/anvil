# Specify

Specify turns a described change into versioned specs and generated code. You plan one or more slices, approve the plan, then each slice runs `refine → build → merge` with validation built into the implementation step.

This repository is the `specify` CLI and the Cursor `/spec:*` skill wrappers that invoke it. Source and target adapters live in `[augentic/specify-adapters](https://github.com/augentic/specify-adapters)`.

## Developer loop

Install the Specify CLI and, optionally, the Cursor skills: [Operator setup](#operator-setup).

```bash
cargo install --path .
cursor-agent --plugin-dir plugins/spec
```

In an agent chat window, run:

```text
/spec:init omnia                          # scaffold .specify/ for your target
/spec:plan "Add a new feature"            # author the change and its plan
specify plan transition <name> approved   # approve the plan (operator gate)
specify plan execute                      # run each slice: refine → build → merge
/spec:finalize                            # archive the change once published
```

### How it works

1. `/spec:plan` surveys bound sources and writes `change.md` + `plan.yaml`.
2. You approve with `specify plan transition <name> approved`.
3. `specify plan execute` drives each slice through refine, build, and merge.
4. `/spec:finalize` archives the change after you publish and finish your repo workflow.

Breakout skills (`/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`) drive a single slice by hand. Code generation lives in target adapters, not in Cursor skills. Vocabulary: [AGENTS.md § Workflow nouns](AGENTS.md#workflow-nouns); longer read: [Core concepts](docs/explanation/concepts.md).


| Target      | Use case                                                      |
| ----------- | ------------------------------------------------------------- |
| `omnia`     | [Omnia](https://omnia.host) Rust WASM services                |
| `vectis`    | Cross-platform [Crux](https://redbadger.github.io/crux/) apps |
| `contracts` | API/interface contract work                                   |


`cargo make specify` is the lab/mock shim — skills need the installed binary above. Adapters live in `[augentic/specify-adapters](https://github.com/augentic/specify-adapters)`. Before committing: `cargo make ci`.

More detail: [Cursor operator plugins](docs/contributing/operator-plugins.md), [developer loop](docs/contributing/dev-loop.md), [CONTRIBUTING.md](CONTRIBUTING.md).

Full walkthrough: [quick start tutorial](docs/tutorials/quick-start.md). Command lookup: [Quick Reference](docs/reference/quick-reference.md).

## Operator setup



### 1. Cursor skills

In Cursor: Settings → Plugins → search **Augentic** → install the marketplace → restart Cursor. That adds the Specify (`/spec:*`) plugin.

### 2. CLI

The binary backs every workflow skill. `/spec:init` can bootstrap a missing CLI after confirmation. For manual setup:

```bash
cargo install --git https://github.com/augentic/specify
```

Or download the platform archive from [GitHub Releases](https://github.com/augentic/specify/releases) and verify it against its `.sha256` companion.

See [Prerequisites](docs/orientation/prerequisites.md) for all install paths, shell completions, and adapter-specific tooling.

## Documentation

- [Developer Guide](docs/SUMMARY.md) — tutorials, how-tos, explanation, and reference (mdBook)
- [Quick reference](docs/reference/quick-reference.md) — single-page cheat sheet
- [What is Specify?](docs/orientation/index.md) — orientation and prerequisites
- [AGENTS.md](AGENTS.md) — repository agent instructions
- [CONTRIBUTING.md](CONTRIBUTING.md) · [GOVERNANCE.md](GOVERNANCE.md) · [Code of Conduct](CODE-OF-CONDUCT.md)



## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.