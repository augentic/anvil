# Contributing to Emery

This section is for developers working on the Emery framework itself — the Rust runtime, guest orchestrations, embedded prompts, Cursor skill wrappers, and docs. If you are looking for how to *use* Emery in your own project, start with [What is Emery?](../orientation/index.md).

## Repository map

The engine and operator plugins live in [`augentic/emery`](https://github.com/augentic/emery); source and target adapters live in the sibling [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters):

| Path | Contents | Language |
| ---- | -------- | -------- |
| `src/`, `crates/`, `examples/` | The runtime, workspace crates (including the `native` host and the `probe` live-rung library), and the wasm / eval examples | Rust |
| `plugins/`, `docs/`, `.cursor-plugin/`, `rfcs/` | Ultrathin Cursor skill wrappers, documentation, and the marketplace manifest | Markdown, YAML |
| `emery-adapters/{sources,targets}/` | Source and target adapter crates plus embedded prose | Rust, Markdown |

The Rust workspace owns deterministic operations and guest orchestrations (lifecycle, validation, synthesis, plan execute, target build). Phase skills under `plugins/emery/` are ultrathin invoke-and-relay wrappers over those verbs. Adapter prompts in `emery-adapters` own domain-specific generation.

## Who you're contributing for

| Audience | Typical edits | Touches the Rust workspace? |
| -------- | ------------- | --------------------------- |
| **Runtime / tooling contributors** | Crates, handlers, orchestrations, integration tests | Yes — `src/` and `crates/` |
| **Adapter authors** | Adapter crates and `prose/prompts/` in `emery-adapters` | In that sibling repo |
| **Docs and skill-wrapper authors** | `docs/`, `plugins/*/skills/*/SKILL.md`, marketplace manifest | No — markdown and YAML only |

Developer Guide link integrity is the book's job — `cargo make links` runs `mdbook build docs` with `mdbook-linkcheck2` (see [Quality gates](quality-gates.md#consistency-links)). Tooling contributors run the full `cargo make ci` gate before opening a PR. To preview working-tree skill wrappers, pass `--plugin-dir plugins/<name>` to Cursor Agent.

## Development environment

**For docs and skill-wrapper work** (repo root):

- [Cursor IDE](https://cursor.com) with the Augentic plugin marketplace
- [mdBook](https://rust-lang.github.io/mdBook/) — for building documentation locally (optional)

**For tooling and CLI work** (the Rust workspace at the repo root):

- Rust stable toolchain — `cargo build` and the test suites use the channel pinned in [`rust-toolchain.toml`](../../rust-toolchain.toml); `cargo make fmt` uses nightly rustfmt
- [cargo-make](https://sagiegurari.github.io/cargo-make/) — the root `Makefile` forwards unknown targets to `Makefile.toml`
- [cargo-nextest](https://nexte.st/) — test runner used by the CI targets
- [cargo-deny](https://embarkstudios.github.io/cargo-deny/) + [cargo-vet](https://mozilla.github.io/cargo-vet/) — supply-chain checks

## Contribution workflow

1. **Discuss first.** Open a GitHub issue before starting work to confirm alignment with the roadmap.
2. **Branch from `main`.** Create a feature branch for your change.
3. **Make your edits.** Follow the conventions described in the sub-pages below.
4. **Run checks.** `cargo make links` for the Developer Guide; `cargo make ci` (or `make ci`) for the full gate. For documentation changes, also run `mdbook build docs` before opening the PR.
5. **Open a pull request** against `main`. All patches require at least one maintainer review.
6. **Sign off.** Every commit must carry a DCO sign-off (`git commit -s`). See [CONTRIBUTING.md](https://github.com/augentic/emery/blob/main/CONTRIBUTING.md) for the full certificate text.

## What to read next

- [Quality gates](quality-gates.md) — test rungs, assertion ownership, and release cadence
- [Testing standards](../standards/testing.md) — the integration-first posture, the three layers, the triage buckets, and the coverage brake (`cargo make cov`)
- [The developer loop](dev-loop.md) — the two local rungs: `cargo make test` (native), `cargo make eval` (prompt evaluation) — plus the operator-run wasm example (`cargo make wasm-run`) for the WASM seam
- [Lifecycle](../reference/lifecycle.md) and [synthesis prompts](../../crates/slice/prompts/synthesize.md) — workflow state, evidence reconciliation, authority, and cache behavior
- [Anatomy of an adapter](../explanation/adapter-anatomy.md) — how adapters declare brief pipelines
- [CLI Architecture](cli-architecture.md) — dispatch pattern and JSON contract
- [Quality gates](quality-gates.md) — test rungs, links gate, and release cadence
- [Cursor operator plugins](operator-plugins.md) — marketplace layout and local `--plugin-dir` preview

## Example Patterns

Advanced examples live beside the adapters that own them:

- [`targets/omnia/prose/prompts/build.md`](https://github.com/augentic/emery-adapters/blob/main/targets/omnia/prose/prompts/build.md) — generated crate patterns, update cases, and provider-backed test patterns (this prompt carries the former crate-writer and test-writer behavior).
- [`targets/vectis/prose/references/`](https://github.com/augentic/emery-adapters/tree/main/targets/vectis/prose/references/) and [`targets/vectis/prose/references/examples/`](https://github.com/augentic/emery-adapters/tree/main/targets/vectis/prose/references/examples/) — Crux core, iOS / Android shell, and design-system reference material consumed by [`targets/vectis/prose/prompts/`](https://github.com/augentic/emery-adapters/tree/main/targets/vectis/prose/prompts/).
