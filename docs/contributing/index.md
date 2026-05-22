# Contributing to Specify

This section is for developers working on the Specify framework itself -- the skills, adapters, references, and CLI that power the `/spec:*` workflow. If you are looking for how to *use* Specify in your own project, start with [What is Specify?](../orientation/index.md).

## Repository map

Specify spans two repositories:

| Repository | Contents | Language |
|------------|----------|----------|
| [`augentic/specify`](https://github.com/augentic/specify) | Skills, adapters, brief templates, shared references, documentation, marketplace manifest | Markdown, YAML, TypeScript (checks) |
| [`augentic/specify-cli`](https://github.com/augentic/specify-cli) | The `specify` binary and its workspace crates | Rust |

The `specify` repo defines *what agents do* (skills) and *how artifacts are generated* (adapters and briefs). The `specify-cli` repo implements *deterministic operations* that skills delegate to -- lifecycle transitions, validation, spec merging, plan management, and task tracking.

The two repos are independently versioned and released. Skills invoke the CLI as a subprocess (`specify plan add ...`, `specify slice validate ...`, etc.) and consume its JSON output. They never import Rust code directly.

## Development environment

**For skill and adapter work** (specify repo):

- [Cursor IDE](https://cursor.com) with the Augentic plugin marketplace
- [Deno](https://deno.land) -- runs `scripts/checks.ts` via `make checks`
- [mdBook](https://rust-lang.github.io/mdBook/) + [D2](https://d2lang.com/) -- for building documentation locally (optional)

**For CLI work** (specify-cli repo):

- Rust stable toolchain
- [cargo-make](https://sagiegurari.github.io/cargo-make/) -- the `Makefile` forwards to `Makefile.toml`
- [cargo-nextest](https://nexte.st/) -- test runner used by the CI targets
- [cargo-deny](https://embarkstudios.github.io/cargo-deny/) + [cargo-vet](https://mozilla.github.io/cargo-vet/) -- supply-chain checks

## Contribution workflow

1. **Discuss first.** Open a GitHub issue before starting work to confirm alignment with the roadmap.
2. **Branch from `main`.** Create a feature branch for your change.
3. **Make your edits.** Follow the conventions described in the sub-pages below.
4. **Run checks.** `make checks` in the specify repo; `cargo make ci` in the specify-cli repo. For documentation changes, also run `make docs` before opening the PR.
5. **Open a pull request** against `main`. All patches require at least one maintainer review.
6. **Sign off.** Every commit must carry a DCO sign-off (`git commit -s`). See [CONTRIBUTING.md](https://github.com/augentic/specify/blob/main/CONTRIBUTING.md) for the full certificate text.

## What to read next

- [RFC-27 synthesis](../../rfcs/archive/rfc-27-synthesis.md) -- evidence fusion, authority, and cache; normative 2.0 workflow contract in [rfc-25-workflow.md](../../rfcs/archive/rfc-25-workflow.md)
- [Skill Authoring Standards](../standards/skill-authoring.md) -- the enforced rules for every `SKILL.md` (frontmatter shape, body caps, references discipline) plus the long-form rationale
- [Anatomy of a Adapter](adapter-anatomy.md) -- how adapters declare brief pipelines
- [Plugin Development](plugin-development.md) -- the dev/prod workflow, marketplace manifest, and testing
- [CLI Architecture](cli-architecture.md) -- crate graph, dispatch pattern, and JSON contract
- [Consistency Checks](checks.md) -- what `make checks` enforces and how to extend it

## Example Patterns

Advanced examples live beside the skills that own them, so they stay close to the implementation rules they illustrate:

- [`adapters/targets/omnia/briefs/build.md`](../../adapters/targets/omnia/briefs/build.md) -- generated crate patterns, update cases, and provider-backed test patterns (the bodies of the retired `omnia-crate-writer` and `omnia-test-writer` skills moved into this brief in RFC-25 W2.5).
- [`plugins/vectis/references/`](../../plugins/vectis/references/) and [`adapters/targets/vectis/examples/`](../../adapters/targets/vectis/examples/) -- Crux core, iOS / Android shell, and design-system reference material consumed by [`adapters/targets/vectis/briefs/`](../../adapters/targets/vectis/briefs/).
- [`tests/plan/`](../../tests/plan/) and [`tests/cross-repo/`](../../tests/cross-repo/) -- plan-time and end-to-end scenario packs covering `/spec:plan`, `/spec:execute`, and `/spec:finalize`.
