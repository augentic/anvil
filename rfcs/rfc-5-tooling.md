# RFC-5: Framework Developer Tooling

> Status: Draft · Tracked by [roadmap RM-16](roadmap.md#rm-16-rfc-5-framework-developer-tooling-workspace) · Enables: [RFC-4](future/rfc-4-dsl.md) · Preserves contract with: [RFC-28](next/rfc-28-codex-rules.md)

## Abstract

Replace this repo's Deno tooling (`scripts/check.ts`, `scripts/gen-envelope-doc.ts`, `tests/cross_repo.ts`) with a small Rust workspace at `**augentic/specify/tooling/**`, introduced in phases. 

The workspace lives in a dedicated subdirectory so skill and adapter authors see a markdown-first repo root (`plugins/`, `adapters/`, `docs/`) without `Cargo.toml`, `crates/`, or `rust-toolchain.toml` beside the content they edit. 

The first phase is deliberately narrow: JSON Schemas are the canonical contract for every artifact whose shape can be expressed declaratively; cross-file rules that schemas cannot express live in a single `rules` library crate; that library backs a thin `**tooling**` binary whose `check` subcommand runs locally and in CI. Once that rule engine is proven, the same binary gains a `docgen` subcommand and an `accept` integration-test crate ports fixture acceptance. 

The operator `specify` binary in `augentic/specify-cli` is deliberately **not** extended; framework tooling stays with the framework it validates.

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

The product principles that follow: **dev tooling for the plugin repo lives in the plugin repo**, not on the operator binary; **Rust contributor surface stays under `tooling/`** so the repo root remains author-facing content, not a second Cargo workspace; and **build steps stay invisible** — authors invoke `make check` (or an optional hook), never `cargo build` directly.

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
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
   ┌───────────────┐ ┌───────────────┐ ┌───────────────┐
   │ Layer 3a:     │ │ Layer 3b:     │ │ Layer 3c:     │
   │ tooling       │ │ lsp (future)  │ │ pre-commit    │
   │ binary        │ │ Cursor LSP    │ │ hook          │
   │ check subcmd  │ │               │ │ Best effort   │
   └───────────────┘ └───────────────┘ └───────────────┘
              │
              ▼
   ┌─────────────────────────────┐
   │ Layer 4: annotations/export  │
   │ Deferred until JSON settles. │
   └─────────────────────────────┘
```

Each layer optimises for its audience: schemas catch the easy 80% in the editor at zero cost; the library carries the hard 20% once; the frontends are thin shells.

### Scope

Initial implementation scope:

- A new Rust workspace under `augentic/specify/tooling/` (this repo).
- `rules` library crate carrying every predicate currently in `scripts/checks/`.
- `tooling` binary with a `check` subcommand that scans the **framework repo root** (`plugins/`, `adapters/`, `docs/`, …) for CI and local use — not the `tooling/` subtree alone.
- Schema-first migration: extract every framework-repo check that can be a JSON Schema into one (or strengthen an existing one), and wire `$schema` references so Cursor surfaces violations inline.
- Minimal JSON output for `tooling check` once rule ids and finding locations are stable enough to fixture.
- `scripts/run-tool` shell entry point that auto-builds the `tooling` binary so `make check` and hooks never require a manual `cargo build`.

Follow-on scope in the same workspace, after `tooling check` proves the dependency and output model:

- `docgen` library crate and a matching `tooling docgen` subcommand that ports `scripts/gen-envelope-doc.ts`.
- `accept` integration-test crate that ports `tests/cross_repo.ts` and its `tests/lib/` helpers.
- PR annotation helpers or a composite GitHub Action wrapping `tooling check --format json`.

Out of scope (future RFCs):

- `lsp` — designed for here but not implemented until rule count justifies it.
- WASI extensibility for third-party rule packs — the library shape allows it; this RFC does not adopt it.
- Any new invariants beyond what the Deno scripts enforce today. New checks belong to RFC-4 Option 1, RFC-28 codex resolution, or successor RFCs.
- Manual scenario packs under `tests/cross-repo/` and `tests/plan/` — operator-driven by design; see [docs/contributing/acceptance.md](../docs/contributing/acceptance.md).

The boundary against the operator CLI is explicit: `specify-cli` validates *consumer projects* at runtime (adapter manifest loads, slice lifecycle transitions, plan validation, merge); this workspace validates *the framework repo itself* (skill integrity, adapter brief discipline, marketplace alignment, docs hygiene, fixture acceptance). The overlap is intentional and narrow: both sides need runtime adapter-manifest parsing and runtime JSON Schemas, while framework-only authoring schemas belong in this repo. Shared parsing comes from `specify-domain` only where the runtime already owns the shape.

### Workspace layout

Two roots, one workspace:

- **Framework root** — `augentic/specify/`; what skill and adapter authors browse. Contains `plugins/`, `adapters/`, `docs/`, `tests/fixtures/`, and the existing `scripts/` / `tests/` trees until Deno retires. Every scanner predicate walks this tree.
- **Tooling root** — `augentic/specify/tooling/`; what Rust contributors build. Contains the Cargo workspace, framework-only schemas, the `tooling` binary, library crates, and hook shims. Nothing author-facing lives here except contributor docs that point back at the framework root.

```text
augentic/specify/                           # framework root (scan target)
├── plugins/
├── adapters/
├── docs/
├── tests/
│   ├── fixtures/                           # acceptance inputs (unchanged location)
│   └── cross-repo/                         # manual scenario packs (out of scope)
├── scripts/
│   ├── check.ts                            # Deno — retired incrementally
│   └── run-tool                            # shell: build + exec tooling binary (survives Deno retirement)
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
│   │   ├── docgen/                         # follow-on: envelope doc generation library
│   │   │   ├── Cargo.toml
│   │   │   └── src/lib.rs
│   │   └── accept/                         # follow-on: integration tests over ../tests/fixtures/
│   │       ├── Cargo.toml
│   │       └── tests/                      # one file per fixture surface (sources / targets / skills)
│   ├── tools/
│   │   └── tooling/                        # single binary: check + docgen subcommands
│   │       ├── Cargo.toml
│   │       └── src/
│   │           ├── main.rs                 # clap root (~150 LOC)
│   │           ├── check.rs                # dispatches to rules
│   │           └── docgen.rs               # dispatches to docgen crate
│   └── hooks/
│       └── pre-commit                      # shell shim → scripts/run-tool check --repo .. --changed
└── .github/
    ├── actions/tooling/                    # follow-on: PR annotations
    └── workflows/ci.yaml                   # build once, run binary; no Deno
```

`rules` depends on `specify-domain` and `specify-error` for predicates that reuse runtime-owned parsers or schemas. The dependency is a **git dep** pinned to a tag for releases, with a `[patch.crates-io]` override available for local sibling-checkout development (mirroring how the existing Deno scripts use `SPECIFY_CLI_DIR=../specify-cli`). CI checks out both repos exactly as it does today.

Failure messages MUST match the current `check.ts` wording during the overlap period so PR diffs stay readable. Each ported module also gets fixtures for its structured findings; message stability keeps humans oriented, while fixture stability verifies it is safe to delete a Deno module.

### Crate naming

The library crate is `**rules`** (`tooling/crates/rules/`). User-facing commands run through a single `**tooling**` binary (`tooling/tools/tooling/`; artifact path `tooling/target/debug/tooling`). Subcommands replace the separate binaries from earlier drafts:


| Subcommand       | Replaces                              | Library  |
| ---------------- | ------------------------------------- | -------- |
| `tooling check`  | `scripts/check.ts`, `framework-check` | `rules`  |
| `tooling docgen` | `scripts/gen-envelope-doc.ts`         | `docgen` |


Acceptance stays `**cargo test -p accept**` — not a CLI subcommand. The `framework-` prefix from earlier drafts is dropped; the `tooling/` workspace scopes these names away from the operator product. This is **not** `specify check` or `specify review` — those surfaces validate consumer projects on the operator binary (rejected in §Alternatives and reserved separately in RFC-28 / roadmap RM-10). Contributors disambiguate the `check` subcommand from `make check`, `scripts/check.ts` (during Deno overlap), and `cargo check` by context: day-to-day invocation is always `./scripts/run-tool check …` or `make check`, never a global install.

**Broader renaming.** This RFC is canonical for the shortened names and the single-binary shape. Landing the workspace must update every cross-reference that still says `framework-rules`, `framework-check`, `framework-lsp`, or separate `check`/`docgen` binaries: [RFC-1](done/rfc-1-cli.md), [RFC-4](future/rfc-4-dsl.md), [RFC-10](done/rfc-10-skills.md), [RFC-13](done/rfc-13-extensibility.md), [RFC-28](next/rfc-28-codex-rules.md), [RFC-30](next/rfc-30-init.md), [roadmap RM-16 / RM-07](roadmap.md), [docs/contributing/checks.md](../docs/contributing/checks.md), and the `.github/actions/tooling/` composite. Module paths in prose become `rules::codex` rather than `framework-rules::codex`. No rename is required in `specify-cli` — the operator binary is unchanged.

### Schema-first layer (do this first)

Most checks in `scripts/checks/` enforce shapes that JSON Schema can express. The earliest, highest-leverage work is to make sure every such shape **is** a schema, and that Cursor sees it.

Concrete moves:

- **Authoritative location.** Runtime schemas consumed by `specify-cli` stay in `specify-cli/schemas/` and are reused through `specify-domain`. Framework-only schemas (skill frontmatter, codex rule authoring, scenario metadata, marketplace manifests) stay in `augentic/specify/tooling/schemas/`; `.cursor/schemas/` contains editor-facing symlinks or aliases only when Cursor needs them. A schema moves to `specify-cli` only when the runtime binary genuinely consumes it.
- **Editor wiring.** Workspace settings (`.cursor/settings.json` or per-file `# yaml-language-server: $schema=` directives) point every `adapter.yaml`, `SKILL.md` frontmatter, scenario file, codex rule, marketplace manifest, and `tools.yaml` at its schema. The YAML/JSON LSPs Cursor already ships then surface violations live, with no extra tooling installed.
- **Schema strengthening.** Rules currently enforced imperatively in `skill_frontmatter.ts` (description grammar, argument-hint shape, 200/45/512 caps on counted fields) are expressed as `pattern`, `maxLength`, and `enum` constraints where they fit. The minority that genuinely cannot be schema'd (variable consistency, cross-skill directive resolution, body-section discipline) stays in `rules`.
- **Documentation.** `docs/contributing/checks.md` gets a new section explaining the editor-first model: most violations are red squigglies before a single CLI command runs.

This phase delivers contributor value before any Rust binary lands.

### `rules` library

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

`Context` carries the resolved **framework root** (never the `tooling/` workspace directory alone), a `specify-domain` adapter resolver where needed, lazily-loaded schemas from `tooling/schemas/`, and a set of changed paths (for `--changed` mode). Predicates are independent and parallelisable (`rayon` or `tokio::task::spawn_blocking`), mirroring the `Promise.all` batches in `scripts/check.ts`.

Rule ids align with RFC-28's reserved namespaces where applicable (the codex namespace-ownership rule lives here as `codex.namespace-ownership-violation` and feeds the future shared finding shape).

### `tooling` binary

One clap root with subcommands. Day-to-day callers at the framework root use `scripts/run-tool` or `make check` instead of invoking Cargo or the binary path directly — see §*Invisible entry points and auto-build*.

#### `tooling check`

A thin dispatcher over `rules`. `**--repo`** names the framework root to scan; it defaults to the parent of the workspace directory when the binary is invoked from `tooling/`.

```bash
# day-to-day (framework root) — build is automatic
make check
./scripts/run-tool check --repo .
./scripts/run-tool check --repo . --changed
./scripts/run-tool check --repo . --format json

# tooling contributors (direct Cargo, optional)
cargo build --manifest-path tooling/Cargo.toml -p tooling
tooling/target/debug/tooling check --repo ..
cargo run --manifest-path tooling/Cargo.toml -p tooling -- check --repo ..
```

Exit codes follow the standard table inherited from `specify-cli`:

- `0` — success.
- `2` — validation findings or argument errors.
- `1` — infrastructure errors (I/O, schema load failures, git not available for `--changed`).

`--changed` is explicitly not equivalent to CI. It expands to changed paths plus any cheap dependency context each predicate declares, and it may fall back to a full-repo predicate when a rule is inherently global (for example, duplicate ids or marketplace consistency). CI always runs the full scan.

The JSON envelope shape matches `specify-cli`'s output shape contract so a future GitHub Action or scorer can consume both. JSON output lands after the first rule ids and locations are stable enough to pin with fixtures; PR annotations wait until that envelope has settled.

#### `tooling docgen`

Follow-on subcommand over the `docgen` library crate. Ports `scripts/gen-envelope-doc.ts`.

```bash
./scripts/run-tool docgen envelopes               # regenerate docs/reference/cli-output-shapes.md
./scripts/run-tool docgen envelopes --check       # CI mode: diff and exit 2 on drift
```

Same generated-block markers (`<!-- generated:begin -->` / `<!-- generated:end -->`), same explicit fixture-to-section mapping table, same `SPECIFY_CLI_DIR` semantics (renamed env var: `SPECIFY_CLI_ROOT`, with the old name accepted as fallback during transition).

### `accept` crate

This is a sibling workspace tool, not part of the linter core and not a CLI subcommand. It ports `tests/cross_repo.ts` and its `tests/lib/` helpers into a Rust integration crate that:

- Uses `specify-domain` directly for provenance parsing (kills `tests/lib/spec_provenance.ts`).
- Uses the same JSON Schema validators as `rules` (kills `tests/lib/validators.ts`).
- Keeps the optional `SPECIFY_BIN` subprocess tests for `specify source resolve` and `specify target resolve`; skips cleanly when the binary is absent (matches today's harness).
- Adopts `specify-cli`'s `REGENERATE_GOLDENS=1` discipline for any byte-stable goldens it asserts.

Test-binary names mirror the existing Deno suites (`sources`, `targets`, `skills_refine`, `skills_loop`) so `cargo test --manifest-path tooling/Cargo.toml --test <name>` is easy. Fixture paths resolve from the framework root (`tests/fixtures/`, `tests/cross-repo/` inputs where applicable), not from inside `tooling/`.

### Pre-commit hook and annotations

The pre-commit hook is part of the first `tooling check` rollout; PR annotations are deferred until JSON findings have fixtures:

- `**tooling/hooks/pre-commit**` — a shell shim that calls `scripts/run-tool check --repo .. --changed` and exits non-zero on findings. The hook reuses the cached debug binary; it does not call `cargo run` on every commit. `make install-hooks` copies or symlinks it into `.git/hooks/pre-commit` at the framework root. Installation is **opt-in** — skill authors who rely on editor schemas and CI never need the hook.
- `**.github/actions/tooling/`** — a follow-on composite action that runs a release-built `tooling check --repo . --format json`, parses the envelope, and posts PR annotations via `actions/github-script`. CI builds the binary once per job, then invokes it directly (see §*Invisible entry points and auto-build*).

### Invisible entry points and auto-build

Framework tooling must **disappear into the background** for day-to-day skill and adapter work. No contributor should need to remember to compile Rust, know where `target/` lives, or type `cargo` unless they are editing the tooling workspace itself.

#### Contract


| Audience                          | What they run                                                      | Rust required locally?                     |
| --------------------------------- | ------------------------------------------------------------------ | ------------------------------------------ |
| Skill / adapter authors (default) | Cursor schemas while editing; `make check` before a PR; CI on push | Optional — schemas and CI cover most needs |
| Authors who want faster feedback  | `make install-hooks` (opt-in pre-commit)                           | Yes (via `tooling/rust-toolchain.toml`)    |
| Tooling contributors              | `cd tooling && cargo test`, `cargo build -p tooling`, …            | Yes                                        |


Authors never run `cargo build` as a separate step before `make check`. Build logic lives in one place; entry points call through it.

#### `scripts/run-tool`

A small shell script at the framework root (alongside the retiring Deno scripts) is the **single auto-build entry point** for the `tooling` binary:

```bash
# scripts/run-tool [subcommand args...]
# 1. Resolve tooling/target/debug/tooling (or release when TOOLING_RELEASE=1).
# 2. Run cargo build --manifest-path tooling/Cargo.toml -p tooling
#    (Cargo no-ops quickly when nothing changed).
# 3. exec the binary with forwarded args.
./scripts/run-tool check --repo .
./scripts/run-tool docgen envelopes --check
```

`Makefile`, pre-commit hooks, and contributor docs call `scripts/run-tool`; they do not embed Cargo flags. Tooling contributors may still invoke Cargo directly inside `tooling/`.

The script avoids `cargo run`, which re-resolves the build graph on every invocation and is slower for repeated local checks and hooks. Incremental compilation stays entirely in Cargo's cache.

#### Makefile

`make check` delegates to `scripts/run-tool`:

```makefile
.PHONY: check test ci install-hooks

check:
	./scripts/run-tool check --repo .

test:
	cargo test --manifest-path tooling/Cargo.toml --workspace

doc-envelopes:
	./scripts/run-tool docgen envelopes

ci: check test

install-hooks:
	install -m 755 tooling/hooks/pre-commit .git/hooks/pre-commit
```

During migration, `make check` may call both `scripts/check.ts` and `scripts/run-tool check` until Deno retires.

#### CI

CI builds once per job, then runs the binary — it does not use `cargo run`:

```yaml
- uses: dtolnay/rust-toolchain@stable
  with:
    toolchain-file: tooling/rust-toolchain.toml
- run: cargo build --manifest-path tooling/Cargo.toml -p tooling --release
- run: tooling/target/release/tooling check --repo .
  env:
    SPECIFY_CLI_ROOT: specify-cli
```

The same pattern applies to `tooling docgen envelopes --check` and `cargo test` once the Deno harness is gone. `tooling/target/` is never committed.

#### Contributor documentation

`docs/contributing/index.md` and `docs/contributing/checks.md` describe the split explicitly:

- **Editor-first** — most violations surface as schema squigglies; no CLI install required.
- **Local gate** — `make check` auto-builds and runs `tooling check`; first run after clone compiles once, subsequent runs reuse the cached binary until tooling sources change.
- **CI gate** — authoritative full scan on every PR; sufficient on its own for contributors who skip local Rust.
- **Optional hook** — `make install-hooks` for `--changed` scans before commit.

Rust appears under prerequisites only for contributors who run `make check` or hooks locally, not as a blanket requirement for every markdown edit.

#### Explicitly out of scope

- **Committed prebuilt binaries** — platform-specific churn; Cargo incremental build is enough.
- `**cargo install tooling`** — adds global install/update ceremony; repo-local binaries via `scripts/run-tool` are simpler.
- **Auto-build on every file save** — cross-file rules are too heavy; editor schemas cover save-time feedback. A future `lsp` would reuse `rules` without changing this contract.

### `lsp` (deferred)

Listed in the architecture diagram so contributors see the full intended shape. Not implemented in this RFC because:

- Schema-first handles the easy 80% via Cursor's built-in language servers without any custom LSP code.
- The cross-file rules that would benefit from an LSP (symlink integrity, marketplace consistency, cross-skill directive resolution, variable coverage) are a small enough surface that the CLI-on-save loop is acceptable until contributor pain justifies the engineering investment.
- A future `lsp` reuses `rules` unchanged, so the architectural commitment is already paid.

### Migration strategy

Sequenced for minimum risk, with Deno retiring incrementally rather than in one cutover:

1. **Schema-first pass.** Keep runtime schemas in `specify-cli/schemas/`, move or strengthen framework-only schemas under `augentic/specify/tooling/schemas/`, and wire Cursor `$schema` references. No Rust code yet. Largest contributor-experience win for smallest cost.
2. **Rule-engine scaffold.** Land `tooling/Cargo.toml`, `tooling/rust-toolchain.toml`, `scripts/run-tool`, `rules`, and the `tooling` binary with a `check` subcommand that compiles and runs a no-op full scan against `--repo ..` via `make check`. CI still runs Deno; Rust is allowed to be empty. Mechanical, self-contained PR.
3. **Port high-leverage checks first.** Start with checks that remove parser/schema duplication: adapter manifest validation and brief discipline, then skill frontmatter/body shape. Add structured finding fixtures as each module lands. Each merged module deletes its Deno counterpart after message and fixture parity.
4. **Stabilize `tooling check` output.** Add `--format json`, rule-id filters, and the best-effort `--changed` mode only after several modules have stable ids and locations. Keep CI on full scans; use `--changed` only for pre-commit speed.
5. **Finish `rules` modules.** Port `prose`, `links`, `plugins`, `docs_quality`, `codex`, `scenarios`, and `tools` in dependency order. Continue side-by-side Deno/Rust checks until every `scripts/checks/` predicate has moved or been retired.
6. **Port `docgen`.** Add the `docgen` library crate and `tooling docgen` subcommand; delete `scripts/gen-envelope-doc.ts`.
7. **Port `accept`.** Replace the worst parser duplication (`spec_provenance.ts`, `validators.ts`) by using `specify-domain` and shared validators directly. Run side-by-side until output parity is trusted, then delete `tests/cross_repo.ts` and `tests/lib/`.
8. **CI cleanup.** When every Deno surface is gone, delete the Deno trees, drop `denoland/setup-deno` from `.github/workflows/ci.yaml`, update `docs/contributing/index.md` with the audience split and optional-Rust prerequisites, update `Makefile` to call `scripts/run-tool` only, and optionally add the GitHub Action wrapper for PR annotations.
9. **Optional follow-ups (not part of this RFC).** `lsp` when rule count grows; WASI extensibility if third-party rule packs become a real ask.

Each step is independently mergeable and leaves CI green.

### Makefile integration

The target end state is documented in §*Invisible entry points and auto-build*. In short: `make check` calls `./scripts/run-tool check --repo .`; `make test` runs `cargo test --manifest-path tooling/Cargo.toml --workspace`; `make install-hooks` installs the opt-in pre-commit shim.

During migration, `make check` calls both `scripts/check.ts` and `scripts/run-tool check`; any discrepancy for a ported rule is treated as a port regression. `make test` keeps the existing Deno acceptance harness until the later `accept` crate reaches parity. The dual-run phase ends per surface: framework checks first, then docgen, then acceptance.

### Coordination with other RFCs

- **RFC-4 (typed skill expression).** Option 1 (framework-tooling skill validation) is satisfied by the schema-first pass plus the `skill_frontmatter` / `skill_body` modules in `rules`. The "CLI" in that RFC is reinterpreted as `tooling check`, not `specify check`. Options 2 and 3 are unchanged.
- **RFC-28 (codex resolution).** RFC-28 cites RFC-5 for namespace-ownership enforcement. That contract is preserved verbatim — the rule moves to `rules::codex` and continues to enforce that first-party files do not use `ORG-`*. Where the rule lives (this repo, not specify-cli) is invisible to RFC-28's resolver and finding shape.
- **Roadmap RM-16.** The roadmap entry tracks this RFC's framework dev-tooling workspace: schema-first authoring feedback, `tooling check`, `accept`, `tooling docgen`, and Deno retirement. The unblocks line (RFC-4 Option 1, declared-WASI-tool helper migration) is unchanged.

## Alternatives Considered

**The original RFC-5: port into `specify-cli` as `specify check`.** Rejected because it puts framework dev tooling on the operator product. Operators running Specify on a consumer project never need to validate `plugins/` or `.cursor-plugin/marketplace.json`; bundling that surface bloats the install for everyone to serve the few. The parser-reuse argument that motivated the original choice is satisfied just as well by depending on `specify-domain` as a library from a sibling workspace, which is the standard Rust pattern.

**One binary per Deno script (`check`, `docgen`).** Considered: mirror `scripts/check.ts` and `scripts/gen-envelope-doc.ts` as separate Cargo packages under `tooling/tools/`, dispatched through `scripts/run-tool <package>`. Rejected because the binaries are thin clap shells over library crates, share one workspace dependency story, and differ only by subcommand. A single `tooling` binary with `check` and `docgen` subcommands matches the `specify` operator pattern, simplifies `run-tool` (one package to build), and keeps acceptance on `cargo test` where it belongs.

**Keep `scripts/check.ts` indefinitely.** Tempting because the script works and is not blocking. Rejected because (a) `tests/lib/spec_provenance.ts` and `tests/lib/validators.ts` actively duplicate `specify-domain`, and that duplication grows with every new schema; (b) schemas without an editor-first contract are invisible to contributors; (c) keeping Deno in CI forever to validate Rust-defined schemas is a coordination tax with no offsetting benefit.

**Merge the repos.** Considered: one workspace with `cli/` and `plugins/` top-level directories would kill the cross-repo coordination entirely. Rejected because it conflates two audiences (Rust contributors vs skill/adapter authors), couples operator-CLI release cadence to plugin-content cadence, and forces a single review style on both halves. The dev-tooling problem does not require it.

**Rust workspace at the framework repo root.** Considered: colocate `Cargo.toml`, `crates/`, and `tools/` beside `plugins/` and `adapters/` (the layout in the original RFC-5 draft). Rejected because it puts Rust contributor surface in the same directory tree skill and adapter authors browse daily — `Cargo.toml` beside `plugins/` reads like consumer-project scaffolding, and a top-level `schemas/` collides mentally with consumer `contracts/schemas/` even when the paths differ. Nesting the workspace under `tooling/` preserves the audience split the RFC already applies against `specify-cli`, without merging the two repos.

**Self-hosted Specify (the framework repo as a Specify project).** Considered as the long-term aspiration: every plugin and adapter as a slice, framework consistency from `specify slice validate`. Rejected as the *only* solution because mechanical checks still need a deterministic engine underneath, and the framework lifecycle (RFC editing, README polish, contributor docs) does not naturally fit slices. Compatible with this RFC: a self-hosted layer can sit on top of `rules` later.

**WASI rules as the day-one shape.** Considered: ship `tooling check` as a WASI module declared via an adapter manifest and invoked through `specify tool run`. Reuses the Vectis pattern and opens third-party rule packs immediately. Rejected as overkill when there is exactly one rule pack and one consumer (CI); the library shape adopted here makes future WASI exposure a small refactor, not a re-architecture.

**Rewrite from scratch (no message preservation).** Rejected for the same reason the original RFC rejected it: the current invariants encode real lessons about repo drift. Preserving wording during overlap lets CI act as a regression test for the port itself.

**Committed prebuilt binaries or global `cargo install`.** Considered so skill authors could skip a local Rust toolchain entirely. Rejected because checked-in platform binaries add release churn and security review overhead, and a global install adds update ceremony outside the repo. The adopted split — editor schemas + CI for everyone, optional local `make check` via auto-build — keeps Rust optional without shipping artifacts.

## References

- `[scripts/check.ts](../scripts/check.ts)` + `[scripts/checks/](../scripts/checks/)` — the framework linter being replaced.
- `[scripts/gen-envelope-doc.ts](../scripts/gen-envelope-doc.ts)` — the doc generator being replaced.
- `[tests/cross_repo.ts](../tests/cross_repo.ts)` + `[tests/lib/](../tests/lib/)` — the acceptance harness being replaced.
- `[docs/standards/skill-authoring.md](../docs/standards/skill-authoring.md)` — invariants the skill-discipline modules enforce.
- `[docs/explanation/adapter-anatomy.md](../docs/explanation/adapter-anatomy.md)` — the adapter model the `adapter` and `brief` modules validate.
- `[docs/contributing/acceptance.md](../docs/contributing/acceptance.md)` — the acceptance surface split between deterministic harness (this RFC) and manual scenario packs (out of scope).
- [Specify CLI `AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md)` — crate graph this workspace consumes via `specify-domain`.
- [Specify CLI handler-shape contract](https://github.com/augentic/specify-cli/blob/main/docs/standards/handler-shape.md) — JSON envelope shape `tooling check --format json` mirrors.
- [RFC-4: Type-Safe Skill Expression](future/rfc-4-dsl.md) — Option 1 is satisfied by the schema-first pass plus the skill-discipline modules.
- [RFC-28: Codex Resolution and Structured Review Findings](next/rfc-28-codex-rules.md) — namespace-ownership contract preserved by `rules::codex`.

