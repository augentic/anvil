# RFC-43: Authoring Configuration (`Specify.toml`)

> Status: Draft · Extends: [RFC-41 version binding](./rfc-41-version-binding.md) · Relates: [RFC-44 authoring vs runtime surface split](./rfc-44-surface-split.md) (consumes this file), [Roadmap §"Keep enforcement surfaces distinct"](./roadmap.md) (`specdev` / `specrun`) · Scope: the `augentic/specify` framework repo, plus one authoring schema shipped from `augentic/specify-cli`

## Abstract

The configuration that governs **how the framework is built and checked** — which `specify` binary to bind to and where it lives on disk — is scattered across `.specify-version`, the `Makefile`, `scripts/specify.sh`, and `.github/workflows/ci.yaml`. There is no single, reviewable, schema-validated home for it, and it is easy to confuse with **runtime** configuration (`.specify/project.yaml`), which governs how a *consumer project* uses Specify.

This RFC introduces a first-class **authoring configuration** file at the framework repo root — `Specify.toml` — modelled on mdBook's `book.toml` and Rust's `Cargo.toml`: a static blueprint consumed by the authoring tooling, distinct from the artifact it produces. It consolidates the RFC-41 scatter into one place, disambiguates the framework repo's dual role (author of adapters *and* dogfooding consumer of Specify), and gives the authoring tier the same schema-validated discipline every other Specify artifact already enjoys.

This is a configuration-consolidation RFC. It introduces no new lifecycle authority and no new binary. The `specify-cli` work that *reads* this file is deferred to [RFC-44](./rfc-44-surface-split.md); RFC-43 stands alone and delivers value without it (the existing `make lint` / `scripts/specify.sh` path keeps working unchanged until the consumer lands).

## Motivation

### The configuration is scattered and untyped

RFC-41 landed a coherent *binding* model but spread its inputs across four surfaces:

- `.specify-version` — a semver pin used when CI or the `next` fallback needs a published binary.
- `Makefile` — `SPECIFY_VERSION ?= next`, `SPECIFY_MANIFEST`, `INSTALL_DIR`, the `install-specify` target.
- `scripts/specify.sh` — resolution/acquisition defaults (`lint` → `lint framework`), hard-coded `./.bin/specify`, release-probe order.
- `.github/workflows/ci.yaml` — the workflow-level `SPECIFY_VERSION` override and the `.specify-version` fallback.

RFC-41's split — a semver *pin* in one file and a `next | X.Y.Z` *knob* in another — made sense mechanically but forced contributors to learn two names for one decision ("which `specify` binary?"). RFC-43 collapses binding into three obvious `[cli]` fields: `version`, `binary`, and optional `path`.

### The framework repo overloads `.specify/`

`augentic/specify` is simultaneously the **author** of adapters/plugins/rules and a **dogfooding consumer** of Specify (it carries a runtime `.specify/`, including `.specify/journal.jsonl`). Today nothing on disk distinguishes "configuration for building the framework" from "configuration for being a Specify project." A reader cannot tell, from the presence of `.specify/`, which hat the repo is wearing. A dedicated authoring file at the repo root makes the distinction physical: `.specify/project.yaml` is *runtime* (this repo as a Specify project); `Specify.toml` is *authoring* (how this repo's adapters and skills are built and checked).

### mdBook precedent

mdBook's `book.toml` is the static blueprint the `mdbook` tool consumes at build time — `[build]`, `[rust]` (edition for doctests), `[preprocessor.*]`, `[output.*]`. The rendered output carries none of it. The framework repo wants the same shape: one blueprint that the authoring tooling reads, kept separate from the artifacts (adapters, skills, rules) it produces and from the runtime config consumer projects carry. The disanalogy — Specify *has* a live runtime where mdBook does not — is exactly why authoring config must be its own file rather than folded into the runtime `project.yaml`.

## Design: one authoring file

A single TOML file at the framework repo root, `Specify.toml`, validated against a new `schemas/authoring/framework.schema.json` shipped from `specify-cli` (alongside `authoring/skill`, `authoring/scenario`, `authoring/marketplace`). TOML, not YAML, for the same reasons mdBook and Cargo use it for build blueprints: table sections map cleanly to the absorbed Makefile / shell / CI knobs, and the filename cadence (`Specify.toml` beside `Cargo.toml` in downstream repos) signals "how this tree is built" without overloading the runtime YAML control files under `.specify/`.

Sketch (illustrative; the schema is the contract):

```toml
# Specify.toml — how to BUILD and CHECK the framework (augentic/specify).
# NOT how to USE Specify in a project (that is .specify/project.yaml).

[cli]
version = "0.1.0"          # next | latest | X.Y.Z — contract for the binary at `binary`
binary = ".bin/specify"    # repo-local installed binary (parent dir gitignored)
path = "~/.local/bin"      # optional: directory on PATH; install-specify symlinks path/specify → binary
```

### `[cli]` binding model

Three fields, one mental model for maintainers:


| Field     | Type                        | Role                                                                                                                                                                              |
| --------- | --------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `version` | `next`, `latest`, or semver | What the file at `binary` must satisfy. Replaces `.specify-version` and `SPECIFY_VERSION ?= next`. Env `SPECIFY_VERSION=…` still overrides for one-off runs.                      |
| `binary`  | file path                   | The **one executable** `make lint` runs. Parent directory stays gitignored; the path itself is committed. CI caches `binary`'s parent (today `.bin/`).                            |
| `path`    | directory path, optional    | A directory that should already be on the operator's `PATH`. `make install-specify` creates `$(path)/specify` → symlink to `binary`. Omit or leave unset for CI (no PATH wiring). |


**Resolution** (`scripts/specify.sh` keeps owning this; the file declares targets):

1. Read `version`, `binary`, and optional `path` from `Specify.toml` (env overrides unchanged).
2. If `binary` exists, is executable, and satisfies `version` → `exec "$binary" …`.
3. Else run today's fallback chain (sibling `specify-cli` source build, `cargo install` acquire, etc.) and **materialize into `binary`** (copy or symlink).
4. Always exec via `binary` — one stable path, not ephemeral `cargo run` prefixes or opportunistic `command -v specify`.

`**version` semantics** (same as today's `SPECIFY_VERSION`):

- `**next`** — build from a sibling/nested `specify-cli` checkout into `binary` (or symlink `target/release/specify` there). When no checkout is present, fall back to acquiring the semver committed in `version` when it is not `next`, or fail with a clear message.
- `**X.Y.Z**` — acquire or reuse a published release that reports that version; materialize at `binary`.

The committed `augentic/specify` tree typically pins a semver in `version` so CI and markdown-only contributors get a deterministic published binary; co-developers override with `SPECIFY_VERSION=next` when a sibling checkout is present.

`**path` semantics** — lint and CI ignore `path`. Only `make install-specify` uses it: `ln -sfn "$binary" "$(path)/specify"`, then warn if `path` is not on `PATH`. Replaces `INSTALL_DIR ?= $(HOME)/.local/bin` in the `Makefile`.

There is no `[framework]` identity block. Unlike runtime `project.yaml.name` — which workspace sync, plan proposals, and topology reads — nothing in the authoring toolchain needs a declared repo name: the file is only `[cli]`. A human-readable label belongs in the file-top comment; a load-bearing identity field waits until a concrete consumer exists.

There is no `platforms` field. Target platform sets (`core`, `ios`, `android`, …) are runtime facts on `.specify/project.yaml` — set at `specify init --platforms`, consumed by plan reconciliation, Vectis build/verify, and related workflow paths. They do not belong in authoring config.

There is no `[acceptance]` section. Generated-output gates and other acceptance policy live in [RFC-42](./rfc-42-acceptance.md) and `[docs/contributing/acceptance.md](../docs/contributing/acceptance.md)`; nothing in the acceptance toolchain reads `Specify.toml` today.

There is no lint scan-root field. `specify lint framework` already defaults `--framework-root` to `.` (the cwd), and `scripts/specify.sh lint` `cd`s to the repo root before invoking it — so a lint root entry would only restate a constant. The authoring lint scan root stays on the CLI flag (`--framework-root` / `SPECIFY_ROOT`) for tests, alternate checkouts, and wrong-cwd escape hatches; [RFC-44](./rfc-44-surface-split.md) may later infer it from the directory containing `Specify.toml`, same as runtime config resolves from `.specify/project.yaml`.

### What each block absorbs


| Block         | Replaces                                             | Notes                                                               |
| ------------- | ---------------------------------------------------- | ------------------------------------------------------------------- |
| `cli.version` | `.specify-version` **and** `SPECIFY_VERSION ?= next` | Closed set `next` or semver `X.Y.Z`. `.specify-version` is removed. |
| `cli.binary`  | hard-coded `./.bin/specify` in `scripts/specify.sh`  | Single declared executable for lint and acquire materialization.    |
| `cli.path`    | `INSTALL_DIR ?= $(HOME)/.local/bin`                  | Optional PATH directory for `make install-specify` symlink only.    |


### `.specify-version` retirement

`.specify-version`, the Makefile `SPECIFY_VERSION ?=` default, hard-coded `./.bin/specify`, and `INSTALL_DIR` all fold into `[cli]`. Migration lands in one PR: (1) add `Specify.toml` with `cli.version` set to the current `.specify-version` semver, `cli.binary = ".bin/specify"`, and `cli.path = "~/.local/bin"`; (2) repoint `scripts/specify.sh`, the `Makefile`, and CI at the new fields, then delete `.specify-version`. A guard test (or a `lint framework` check) asserts the semver never drifts while both files exist.

### Schema home and validation

The authoring schema lives in `specify-cli` under `schemas/authoring/framework.schema.json`, embedded in the binary like every other schema (per `specify-cli`'s `crates/schema/`), and surfaced for editors via Taplo's schema directive (`#:schema …raw.githubusercontent.com/augentic/specify-cli/main/schemas/authoring/framework.schema.json`) documented in `[docs/contributing/checks.md](../docs/contributing/checks.md)`. A `CORE-`* rule (Road A `schema` hint) validates `Specify.toml` on `make lint` — the linter parses TOML to the same JSON shape the schema describes — exactly as adapter manifests and skill frontmatter are validated today.

### How the tooling reads it

RFC-43 keeps the existing consumers (`scripts/specify.sh`, `Makefile`, CI) authoritative and teaches them to read `[cli]` with shell/Make primitives — `version`, `binary`, and optional `path` via `toml-cli`, `taplo`, or an equivalent one-liner — replacing the `.specify-version` `tr` read, the Make `?=` defaults, and hard-coded `./.bin`. The bash resolver stays the source of truth for *resolution and acquisition*; the file is the source of truth for *where the binary lives* and *what version it must report*. Teaching the `specify` binary itself to auto-discover `Specify.toml` is explicitly deferred to [RFC-44](./rfc-44-surface-split.md), which owns the binary surface.

## Migration

A single PR, additive then subtractive:

- Add `Specify.toml` at the repo root with `[cli]` mirroring today's `.specify-version` semver, `.bin/specify`, and `~/.local/bin` defaults.
- Add `schemas/authoring/framework.schema.json` in `specify-cli` and a `CORE-`* `schema` rule that validates the file (this half lands in `specify-cli` and bumps the committed semver once released).
- Repoint `scripts/specify.sh` to read `cli.version` / `cli.binary`, materialize into `binary`, and always exec via `binary`. Co-developers who today rely on the implicit `next` default keep using `SPECIFY_VERSION=next` (env override unchanged).
- Repoint `make install-specify` at `cli.path` + `cli.binary` (drop `INSTALL_DIR`).
- Repoint `.github/workflows/ci.yaml` `Resolve SPECIFY_VERSION` at `cli.version` (workflow `env.SPECIFY_VERSION` override still wins); CI omits or ignores `cli.path`.
- Delete `.specify-version`; update `[docs/contributing/checks.md](../docs/contributing/checks.md)`, `[docs/contributing/index.md](../docs/contributing/index.md)`, `[docs/contributing/acceptance.md](../docs/contributing/acceptance.md)`, and `[AGENTS.md](../AGENTS.md)` to reference `Specify.toml`.

Because the `schema` validation half ships from `specify-cli`, the committed semver in `cli.version` must name a published release that carries the new `CORE-*` rule before CI can enforce it — the standard RFC-41 bump-on-release discipline applies.

## Non-Goals

- **No change to runtime `.specify/project.yaml`.** Runtime config is unchanged; this RFC only carves authoring config out of the scatter and away from `.specify/`.
- **No new binary, no surface split.** Renaming/splitting the `specify` surface is [RFC-44](./rfc-44-surface-split.md)'s scope.
- **No new lifecycle or gate authority.** This is build tooling for one repo; it touches no workflow contract, lint authority, or artifact.
- **No replacement of `scripts/specify.sh` resolution logic.** The script keeps owning resolution and acquisition; the file declares version contract and binary location. Auto-discovery by the binary is deferred to RFC-44.
- **No YAML for authoring config.** Runtime workflow artifacts (`.specify/project.yaml`, `plan.yaml`, adapter manifests, and so on) stay YAML; only the framework authoring blueprint uses TOML.
- **No repo identity block.** Runtime `project.yaml` keeps `name` for workflow topology; authoring config carries only operational knobs until a consumer needs a declared identity.
- **No lint scan-root in authoring config.** The plugin-repo scan root stays on `specify lint framework --framework-root` (CLI default `.`); it is not duplicated in `Specify.toml`.
- **No separate pin vs source fields.** One `cli.version` (`next | X.Y.Z`); no parallel `.specify-version` file or `cli.source` key.
- `**path` is not a binary path.** `binary` is the executable file; `path` is an optional directory for a PATH-facing symlink only.
- **No acceptance policy in authoring config.** Generated-output gates and scenario catalog policy stay in RFC-42 and `docs/contributing/acceptance.md`.
- **No platforms in authoring config.** Platform sets stay on runtime `project.yaml` only.

## Open Questions

1. **Naming.** `Specify.toml` (the `Cargo.toml` cadence, chosen here) vs `specify-authoring.toml` (more explicit, less collision-prone in polyglot repos) vs `.specify/authoring.toml` (keeps the root uncluttered but re-enters the overloaded `.specify/` tree).
2. **Schema namespace.** `schemas/authoring/framework.schema.json` vs a broader `schemas/authoring/authoring-config.schema.json` if non-framework authoring repos ever adopt the file.
3. `**path` in committed config.** Ship `path = "~/.local/bin"` in the repo default vs document it as a local-only override (CI never needs it).
4. **How much of `scripts/specify.sh` survives.** Once RFC-44 lands binary-side auto-discovery, how thin can the bash resolver become before it is just materialize-into-`binary` + exec?

## References

- `[rfc-41-version-binding.md](./rfc-41-version-binding.md)` — the binding model this RFC consolidates; `[rfc-41-plan.md](./rfc-41-plan.md)` — its execution slices.
- `[rfc-44-surface-split.md](./rfc-44-surface-split.md)` — the binary-surface RFC that consumes this file.
- `[roadmap.md](./roadmap.md)` — the `specdev` / `specrun` enforcement-surface split this RFC's authoring tier serves.
- `[Makefile](../Makefile)`, `[scripts/specify.sh](../scripts/specify.sh)`, `[.specify-version](../.specify-version)`, `[.github/workflows/ci.yaml](../.github/workflows/ci.yaml)` — the four surfaces consolidated.
- `[docs/contributing/checks.md](../docs/contributing/checks.md)` — the binding model and the editor-schema wiring this RFC extends.
- `[docs/explanation/standards-layer.md](../docs/explanation/standards-layer.md)` — the authoring-vs-engineering-standards boundary this file sits inside.
- `[specify-cli` `crates/schema/src/](https://github.com/augentic/specify-cli/tree/main/crates/schema/src)` — where the new authoring schema is embedded.

