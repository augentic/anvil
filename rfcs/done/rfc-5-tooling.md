# RFC-5: Framework Developer Tooling

> Status: Implemented · Tracked by [roadmap RM-16](roadmap.md#rm-16-rfc-5-framework-developer-tooling-workspace) · Enables: [RFC-4](future/rfc-4-dsl.md) · Preserves contract with: [RFC-28](rfc-28-codex-rules.md), [RFC-31](rfc-31-workspace-model.md)

## Abstract

Replace this repo's Deno tooling (`scripts/check.ts`, `scripts/gen-envelope-doc.ts`, `tests/cross_repo.ts`) with a single Rust binary crate at `augentic/specify/tooling/`, landed as one atomic implementation PR.

The crate lives in a dedicated subdirectory so skill and adapter authors see a markdown-first repo root (`plugins/`, `adapters/`, `docs/`) without `Cargo.toml` beside the content they edit.

Inside that atomic PR the implementation order stays narrow-to-broad: JSON Schemas become the canonical contract for every artifact whose shape can be expressed declaratively; cross-file rules that schemas cannot express live in `check` modules behind a thin `tooling` binary whose `check` subcommand runs locally and in CI. The same binary gains a `docgen` subcommand, integration tests port fixture acceptance, and Deno is removed from the local and CI gates before review.

The operator `specify` binary in `augentic/specify-cli` is deliberately **not** extended; framework tooling stays with the framework it validates.

## Motivation

The original RFC-5 framed this work as a one-for-one port of `scripts/check.ts` into `specify-cli/crates/check/`, exposed as `specify check`. That framing collapses several different problems into one binary on the wrong product:

- **Authoring feedback** (catch typos in `SKILL.md` / `adapter.yaml` as you type) belongs in the editor, not in CI.
- **CI gating** is the authoritative final check — but should run the same predicates as the local layers.
- **Cross-repo coherence** (specify ↔ specify-cli schemas, envelopes, error codes) is a library concern; sharing it through a CLI subcommand is awkward.
- **Doc generation** (`gen-envelope-doc.ts`) and **fixture acceptance** (`tests/cross_repo.ts`) are not "linting" at all but share the same Deno toolchain we are trying to retire.

Putting all of this on the operator `specify` binary conflates two audiences. Operators running Specify on a consumer project never need to validate `plugins/`, `adapters/{sources,targets}/`, or `.cursor-plugin/marketplace.json`. The runtime CLI must stay focused on its job: deterministic workflow primitives for consumer projects (init, plan, slice lifecycle, adapter resolution, merge, workspace sync, WASI tool dispatch).

The Deno scripts work today. This RFC is not motivated by breakage but by three durable goals:

1. **Shift authoring feedback left.** Schema-first contracts let Cursor's built-in JSON/YAML language servers surface plain YAML/JSON violations as red squigglies, and give Markdown frontmatter the same canonical shape enforced by `tooling check`, removing a class of PR round-trips without depending on unproven editor behaviour.
2. **Eliminate parser duplication.** `tests/lib/spec_provenance.ts` mirrors `specify-domain`'s requirement-block parser; `scripts/checks/adapter.ts` re-runs Ajv against the same schemas `specify-domain` already ships. Sharing the Rust library kills both.
3. **Collapse the toolchain.** Remove Deno from `make check`, `make test`, and contributor prerequisites without adding it to the operator CLI's install surface.

The product principles that follow: **dev tooling for the plugin repo lives in the plugin repo**, not on the operator binary; **Rust contributor surface stays under `tooling/`** so the repo root remains author-facing content, not a second Cargo workspace; and **build steps stay invisible** — authors invoke `make check`, never `cargo build` directly.

## Detailed Design

### Architecture

Three layers, one rule engine, one binary.

```text
┌────────────────────────────────────────────────────────────────┐
│ Layer 1: Schemas                                               │
│   Runtime schemas stay in specify-cli/schemas/. Framework-only │
│   schemas stay in augentic/specify/tooling/schemas/ and are    │
│   wired into Cursor JSON/YAML LSPs for in-editor squigglies.   │
└────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────────┐
│ Layer 2: check modules                                         │
│   Cross-file predicates that schemas cannot express: symlink   │
│   integrity, marketplace ↔ plugins consistency, variable       │
│   coverage, cross-skill directive resolution, codex namespace  │
│   ownership, brief size, declared-tool invocation equivalence, │
│   link resolution. Reuses specify-domain for runtime-owned     │
│   shapes.                                                      │
└────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────────┐
│ Layer 3: tooling binary                                        │
│   clap root with `check` and `docgen` subcommands.             │
└────────────────────────────────────────────────────────────────┘
```

Each layer optimises for its audience: schemas catch the easy 80% in the editor at zero cost; the modules carry the hard 20% once; the binary is a thin shell.

### Scope

Atomic implementation scope:

- A single Rust binary crate at `augentic/specify/tooling/` (this repo).
- `check` modules carrying every predicate currently in `scripts/checks/`.
- `tooling` binary with a `check` subcommand that scans the framework repo root (`plugins/`, `adapters/`, `docs/`, …) for CI and local use.
- Schema-first migration: extract every framework-repo check that can be a JSON Schema into one (or strengthen an existing one), and wire `$schema` references so Cursor surfaces violations inline.
- `tooling docgen` subcommand that ports `scripts/gen-envelope-doc.ts`.
- Integration tests under `tooling/tests/` that port `tests/cross_repo.ts` and `tests/lib/`.
- `Makefile`, CI, and contributor-doc updates that remove Deno from the framework repo's normal validation path in the same PR.

Out of scope (future RFCs):

- A custom `lsp` and WASI extensibility for third-party rule packs — the module shape allows either; this RFC adopts neither.
- Any new invariants beyond what the Deno scripts enforce today. New checks belong to RFC-4 Option 1, RFC-28 codex resolution, or successor RFCs.
- `--format json` output, PR annotation helpers, and any composite GitHub Action that consumes them.
- A pre-commit hook and a `--changed` scoping mode — `make check` and CI cover the local + authoritative gates; both are additive if they ever become real asks.
- Manual scenario packs under `tests/cross-repo/` and `tests/plan/` — operator-driven by design; see [docs/contributing/acceptance.md](../docs/contributing/acceptance.md).

The boundary against the operator CLI is explicit: `specify-cli` validates *consumer projects* at runtime (adapter manifest loads, slice lifecycle transitions, plan validation, merge); this crate validates *the framework repo itself* (skill integrity, adapter brief discipline, marketplace alignment, docs hygiene, fixture acceptance). The overlap is intentional and narrow: both sides need runtime adapter-manifest parsing and runtime JSON Schemas. Shared parsing comes from `specify-domain` only where the runtime already owns the shape.

The current Deno surface is ~4,027 LOC across `scripts/check.ts`, `scripts/checks/*.ts`, `scripts/gen-envelope-doc.ts`, `tests/cross_repo.ts`, and `tests/lib/*.ts`. A library port that delegates to `specify-domain` should shrink the surface, not grow it; materially exceeding the current LOC is a reviewer signal to revisit predicate factoring before opening the PR.

#### Deno parity plan

The implementation PR deletes each Deno module only after the Rust equivalent has module-level fixtures that prove the same invariant class is still covered. Diagnostic wording may change; rule coverage and stable locations may not. Every ported rule lands with at least one positive and one negative fixture.


| Current Deno surface                                                                       | Rust home                                            | Schema-backed where possible?             |
| ------------------------------------------------------------------------------------------ | ---------------------------------------------------- | ----------------------------------------- |
| `scripts/checks/adapter.ts`                                                                | `check::adapter`                                     | Yes, via `specify-domain` runtime schemas |
| `scripts/checks/agent_teams.ts`                                                            | `check::agent_teams`                                 | No                                        |
| `scripts/checks/brief_size.ts`                                                             | `check::brief`                                       | No                                        |
| `scripts/checks/codex.ts`                                                                  | `check::codex`                                       | Yes, for frontmatter shape                |
| `scripts/checks/docs_quality.ts`                                                           | `check::docs_quality`                                | No                                        |
| `scripts/checks/links.ts`                                                                  | `check::links`                                       | No                                        |
| `scripts/checks/plugins.ts`                                                                | `check::plugins`                                     | Yes, for marketplace shape                |
| `scripts/checks/prose.ts`                                                                  | `check::prose`                                       | Partly                                    |
| `scripts/checks/scenarios.ts`                                                              | `check::scenarios`                                   | Yes, for scenario frontmatter             |
| `scripts/checks/skill_body.ts`                                                             | `check::skill_body`                                  | No                                        |
| `scripts/checks/skill_frontmatter.ts`                                                      | `check::skill_frontmatter`                           | Yes                                       |
| `scripts/checks/tools.ts`                                                                  | `check::tools`                                       | No                                        |
| `scripts/check.ts`, `scripts/gen-envelope-doc.ts`, `tests/cross_repo.ts`, `tests/lib/*.ts` | `tooling check`, `tooling docgen`, integration tests | Mixed                                     |


### Crate layout

Two roots, one crate:

- **Framework root** — `augentic/specify/`; what skill and adapter authors browse. Contains `plugins/`, `adapters/`, `docs/`, `tests/fixtures/`, and the existing `scripts/` / `tests/` trees until Deno retires. Every scanner predicate walks this tree.
- **Tooling root** — `augentic/specify/tooling/`; what Rust contributors build. Contains the single Cargo crate, framework-only schemas, and the `tooling` binary.

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
├── tooling/                                # Rust dev-tooling crate
│   ├── Cargo.toml                          # single crate manifest
│   ├── schemas/                            # framework-only authoring schemas
│   ├── src/
│   │   ├── main.rs                         # clap root
│   │   ├── check.rs                        # dispatcher
│   │   ├── check/                          # one module per predicate group
│   │   │   ├── adapter.rs                  # adapter.yaml ↔ source/target schemas
│   │   │   ├── agent_teams.rs              # per-target review-team overlays
│   │   │   ├── brief.rs                    # brief size + no-frontmatter discipline
│   │   │   ├── codex.rs                    # codex rule shape + RFC-28 namespace ownership
│   │   │   ├── docs_quality.rs             # RFC citation hygiene, diagram assets
│   │   │   ├── links.rs                    # markdown links + symlink-aware references
│   │   │   ├── plugins.rs                  # symlinks + marketplace.json consistency
│   │   │   ├── prose.rs                    # invocation positionals, operational vocab, caps
│   │   │   ├── scenarios.rs                # scenario frontmatter + recorded-trace freshness
│   │   │   ├── skill_body.rs               # skill body discipline (12 predicates)
│   │   │   ├── skill_frontmatter.rs        # skill frontmatter discipline (7 predicates)
│   │   │   └── tools.rs                    # declared-tool equivalence
│   │   └── docgen.rs                       # envelope doc generation
│   └── tests/                              # integration tests over ../tests/fixtures/
│       ├── sources.rs
│       ├── targets.rs
│       ├── skills_refine.rs
│       └── skills_loop.rs
└── .github/
    └── workflows/ci.yaml                   # cargo run; no Deno
```

A single binary crate (no Cargo workspace) is enough: the modules share one dependency story and one consumer (the binary itself). A future `lsp` that wants to reuse them is a small refactor — splitting `src/check/` into a library crate adds nothing today.

The Rust toolchain is not pinned via `rust-toolchain.toml`. Stable suffices; `Cargo.lock` is committed and is the lockfile of record.

#### Cross-repo dependency

`tooling` depends on `specify-domain` and `specify-error` for predicates that reuse runtime-owned parsers or schemas. Keep the default dependency story simple: both crates are pulled as **git deps pinned to a released tag**. `tooling/Cargo.toml` may also carry a commented local-development `[patch]` block for contributors working against a sibling `specify-cli` checkout.

```toml
# tooling/Cargo.toml
[dependencies]
specify-domain = { git = "https://github.com/augentic/specify-cli.git", tag = "v<X.Y.Z>" }
specify-error  = { git = "https://github.com/augentic/specify-cli.git", tag = "v<X.Y.Z>" }

# Optional local-development override. Leave commented in committed code.
# Uncomment only when testing framework tooling against a sibling specify-cli checkout.
#
# [patch."https://github.com/augentic/specify-cli.git"]
# specify-domain = { path = "../../specify-cli/crates/domain" }
# specify-error  = { path = "../../specify-cli/crates/error" }
```

Normal local runs and CI use the same tag, so a contributor can run `make check` with only this repository checked out. When the framework tooling needs a newer CLI parser or schema, the `specify-cli` change lands and tags first; `cargo add specify-domain --git https://github.com/augentic/specify-cli.git --tag v<X.Y.Z> --manifest-path tooling/Cargo.toml` (and the same for `specify-error`) updates the tag in `tooling/Cargo.toml` as a normal framework PR. Cross-repo development before that tag is a contributor-local escape hatch: uncomment the `[patch."https://github.com/augentic/specify-cli.git"]` block while testing, then re-comment it before opening the framework PR.

Rejected alternatives: an active committed path patch (Cargo does not conditionally activate it; missing sibling checkouts would break normal local runs), publishing `specify-domain` to crates.io (premature — adds release ceremony, exposes internal API, and the workspace genuinely is dev-time tooling that consumes unstable surfaces), or depending on an untagged branch (too easy for CI to drift under an unchanged lockfile review).

### Naming

The binary is `tooling` (`tooling/Cargo.toml`; artifact at `tooling/target/debug/tooling`). Subcommands `check` and `docgen` replace the separate `scripts/check.ts` and `scripts/gen-envelope-doc.ts` entry points; integration acceptance remains `cargo test`, not a subcommand. This is **not** `specify check` or `specify review` — those surfaces validate consumer projects on the operator binary (rejected in §Alternatives and reserved separately in RFC-28 / roadmap RM-10). Day-to-day callers use `make check`; the `cargo check` collision is resolved by context.

Landing this RFC must update every cross-reference that still says `framework-rules`, `framework-check`, `framework-lsp`, or separate `check`/`docgen` binaries: [RFC-1](done/rfc-1-cli.md), [RFC-4](future/rfc-4-dsl.md), [RFC-10](done/rfc-10-skills.md), [RFC-13](done/rfc-13-extensibility.md), [RFC-28](next/rfc-28-codex-rules.md), [RFC-30](next/rfc-30-init.md), [roadmap RM-16 / RM-07](roadmap.md), and [docs/contributing/checks.md](../docs/contributing/checks.md). Module paths in prose become `check::codex` rather than `framework-rules::codex`. No rename is required in `specify-cli` — the operator binary is unchanged.

### Schema-first layer (do this first)

Most checks in `scripts/checks/` enforce shapes that JSON Schema can express. The earliest, highest-leverage work is to make sure every such shape **is** a schema, and that Cursor sees it where the active language service can bind the schema directly.

Plain YAML and JSON files get inline diagnostics through Cursor's built-in language servers. Markdown-frontmatter files (`SKILL.md`, codex rules, scenario docs) still use JSON Schema as the canonical shape, but a local Cursor proof spike showed the YAML language service validates a matching `.yaml` control file while not reporting diagnostics for the same invalid schema fields inside Markdown frontmatter. Until a frontmatter-aware editor integration or a future `lsp` exists, `tooling check` owns Markdown-frontmatter enforcement by extracting the leading `---` block and validating it with the same schema.

Concrete moves:

- **Authoritative location.** Runtime schemas consumed by `specify-cli` stay in `specify-cli/schemas/` and are reused through `specify-domain`. Framework-only schemas (skill frontmatter, codex rule authoring, scenario metadata, marketplace manifests) stay in `augentic/specify/tooling/schemas/`; `.cursor/schemas/` contains editor-facing symlinks or aliases only when Cursor needs them.
- **Editor wiring.** Workspace settings or per-file `# yaml-language-server: $schema=` directives point plain YAML/JSON files (`adapter.yaml`, marketplace manifests, scenario YAML when present, target-owned YAML artifacts, and `tools.yaml` during migration) at their schemas. The YAML/JSON LSPs Cursor already ships then surface those violations live, with no extra tooling installed.
- **Markdown-frontmatter enforcement.** `SKILL.md`, codex rules, and scenario Markdown files still declare and share schemas, but `tooling check` remains the enforcement surface for their frontmatter unless a future editor integration proves reliable inline diagnostics for Markdown frontmatter.
- **Schema strengthening.** Rules currently enforced imperatively in `skill_frontmatter.ts` (description grammar, argument-hint shape, 200/45/512 caps on counted fields) are expressed as `pattern`, `maxLength`, and `enum` constraints where they fit. The minority that genuinely cannot be schema'd (variable consistency, cross-skill directive resolution, body-section discipline) stays in the `check` modules.
- **Documentation.** `docs/contributing/checks.md` gets a new section explaining the split: plain YAML/JSON shape violations appear as editor diagnostics, while Markdown-frontmatter and cross-file rules are caught by `tooling check`.

This step delivers contributor value early inside the atomic implementation branch and gives the Rust rule engine canonical schemas to reuse for Markdown frontmatter.

### `check` modules

One module per predicate group, each exposing predicates that return structured findings:

```rust
pub struct Finding {
    pub rule_id: &'static str,         // stable kebab-case id
    pub message: String,               // human-readable diagnostic
    pub location: Option<Location>,    // file + 1-based line + optional column
}

pub trait Check {
    fn id(&self) -> &'static str;
    fn run(&self, ctx: &Context) -> Vec<Finding>;
}
```

`Context` carries the resolved framework root (never the `tooling/` directory alone), a `specify-domain` adapter resolver where needed, and lazily-loaded schemas from `tooling/schemas/`. Predicates run sequentially — the Deno scripts are already fast enough today, and the Rust port will be faster still over a 4K-LOC repo. Every invocation runs a full repo scan; there is no `--changed` mode, and predicates are written under that assumption.

Rule ids align with RFC-28's reserved namespaces where applicable (the codex namespace-ownership rule lives here as `codex.namespace-ownership-violation` and feeds the future shared finding shape).

**RFC-28 interlock.** Rule ids minted by `check::codex` follow RFC-28's namespace ownership and id-stability rules from the first ported predicate. Other modules' ids may evolve until structured-output fixtures (deferred to a follow-on RFC alongside `--format json`) pin them; codex ids are fixed from day one.

### `tooling` binary

One clap root with subcommands. Day-to-day callers at the framework root use `make` targets instead of invoking Cargo directly — see §*Makefile entry points*.

#### `tooling check`

A thin dispatcher over the `check` modules. Operates on the current working directory; `make check` runs it from the framework root.

```bash
# day-to-day (framework root)
make check

# tooling contributors (direct Cargo, optional)
cargo run --manifest-path tooling/Cargo.toml -- check
```

Exit codes follow the standard table inherited from `specify-cli`:

- `0` — success.
- `2` — validation findings or argument errors.
- `1` — infrastructure errors (I/O, schema load failures).

Every invocation runs a full repo scan. The Rust port is fast enough that scoping by changed paths is unnecessary at this repo's size, and a single code path keeps local and CI behaviour identical — no per-rule classification of which predicates are safe to restrict, no risk of silently missing global invariants (marketplace consistency, codex namespace ownership, duplicate ids, symlink integrity).

#### `tooling docgen`

Subcommand that ports `scripts/gen-envelope-doc.ts`.

```bash
cargo run --manifest-path tooling/Cargo.toml -- docgen envelopes          # regenerate docs/reference/cli-output-shapes.md
cargo run --manifest-path tooling/Cargo.toml --release -- docgen envelopes --check  # CI mode: diff and exit 2 on drift
```

Same generated-block markers (`<!-- generated:begin -->` / `<!-- generated:end -->`), same explicit fixture-to-section mapping table, same sibling-checkout discovery semantics via `SPECIFY_CLI_DIR`.

### Acceptance tests

Integration tests live under `tooling/tests/` and port `tests/cross_repo.ts` and its `tests/lib/` helpers directly. They:

- Use `specify-domain` for provenance parsing (kills `tests/lib/spec_provenance.ts`).
- Use the same JSON Schema validators as the `check` modules (kills `tests/lib/validators.ts`).
- Keep the optional `SPECIFY_BIN` subprocess tests for `specify source resolve` and `specify target resolve`; skip cleanly when the binary is absent (matches today's harness).
- Adopt `specify-cli`'s `REGENERATE_GOLDENS=1` discipline for any byte-stable goldens they assert.

Test-binary names mirror the existing Deno suites (`sources`, `targets`, `skills_refine`, `skills_loop`) so `cargo test --manifest-path tooling/Cargo.toml --test <name>` is easy. Fixture paths resolve from the framework root (`tests/fixtures/`), not from inside `tooling/`.

Manual scenario packs under `tests/cross-repo/` and `tests/plan/` continue to run via the `gh` recipe documented in [docs/contributing/acceptance.md](../docs/contributing/acceptance.md). Nothing under `tooling/` invokes them automatically. The implementation PR that deletes `tests/cross_repo.ts` updates that doc in the same change so the manual harness retains a documented entry point.

### Out-of-scope follow-ons

`--format json` output, PR annotations, a custom `lsp`, WASI rule packs, a pre-commit hook, and a `--changed` scoping mode are all deliberate non-goals for this RFC. The `Check` / `Finding` shape is JSON-friendly so any of them can be added later without re-architecting. A future composite GitHub Action that runs `tooling check --format json` and posts annotations is the natural place for the structured-output envelope to land.

### Makefile entry points

Framework tooling must **disappear into the background** for day-to-day skill and adapter work. No contributor should need to remember Cargo flags or know where `target/` lives unless they are editing the tooling crate itself.

#### Contract


| Audience                          | What they run                                                                      | Rust required locally?                     |
| --------------------------------- | ---------------------------------------------------------------------------------- | ------------------------------------------ |
| Skill / adapter authors (default) | Cursor schemas while editing where available; `make check` before a PR; CI on push | Optional — schemas and CI cover most needs |
| Tooling contributors              | `cd tooling && cargo test`, `cargo run -- check`, …                                | Yes                                        |


Authors never run `cargo build` as a separate step before `make check`. Cargo's incremental build keeps repeat invocations fast.

#### Makefile

`make check` invokes the binary via `cargo run --release`. The Makefile keeps only the commands contributors actually run:

```makefile
TOOLING_MANIFEST := tooling/Cargo.toml

.PHONY: check test ci

check:
	cargo run --release --manifest-path $(TOOLING_MANIFEST) -- check

test:
	cargo test --manifest-path $(TOOLING_MANIFEST)

ci: check test
```

Inside the implementation branch, `make check` may temporarily call both `scripts/check.ts` for unported predicates and `tooling check` for ported ones — each rule lives in exactly one of the two surfaces at any commit. Before the PR opens for review, every predicate has moved to Rust and `scripts/check.ts` is deleted.

#### CI

```yaml
- uses: dtolnay/rust-toolchain@stable
- uses: Swatinem/rust-cache@v2
- run: cargo test --manifest-path tooling/Cargo.toml
  env:
    SPECIFY_CLI_DIR: specify-cli
- run: cargo run --release --manifest-path tooling/Cargo.toml -- check
- run: cargo run --release --manifest-path tooling/Cargo.toml -- docgen envelopes --check
  env:
    SPECIFY_CLI_DIR: specify-cli
```

`Swatinem/rust-cache@v2` is included from day one rather than added after the first slow PR — a cold Cargo build of `tooling` plus transitive `specify-domain` deps can run 1–3 minutes per job. `tooling/target/` is never committed.

#### Contributor documentation

`docs/contributing/index.md` and `docs/contributing/checks.md` describe the split explicitly:

- **Editor-first where available** — plain YAML/JSON violations surface as schema squigglies; Markdown frontmatter and cross-file rules surface through `tooling check`.
- **Local gate** — `make check` runs `tooling check`; first run after clone compiles once, subsequent runs reuse Cargo's cache until tooling sources change.
- **CI gate** — authoritative full scan on every PR; sufficient on its own for contributors who skip local Rust.

Rust appears under prerequisites only for contributors who run `make check` locally, not as a blanket requirement for every markdown edit.

#### Explicitly out of scope

- **Committed prebuilt binaries** — platform-specific churn; Cargo incremental build is enough.
- `cargo install tooling` — adds global install/update ceremony; repo-local invocations via `make` are simpler.
- **Auto-build on every file save** — cross-file rules are too heavy; editor schemas cover save-time feedback. A future `lsp` would reuse the `check` modules without changing this contract.

### Implementation outline

The RFC lands as a **single PR** in this rough order:

1. **Schema-first pass and cross-RFC rename sweep.** Strengthen framework-only schemas under `tooling/schemas/`, wire Cursor `$schema` references for plain YAML/JSON files, and update every cross-reference that still says `framework-rules`, `framework-check`, `framework-lsp`, or separate `check`/`docgen` binaries across the RFCs and docs listed in §Naming.
2. **Scaffold and port checks.** Land `tooling/Cargo.toml`, the `tooling` binary with a `check` subcommand, and the `make check` target. Move each predicate from `scripts/checks/` into `src/check/`, deleting the matching Deno file as it lands. Start with the schema/parser-duplication wins (adapter manifest, brief discipline, skill frontmatter/body), then proceed through `agent_teams`, `prose`, `links`, `plugins`, `docs_quality`, `codex`, `scenarios`, and `tools` in dependency order. Add fixtures alongside each module.
3. **Port `docgen` and acceptance.** Add the `docgen` subcommand and the `tooling/tests/` integration tests; replace parser duplication by using `specify-domain` and shared validators directly. Delete `scripts/gen-envelope-doc.ts`, `tests/cross_repo.ts`, and `tests/lib/` in the same step.
4. **CI cleanup.** Drop `denoland/setup-deno` from `.github/workflows/ci.yaml`, switch `Makefile` to the Cargo-backed targets above, and update `docs/contributing/index.md` with the audience split and optional-Rust prerequisites.

### Coordination with other RFCs

- **RFC-4 (typed skill expression).** Option 1 (framework-tooling skill validation) is satisfied by the schema-first pass plus the `skill_frontmatter` / `skill_body` modules. The "CLI" in that RFC is reinterpreted as `tooling check`, not `specify check`. Options 2 and 3 are unchanged.
- **RFC-28 (codex resolution).** RFC-28 cites RFC-5 for namespace-ownership enforcement. That contract is preserved verbatim — the rule moves to `check::codex` and continues to enforce that first-party files do not use `ORG-`*. Where the rule lives (this repo, not specify-cli) is invisible to RFC-28's resolver and finding shape.
- **Roadmap RM-16.** The roadmap entry tracks this RFC's framework dev-tooling crate: schema-first authoring feedback, `tooling check`, acceptance tests, `tooling docgen`, and Deno retirement. The unblocks line (RFC-4 Option 1, declared-WASI-tool helper migration) is unchanged.

## Alternatives Considered

**The original RFC-5: port into `specify-cli` as `specify check`.** Rejected because it puts framework dev tooling on the operator product. Operators running Specify on a consumer project never need to validate `plugins/` or `.cursor-plugin/marketplace.json`; bundling that surface bloats the install for everyone to serve the few. The parser-reuse argument that motivated the original choice is satisfied just as well by depending on `specify-domain` as a library from a sibling workspace, which is the standard Rust pattern.

**One binary per Deno script (`check`, `docgen`).** Rejected because the binaries are thin clap shells, share one dependency story, and differ only by subcommand. A single `tooling` binary with `check` and `docgen` subcommands matches the `specify` operator pattern, keeps the Makefile simple, and keeps acceptance on `cargo test` where it belongs.

**A multi-crate workspace (`rules` library, `docgen` library, `accept` test crate, `tooling` binary).** Considered to allow a future `lsp` to consume the rule library, with `accept` exposed as a sibling integration crate. Rejected as YAGNI: with one current consumer (the binary), one rule pack, and `lsp` itself deferred indefinitely, splitting into four crates costs more in coordination today than it saves later. Splitting `src/check/` into a library crate when an `lsp` actually materializes is a small refactor — the architectural commitment is paid by the modular shape inside the single crate.

**Keep `scripts/check.ts` indefinitely.** Tempting because the script works and is not blocking. Rejected because (a) `tests/lib/spec_provenance.ts` and `tests/lib/validators.ts` actively duplicate `specify-domain`, and that duplication grows with every new schema; (b) schemas without an editor or `tooling check` contract are invisible to contributors; (c) keeping Deno in CI forever to validate Rust-defined schemas is a coordination tax with no offsetting benefit.

**Merge the repos.** Considered: one workspace with `cli/` and `plugins/` top-level directories would kill the cross-repo coordination entirely. Rejected because it conflates two audiences (Rust contributors vs skill/adapter authors), couples operator-CLI release cadence to plugin-content cadence, and forces a single review style on both halves. The dev-tooling problem does not require it.

**Rust crate at the framework repo root.** Considered: colocate `Cargo.toml` and `src/` beside `plugins/` and `adapters/`. Rejected because it puts Rust contributor surface in the same directory tree skill and adapter authors browse daily — `Cargo.toml` beside `plugins/` reads like consumer-project scaffolding. Nesting the crate under `tooling/` preserves the audience split the RFC already applies against `specify-cli`, without merging the two repos.

**Self-hosted Specify (the framework repo as a Specify project).** Considered as the long-term aspiration: every plugin and adapter as a slice, framework consistency from `specify slice validate`. Rejected as the *only* solution because mechanical checks still need a deterministic engine underneath, and the framework lifecycle (RFC editing, README polish, contributor docs) does not naturally fit slices. Compatible with this RFC: a self-hosted layer can sit on top of the `check` modules later.

**WASI rules as the day-one shape.** Considered: ship `tooling check` as a WASI module declared via an adapter manifest and invoked through `specify tool run`. Reuses the Vectis pattern and opens third-party rule packs immediately. Rejected as overkill when there is exactly one rule pack and one consumer (CI); the module shape adopted here makes future WASI exposure a small refactor, not a re-architecture.

**Rewrite from scratch (no rule preservation).** Rejected because the current invariants — what each predicate actually checks — encode real lessons about repo drift, and dropping them on the floor would silently regress coverage. Diagnostic *wording* is not preserved (the Rust port may reword freely); the *rules themselves* are. Fixtures, not message strings, are how each port proves it covers the same ground as the Deno predicate it replaces.

**Committed prebuilt binaries or global `cargo install`.** Considered so skill authors could skip a local Rust toolchain entirely. Rejected because checked-in platform binaries add release churn and security review overhead, and a global install adds update ceremony outside the repo. The adopted split — editor schemas + CI for everyone, optional local `make check` — keeps Rust optional without shipping artifacts.

**Day-one `--format json` and PR annotations.** Considered to give CI a structured envelope from the first commit. Rejected because the only consumer (a composite GitHub Action) is itself a future RFC; pinning the envelope shape before that consumer exists is premature. The `Finding` type stays JSON-friendly so adoption is additive.

**A pinned `rust-toolchain.toml`.** Considered to mirror `specify-cli`. Rejected because the workspace is dev tooling, not a release artifact; `Cargo.lock` is the lockfile of record and stable suffices. A pinned toolchain adds bump churn without protecting anything.

## References

- `[scripts/check.ts](../scripts/check.ts)` + `[scripts/checks/](../scripts/checks/)` — the framework linter being replaced.
- `[scripts/gen-envelope-doc.ts](../scripts/gen-envelope-doc.ts)` — the doc generator being replaced.
- `[tests/cross_repo.ts](../tests/cross_repo.ts)` + `[tests/lib/](../tests/lib/)` — the acceptance harness being replaced.
- `[docs/standards/skill-authoring.md](../docs/standards/skill-authoring.md)` — invariants the skill-discipline modules enforce.
- `[docs/explanation/adapter-anatomy.md](../docs/explanation/adapter-anatomy.md)` — the adapter model the `adapter` and `brief` modules validate.
- `[docs/contributing/acceptance.md](../docs/contributing/acceptance.md)` — the acceptance surface split between deterministic harness (this RFC) and manual scenario packs (out of scope).
- `[Specify CLI AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md)` — crate graph this workspace consumes via `specify-domain`.
- [RFC-4: Type-Safe Skill Expression](future/rfc-4-dsl.md) — Option 1 is satisfied by the schema-first pass plus the skill-discipline modules.
- [RFC-28: Codex Resolution and Structured Review Findings](../rfc-28-codex-rules.md) — namespace-ownership contract preserved by `check::codex`.
- [RFC-31: WorkspaceModel and Declarative Rule Execution](../rfc-31-workspace-model.md) — optional Phase 3 framework convergence; does not replace RFC-5 check modules by default.

