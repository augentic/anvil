# Contributing to Emery

This section is for developers working on the Emery framework itself — the Rust runtime, the Cursor skill wrapper, and docs. The v1 workflow is archived at git tag `v1`; feature work is frozen until the spec walking skeleton is green.

## Repository map

The engine and operator plugin live in [`augentic/emery`](https://github.com/augentic/emery); source adapters live in the sibling [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters):

| Path | Contents | Language |
| ---- | -------- | -------- |
| `src/`, `crates/` | The runtime and workspace crates | Rust |
| `plugins/`, `docs/` | The ultrathin Cursor skill wrapper and documentation | Markdown, YAML |
| `emery-adapters/sources/` | Source adapter crates plus embedded prose | Rust, Markdown |

The Rust workspace owns deterministic operations. The `/emery:specify` skill under `plugins/emery/` is an ultrathin invoke-and-relay wrapper over the CLI verb.

## Development environment

**For docs and skill-wrapper work** (repo root):

- [Cursor IDE](https://cursor.com) with the Augentic plugin marketplace
- [mdBook](https://rust-lang.github.io/mdBook/) — for building documentation locally (optional)

**For tooling and CLI work** (the Rust workspace at the repo root):

- Rust stable toolchain — `cargo build` and the test suites use the channel pinned in [`rust-toolchain.toml`](../../rust-toolchain.toml); `cargo make fmt` uses nightly rustfmt
- [cargo-make](https://sagiegurari.github.io/cargo-make/) — the root `Makefile` forwards unknown targets to `Makefile.toml`
- [cargo-nextest](https://nexte.st/) — test runner used by the CI targets
- [cargo-deny](https://embarkstudios.github.io/cargo-deny/) + [cargo-vet](https://mozilla.github.io/cargo-vet/) — supply-chain checks

## Building from a checkout

Contributing needs only a Rust toolchain, not a separately installed `emery`:

```bash
cargo make links # Developer Guide link integrity
make ci          # the full Rust workspace gate (cargo make ci)
cargo install --path . --locked # install the working-tree CLI into ~/.cargo/bin
```

No published binary is downloaded — every invocation builds from the in-tree Cargo workspace, so CI and clean clones build the same source. See [Quality gates](quality-gates.md#consistency-links).

## Contribution workflow

1. **Discuss first.** Open a GitHub issue before starting work to confirm alignment with the remediation plan.
2. **Branch from `main`.** Create a feature branch for your change.
3. **Make your edits.** Follow the conventions described in the sub-pages below.
4. **Run checks.** `cargo make links` for the Developer Guide; `cargo make ci` (or `make ci`) for the full gate.
5. **Open a pull request** against `main`. All patches require at least one maintainer review.
6. **Sign off.** Every commit must carry a DCO sign-off (`git commit -s`). See [CONTRIBUTING.md](https://github.com/augentic/emery/blob/main/CONTRIBUTING.md) for the full certificate text.

## What to read next

- [Quality gates](quality-gates.md) — the gate model and the links gate
- [Testing standards](../standards/testing.md) — the integration-first posture, the triage buckets, and the coverage brake (`cargo make cov`)
- [The developer loop](dev-loop.md) — the local rungs
- [CLI Architecture](cli-architecture.md) — dispatch pattern and JSON contract
- [Cursor operator plugins](operator-plugins.md) — marketplace layout and local `--plugin-dir` preview
