# RFC-5: Framework Developer Tooling

> Status: Draft · Tracked by [roadmap RM-16](roadmap.md#rm-16-rfc-5-specify-check-framework-linter-port) (entry to be renamed when this RFC lands) · Enables: [RFC-4](rfc-4-dsl.md) · Preserves contract with: [RFC-28](next/rfc-28-codex-rules.md)

## Abstract

Replace this repo's Deno tooling (`scripts/check.ts`, `scripts/gen-envelope-doc.ts`, `tests/cross_repo.ts`) with a small Rust workspace **inside `augentic/specify`** that follows a layered, schema-first architecture: JSON Schemas are the canonical contract for every artifact whose shape can be expressed declaratively; cross-file rules that schemas cannot express live in a single `framework-rules` library crate; that library backs several thin frontends — a CLI binary for CI, an optional LSP for in-editor feedback, a pre-commit hook, and a GitHub Action. The operator `specify` binary in `augentic/specify-cli` is deliberately **not** extended; framework tooling stays with the framework it validates.

## Motivation

The original RFC-5 framed this work as a one-for-one port of `scripts/check.ts` into `specify-cli/crates/check/`, exposed as `specify check`. That framing collapses several different problems into one binary on the wrong product:

- **Authoring feedback** (catch typos in `SKILL.md` / `adapter.yaml` as you type) belongs in the editor, not in CI.
- **Pre-commit safety** belongs in a fast local hook scoped to changed files.
- **CI gating** is the authoritative final check — but should run the same predicates as the local layers.
- **Cross-repo coherence** (specify ↔ specify-cli schemas, envelopes, error codes) is a library concern; sharing it through a CLI subcommand is awkward.
- **Doc generation** (`gen-envelope-doc.ts`) and **fixture acceptance** (`tests/cross_repo.ts`) are not "linting" at all but share the same Deno toolchain we are trying to retire.

Putting all of this on the operator `specify` binary conflates two audiences. Operators running Specify on a consumer project never need to validate `plugins/`, `adapters/{sources,targets}/`, or `.cursor-plugin/marketplace.json`. The runtime CLI must stay focused on its job: deterministic workflow primitives for consumer projects (init, plan, slice lifecycle, adapter resolution, merge, workspace sync, WASI tool dispatch).

The Deno scripts work today. They are ~5,500 lines across three surfaces and run reliably in CI. This RFC is therefore not motivated by breakage but by three durable goals:

1. **Shift authoring feedback left.** Schema-first contracts let Cursor's built-in JSON/YAML language servers surface most violations as red squigglies, removing a class of PR round-trips.
2. **Eliminate parser duplication.** `tests/lib/spec_provenance.ts` mirrors `specify-domain`'s requirement-block parser; `scripts/checks/adapter.ts` re-runs Ajv against the same schemas `specify-domain` already ships. Sharing the Rust library kills both.
3. **Collapse the toolchain.** Remove Deno from `make check`, `make test`, and contributor prerequisites without adding it to the operator CLI's install surface.

The product principle that follows: **dev tooling for the plugin repo lives in the plugin repo**, not on the operator binary.

## Detailed Design

### Architecture

Four layers, one rule engine.

```text
┌────────────────────────────────────────────────────────────────┐
│ Layer 1: Schemas (specify-cli/schemas/, .cursor/schemas/)      │
│   Authored once. Consumed by Cursor JSON/YAML LSPs for         │
│   in-editor squigglies. Also embedded in framework-rules and   │
│   in specify-domain for runtime validation.                    │
└────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────────┐
│ Layer 2: framework-rules (Rust library crate)                  │
│   Cross-file predicates that schemas cannot express:           │
│   symlink integrity, marketplace ↔ plugins consistency,        │
│   variable-definition coverage, cross-skill directive          │
│   resolution, codex namespace ownership, brief size,           │
│   declared-tool invocation equivalence, link resolution.       │
│   Depends on specify-domain for shared parsers and schemas.    │
└────────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
   ┌───────────────┐ ┌───────────────┐ ┌───────────────┐
   │ Layer 3a:     │ │ Layer 3b:     │ │ Layer 3c:     │
   │ framework-    │ │ framework-lsp │ │ pre-commit    │
   │ check binary  │ │ (future)      │ │ hook          │
   │ CI + local    │ │ Cursor LSP    │ │ Changed files │
   └───────────────┘ └───────────────┘ └───────────────┘
              │
              ▼
   ┌───────────────────────┐
   │ Layer 4: GitHub Action │
   │ Wraps framework-check  │
   │ for PR annotations.    │
   └───────────────────────┘
```

Each layer optimises for its audience: schemas catch the easy 80% in the editor at zero cost; the library carries the hard 20% once; the frontends are thin shells.

### Scope

In scope:

- A new Rust workspace under `augentic/specify` (this repo).
- `framework-rules` library crate carrying every predicate currently in `scripts/checks/`.
- `framework-check` binary that runs the library across the repo for CI and local use.
- `accept` crate that ports `tests/cross_repo.ts` and its `tests/lib/` helpers.
- `docgen` binary that ports `scripts/gen-envelope-doc.ts`.
- Schema-first migration: extract every check that can be a JSON Schema into one (or strengthen an existing one), and wire `$schema` references so Cursor surfaces violations inline.

Out of scope (future RFCs):

- `framework-lsp` — designed for here but not implemented until rule count justifies it.
- WASI extensibility for third-party rule packs — the library shape allows it; this RFC does not adopt it.
- Any new invariants beyond what the Deno scripts enforce today. New checks belong to RFC-4 Option 1, RFC-28 codex resolution, or successor RFCs.
- Manual scenario packs under `tests/cross-repo/` and `tests/plan/` — operator-driven by design; see [docs/contributing/acceptance.md](../docs/contributing/acceptance.md).

The boundary against the operator CLI is explicit: `specify-cli` validates *consumer projects* at runtime (adapter manifest loads, slice lifecycle transitions, plan validation, merge); this workspace validates *the framework repo itself* (skill integrity, adapter brief discipline, marketplace alignment, docs hygiene, fixture acceptance). The overlap is intentional and narrow — both sides need the same adapter-manifest parser and the same JSON Schemas — and is handled by depending on `specify-domain` as a library.

### Workspace layout

```text
augentic/specify/
├── Cargo.toml                              # new workspace manifest
├── rust-toolchain.toml                     # pinned to match specify-cli
├── crates/
│   ├── framework-rules/                    # library: predicates + shared walkers
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── adapter.rs                  # adapter.yaml ↔ source/target schemas
│   │       ├── brief.rs                    # brief size + no-frontmatter discipline
│   │       ├── codex.rs                    # codex rule shape + RFC-28 namespace ownership
│   │       ├── docs_quality.rs             # RFC citation hygiene, diagram assets
│   │       ├── links.rs                    # markdown links + symlink-aware references
│   │       ├── plugins.rs                  # symlinks + marketplace.json consistency
│   │       ├── prose.rs                    # invocation positionals, operational vocab, caps
│   │       ├── scenarios.rs                # scenario frontmatter + recorded-trace freshness
│   │       ├── skill_body.rs               # skill body discipline (12 predicates)
│   │       ├── skill_frontmatter.rs        # skill frontmatter discipline (7 predicates)
│   │       └── tools.rs                    # declared-tool equivalence
│   └── accept/                             # integration tests over tests/fixtures/
│       ├── Cargo.toml
│       └── tests/                          # one file per fixture surface (sources / targets / skills)
├── tools/
│   ├── framework-check/                    # binary: CI + local
│   │   ├── Cargo.toml
│   │   └── src/main.rs                     # ~100 LOC dispatcher
│   └── docgen/                             # binary: envelope doc regeneration
│       ├── Cargo.toml
│       └── src/main.rs
├── hooks/
│   └── pre-commit                          # shell shim → framework-check --changed
└── .github/
    ├── actions/framework-check/            # composite action wrapping the binary
    └── workflows/ci.yaml                   # cargo, no Deno
```

`framework-rules` depends on `specify-domain` and `specify-error`. The dependency is a **git dep** pinned to a tag for releases, with a `[patch.crates-io]` override available for local sibling-checkout development (mirroring how the existing Deno scripts use `SPECIFY_CLI_DIR=../specify-cli`). CI checks out both repos exactly as it does today.

Failure messages MUST match the current `check.ts` wording during the overlap period so PR diffs stay readable; message stability is how we verify it is safe to delete a Deno module.

### Schema-first layer (do this first)

Most checks in `scripts/checks/` enforce shapes that JSON Schema can express. The earliest, highest-leverage work is to make sure every such shape **is** a schema, and that Cursor sees it.

Concrete moves:

- **Authoritative location.** Every schema consumed by both `specify-cli` (runtime) and `framework-rules` (CI) lives in `specify-cli/schemas/` and is `include_str!`-ed there. Framework-only schemas (skill frontmatter, codex rule, scenario) move from `.cursor/schemas/` into `specify-cli/schemas/` so both sides consume the same artifacts. The `.cursor/schemas/` aliases stay as symlinks for editor convenience until Cursor settings are updated.
- **Editor wiring.** Workspace settings (`.cursor/settings.json` or per-file `# yaml-language-server: $schema=` directives) point every `adapter.yaml`, `SKILL.md` frontmatter, scenario file, codex rule, marketplace manifest, and `tools.yaml` at its schema. The YAML/JSON LSPs Cursor already ships then surface violations live, with no extra tooling installed.
- **Schema strengthening.** Rules currently enforced imperatively in `skill_frontmatter.ts` (description grammar, argument-hint shape, 200/45/512 caps on counted fields) are expressed as `pattern`, `maxLength`, and `enum` constraints where they fit. The minority that genuinely cannot be schema'd (variable consistency, cross-skill directive resolution, body-section discipline) stays in `framework-rules`.
- **Documentation.** `docs/contributing/checks.md` gets a new section explaining the editor-first model: most violations are red squigglies before a single CLI command runs.

This phase delivers contributor value before any Rust binary lands.

### `framework-rules` library

A single crate exposing each predicate as a `Check` returning structured findings:

```rust
pub struct Finding {
    pub rule_id: &'static str,         // stable kebab-case id
    pub severity: Severity,            // error | warning
    pub message: String,               // matches check.ts wording during overlap
    pub location: Option<Location>,    // file + 1-based line + optional column
}

pub trait Check {
    fn id(&self) -> &'static str;
    fn run(&self, ctx: &Context) -> Vec<Finding>;
}
```

`Context` carries the resolved repo root, a `specify-domain` adapter resolver, lazily-loaded schemas, and a set of changed paths (for `--changed` mode). Predicates are independent and parallelisable (`rayon` or `tokio::task::spawn_blocking`), mirroring the `Promise.all` batches in `scripts/check.ts`.

Rule ids align with RFC-28's reserved namespaces where applicable (the codex namespace-ownership rule lives here as `codex.namespace-ownership-violation` and feeds the future shared finding shape).

### `framework-check` binary

A ~100 LOC clap dispatcher with three modes:

```bash
framework-check                            # full repo scan (CI default)
framework-check --changed                  # only files changed vs origin/main
framework-check --rule codex.namespace-*   # rule-id glob filter (CI debugging)
framework-check --format json              # JSON envelope for tooling
```

Exit codes follow the standard table inherited from `specify-cli`:

- `0` — success.
- `2` — validation findings or argument errors.
- `1` — infrastructure errors (I/O, schema load failures, git not available for `--changed`).

The JSON envelope shape matches `specify-cli`'s output shape contract so a future GitHub Action or scorer can consume both.

### `accept` crate

Ports `tests/cross_repo.ts` and its `tests/lib/` helpers into a Rust integration crate that:

- Uses `specify-domain` directly for provenance parsing (kills `tests/lib/spec_provenance.ts`).
- Uses the same JSON Schema validators as `framework-rules` (kills `tests/lib/validators.ts`).
- Keeps the optional `SPECIFY_BIN` subprocess tests for `specify source resolve` and `specify target resolve`; skips cleanly when the binary is absent (matches today's harness).
- Adopts `specify-cli`'s `REGENERATE_GOLDENS=1` discipline for any byte-stable goldens it asserts.

Test-binary names mirror the existing Deno suites (`sources`, `targets`, `skills_refine`, `skills_loop`) so `cargo test --test <name>` is easy.

### `docgen` binary

Ports `scripts/gen-envelope-doc.ts`:

```bash
docgen envelopes               # regenerate docs/reference/cli-output-shapes.md
docgen envelopes --check       # CI mode: diff and exit 2 on drift
```

Same generated-block markers (`<!-- generated:begin -->` / `<!-- generated:end -->`), same explicit fixture-to-section mapping table, same `SPECIFY_CLI_DIR` semantics (renamed env var: `SPECIFY_CLI_ROOT`, with the old name accepted as fallback during transition).

### Pre-commit hook and GitHub Action

Both are thin wrappers, present in this RFC for completeness but trivial to implement:

- **`hooks/pre-commit`** — a shell shim that runs `framework-check --changed` and exits non-zero on findings. Installable via `make install-hooks` or `pre-commit install` for projects that adopt the framework.
- **`.github/actions/framework-check/`** — a composite action that runs `cargo run -p framework-check -- --format json`, parses the envelope, and posts PR annotations via `actions/github-script`. Replaces the inline `make check` step in `.github/workflows/ci.yaml`.

### `framework-lsp` (deferred)

Listed in the architecture diagram so contributors see the full intended shape. Not implemented in this RFC because:

- Schema-first handles the easy 80% via Cursor's built-in language servers without any custom LSP code.
- The cross-file rules that would benefit from an LSP (symlink integrity, marketplace consistency, cross-skill directive resolution, variable coverage) are a small enough surface that the CLI-on-save loop is acceptable until contributor pain justifies the engineering investment.
- A future `framework-lsp` reuses `framework-rules` unchanged, so the architectural commitment is already paid.

### Migration strategy

Sequenced for minimum risk, with Deno retiring incrementally rather than in one cutover:

1. **Schema-first pass.** Move framework-only schemas into `specify-cli/schemas/`, strengthen constraints where they currently live in imperative code, and wire Cursor `$schema` references. No Rust code yet. Largest contributor-experience win for smallest cost.
2. **Workspace scaffold.** Land `Cargo.toml`, `rust-toolchain.toml`, empty `framework-rules`, `framework-check`, `accept`, and `docgen` crates that compile and run trivially. CI runs both Deno and Rust; Rust is allowed to be empty. Mechanical, self-contained PR.
3. **Port `docgen` first.** Smallest surface (~250 LOC), proves the workspace dependency story end-to-end, and lets us delete `scripts/gen-envelope-doc.ts` early.
4. **Port `accept`.** Replaces the worst parser duplication (`spec_provenance.ts`, `validators.ts`). Run side-by-side until output parity is trusted, then delete `tests/cross_repo.ts` and `tests/lib/`.
5. **Port `framework-rules` modules in dependency order.** `adapter` and `brief` first (highest `specify-domain` reuse), then `skill_frontmatter` / `skill_body` / `prose`, then `links` / `plugins` / `docs_quality`, then `codex` / `scenarios` / `tools`. Each merged module deletes its Deno counterpart. Message-preserving throughout.
6. **CI cleanup.** When every Deno script is empty, delete the trees, drop `denoland/setup-deno` from `.github/workflows/ci.yaml`, remove Deno from `docs/contributing/index.md` prerequisites, update `Makefile` to call `cargo` directly, and switch the GitHub Action over.
7. **Optional follow-ups (not part of this RFC).** `framework-lsp` when rule count grows; WASI extensibility if third-party rule packs become a real ask.

Each step is independently mergeable and leaves CI green.

### Makefile integration

Target end state (Deno fully removed):

```makefile
.PHONY: check test ci docs

check:
	cargo run -p framework-check -- --repo .

test:
	cargo test --workspace

ci: check test
```

During migration, `make check` and `make test` each call both the Deno script and the Rust binary; any discrepancy is treated as a port regression. The dual-run phase ends per surface (docgen first, then accept, then framework-check).

### Coordination with other RFCs

- **RFC-4 (typed skill expression).** Option 1 (CLI-integrated skill validation) is satisfied by the schema-first pass plus the `skill_frontmatter` / `skill_body` modules in `framework-rules`. The "CLI" in that RFC is reinterpreted as `framework-check`, not `specify check`. Options 2 and 3 are unchanged.
- **RFC-28 (codex resolution).** RFC-28 cites RFC-5 for namespace-ownership enforcement. That contract is preserved verbatim — the rule moves to `framework-rules::codex` and continues to enforce that first-party files do not use `ORG-*`. Where the rule lives (this repo, not specify-cli) is invisible to RFC-28's resolver and finding shape.
- **Roadmap RM-16.** The roadmap entry currently reads "Port `scripts/check.ts` from Deno into a Rust `specify-check` crate exposed as `specify check`." When this RFC lands, RM-16's goal becomes "Land the framework dev-tooling workspace in `augentic/specify` per RFC-5: schema-first authoring feedback, `framework-check` binary, `accept` and `docgen` crates, Deno retirement." The unblocks line (RFC-4 Option 1, declared-WASI-tool helper migration) is unchanged.

## Alternatives Considered

**The original RFC-5: port into `specify-cli` as `specify check`.** Rejected because it puts framework dev tooling on the operator product. Operators running Specify on a consumer project never need to validate `plugins/` or `.cursor-plugin/marketplace.json`; bundling that surface bloats the install for everyone to serve the few. The parser-reuse argument that motivated the original choice is satisfied just as well by depending on `specify-domain` as a library from a sibling workspace, which is the standard Rust pattern.

**Keep `scripts/check.ts` indefinitely.** Tempting because the script works and is not blocking. Rejected because (a) `tests/lib/spec_provenance.ts` and `tests/lib/validators.ts` actively duplicate `specify-domain`, and that duplication grows with every new schema; (b) schemas without an editor-first contract are invisible to contributors; (c) keeping Deno in CI forever to validate Rust-defined schemas is a coordination tax with no offsetting benefit.

**Merge the repos.** Considered: one workspace with `cli/` and `plugins/` top-level directories would kill the cross-repo coordination entirely. Rejected because it conflates two audiences (Rust contributors vs skill/adapter authors), couples operator-CLI release cadence to plugin-content cadence, and forces a single review style on both halves. The dev-tooling problem does not require it.

**Self-hosted Specify (the framework repo as a Specify project).** Considered as the long-term aspiration: every plugin and adapter as a slice, framework consistency from `specify slice validate`. Rejected as the *only* solution because mechanical checks still need a deterministic engine underneath, and the framework lifecycle (RFC editing, README polish, contributor docs) does not naturally fit slices. Compatible with this RFC: a self-hosted layer can sit on top of `framework-rules` later.

**WASI rules as the day-one shape.** Considered: ship `framework-check.wasm` declared via an adapter manifest and invoked through `specify tool run framework-check`. Reuses the Vectis pattern and opens third-party rule packs immediately. Rejected as overkill when there is exactly one rule pack and one consumer (CI); the library shape adopted here makes future WASI exposure a small refactor, not a re-architecture.

**Rewrite from scratch (no message preservation).** Rejected for the same reason the original RFC rejected it: the current invariants encode real lessons about repo drift. Preserving wording during overlap lets CI act as a regression test for the port itself.

## References

- [`scripts/check.ts`](../scripts/check.ts) + [`scripts/checks/`](../scripts/checks/) — the framework linter being replaced.
- [`scripts/gen-envelope-doc.ts`](../scripts/gen-envelope-doc.ts) — the doc generator being replaced.
- [`tests/cross_repo.ts`](../tests/cross_repo.ts) + [`tests/lib/`](../tests/lib/) — the acceptance harness being replaced.
- [`docs/standards/skill-authoring.md`](../docs/standards/skill-authoring.md) — invariants the skill-discipline modules enforce.
- [`docs/explanation/adapter-anatomy.md`](../docs/explanation/adapter-anatomy.md) — the adapter model the `adapter` and `brief` modules validate.
- [`docs/contributing/acceptance.md`](../docs/contributing/acceptance.md) — the acceptance surface split between deterministic harness (this RFC) and manual scenario packs (out of scope).
- [Specify CLI `AGENTS.md`](https://github.com/augentic/specify-cli/blob/main/AGENTS.md) — crate graph this workspace consumes via `specify-domain`.
- [Specify CLI handler-shape contract](https://github.com/augentic/specify-cli/blob/main/docs/standards/handler-shape.md) — JSON envelope shape `framework-check --format json` mirrors.
- [RFC-4: Type-Safe Skill Expression](rfc-4-dsl.md) — Option 1 is satisfied by the schema-first pass plus the skill-discipline modules.
- [RFC-28: Codex Resolution and Structured Review Findings](next/rfc-28-codex-rules.md) — namespace-ownership contract preserved by `framework-rules::codex`.
