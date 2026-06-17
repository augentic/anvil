# Contributing to Specify

This section is for developers working on the Specify framework itself -- the skills, adapters, references, and CLI that power the `/spec:*` workflow. If you are looking for how to *use* Specify in your own project, start with [What is Specify?](../orientation/index.md).

## Repository map

The platform lives in one repository, [`augentic/specify`](https://github.com/augentic/specify):

| Path | Contents | Language |
|------|----------|----------|
| repo root (`plugins/`, `adapters/`, `docs/`, `.cursor-plugin/`, `rfcs/`, `evals/`) | Skills, adapters, brief templates, shared references, documentation, marketplace manifest | Markdown, YAML |
| [`engine/`](https://github.com/augentic/specify/tree/main/engine) | The `specify` binary (workflow runtime + `specify lint framework`) and workspace crates | Rust |

The prose at the repo root defines *what agents do* (skills) and *how artifacts are generated* (adapters and briefs). The Rust workspace under `engine/` implements *deterministic operations* that skills delegate to -- lifecycle transitions, validation, spec merging, plan management, and task tracking.

The prose and the runtime share one version line and ship in one release. Skills invoke the CLI as a subprocess (`specify plan add ...`, `specify slice validate ...`, etc.) and consume its JSON output. They never import Rust code directly.

## Who you're contributing for

Two audiences share this repository:

| Audience | Typical edits | Touches the Rust workspace? |
|----------|---------------|-----------------------------|
| **Skill and adapter authors** | `SKILL.md`, adapter briefs, references, docs | No — markdown and YAML only |
| **Tooling contributors** | `specify-standards` framework predicates, schemas, deterministic tests | Yes — they work under `engine/` |

Every contributor runs `make lint` locally with only a Rust toolchain: it builds the in-tree `specify` binary under `engine/` and runs the framework checks (see [Consistency Checks](checks.md)). Tooling contributors additionally run `cargo make test` under `engine/` to exercise the `specify-standards` framework predicate suite before opening a PR. To preview working-tree plugin changes in Cursor, `make use-local-plugins` mirrors `plugins/` into the Cursor plugin cache.

## Development environment

**For skill and adapter work** (repo root):

- [Cursor IDE](https://cursor.com) with the Augentic plugin marketplace
- [mdBook](https://rust-lang.github.io/mdBook/) — for building documentation locally (optional)

**For tooling and CLI work** (`engine/`):

- Rust stable toolchain — `make lint` and `cd engine && cargo build` use the channel pinned in [`engine/rust-toolchain.toml`](../../engine/rust-toolchain.toml); `cargo make fmt` uses nightly rustfmt
- [cargo-make](https://sagiegurari.github.io/cargo-make/) -- the `engine/Makefile` forwards to `engine/Makefile.toml`
- [cargo-nextest](https://nexte.st/) -- test runner used by the CI targets
- [cargo-deny](https://embarkstudios.github.io/cargo-deny/) + [cargo-vet](https://mozilla.github.io/cargo-vet/) -- supply-chain checks

## Contribution workflow

1. **Discuss first.** Open a GitHub issue before starting work to confirm alignment with the roadmap.
2. **Branch from `main`.** Create a feature branch for your change.
3. **Make your edits.** Follow the conventions described in the sub-pages below.
4. **Run checks.** `make lint` over the prose (works for every contributor). `cd engine && cargo make ci` for CLI work, or `make ci` for the full Rust + prose gate. For documentation changes, also run `mdbook build docs` before opening the PR.
5. **Open a pull request** against `main`. All patches require at least one maintainer review.
6. **Sign off.** Every commit must carry a DCO sign-off (`git commit -s`). See [CONTRIBUTING.md](https://github.com/augentic/specify/blob/main/CONTRIBUTING.md) for the full certificate text.

## What to read next

- [Lifecycle](../reference/lifecycle.md) and [synthesis references](../../plugins/spec/references/synthesis/) -- workflow state, evidence reconciliation, authority, and cache behavior
- [Skill Authoring Standards](../standards/skill-authoring.md) -- the enforced rules for every `SKILL.md` (frontmatter shape, body caps, references discipline) plus the long-form rationale
- [Anatomy of an adapter](../explanation/adapter-anatomy.md) -- how adapters declare brief pipelines
- [Plugin Development](plugin-development.md) -- the dev/prod workflow, marketplace manifest, and testing
- [CLI Architecture](cli-architecture.md) -- crate graph, dispatch pattern, and JSON contract
- [Consistency Checks](checks.md) -- what `specify lint framework` enforces and how to extend it

## Example Patterns

Advanced examples live beside the skills that own them, so they stay close to the implementation rules they illustrate:

- [`adapters/targets/omnia/briefs/build.md`](../../adapters/targets/omnia/briefs/build.md) -- generated crate patterns, update cases, and provider-backed test patterns (this brief carries the crate-writer and test-writer behavior).
- [`adapters/targets/vectis/references/`](https://github.com/augentic/specify-adapters/tree/main/adapters/targets/vectis/references/) and [`adapters/targets/vectis/examples/`](https://github.com/augentic/specify-adapters/tree/main/adapters/targets/vectis/examples/) -- Crux core, iOS / Android shell, and design-system reference material consumed by [`adapters/targets/vectis/briefs/`](https://github.com/augentic/specify-adapters/tree/main/adapters/targets/vectis/briefs/).
- [`evals/scenarios/`](../../evals/scenarios/README.md) -- the unified scenario pack covering the `/spec:*` change lifecycle (`/spec:plan`, `/spec:execute`, `/spec:finalize`) from N=1 through multi-repo, happy-path through failure and recovery.
