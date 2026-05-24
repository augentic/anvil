# RFC-5: Framework Developer Tooling

> Status: Draft · Tracked by [roadmap RM-16](roadmap.md#rm-16-rfc-5-framework-developer-tooling-workspace) · Enables: [RFC-4](future/rfc-4-dsl.md) · Preserves contract with: [RFC-28](next/rfc-28-codex-rules.md)

## Abstract

Replace this repo's Deno tooling (`scripts/check.ts`, `scripts/gen-envelope-doc.ts`, `tests/cross_repo.ts`) with a small Rust workspace at `augentic/specify/tooling/`, landed as one atomic implementation PR.

The workspace lives in a dedicated subdirectory so skill and adapter authors see a markdown-first repo root (`plugins/`, `adapters/`, `docs/`) without `Cargo.toml`, `crates/`, or `rust-toolchain.toml` beside the content they edit.

Inside that atomic PR, the implementation order stays narrow-to-broad: JSON Schemas become the canonical contract for every artifact whose shape can be expressed declaratively; cross-file rules that schemas cannot express live in a single `rules` library crate; that library backs a thin `tooling` binary whose `check` subcommand runs locally and in CI. Once rule parity is proven on the branch, the same binary gains a `docgen` subcommand, an `accept` integration-test crate ports fixture acceptance, and Deno is removed from the local and CI gates before review.

The operator `specify` binary in `augentic/specify-cli` is deliberately **not** extended; framework tooling stays with the framework it validates.

## Motivation

The original RFC-5 framed this work as a one-for-one port of `scripts/check.ts` into `specify-cli/crates/check/`, exposed as `specify check`. That framing collapses several different problems into one binary on the wrong product:

- **Authoring feedback** (catch typos in `SKILL.md` / `adapter.yaml` as you type) belongs in the editor, not in CI.
- **CI gating** is the authoritative final check — but should run the same predicates as the local layers.
- **Cross-repo coherence** (specify ↔ specify-cli schemas, envelopes, error codes) is a library concern; sharing it through a CLI subcommand is awkward.
- **Doc generation** (`gen-envelope-doc.ts`) and **fixture acceptance** (`tests/cross_repo.ts`) are not "linting" at all but share the same Deno toolchain we are trying to retire.

Putting all of this on the operator `specify` binary conflates two audiences. Operators running Specify on a consumer project never need to validate `plugins/`, `adapters/{sources,targets}/`, or `.cursor-plugin/marketplace.json`. The runtime CLI must stay focused on its job: deterministic workflow primitives for consumer projects (init, plan, slice lifecycle, adapter resolution, merge, workspace sync, WASI tool dispatch).

The Deno scripts work today. The replaceable Deno surface is ~4,027 LOC across three surfaces and runs reliably in CI. This RFC is therefore not motivated by breakage but by three durable goals:

1. **Shift authoring feedback left.** Schema-first contracts let Cursor's built-in JSON/YAML language servers surface plain YAML/JSON violations as red squigglies, and give Markdown frontmatter the same canonical shape enforced by `tooling check`, removing a class of PR round-trips without depending on unproven editor behaviour.
2. **Eliminate parser duplication.** `tests/lib/spec_provenance.ts` mirrors `specify-domain`'s requirement-block parser; `scripts/checks/adapter.ts` re-runs Ajv against the same schemas `specify-domain` already ships. Sharing the Rust library kills both.
3. **Collapse the toolchain.** Remove Deno from `make check`, `make test`, and contributor prerequisites without adding it to the operator CLI's install surface.

The product principles that follow: **dev tooling for the plugin repo lives in the plugin repo**, not on the operator binary; **Rust contributor surface stays under `tooling/`** so the repo root remains author-facing content, not a second Cargo workspace; and **build steps stay invisible** — authors invoke `make check`, never `cargo build` directly.

## Detailed Design

### Architecture

Four layers, one rule engine, one binary.

```text
┌────────────────────────────────────────────────────────────────┐
│ Layer 1: Schemas                                               │
│   Runtime schemas stay in specify-cli/schemas/. Framework-only │
│   schemas stay in augentic/specify/tooling/schemas/ and are    │
│   wired into Cursor JSON/YAML LSPs for in-editor squigglies. │
└────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────────┐
│ Layer 2: rules (Rust library crate)                            │
│   Cross-file predicates that schemas cannot express:           │
│   symlink integrity, marketplace ↔ plugins consistency,        │
│   variable-definition coverage, cross-skill directive          │
│   resolution, codex namespace ownership, brief size,           │
│   declared-tool invocation equivalence, link resolution.       │
│   Reuses specify-domain only for runtime-owned shapes.         │
└────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
   ┌───────────────┐                 ┌───────────────┐
   │ Layer 3a:     │                 │ Layer 3b:     │
   │ tooling       │                 │ lsp (future)  │
   │ binary        │                 │ Cursor LSP    │
   │ check subcmd  │                 │               │
   └───────────────┘                 └───────────────┘
              │
              ▼
   ┌─────────────────────────────┐
   │ Layer 4: annotations/export │
   │ Deferred until JSON settles.│
   └─────────────────────────────┘
```

Each layer optimises for its audience: schemas catch the easy 80% in the editor at zero cost; the library carries the hard 20% once; the frontends are thin shells.

### Scope

Atomic implementation scope:

- A new Rust workspace under `augentic/specify/tooling/` (this repo).
- `rules` library crate carrying every predicate currently in `scripts/checks/`.
- `tooling` binary with a `check` subcommand that scans the **framework repo root** (`plugins/`, `adapters/`, `docs/`, …) for CI and local use — not the `tooling/` subtree alone.
- Schema-first migration: extract every framework-repo check that can be a JSON Schema into one (or strengthen an existing one), and wire `$schema` references so Cursor surfaces violations inline.
- Minimal JSON output for `tooling check` once rule ids and finding locations are stable enough to fixture.
- `docgen` library crate and matching `tooling docgen` subcommand that ports `scripts/gen-envelope-doc.ts`.
- `accept` integration-test crate that ports `tests/cross_repo.ts` and its `tests/lib/` helpers.
- `Makefile`, CI, and contributor-doc updates that remove Deno from the framework repo's normal validation path in the same PR.

Out of scope (future RFCs):

- `lsp` — designed for here but not implemented until rule count justifies it.
- WASI extensibility for third-party rule packs — the library shape allows it; this RFC does not adopt it.
- Any new invariants beyond what the Deno scripts enforce today. New checks belong to RFC-4 Option 1, RFC-28 codex resolution, or successor RFCs.
- PR annotation helpers or a composite GitHub Action wrapping `tooling check --format json`.
- Manual scenario packs under `tests/cross-repo/` and `tests/plan/` — operator-driven by design; see [docs/contributing/acceptance.md](../docs/contributing/acceptance.md).

The boundary against the operator CLI is explicit: `specify-cli` validates *consumer projects* at runtime (adapter manifest loads, slice lifecycle transitions, plan validation, merge); this workspace validates *the framework repo itself* (skill integrity, adapter brief discipline, marketplace alignment, docs hygiene, fixture acceptance). The overlap is intentional and narrow: both sides need runtime adapter-manifest parsing and runtime JSON Schemas, while framework-only authoring schemas belong in this repo. Shared parsing comes from `specify-domain` only where the runtime already owns the shape.

**Soft LOC budget.** Today's Deno surface is ~4,027 LOC across `scripts/check.ts`, `scripts/checks/*.ts`, `scripts/gen-envelope-doc.ts`, `tests/cross_repo.ts`, and `tests/lib/*.ts`. The Rust replacement should land at ≤ ~5,000 LOC across `rules` + `tooling` + `accept` (including tests), with at least 500 LOC of net deletion in `specify` (Deno) when the PR lands. A naive port will balloon (filesystem walks, YAML parsing); a library port that delegates to `specify-domain` should shrink. This is a guard rail for reviewers, not a hard gate — materially exceeding it is a signal to revisit predicate factoring before opening the PR.

#### Deno parity plan

The implementation PR deletes each Deno module only after the Rust equivalent has module-level fixtures that prove the same invariant class is still covered. Diagnostic wording may change; rule coverage and stable locations may not.


| Current Deno surface                                                                       | Rust home                                   | Schema-backed where possible?             | Parity fixture expectation                                                 |
| ------------------------------------------------------------------------------------------ | ------------------------------------------- | ----------------------------------------- | -------------------------------------------------------------------------- |
| `scripts/checks/adapter.ts`                                                                | `rules::adapter`                            | Yes, via `specify-domain` runtime schemas | Valid/invalid source and target manifests                                  |
| `scripts/checks/agent_teams.ts`                                                            | `rules::agent_teams`                        | No                                        | Missing, broken, wrong-target, and content-drift overlays                  |
| `scripts/checks/brief_size.ts`                                                             | `rules::brief`                              | No                                        | Parent cap, phase soft cap, phase hard cap, and no-frontmatter cases       |
| `scripts/checks/codex.ts`                                                                  | `rules::codex`                              | Yes, for frontmatter shape                | Rule shape, duplicate ids, body heading, and namespace ownership           |
| `scripts/checks/docs_quality.ts`                                                           | `rules::docs_quality`                       | No                                        | RFC citation hygiene and generated-asset checks                            |
| `scripts/checks/links.ts`                                                                  | `rules::links`                              | No                                        | Relative markdown links, anchors, and symlink-aware references             |
| `scripts/checks/plugins.ts`                                                                | `rules::plugins`                            | Yes, for marketplace shape                | Plugin symlinks and marketplace consistency                                |
| `scripts/checks/prose.ts`                                                                  | `rules::prose`                              | Partly                                    | Retired vocabulary, skill caps, and positional slash-skill invocations     |
| `scripts/checks/scenarios.ts`                                                              | `rules::scenarios`                          | Yes, for scenario frontmatter             | Scenario discovery, duplicate ids, expected artifacts, and trace freshness |
| `scripts/checks/skill_body.ts`                                                             | `rules::skill_body`                         | No                                        | Body-section discipline, directive resolution, and variable coverage       |
| `scripts/checks/skill_frontmatter.ts`                                                      | `rules::skill_frontmatter`                  | Yes                                       | Description grammar, argument hints, field caps, and schema failures       |
| `scripts/checks/tools.ts`                                                                  | `rules::tools`                              | No                                        | Declared-tool equivalence for active skill bodies                          |
| `scripts/check.ts`, `scripts/gen-envelope-doc.ts`, `tests/cross_repo.ts`, `tests/lib/*.ts` | `tooling check`, `tooling docgen`, `accept` | Mixed                                     | End-to-end command fixtures and golden acceptance coverage                 |


### Workspace layout

Two roots, one workspace:

- **Framework root** — `augentic/specify/`; what skill and adapter authors browse. Contains `plugins/`, `adapters/`, `docs/`, `tests/fixtures/`, and the existing `scripts/` / `tests/` trees until Deno retires. Every scanner predicate walks this tree.
- **Tooling root** — `augentic/specify/tooling/`; what Rust contributors build. Contains the Cargo workspace, framework-only schemas, the `tooling` binary, and library crates. Nothing author-facing lives here except contributor docs that point back at the framework root.

```text
augentic/specify/                           # framework root (scan target)
├── plugins/
├── adapters/
├── docs/
├── tests/
│   ├── fixtures/                           # acceptance inputs (unchanged location)
│   └── cross-repo/                         # manual scenario packs (out of scope)
├── scripts/
│   └── check.ts                            # Deno — deleted by this RFC
├── tooling/                                # Rust dev-tooling workspace
│   ├── Cargo.toml                          # workspace manifest
│   ├── rust-toolchain.toml                 # pinned to match specify-cli
│   ├── schemas/                            # framework-only authoring schemas
│   ├── crates/
│   │   ├── rules/                          # library: predicates + shared walkers
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   │       ├── lib.rs
│   │   │       ├── adapter.rs              # adapter.yaml ↔ source/target schemas
│   │   │       ├── agent_teams.rs          # per-target review-team overlays
│   │   │       ├── brief.rs                # brief size + no-frontmatter discipline
│   │   │       ├── codex.rs                # codex rule shape + RFC-28 namespace ownership
│   │   │       ├── docs_quality.rs         # RFC citation hygiene, diagram assets
│   │   │       ├── links.rs                # markdown links + symlink-aware references
│   │   │       ├── plugins.rs              # symlinks + marketplace.json consistency
│   │   │       ├── prose.rs                # invocation positionals, operational vocab, caps
│   │   │       ├── scenarios.rs            # scenario frontmatter + recorded-trace freshness
│   │   │       ├── skill_body.rs           # skill body discipline (12 predicates)
│   │   │       ├── skill_frontmatter.rs    # skill frontmatter discipline (7 predicates)
│   │   │       └── tools.rs                # declared-tool equivalence
│   │   ├── docgen/                         # envelope doc generation library
│   │   │   ├── Cargo.toml
│   │   │   └── src/lib.rs
│   │   └── accept/                         # integration tests over ../tests/fixtures/
│   │       ├── Cargo.toml
│   │       └── tests/                      # one file per fixture surface (sources / targets / skills)
│   └── tools/
│       └── tooling/                        # single binary: check + docgen subcommands
│           ├── Cargo.toml
│           └── src/
│               ├── main.rs                 # clap root (~150 LOC)
│               ├── check.rs                # dispatches to rules
│               └── docgen.rs               # dispatches to docgen crate
└── .github/
    └── workflows/ci.yaml                   # build once, run binary; no Deno
```

Each ported module gets fixtures for its structured findings. The Rust port may reword diagnostics freely — there is no requirement to match `check.ts` wording. Fixtures, not message strings, are what verify a Deno module is safe to delete.

#### Cross-repo dependency

`rules` depends on `specify-domain` and `specify-error` for predicates that reuse runtime-owned parsers or schemas. Keep the default dependency story simple: both crates are pulled as **git deps pinned to a released tag**. `tooling/Cargo.toml` may also carry a commented local-development `[patch]` block for contributors working against a sibling `specify-cli` checkout.

```toml
# tooling/Cargo.toml
[workspace.dependencies]
specify-domain = { git = "https://github.com/augentic/specify-cli.git", tag = "v<X.Y.Z>" }
specify-error  = { git = "https://github.com/augentic/specify-cli.git", tag = "v<X.Y.Z>" }

# Optional local-development override. Leave commented in committed code.
# Uncomment only when testing framework tooling against a sibling specify-cli checkout.
#
# [patch."https://github.com/augentic/specify-cli.git"]
# specify-domain = { path = "../../specify-cli/crates/domain" }
# specify-error  = { path = "../../specify-cli/crates/error" }
```

Normal local runs and CI use the same tag, so a contributor can run `make check` with only this repository checked out. When the framework tooling needs a newer CLI parser or schema, the `specify-cli` change lands and tags first; a small `scripts/bump-specify-cli` helper updates the tag in `tooling/Cargo.toml` as a normal framework PR. Cross-repo development before that tag is a contributor-local escape hatch: uncomment the `[patch."https://github.com/augentic/specify-cli.git"]` block while testing, then re-comment it before opening the framework PR.

Rejected alternatives: an active committed path patch (Cargo does not conditionally activate it; missing sibling checkouts would break normal local runs), publishing `specify-domain` to crates.io (premature — adds release ceremony, exposes internal API, and the workspace genuinely is dev-time tooling that consumes unstable surfaces), or depending on an untagged branch (too easy for CI to drift under an unchanged lockfile review).

### Crate naming

The library crate is `rules` (`tooling/crates/rules/`). User-facing commands run through a single `tooling` binary (`tooling/tools/tooling/`; artifact path `tooling/target/debug/tooling`). Subcommands replace the separate binaries from earlier drafts:


| Subcommand       | Replaces                              | Library  |
| ---------------- | ------------------------------------- | -------- |
| `tooling check`  | `scripts/check.ts`, `framework-check` | `rules`  |
| `tooling docgen` | `scripts/gen-envelope-doc.ts`         | `docgen` |


Acceptance stays `cargo test -p accept` — not a CLI subcommand. The `framework-` prefix from earlier drafts is dropped; the `tooling/` workspace scopes these names away from the operator product. This is **not** `specify check` or `specify review` — those surfaces validate consumer projects on the operator binary (rejected in §Alternatives and reserved separately in RFC-28 / roadmap RM-10). Contributors disambiguate the `check` subcommand from `make check`, `scripts/check.ts` (during Deno overlap), and `cargo check` by context: day-to-day invocation is always `make check`, never a global install.

**Broader renaming.** This RFC is canonical for the shortened names and the single-binary shape. Landing the workspace must update every cross-reference that still says `framework-rules`, `framework-check`, `framework-lsp`, or separate `check`/`docgen` binaries: [RFC-1](done/rfc-1-cli.md), [RFC-4](future/rfc-4-dsl.md), [RFC-10](done/rfc-10-skills.md), [RFC-13](done/rfc-13-extensibility.md), [RFC-28](next/rfc-28-codex-rules.md), [RFC-30](next/rfc-30-init.md), [roadmap RM-16 / RM-07](roadmap.md), and [docs/contributing/checks.md](../docs/contributing/checks.md). Module paths in prose become `rules::codex` rather than `framework-rules::codex`. No rename is required in `specify-cli` — the operator binary is unchanged.

### Schema-first layer (do this first)

Most checks in `scripts/checks/` enforce shapes that JSON Schema can express. The earliest, highest-leverage work is to make sure every such shape **is** a schema, and that Cursor sees it where the active language service can bind the schema directly.

Plain YAML and JSON files get inline diagnostics through Cursor's built-in language servers. Markdown-frontmatter files (`SKILL.md`, codex rules, scenario docs) still use JSON Schema as the canonical shape, but a local Cursor proof spike showed the YAML language service validates a matching `.yaml` control file while not reporting diagnostics for the same invalid schema fields inside Markdown frontmatter. Until a frontmatter-aware editor integration or the deferred `lsp` exists, `tooling check` owns Markdown-frontmatter enforcement by extracting the leading `---` block and validating it with the same schema.

Concrete moves:

- **Authoritative location.** Runtime schemas consumed by `specify-cli` stay in `specify-cli/schemas/` and are reused through `specify-domain`. Framework-only schemas (skill frontmatter, codex rule authoring, scenario metadata, marketplace manifests) stay in `augentic/specify/tooling/schemas/`; `.cursor/schemas/` contains editor-facing symlinks or aliases only when Cursor needs them. A schema moves to `specify-cli` only when the runtime binary genuinely consumes it.
- **Editor wiring.** Workspace settings or per-file `# yaml-language-server: $schema=` directives point plain YAML/JSON files (`adapter.yaml`, marketplace manifests, scenario YAML when present, target-owned YAML artifacts, and `tools.yaml` during migration) at their schemas. The YAML/JSON LSPs Cursor already ships then surface those violations live, with no extra tooling installed.
- **Markdown-frontmatter enforcement.** `SKILL.md`, codex rules, and scenario Markdown files still declare and share schemas, but `tooling check` remains the enforcement surface for their frontmatter unless a future editor integration proves reliable inline diagnostics for Markdown frontmatter.
- **Schema strengthening.** Rules currently enforced imperatively in `skill_frontmatter.ts` (description grammar, argument-hint shape, 200/45/512 caps on counted fields) are expressed as `pattern`, `maxLength`, and `enum` constraints where they fit. The minority that genuinely cannot be schema'd (variable consistency, cross-skill directive resolution, body-section discipline) stays in `rules`.
- **Documentation.** `docs/contributing/checks.md` gets a new section explaining the split: plain YAML/JSON shape violations appear as editor diagnostics, while Markdown-frontmatter and cross-file rules are caught by `tooling check`.

This step delivers contributor value early inside the atomic implementation branch and gives the Rust rule engine canonical schemas to reuse for Markdown frontmatter.

#### Schema graduation

The split between `specify-cli/schemas/` and `tooling/schemas/` is not load-bearing — `adapter.schema.json` is consumed by both sides — so the canonical placement rule is consumer-driven, not directory-driven:

> A schema lives in `specify-cli/schemas/` if and only if the operator `specify` binary loads it at runtime to validate consumer-project artifacts. A schema lives in `tooling/schemas/` if it only describes framework authoring shapes (`SKILL.md` frontmatter, codex authoring, `marketplace.json`, scenario YAML). When a framework-only schema becomes a runtime concern (e.g. a future runtime feature consumes `marketplace.json`), it moves to `specify-cli/schemas/` in the same PR that introduces the runtime use; `tooling/schemas/` then re-exports it through `specify-domain`. Graduation in the other direction is rare but follows the inverse path.

Current placement at this RFC's landing:


| Schema / category                                                                                       | Lives in               | Loaded by                                                                                                           |
| ------------------------------------------------------------------------------------------------------- | ---------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Adapter manifests (`adapter.schema.json`, `source.schema.json`, `target.schema.json`)                   | `specify-cli/schemas/` | `specify-domain` at runtime (consumer-project adapter resolve); `rules` reuses them for framework-author lints      |
| Workflow artifacts (`evidence.schema.json`, `plan/plan.schema.json`, `discovery/candidate.schema.json`) | `specify-cli/schemas/` | Operator binary validates these on consumer-project workflow state                                                  |
| Skill frontmatter (`skill.schema.json`)                                                                 | `tooling/schemas/`     | Cursor's editor wiring (where it works on YAML) and `rules` (Markdown-frontmatter enforcement); no runtime consumer |
| Codex authoring                                                                                         | `tooling/schemas/`     | `rules::codex`; `.cursor/schemas/` symlinks for editor diagnostics                                                  |
| Marketplace manifest (`.cursor-plugin/marketplace.json`)                                                | `tooling/schemas/`     | `rules::plugins`                                                                                                    |
| Scenario YAML                                                                                           | `tooling/schemas/`     | `rules::scenarios`                                                                                                  |


### `rules` library

A single crate exposing each predicate as a `Check` returning structured findings:

```rust
pub struct Finding {
    pub rule_id: &'static str,         // stable kebab-case id
    pub severity: Severity,            // error | warning
    pub message: String,               // human-readable diagnostic
    pub location: Option<Location>,    // file + 1-based line + optional column
}

pub trait Check {
    fn id(&self) -> &'static str;
    fn run(&self, ctx: &Context) -> Vec<Finding>;
}
```

`Context` carries the resolved **framework root** (never the `tooling/` workspace directory alone), a `specify-domain` adapter resolver where needed, and lazily-loaded schemas from `tooling/schemas/`. Predicates are independent and parallelisable (`rayon` or `tokio::task::spawn_blocking`), mirroring the `Promise.all` batches in `scripts/check.ts`. Every invocation runs a full repo scan; there is no `--changed` mode, and predicates are written under that assumption.

Rule ids align with RFC-28's reserved namespaces where applicable (the codex namespace-ownership rule lives here as `codex.namespace-ownership-violation` and feeds the future shared finding shape).

**RFC-28 interlock.** Rule ids minted by `rules::codex` follow RFC-28's namespace ownership and id-stability rules from the first ported predicate, even though the wider `tooling check --format json` envelope is fixtured later in the same PR. Other modules' ids may evolve until the JSON envelope is fixtured and pinned; codex ids are fixed from day one. This makes the cross-RFC interlock explicit without delaying the JSON-envelope work.

### `tooling` binary

One clap root with subcommands. Day-to-day callers at the framework root use `make` targets instead of invoking Cargo or the binary path directly — see §*Makefile entry points*.

#### `tooling check`

A thin dispatcher over `rules`. `--repo` names the framework root to scan; it defaults to the parent of the workspace directory when the binary is invoked from `tooling/`.

```bash
# day-to-day (framework root)
make check

# tooling contributors (direct Cargo, optional)
cargo build --manifest-path tooling/Cargo.toml -p tooling
tooling/target/debug/tooling check --repo ..
```

Exit codes follow the standard table inherited from `specify-cli`:

- `0` — success.
- `2` — validation findings or argument errors.
- `1` — infrastructure errors (I/O, schema load failures).

Every invocation runs a full repo scan. The Rust port is fast enough that scoping by changed paths is unnecessary at this repo's size, and a single code path keeps local and CI behaviour identical — no per-rule classification of which predicates are safe to restrict, no risk of silently missing global invariants (marketplace consistency, codex namespace ownership, duplicate ids, symlink integrity). If full-scan latency ever becomes a contributor pain point, a future RFC can add a scoped mode; until then, YAGNI.

The JSON envelope shape matches `specify-cli`'s output shape contract so a future GitHub Action or scorer can consume both. JSON output lands after the first rule ids and locations are stable enough to pin with fixtures; PR annotations wait until that envelope has settled.

#### `tooling docgen`

Subcommand over the `docgen` library crate. Ports `scripts/gen-envelope-doc.ts`.

```bash
tooling/target/debug/tooling docgen envelopes         # regenerate docs/reference/cli-output-shapes.md
tooling/target/release/tooling docgen envelopes --check # CI mode: diff and exit 2 on drift
```

Same generated-block markers (`<!-- generated:begin -->` / `<!-- generated:end -->`), same explicit fixture-to-section mapping table, same sibling-checkout discovery semantics — but the env var is renamed to `SPECIFY_CLI_ROOT`. The old `SPECIFY_CLI_DIR` is removed in the same PR; there is no fallback or deprecation period.

### `accept` crate

This is a sibling workspace tool, not part of the linter core and not a CLI subcommand. It ports `tests/cross_repo.ts` and its `tests/lib/` helpers into a Rust integration crate that:

- Uses `specify-domain` directly for provenance parsing (kills `tests/lib/spec_provenance.ts`).
- Uses the same JSON Schema validators as `rules` (kills `tests/lib/validators.ts`).
- Keeps the optional `SPECIFY_BIN` subprocess tests for `specify source resolve` and `specify target resolve`; skips cleanly when the binary is absent (matches today's harness).
- Adopts `specify-cli`'s `REGENERATE_GOLDENS=1` discipline for any byte-stable goldens it asserts.

Test-binary names mirror the existing Deno suites (`sources`, `targets`, `skills_refine`, `skills_loop`) so `cargo test --manifest-path tooling/Cargo.toml --test <name>` is easy. Fixture paths resolve from the framework root (`tests/fixtures/`, `tests/cross-repo/` inputs where applicable), not from inside `tooling/`.

Manual scenario packs under `tests/cross-repo/` and `tests/plan/` continue to run via the `gh` recipe documented in [docs/contributing/acceptance.md](../docs/contributing/acceptance.md). Nothing under `tooling/` invokes them automatically, and `cargo test -p accept` does not exercise them. The implementation PR that deletes `tests/cross_repo.ts` updates that doc in the same change so the manual harness retains a documented entry point.

### PR annotations

PR annotations are deferred until JSON findings have stable fixtures:

- `.github/actions/tooling/` — a follow-on composite action that runs a release-built `tooling check --repo . --format json`, parses the envelope, and posts PR annotations via `actions/github-script`. CI builds the binary once per job, then invokes it directly (see §*Makefile entry points*).

A pre-commit hook is **explicitly out of scope** for this RFC. CI is the authoritative gate; `make check` is the local equivalent for contributors who want to mirror it before pushing. If commit-time feedback ever becomes a real ask, a future RFC can add it — but it requires no architectural commitment now (no `--changed` mode, no `ChangedStrategy` trait, no per-rule classification). Adopting it later is additive, not a refactor.

### Makefile entry points

Framework tooling must **disappear into the background** for day-to-day skill and adapter work. No contributor should need to remember Cargo flags or know where `target/` lives unless they are editing the tooling workspace itself. The repo already has a `Makefile`; use it as the single local abstraction.

#### Contract


| Audience                          | What they run                                                                      | Rust required locally?                     |
| --------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------ |
| Skill / adapter authors (default) | Cursor schemas while editing where available; `make check` before a PR; CI on push | Optional — schemas and CI cover most needs |
| Tooling contributors              | `cd tooling && cargo test`, `cargo build -p tooling`, …                            | Yes                                        |


Authors never run `cargo build` as a separate step before `make check`. Build logic lives in one place; entry points call through it.

#### Makefile

`make check` builds the repo-local binary and invokes it directly. The Makefile keeps only the commands contributors actually run:

```makefile
TOOLING := tooling/target/debug/tooling
TOOLING_MANIFEST := tooling/Cargo.toml

.PHONY: check test ci

check:
	cargo build --manifest-path $(TOOLING_MANIFEST) -p tooling
	$(TOOLING) check --repo .

test:
	cargo test --manifest-path $(TOOLING_MANIFEST) --workspace

ci: check test
```

Inside the implementation branch, `make check` may call `scripts/check.ts` for unported predicates and `tooling/target/debug/tooling check` for ported ones — each rule lives in exactly one of the two surfaces at any commit. Before the PR opens for review, every predicate has moved to Rust and `scripts/check.ts` is deleted.

#### CI

CI builds once per job, then runs the binary — it does not use `cargo run`:

```yaml
- uses: dtolnay/rust-toolchain@stable
  with:
    toolchain-file: tooling/rust-toolchain.toml
- uses: Swatinem/rust-cache@v2
- run: cargo build --manifest-path tooling/Cargo.toml -p tooling --release
- run: tooling/target/release/tooling check --repo .
  env:
    SPECIFY_CLI_ROOT: specify-cli
- run: tooling/target/release/tooling docgen envelopes --check
  env:
    SPECIFY_CLI_ROOT: specify-cli
- run: cargo test --manifest-path tooling/Cargo.toml --workspace
  env:
    SPECIFY_CLI_ROOT: specify-cli
```

The `Swatinem/rust-cache@v2` step is included from day one rather than added after the first slow PR — a cold Cargo build of `tooling` plus transitive `specify-domain` deps can run 1–3 minutes per job, and adoption pain is highest in the weeks after switchover. Sccache via shared remote storage was considered and rejected: premature for one workspace this size, adds infra dependency. The same direct build + binary invocation pattern applies to `tooling docgen envelopes --check` and `cargo test` in the atomic PR's final CI shape. `tooling/target/` is never committed.

#### Contributor documentation

`docs/contributing/index.md` and `docs/contributing/checks.md` describe the split explicitly:

- **Editor-first where available** — plain YAML/JSON violations surface as schema squigglies; Markdown frontmatter and cross-file rules surface through `tooling check`.
- **Local gate** — `make check` builds and runs `tooling check`; first run after clone compiles once, subsequent runs reuse Cargo's cache until tooling sources change.
- **CI gate** — authoritative full scan on every PR; sufficient on its own for contributors who skip local Rust.

Rust appears under prerequisites only for contributors who run `make check` locally, not as a blanket requirement for every markdown edit.

#### Explicitly out of scope

- **Committed prebuilt binaries** — platform-specific churn; Cargo incremental build is enough.
- `cargo install tooling` — adds global install/update ceremony; repo-local binaries via `make` are simpler.
- **Auto-build on every file save** — cross-file rules are too heavy; editor schemas cover save-time feedback. A future `lsp` would reuse `rules` without changing this contract.
- **Pre-commit hook and `--changed` mode** — `make check` and CI cover the local + authoritative gates; per-rule classification of which predicates are safe to scope is YAGNI at this repo's size. Both are additive if they ever become real asks.

### `lsp` (deferred)

Listed in the architecture diagram so contributors see the full intended shape. Not implemented in this RFC because:

- Schema-first handles the easy 80% via Cursor's built-in language servers without any custom LSP code.
- The cross-file rules that would benefit from an LSP (symlink integrity, marketplace consistency, cross-skill directive resolution, variable coverage) are a small enough surface that the CLI-on-save loop is acceptable until contributor pain justifies the engineering investment.
- A future `lsp` reuses `rules` unchanged, so the architectural commitment is already paid.

### Implementation outline

The RFC lands as a **single PR** that introduces the `tooling/` workspace, ports every Deno surface, renames every cross-RFC reference, and removes Deno from CI. The order below is the logical sequence of the work *inside* that PR — it is not a multi-PR rollout.

1. **Cross-RFC rename sweep.** Update every cross-reference that still says `framework-rules`, `framework-check`, `framework-lsp`, or separate `check`/`docgen` binaries: `rfcs/roadmap.md` (RM-07, RM-16), `rfcs/next/rfc-28-codex-rules.md`, `rfcs/next/rfc-30-init.md`, `rfcs/done/rfc-1-cli.md`, `rfcs/done/rfc-10-skills.md`, `rfcs/done/rfc-13-extensibility.md`, `rfcs/future/rfc-4-dsl.md`, and `docs/contributing/checks.md`. Module paths in prose become `rules::codex` rather than `framework-rules::codex`.
2. **Schema-first pass.** Keep runtime schemas in `specify-cli/schemas/`, move or strengthen framework-only schemas under `augentic/specify/tooling/schemas/`, wire Cursor `$schema` references for plain YAML/JSON files, and document that Markdown frontmatter is schema-backed but enforced by `tooling check`.
3. **Rule-engine scaffold.** Land `tooling/Cargo.toml`, `tooling/rust-toolchain.toml`, `rules`, the `tooling` binary with a `check` subcommand, and the `make check` target that builds the binary and runs a full scan against `--repo .`.
4. **Port every check.** Move every predicate from `scripts/checks/` into `rules`, deleting the matching Deno file as each lands. Order — start with the schema/parser-duplication wins (adapter manifest validation, brief discipline, skill frontmatter/body shape), then proceed through `agent_teams`, `prose`, `links`, `plugins`, `docs_quality`, `codex`, `scenarios`, and `tools` in dependency order. Add structured-finding fixtures alongside each module.
5. **Stabilize `tooling check` output.** Add `--format json` and rule-id filters once rule ids and finding locations are stable enough to fixture.
6. **Port `docgen`.** Add the `docgen` library crate and `tooling docgen` subcommand; delete `scripts/gen-envelope-doc.ts`.
7. **Port `accept`.** Replace parser duplication (`spec_provenance.ts`, `validators.ts`) by using `specify-domain` and shared validators directly. The new crate's fixtures stand alone; delete `tests/cross_repo.ts` and `tests/lib/` in the same step.
8. **CI cleanup.** Delete the remaining Deno trees, drop `denoland/setup-deno` from `.github/workflows/ci.yaml`, update `docs/contributing/index.md` with the audience split and optional-Rust prerequisites, and switch `Makefile` to the Cargo-backed targets above.

`lsp` and WASI extensibility for third-party rule packs are deferred to future RFCs (see §*`lsp` (deferred)* and §*Scope*); they are explicitly *not* in this PR's scope.

### Makefile summary

The target end state is documented in §*Makefile entry points*. In short: `make check` builds `tooling` and runs `tooling check --repo .`; `make test` runs `cargo test --manifest-path tooling/Cargo.toml --workspace`.

While the implementation PR is being drafted, branch commits may invoke whichever surface currently owns each rule — `scripts/check.ts` for unported predicates, `tooling check` for ported ones. Each rule lives in exactly one of the two surfaces at any commit. By the time the PR opens for review, every rule has moved to Rust and `scripts/check.ts` is deleted.

### Coordination with other RFCs

- **RFC-4 (typed skill expression).** Option 1 (framework-tooling skill validation) is satisfied by the schema-first pass plus the `skill_frontmatter` / `skill_body` modules in `rules`. The "CLI" in that RFC is reinterpreted as `tooling check`, not `specify check`. Options 2 and 3 are unchanged.
- **RFC-28 (codex resolution).** RFC-28 cites RFC-5 for namespace-ownership enforcement. That contract is preserved verbatim — the rule moves to `rules::codex` and continues to enforce that first-party files do not use `ORG-`*. Where the rule lives (this repo, not specify-cli) is invisible to RFC-28's resolver and finding shape.
- **Roadmap RM-16.** The roadmap entry tracks this RFC's framework dev-tooling workspace: schema-first authoring feedback, `tooling check`, `accept`, `tooling docgen`, and Deno retirement. The unblocks line (RFC-4 Option 1, declared-WASI-tool helper migration) is unchanged.

## Alternatives Considered

**The original RFC-5: port into `specify-cli` as `specify check`.** Rejected because it puts framework dev tooling on the operator product. Operators running Specify on a consumer project never need to validate `plugins/` or `.cursor-plugin/marketplace.json`; bundling that surface bloats the install for everyone to serve the few. The parser-reuse argument that motivated the original choice is satisfied just as well by depending on `specify-domain` as a library from a sibling workspace, which is the standard Rust pattern.

**One binary per Deno script (`check`, `docgen`).** Considered: mirror `scripts/check.ts` and `scripts/gen-envelope-doc.ts` as separate Cargo packages under `tooling/tools/`. Rejected because the binaries are thin clap shells over library crates, share one workspace dependency story, and differ only by subcommand. A single `tooling` binary with `check` and `docgen` subcommands matches the `specify` operator pattern, keeps the Makefile simple, and keeps acceptance on `cargo test` where it belongs.

**Keep `scripts/check.ts` indefinitely.** Tempting because the script works and is not blocking. Rejected because (a) `tests/lib/spec_provenance.ts` and `tests/lib/validators.ts` actively duplicate `specify-domain`, and that duplication grows with every new schema; (b) schemas without an editor or `tooling check` contract are invisible to contributors; (c) keeping Deno in CI forever to validate Rust-defined schemas is a coordination tax with no offsetting benefit.

**Merge the repos.** Considered: one workspace with `cli/` and `plugins/` top-level directories would kill the cross-repo coordination entirely. Rejected because it conflates two audiences (Rust contributors vs skill/adapter authors), couples operator-CLI release cadence to plugin-content cadence, and forces a single review style on both halves. The dev-tooling problem does not require it.

**Rust workspace at the framework repo root.** Considered: colocate `Cargo.toml`, `crates/`, and `tools/` beside `plugins/` and `adapters/` (the layout in the original RFC-5 draft). Rejected because it puts Rust contributor surface in the same directory tree skill and adapter authors browse daily — `Cargo.toml` beside `plugins/` reads like consumer-project scaffolding, and a top-level `schemas/` collides mentally with consumer `contracts/schemas/` even when the paths differ. Nesting the workspace under `tooling/` preserves the audience split the RFC already applies against `specify-cli`, without merging the two repos.

**Self-hosted Specify (the framework repo as a Specify project).** Considered as the long-term aspiration: every plugin and adapter as a slice, framework consistency from `specify slice validate`. Rejected as the *only* solution because mechanical checks still need a deterministic engine underneath, and the framework lifecycle (RFC editing, README polish, contributor docs) does not naturally fit slices. Compatible with this RFC: a self-hosted layer can sit on top of `rules` later.

**WASI rules as the day-one shape.** Considered: ship `tooling check` as a WASI module declared via an adapter manifest and invoked through `specify tool run`. Reuses the Vectis pattern and opens third-party rule packs immediately. Rejected as overkill when there is exactly one rule pack and one consumer (CI); the library shape adopted here makes future WASI exposure a small refactor, not a re-architecture.

**Rewrite from scratch (no rule preservation).** Rejected because the current invariants — what each predicate actually checks — encode real lessons about repo drift, and dropping them on the floor would silently regress coverage. Diagnostic *wording* is not preserved (the Rust port may reword freely); the *rules themselves* are. Fixtures, not message strings, are how each port proves it covers the same ground as the Deno predicate it replaces.

**Committed prebuilt binaries or global `cargo install`.** Considered so skill authors could skip a local Rust toolchain entirely. Rejected because checked-in platform binaries add release churn and security review overhead, and a global install adds update ceremony outside the repo. The adopted split — editor schemas + CI for everyone, optional local `make check` — keeps Rust optional without shipping artifacts.

## References

- [`scripts/check.ts`](../scripts/check.ts) + [`scripts/checks/`](../scripts/checks/) — the framework linter being replaced.
- [`scripts/gen-envelope-doc.ts`](../scripts/gen-envelope-doc.ts) — the doc generator being replaced.
- [`tests/cross_repo.ts`](../tests/cross_repo.ts) + [`tests/lib/`](../tests/lib/) — the acceptance harness being replaced.
- [`docs/standards/skill-authoring.md`](../docs/standards/skill-authoring.md) — invariants the skill-discipline modules enforce.
- [`docs/explanation/adapter-anatomy.md`](../docs/explanation/adapter-anatomy.md) — the adapter model the `adapter` and `brief` modules validate.
- [`docs/contributing/acceptance.md`](../docs/contributing/acceptance.md) — the acceptance surface split between deterministic harness (this RFC) and manual scenario packs (out of scope).
- [`Specify CLI AGENTS.md`](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) — crate graph this workspace consumes via `specify-domain`.
- [Specify CLI handler-shape contract](https://github.com/augentic/specify-cli/blob/main/docs/standards/handler-shape.md) — JSON envelope shape `tooling check --format json` mirrors.
- [RFC-4: Type-Safe Skill Expression](future/rfc-4-dsl.md) — Option 1 is satisfied by the schema-first pass plus the skill-discipline modules.
- [RFC-28: Codex Resolution and Structured Review Findings](next/rfc-28-codex-rules.md) — namespace-ownership contract preserved by `rules::codex`.

