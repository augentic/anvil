# Contributing to Specify

This section is for developers working on the Specify framework itself -- the skills, capabilities, references, and CLI that power the `/spec:*` workflow. If you are looking for how to *use* Specify in your own project, start with [What is Specify?](../orientation/index.md).

## Repository map

Specify spans two repositories:

| Repository | Contents | Language |
|------------|----------|----------|
| [`augentic/specify`](https://github.com/augentic/specify) | Skills, capabilities, brief templates, shared references, documentation, marketplace manifest | Markdown, YAML, TypeScript (checks) |
| [`augentic/specify-cli`](https://github.com/augentic/specify-cli) | The `specify` binary and its workspace crates | Rust |

The `specify` repo defines *what agents do* (skills) and *how artifacts are generated* (capabilities and briefs). The `specify-cli` repo implements *deterministic operations* that skills delegate to -- lifecycle transitions, validation, spec merging, plan management, and task tracking.

The two repos are independently versioned and released. Skills invoke the CLI as a subprocess (`specify change create ...`, `specify slice validate ...`, etc.) and consume its JSON output. They never import Rust code directly.

## Development environment

**For skill and capability work** (specify repo):

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
4. **Run checks.** `make checks` in the specify repo; `cargo make ci` in the specify-cli repo.
5. **Open a pull request** against `main`. All patches require at least one maintainer review.
6. **Sign off.** Every commit must carry a DCO sign-off (`git commit -s`). See [CONTRIBUTING.md](https://github.com/augentic/specify/blob/main/CONTRIBUTING.md) for the full certificate text.

## What to read next

- [Skill Authoring Standards](../standards/skill-authoring.md) -- the enforced rules for every `SKILL.md` (frontmatter shape, body caps, references discipline) plus the long-form rationale
- [Anatomy of a Capability](capability-anatomy.md) -- how capabilities declare brief pipelines
- [Plugin Development](plugin-development.md) -- the dev/prod workflow, marketplace manifest, and testing
- [CLI Architecture](cli-architecture.md) -- crate graph, dispatch pattern, and JSON contract
- [Consistency Checks](checks.md) -- what `make checks` enforces and how to extend it

## Example Patterns

Advanced examples live beside the skills that own them, so they stay close to the implementation rules they illustrate:

- `plugins/omnia/skills/crate-writer/examples/` -- generated crate patterns and update cases.
- `plugins/omnia/skills/test-writer/examples/` -- provider-backed test patterns.
- `plugins/vectis/skills/core-writer/references/examples/` -- Crux core examples.
- `plugins/vectis/skills/ios-writer/references/examples/` and `plugins/vectis/skills/android-writer/references/examples/` -- shell integration examples.
- `plugins/change/skills/execute/fixtures/` and `plugins/change/skills/plan/fixtures/` -- plan and execution transcripts for workflow behavior.
