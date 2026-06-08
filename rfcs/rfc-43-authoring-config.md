# RFC-43: Authoring Configuration (`specify-authoring.yaml`)

> Status: Draft · Extends: [RFC-41 version binding](./rfc-41-version-binding.md) · Relates: [RFC-44 authoring vs runtime surface split](./rfc-44-surface-split.md) (consumes this file), [Roadmap §"Keep enforcement surfaces distinct"](./roadmap.md) (`specdev` / `specrun`) · Scope: the `augentic/specify` framework repo, plus one authoring schema shipped from `augentic/specify-cli`

## Abstract

The configuration that governs **how the framework is built and checked** — which `specify` binary to bind to, the cross-repo compatibility pin, lint defaults, acceptance prep, the declared platform set — is scattered across `.specify-version`, the `Makefile`, `scripts/specify.sh`, and `.github/workflows/ci.yaml`. There is no single, reviewable, schema-validated home for it, and it is easy to confuse with **runtime** configuration (`.specify/project.yaml`), which governs how a *consumer project* uses Specify.

This RFC introduces a first-class **authoring configuration** file at the framework repo root — `specify-authoring.yaml` — modelled on mdBook's `book.toml`: a static blueprint consumed by the authoring tooling, distinct from the artifact it produces. It consolidates the RFC-41 scatter into one place, disambiguates the framework repo's dual role (author of adapters *and* dogfooding consumer of Specify), and gives the authoring tier the same schema-validated discipline every other Specify artifact already enjoys.

This is a configuration-consolidation RFC. It introduces no new lifecycle authority and no new binary. The `specify-cli` work that *reads* this file is deferred to [RFC-44](./rfc-44-surface-split.md); RFC-43 stands alone and delivers value without it (the existing `make lint` / `scripts/specify.sh` path keeps working unchanged until the consumer lands).

## Motivation

### The configuration is scattered and untyped

RFC-41 landed a coherent *binding* model but spread its inputs across four surfaces:

- `.specify-version` — the single-line published-CLI compatibility pin.
- `Makefile` — `SPECIFY_VERSION ?= next`, `SPECIFY_MANIFEST`, `INSTALL_DIR`, the `install-specify` target.
- `scripts/specify.sh` — resolution/acquisition defaults (`lint` → `lint framework --framework-root .`), `./.bin` target, release-probe order.
- `.github/workflows/ci.yaml` — the workflow-level `SPECIFY_VERSION` override and the `.specify-version` fallback.

A contributor who wants to know "what does this repo target, and how is it checked?" must read four files in three languages (Make, Bash, YAML) and a bare semver file. None of it is schema-validated, so a typo in the pin or a stray Makefile variable surfaces only at runtime.

### The framework repo overloads `.specify/`

`augentic/specify` is simultaneously the **author** of adapters/plugins/rules and a **dogfooding consumer** of Specify (it carries a runtime `.specify/`, including `.specify/journal.jsonl`). Today nothing on disk distinguishes "configuration for building the framework" from "configuration for being a Specify project." A reader cannot tell, from the presence of `.specify/`, which hat the repo is wearing. A dedicated authoring file at the repo root makes the distinction physical: `.specify/project.yaml` is *runtime* (this repo as a Specify project); `specify-authoring.yaml` is *authoring* (how this repo's adapters and skills are built and checked).

### mdBook precedent

mdBook's `book.toml` is the static blueprint the `mdbook` tool consumes at build time — `[build]`, `[rust]` (edition for doctests), `[preprocessor.*]`, `[output.*]`. The rendered output carries none of it. The framework repo wants the same shape: one blueprint that the authoring tooling reads, kept separate from the artifacts (adapters, skills, rules) it produces and from the runtime config consumer projects carry. The disanalogy — Specify *has* a live runtime where mdBook does not — is exactly why authoring config must be its own file rather than folded into the runtime `project.yaml`.

## Design: one authoring file

A single YAML file at the framework repo root, `specify-authoring.yaml`, validated against a new `schemas/authoring/framework.schema.json` shipped from `specify-cli` (alongside `authoring/skill`, `authoring/scenario`, `authoring/marketplace`). YAML, not TOML, for consistency with every other Specify control file; a plain file at the repo root for the same readability reasons RFC-41 chose a plain `.specify-version` over a Make variable.

Sketch (illustrative; the schema is the contract):

```yaml
# specify-authoring.yaml — how to BUILD and CHECK the framework.
# NOT how to USE Specify in a project (that is .specify/project.yaml).

framework:
  name: augentic-specify

cli:                      # absorbs .specify-version + the SPECIFY_VERSION default
  version: 0.1.0          # the reviewable cross-repo compatibility pin
  source: next            # next | X.Y.Z  (the SPECIFY_VERSION knob's default)
  bin-dir: .bin           # repo-local acquisition target (gitignored)

lint:                     # absorbs scripts/specify.sh `lint` defaults
  framework-root: .

acceptance:               # absorbs Makefile INSTALL_DIR + acceptance posture
  install-dir: ~/.local/bin
  generated-output-gates: [omnia, vectis, contracts]

platforms: [core, ios, android]
```

### What each block absorbs

| Block | Replaces | Notes |
| --- | --- | --- |
| `cli.version` | `.specify-version` | The single declared-compatible published version. `.specify-version` is removed; this becomes the one place the pin lives. |
| `cli.source` | `SPECIFY_VERSION ?= next` default in `Makefile` | The `next | X.Y.Z` knob keeps its env override (`SPECIFY_VERSION=…`); the file supplies the default. |
| `cli.bin-dir` | hard-coded `./.bin` in `scripts/specify.sh` | Stays gitignored; configurable for CI caching. |
| `lint.framework-root` | `lint` shorthand argument | The default scan root for the authoring lint. |
| `acceptance.install-dir` | `INSTALL_DIR ?= $(HOME)/.local/bin` | Where `make install-specify` symlinks the build under test. |
| `acceptance.generated-output-gates` | implicit per-target list in RFC-42 | The targets whose generated output must pass `cargo check` / `test` / replay (RFC-42 Phase 2). |
| `platforms` | nothing today (implicit) | The framework's declared platform set, mirroring the runtime `project.yaml.platforms`. |

### `.specify-version` retirement vs. coexistence

The pin moves into `cli.version`. To keep RFC-41's CI and script paths working during migration, RFC-43 lands in two steps within one PR: (1) add `specify-authoring.yaml` with `cli.version` equal to the current `.specify-version`; (2) point `scripts/specify.sh`, the `Makefile`, and CI at the new file, then delete `.specify-version`. A guard test (or a `lint framework` check, see below) asserts the two never drift while both exist.

### Schema home and validation

The authoring schema lives in `specify-cli` under `schemas/authoring/framework.schema.json`, embedded in the binary like every other schema (per `specify-cli`'s `crates/schema/`), and surfaced for editors via the standard `# yaml-language-server: $schema=…raw.githubusercontent.com/augentic/specify-cli/main/schemas/authoring/framework.schema.json` directive documented in [`docs/contributing/checks.md`](../docs/contributing/checks.md). A `CORE-*` rule (Road A `schema` hint) validates `specify-authoring.yaml` on `make lint`, exactly as adapter manifests and skill frontmatter are validated today.

### How the tooling reads it

RFC-43 keeps the existing consumers (`scripts/specify.sh`, `Makefile`, CI) authoritative and teaches them to read the file with shell/Make primitives — a `yq`/`awk` read of `cli.version` and `cli.source`, replacing the `.specify-version` `tr` read and the Make `?=` default. The bash resolver stays the source of truth for *resolution and acquisition*; the file is only the source of truth for *the declared values*. Teaching the `specify` binary itself to auto-discover `specify-authoring.yaml` (so the script shrinks to a bootstrap-only shim) is explicitly deferred to [RFC-44](./rfc-44-surface-split.md), which owns the binary surface.

## Migration

A single PR, additive then subtractive:

- Add `specify-authoring.yaml` at the repo root with values mirroring today's `.specify-version` (`0.1.0`), `Makefile`, and CI defaults.
- Add `schemas/authoring/framework.schema.json` in `specify-cli` and a `CORE-*` `schema` rule that validates the file (this half lands in `specify-cli` and bumps the `cli.version` pin once released).
- Repoint `scripts/specify.sh` `read_pin` and the `Makefile` `SPECIFY_VERSION` / `INSTALL_DIR` defaults at the new file.
- Repoint `.github/workflows/ci.yaml` `Resolve SPECIFY_VERSION` at `cli.version` (workflow `env.SPECIFY_VERSION` override still wins).
- Delete `.specify-version`; update [`docs/contributing/checks.md`](../docs/contributing/checks.md), [`docs/contributing/index.md`](../docs/contributing/index.md), [`docs/contributing/acceptance.md`](../docs/contributing/acceptance.md), and [`AGENTS.md`](../AGENTS.md) to reference `specify-authoring.yaml`.

Because the `schema` validation half ships from `specify-cli`, the `cli.version` pin must name a published release that carries the new `CORE-*` rule before CI can enforce it — the standard RFC-41 pin-bump-on-release discipline applies.

## Non-Goals

- **No change to runtime `.specify/project.yaml`.** Runtime config is unchanged; this RFC only carves authoring config out of the scatter and away from `.specify/`.
- **No new binary, no surface split.** Renaming/splitting the `specify` surface is [RFC-44](./rfc-44-surface-split.md)'s scope.
- **No new lifecycle or gate authority.** This is build tooling for one repo; it touches no workflow contract, lint authority, or artifact.
- **No replacement of `scripts/specify.sh` resolution logic.** The script keeps owning resolution and acquisition; the file only supplies declared values. Auto-discovery by the binary is deferred to RFC-44.
- **No TOML.** YAML stays the single control-file format across Specify.

## Open Questions

1. **Naming.** `specify-authoring.yaml` vs `Specify.toml` (the `Cargo.toml` cadence) vs `.specify/authoring.yaml`. Current preference: root `specify-authoring.yaml`, YAML, to keep it discoverable and out of the overloaded `.specify/` tree.
2. **Schema namespace.** `schemas/authoring/framework.schema.json` vs a broader `schemas/authoring/authoring-config.schema.json` if non-framework authoring repos ever adopt the file.
3. **`platforms` duplication.** The framework's `platforms` mirrors runtime `project.yaml.platforms`; should one project derive the other, or do they stay independently declared?
4. **How much of `scripts/specify.sh` survives.** Once RFC-44 lands binary-side auto-discovery, how thin can the bash resolver become before it is just a `./.bin` bootstrap?

## References

- [`rfc-41-version-binding.md`](./rfc-41-version-binding.md) — the binding model this RFC consolidates; [`rfc-41-plan.md`](./rfc-41-plan.md) — its execution slices.
- [`rfc-44-surface-split.md`](./rfc-44-surface-split.md) — the binary-surface RFC that consumes this file.
- [`roadmap.md`](./roadmap.md) — the `specdev` / `specrun` enforcement-surface split this RFC's authoring tier serves.
- [`Makefile`](../Makefile), [`scripts/specify.sh`](../scripts/specify.sh), [`.specify-version`](../.specify-version), [`.github/workflows/ci.yaml`](../.github/workflows/ci.yaml) — the four surfaces consolidated.
- [`docs/contributing/checks.md`](../docs/contributing/checks.md) — the binding model and the editor-schema wiring this RFC extends.
- [`docs/explanation/standards-layer.md`](../docs/explanation/standards-layer.md) — the authoring-vs-engineering-standards boundary this file sits inside.
- [`specify-cli` `crates/schema/src/`](https://github.com/augentic/specify-cli/tree/main/crates/schema/src) — where the new authoring schema is embedded.
