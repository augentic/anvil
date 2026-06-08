# Contributing to Specify

This section is for developers working on the Specify framework itself -- the skills, adapters, references, and CLI that power the `/spec:*` workflow. If you are looking for how to *use* Specify in your own project, start with [What is Specify?](../orientation/index.md).

## Repository map

Specify spans two repositories:

| Repository | Contents | Language |
|------------|----------|----------|
| [`augentic/specify`](https://github.com/augentic/specify) | Skills, adapters, brief templates, shared references, documentation, marketplace manifest | Markdown, YAML |
| [`augentic/specify-cli`](https://github.com/augentic/specify-cli) | The `specify` binary (workflow runtime + `specify lint framework`) and workspace crates | Rust |

The `specify` repo defines *what agents do* (skills) and *how artifacts are generated* (adapters and briefs). The `specify-cli` repo implements *deterministic operations* that skills delegate to -- lifecycle transitions, validation, spec merging, plan management, and task tracking.

The two repos are independently versioned and released. Skills invoke the CLI as a subprocess (`specify plan add ...`, `specify slice validate ...`, etc.) and consume its JSON output. They never import Rust code directly.

## Who you're contributing for

Two audiences share this repository:

| Audience | Typical edits | `specify-cli` checkout needed? |
|----------|---------------|--------------------------------|
| **Skill and adapter authors** | `SKILL.md`, adapter briefs, references, docs | No — markdown and YAML only |
| **Tooling contributors** | `specify-standards` framework predicates, schemas, acceptance tests | Yes — they work in the Rust workspace |

Markdown-only contributors can run `make lint` locally without a `specify-cli` checkout: it acquires the `.specify-version`-pinned published `specify` binary into a repo-local `./.bin` (see [Consistency Checks](checks.md#binding-to-a-specify-binary)). Tooling contributors keep a sibling `specify-cli` checkout — `make lint` then builds the binary from source (the default `next` mode), and `cargo make test` in that checkout exercises the `specify-standards` framework predicate suite before opening a PR.

## Development environment

**For skill and adapter work** (specify repo):

- [Cursor IDE](https://cursor.com) with the Augentic plugin marketplace
- [mdBook](https://rust-lang.github.io/mdBook/) — for building documentation locally (optional)

**For tooling work** (`specify-cli` repo, `crates/standards/`):

- Rust stable toolchain
- A sibling checkout of [`augentic/specify-cli`](https://github.com/augentic/specify-cli) to build the framework checker from source (the default `next` mode); optional for `make lint`, which falls back to a published binary when no checkout is present

**For CLI work** (specify-cli repo):

- Rust stable toolchain
- [cargo-make](https://sagiegurari.github.io/cargo-make/) -- the `Makefile` forwards to `Makefile.toml`
- [cargo-nextest](https://nexte.st/) -- test runner used by the CI targets
- [cargo-deny](https://embarkstudios.github.io/cargo-deny/) + [cargo-vet](https://mozilla.github.io/cargo-vet/) -- supply-chain checks

## Contribution workflow

1. **Discuss first.** Open a GitHub issue before starting work to confirm alignment with the roadmap.
2. **Branch from `main`.** Create a feature branch for your change.
3. **Make your edits.** Follow the conventions described in the sub-pages below.
4. **Run checks.** `make lint` in the specify repo (works for every contributor — no `specify-cli` checkout required). `cargo make ci` in the specify-cli repo for CLI work. For documentation changes, also run `mdbook build docs` before opening the PR.
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
- [`adapters/targets/vectis/references/`](../../adapters/targets/vectis/references/) and [`adapters/targets/vectis/examples/`](../../adapters/targets/vectis/examples/) -- Crux core, iOS / Android shell, and design-system reference material consumed by [`adapters/targets/vectis/briefs/`](../../adapters/targets/vectis/briefs/).
- [`acceptance/scenarios/`](../../acceptance/scenarios/README.md) -- the unified scenario pack covering the `/spec:*` change lifecycle (`/spec:plan`, `/spec:execute`, `/spec:finalize`) from N=1 through multi-repo, happy-path through failure and recovery.
