# Emery

[CI](https://github.com/augentic/emery/actions/workflows/ci.yaml)
[License: MIT OR Apache-2.0](#license)
[Docs](https://emery.augentic.io/)

Emery reconciles intent, documentation, existing code, and captured behaviour into reviewable specifications — durable artifacts, not chat history.

> **Remediation programme in flight.** The v1 delivery workflow (survey/extract, plan/refine/execute/finalize, target adapters) is archived at git tag `v1`. This tree carries the spec generator: `emery specify` synthesises the reviewable set from the sources named on the invocation, `emery show` renders it. Retrieve the archived implementation with `git worktree add ../emery-v1 v1`.

## The live surface

```bash
emery specify <adapter>...  # extract, reconcile, synthesise spec.md + design.md
emery show spec             # print a reviewable document of the current revision
emery completions <sh>      # shell completions
```

In Cursor, `/emery:specify` wraps `emery specify`. Everything else was deleted from the grammar, not hidden — see the [CLI reference](docs/reference/cli/index.md).

Install from source:

```bash
cargo install --git https://github.com/augentic/emery --locked
emery --version
```

## Documentation

- **CLI reference:** [docs/reference/cli/index.md](docs/reference/cli/index.md)
- **Contributing:** [docs/contributing/index.md](docs/contributing/index.md)
- **Full Developer Guide:** [emery.augentic.io](https://emery.augentic.io/) · [In-tree book source](docs/SUMMARY.md)

## Developing Emery (contributors)

The repository root is a Rust workspace producing the `emery` binary. The root `Makefile` forwards every goal to [mise](mise.toml).

```bash
make test    # native integration suite
make check   # format, lint, tests, doctests, and docs
make ci      # full pre-commit gate, including vet and deny
```

Preview the working-tree Cursor skill against a local CLI:

```bash
cursor-agent --plugin-dir plugins/emery
```

Start with the [developer loop](docs/contributing/dev-loop.md), then [Cursor operator plugins](docs/contributing/operator-plugins.md) and [CONTRIBUTING.md](CONTRIBUTING.md). See also [GOVERNANCE.md](GOVERNANCE.md) and [Code of Conduct](CODE-OF-CONDUCT.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
