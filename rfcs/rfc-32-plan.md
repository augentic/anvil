# RFC-32 Implementation Plan

> Source: [RFC-32: Engineering Standards — Deterministic Enforcement](rfc-32-standards-enforcement.md) · Scope: Phase 2 (required) only — Phase 3a is owned by RFC-28 Phase 3 (already shipped) and Phase 3b is carved out into [RFC-34](rfc-34-framework-convergence.md).

## Purpose

Break RFC-32 Phase 2 into sub-agent-sized changes, sequence them by dependency, and call out which can be executed in parallel.

Each row in the change table is intended to be picked up by a single sub-agent with a fresh context window. Slices are sized so that the brief, the source files in scope, and the relevant tests fit comfortably inside one context.

## Repository scope

Work spans two repos. The split matters because cross-repo `rg` and the workflow-contract rule in `specify-cli/AGENTS.md` §"When working in this repo" both apply.

| Repo | Slices |
| --- | --- |
| [`augentic/specify-cli`](https://github.com/augentic/specify-cli) | S1, S2, S3, S4, S5, S6, S7, S8, S9, S12 |
| [`augentic/specify`](https://github.com/augentic/specify) (this repo) | S10, S11, S13 |

## Dependency graph

```text
                       S1 (specify-schema crate)
                      /    \
                     /      \
                    v        v
              S2 (specify-   S4 (review schemas
              codex crate)    + reserved hint kinds)
                   |  \      /
                   |   \    /
                   v    v  v
              S3 (drop    S5 (WorkspaceModel DTOs
              vendored      + crate scaffold)
              schema)              |
                                   v
                              S6 (consumer indexer §D1)
                                   |
                                   v
                              S7 (hint interpreter ×4 kinds, §D3–D5)
                                   |
              S8 (formatters ×4) <-+-> S9 (`specrun review` CLI, §D2/D6/D7/D8/D9)
                          (parallel after S5)              |
                                                           v
                              +----------+-----------+-----+
                              |          |           |     |
                              v          v           v     v
                            S10        S11         S12   S13
                          (seed       (review     (cli   (specify
                          hints)      briefs)     docs)   docs)
```

S1+S2+S3 must land as a single PR per RFC-32 §"Eliminates the vendored codex-rule schema" ("leaving the vendoring in place after the split would ship a confused intermediate state on `main`"). They are split into three sub-agent slices for context-size reasons; the slices are then assembled into one PR before merge. Every other slice is its own PR.

## Sequenced changes

### S1 — `specify-schema` leaf crate

**Repo:** `specify-cli` · **Depends on:** none · **Parallel with:** — (foundational) · **Co-merges with:** S2, S3

Extract every embedded JSON Schema constant and the JSON-Schema plumbing out of `crates/domain/src/schema.rs` into a new leaf crate per RFC-32 §"Library layout".

- Create `crates/schema/{Cargo.toml,src/lib.rs,src/constants.rs,src/validate.rs}`.
- Move constants: `PLAN_JSON_SCHEMA`, `EVIDENCE_JSON_SCHEMA`, `FUSION_JSON_SCHEMA`, `COMPONENTS_JSON_SCHEMA`, `CODEX_RULE_JSON_SCHEMA`, `RESOLVED_CODEX_JSON_SCHEMA`, `REVIEW_FINDING_JSON_SCHEMA`.
- Move helpers: `compile_schema`, `validate_value`, `validate_serialisable`, `validation_error_detail`, `child_pointer`, `read_yaml_as_json`.
- Add `specify-schema.workspace = true` to `specify-domain/Cargo.toml`; update `crates/domain/src/schema.rs` to re-export or delegate during the transition.
- Workspace `Cargo.toml`: register the new crate; pin only existing workspace deps (`specify-error`, `jsonschema`, `serde`, `serde_json`, `serde-saphyr`).
- Port the schema-compile smoke tests from `crates/domain/src/schema.rs::tests` to `crates/schema/tests/` (one `#[test]` per embedded constant; RFC-28 example round-trip tests stay with their domain owner).
- `cargo make ci` green.

**Done when:** `specify-domain` compiles against `specify-schema` re-exports with no behavioural change.

---

### S2 — `specify-codex` crate (relocate codex module)

**Repo:** `specify-cli` · **Depends on:** S1 · **Parallel with:** S4 · **Co-merges with:** S1, S3

Carve `crates/domain/src/codex/` and `crates/domain/src/codex.rs` out of `specify-domain` into a sibling `specify-codex` crate per RFC-32 §"Library layout".

- Create `crates/codex/{Cargo.toml,src/lib.rs,src/codex.rs}`.
- Relocate `parse.rs`, `resolve.rs`, `resolve/{filter,sort}.rs`, `finding.rs`, `fingerprint.rs` to `crates/codex/src/codex/`.
- `specify-codex/Cargo.toml` depends on `specify-error`, `specify-tool`, `specify-schema` (no `specify-domain` — the standards-vs-workflow separation is a type-system invariant per §"Library layout").
- `specify-domain` drops its `codex` module and the `pub use` re-exports; downstream consumers in the root `specify` binary import from `specify_codex::codex::*`.
- Migrate codex-targeted integration tests (`crates/domain/tests/codex_*.rs`) to `crates/codex/tests/`.
- Update workspace `Cargo.toml` and the root `specify` binary `Cargo.toml`.
- `cargo make ci` green.

**Done when:** the codex resolver, parser, fingerprint, and finding validator are accessed exclusively through `specify-codex`.

---

### S3 — Drop vendored codex-rule schema

**Repo:** `specify-cli` · **Depends on:** S1, S2 · **Parallel with:** — · **Co-merges with:** S1, S2

Execute RFC-32 §"Eliminates the vendored codex-rule schema" in full.

- Add `specify-codex.workspace = true` to `crates/authoring/Cargo.toml`.
- Replace local schema reads in `crates/authoring/src/check/` with `specify_schema::CODEX_RULE_JSON_SCHEMA` (typed `CodexRule` DTO via `specify_codex`).
- Delete:
  - `crates/authoring/schemas/codex-rule.schema.json`
  - `scripts/sync-codex-schema.sh`
  - `crates/authoring/src/check/codex_schema_drift.rs`
  - `crates/authoring/tests/check_codex_schema_drift.rs`
  - The `codex.schema-drift` (CH-09) rule-id registration in the check registry
- Fix the `$id` URL on `specify-cli/schemas/codex/codex-rule.schema.json` to match its on-disk location (today it points at an aspirational `schemas/authoring/` path).
- Update `crates/authoring/README` or module docs that mention `codex.schema-drift` or the sync script.
- `cargo make ci` green; `make check` in the framework repo still green.

**Done when:** `rg codex.schema-drift` and `rg sync-codex-schema` across both repos return no hits.

---

### S4 — Review v1 schemas + reserved hint kinds

**Repo:** `specify-cli` · **Depends on:** S1 · **Parallel with:** S2, S3

Add the two new JSON Schemas RFC-32 §D9 and §"Schema location" introduce, and extend the codex authoring schema with the reserved hint-kind enum.

- New files:
  - `specify-cli/schemas/review/workspace-model.schema.json` (v1; documents the entity families from §"Core entity families (v1)"; reserves `.specify/cache/workspace-model.v1.json` and `specrun model query <selector>` as documented "reserved" surfaces per §"Persistence and query (v1 decision)")
  - `specify-cli/schemas/review/review-result.schema.json` (per §D9; `$ref`s `schemas/review/finding.schema.json`; `version: 1` discriminant)
- Embed both in `specify_schema::constants` (`WORKSPACE_MODEL_JSON_SCHEMA`, `REVIEW_RESULT_JSON_SCHEMA`).
- Schema-compile smoke tests in `crates/schema/tests/`.
- Extend the codex authoring schema's `deterministic_hints[].kind` enum with the reserved kinds from §"Hint kinds — reserved" (`unique`, `reference-resolves`, `set-coverage`, `cardinality`, `constant-eq`, `set-eq`, `content-digest-eq`, `namespace-owner`); each carries `"x-rfc32-status": "reserved"` matching the RFC-28 annotation pattern.
- Validate at least one fixture rule per reserved kind round-trips through the schema.

**Done when:** the three new schemas embed cleanly and the codex authoring schema accepts files declaring reserved hint kinds without execution.

---

### S5 — WorkspaceModel DTOs + `review` module scaffold

**Repo:** `specify-cli` · **Depends on:** S2, S4 · **Parallel with:** —

Land the standalone DTO layer for WorkspaceModel and the empty umbrellas for index/eval/diagnostics so subsequent slices have stable module paths to fill in.

- Add to `crates/codex/src/`:
  - `review.rs` (umbrella)
  - `review/model.rs` — DTOs for `file`, `frontmatter`, `markdown_section`, `markdown_link`, `symlink`, `skill`, `adapter_manifest`, `marketplace_entry`, `codex_rule`, `text_match`; `WorkspaceModel { version: 1, project_dir, scan_profile, … }` envelope.
  - Empty `review/index.rs`, `review/eval.rs`, `review/diagnostics.rs` umbrellas (each just a doc comment + `pub mod` lines that subsequent slices fill in).
- Add `rayon` and `ignore` to `specify-codex/Cargo.toml`. Both are net-new to the workspace; defer pinning to PR review.
- Round-trip serde test: fixture `WorkspaceModel` instance ↔ JSON ↔ schema validation against `WORKSPACE_MODEL_JSON_SCHEMA`.
- `cargo make ci` green.

**Done when:** the `review::model` API compiles in isolation and `WorkspaceModel { version: 1 }` round-trips through the schema.

---

### S6 — Consumer indexer (`scan_profile: consumer`)

**Repo:** `specify-cli` · **Depends on:** S5 · **Parallel with:** S8

Implement the §D1 consumer scan: file walk + every per-file extractor + the byte-stable assembly pass.

- Files under `crates/codex/src/review/index/`:
  - `files.rs` — `.gitignore`-aware walk via the `ignore` crate; default include/ignore globs from §D1; binary-file detection (NUL byte in first 8 KiB); UTF-8 with U+FFFD replacement + one `index.warning` per non-UTF-8 file.
  - `frontmatter.rs` — markdown `---` block extraction + YAML parse.
  - `markdown.rs` — fence-aware section and link scan.
  - `symlinks.rs` — record `{ path, target, broken }`; do not traverse.
  - `codex.rs` — reuses `crate::codex::parse` over consumer overlays.
- `review/index.rs` umbrella exposes `build(project_dir, scan_profile, artifact_paths, languages) -> Result<WorkspaceModel>`. Per-file extractors run in parallel via `rayon`; cross-file edges + codex discovery run sequentially after the parallel pass; final collection sort per §"Stability".
- Golden fixtures under `crates/codex/tests/fixtures/review/minimal/`: cover ignore-glob coverage, a symlink, a binary, a non-UTF-8 file, a markdown file with frontmatter + links + a fenced block.
- Integration tests:
  - `crates/codex/tests/review_indexer_consumer.rs` — full-scan golden against `minimal/`; byte-stable output across runs.
- Honour `REGENERATE_GOLDENS` per `docs/standards/testing.md`.

**Done when:** `WorkspaceModel { version: 1 }` builds reproducibly from `minimal/` with the entity families §D1 prescribes.

---

### S7 — Hint interpreter (Phase 2 kinds)

**Repo:** `specify-cli` · **Depends on:** S6 · **Parallel with:** S8

Implement the four executable hint kinds plus the runner, with reserved-kind policy.

- Files under `crates/codex/src/review/eval/`:
  - `path_pattern.rs` (§"Evaluation algorithm" — must run first; builds the candidate file set)
  - `regex.rs` (raw file bytes; skips `file.kind == "binary"`)
  - `schema.rs` (§D3 — registered-id and project-relative `$ref` value shapes; rejects `http(s)://`; supports `target: frontmatter`)
  - `tool.rs` (§D4 — declared WASI tools only; closed `{artifact}`, `{project_dir}`, `{rule_id}` placeholders; `tool.invocation-failed` / `tool.undeclared` mapping)
- `review/eval.rs` umbrella exposes `evaluate(rule, hints, model, project_dir)` returning `Result<Vec<ReviewFinding>, HintError>`. Ordering: `path-pattern → schema → regex → tool` per §"Evaluation algorithm".
- Reserved-kind policy §D5: emit exactly one `review.reserved-hint-skipped` summary finding per scan (default severity `optional`; upgraded to `important` under `--strict-hints`); same rule-id across strict/non-strict.
- Closed `HintError` enum (`thiserror`) mapping to the §D8 exit-code table at the handler boundary.
- One golden per kind: `crates/codex/tests/review_hint_{path_pattern,regex,schema,tool}.rs` against `fixtures/review/minimal/`.
- The `tool` fixture exercises the `contract` WASI tool (already declared) per §D4 / Phase 2 step 3.

**Done when:** every Phase 2 kind emits stable `ReviewFinding[]` JSON for the minimal fixture and reserved-kind hints surface exactly one summary finding.

---

### S8 — Diagnostic formatters (×4)

**Repo:** `specify-cli` · **Depends on:** S5 · **Parallel with:** S6, S7

Implement the four formatters RFC-32 §D6 names as the closed Phase 2 set. All four ship together — there is no JSON-only intermediate.

- Files under `crates/codex/src/review/diagnostics/`:
  - `json.rs` — RFC-28 wire envelope; validates against `REVIEW_RESULT_JSON_SCHEMA` before emit (§D9).
  - `pretty.rs` — terminal output with severity colour and source location.
  - `github.rs` — `::error file=…,line=…,title=…::…` workflow-annotation format.
  - `compact.rs` — one finding per line; grep- and PR-bot-friendly.
- `review/diagnostics.rs` umbrella exposes a `render(format, ReviewResult) -> String` entry point used by `specrun review` and (later) `specdev check --format json` for Option A reuse.
- Per-formatter golden test under `crates/codex/tests/` against a shared minimal `ReviewResult` fixture.

**Done when:** all four formatters produce stable output and the `json` formatter refuses to emit envelopes that fail `REVIEW_RESULT_JSON_SCHEMA`.

---

### S9 — `specrun review` CLI

**Repo:** `specify-cli` · **Depends on:** S7, S8 · **Parallel with:** —

Wire the Phase 2 CLI surface per §"`specrun review` (Phase 2 CLI)" and §D2 / §D6 / §D7 / §D8 / §D9.

- Files:
  - `src/runtime/commands/review.rs` (umbrella)
  - `src/runtime/commands/review/cli.rs` (`ReviewAction` clap-derive subcommand — match the Implementation Guide sketch)
  - `src/runtime/commands/review/run.rs` (handler: `specrun codex export` → `review::index::build` → `review::eval::evaluate` → `review::diagnostics::render`)
- `--codex-root` resolution per §D7 (flag → `$CODEX_ROOT` → `.specify/cache/codex/` → bundled tree); resolution failure is `Error::Validation` exit 2 with a hint pointing at `specrun init` / `--codex-root`.
- `--slice` / `--artifact` semantics per §D2; pass the same filters to `specrun codex export`.
- `--dump-model` emits only the WorkspaceModel (schema-validated against `WORKSPACE_MODEL_JSON_SCHEMA` before stdout).
- `--strict-hints` upgrades the §D5 summary finding to `important`.
- `--format` accepts the closed `{ json, pretty, github, compact }` set; default is `pretty`.
- Exit codes per §D8 (`Exit::from(&Error)`).
- Register `Commands::Review` in `src/runtime/cli.rs`; route through `src/runtime/output.rs` for envelope emission.
- End-to-end binary test `tests/review_run.rs`: invokes the binary against `fixtures/review/minimal/`, asserts stable JSON envelope, stable exit code, and `--dump-model` round-trips through `WORKSPACE_MODEL_JSON_SCHEMA`.
- `cargo make ci` green.

**Done when:** `specrun review --target <name> --format json` produces a stable RFC-28 review envelope on the minimal fixture and exit codes match §D8.

---

### S10 — Seed deterministic hints (UNI-014 + one target rule)

**Repo:** `specify` · **Depends on:** S9 · **Parallel with:** S11, S12, S13

Execute RFC-32 Phase 2 step 6 ("Seed policy"). Without this the scanner ships but emits zero findings on real projects.

- Add `deterministic_hints` to at least one shared `UNI-*` rule (Phase 2 spec names `UNI-014` URL-in-generated-code via `kind: regex`). File lives under `adapters/shared/codex/universal/`.
- Add a target-namespaced rule with at least one hint — pick a small Omnia or contracts rule whose existing prose maps cleanly to `kind: regex` or `kind: schema`.
- Acceptance fixture: a minimal consumer project tree whose contents trigger both hints; assert non-empty `findings` from `specrun review`.

**Done when:** `specrun review` against the seed fixture produces ≥ 1 finding per seeded rule with stable fingerprints.

---

### S11 — Update target review briefs

**Repo:** `specify` · **Depends on:** S9 · **Parallel with:** S10, S12, S13

Execute RFC-32 Phase 2 step 7. Briefs continue to own model-assisted judgment; deterministic findings sit alongside `REVIEW.md`.

- Update `adapters/targets/omnia/briefs/build.md` (and any other target adapter that emits a `REVIEW.md`) to:
  - Reference `specrun review --format json` output as the deterministic complement to the human review.
  - Note that deterministic findings may block CI but never transition lifecycle (§"Principles", §"No lifecycle authority in review").
- Touch only the brief prose. Do not modify the universal review-team protocol — the symlink boundary stands.

**Done when:** every target adapter brief that references review acknowledges the deterministic surface.

---

### S12 — Documentation touch-points (`specify-cli`)

**Repo:** `specify-cli` · **Depends on:** S9 · **Parallel with:** S10, S11, S13

Execute the `specify-cli` half of RFC-32 §"Documentation touch-points (post-merge)".

- `AGENTS.md`:
  - Replace the `crates/domain/src/codex/` "Modules of note" row with one row each for `crates/codex/src/codex/` and `crates/codex/src/review/`.
  - Add a row for `crates/schema/`.
  - Update §"Crate graph" to show `specify-schema` as a leaf and `specify-codex` as a sibling of `specify-domain`.
  - Update §"When working in this repo" cross-repo `rg` rule to name `crates/codex/src/codex/` and `crates/codex/src/review/`.
  - Add a documentation-map row pointing at RFC-32.
- `docs/standards/architecture.md` — extend workspace-layout section with `specify-codex` and `specify-schema`; add the crate-graph diagram from RFC-32 §"Library layout"; note the standards-layer-vs-workflow split.
- `DECISIONS.md` — record (a) the standards-layer split (new `specify-codex` and `specify-schema` crates, sibling shape, no workflow→standards dependency); (b) the vendored-codex-rule-schema removal.
- `crates/authoring/` README or module docs — note the new `specify-codex` dependency; remove prose mentioning `codex.schema-drift` or the sync script.

**Done when:** `rg specify-codex AGENTS.md docs/ DECISIONS.md` covers every new public surface.

---

### S13 — Documentation touch-points (`specify`)

**Repo:** `specify` · **Depends on:** S9 · **Parallel with:** S10, S11, S12

Execute the framework-repo half of RFC-32 §"Documentation touch-points (post-merge)".

- `docs/contributing/checks.md` — note `specrun review` as the consumer-project counterpart to `specdev check --format json`; remove the `codex.schema-drift` (CH-09) entry.
- `docs/explanation/standards-layer.md` — replace references to "shared codex parser in `specify-domain`" with `specify-codex`; document the new crate split as the type-system enforcement of the "no lifecycle authority in review" rule.
- `make check` green after the doc edits (the framework-repo `specdev check` enforces doc consistency per the always-applied workspace rules).

**Done when:** the docs no longer refer to `specify-domain` as the codex parser owner and `codex.schema-drift` is gone from contributor-facing prose.

---

## Sub-agent parallelism summary

| Wave | Runs in parallel | Notes |
| --- | --- | --- |
| 1 | S1 (alone) | Foundational; everything else depends on it. |
| 2 | S2, S4 | Both depend only on S1. |
| 3 | S3 | Bundles with S1+S2 into one PR; develop in this wave. |
| 4 | S5 (alone) | Depends on S2 and S4. |
| 5 | S6, S8 | S8 only needs the DTOs from S5; S7 wants S6 done first. |
| 6 | S7 (alone) | Depends on S6. |
| 7 | S9 (alone) | Depends on S7 and S8. |
| 8 | S10, S11, S12, S13 | All depend on S9, all touch independent surfaces. |

The critical path is S1 → S2 → S5 → S6 → S7 → S9 → (any of S10/S11/S12/S13). Wave-5 parallelism (S6 + S8) and wave-8 parallelism (4-way) are the only places where doubling sub-agent count shortens wall time meaningfully.

## Co-merge gate (Slices 1+2+3)

Per RFC-32 §"Eliminates the vendored codex-rule schema", the standards-layer split and the vendored-schema removal must land in the same PR. The mechanics are:

- Each of S1, S2, S3 develops on its own topic branch.
- A coordinator (parent agent or human) assembles the three branches into a single integration branch in dependency order (S1 → S2 → S3) before opening the PR.
- `cargo make ci` runs on the integrated branch; any rebase work is the coordinator's, not the slice sub-agents'.

Every other slice opens its own PR.

## Acceptance gate (RFC-32 Phase 2 step 5)

Acceptance test coverage is spread across the slices rather than carved out as a separate slice. The full §"Acceptance" checklist resolves as follows:

| Acceptance item | Lands in |
| --- | --- |
| Golden tests: resolved rules + sample crate tree → stable findings JSON | S6 + S7 + S9 |
| Fingerprint stability | S7 (per-kind goldens) + S9 (end-to-end) |
| Evidence size cap enforcement from RFC-28 | S7 (`tool` kind golden — stderr truncation) |
| One fixture per Phase 2 hint kind | S7 |
| One fixture per formatter | S8 |
| `--dump-model` schema-validates against `workspace-model.schema.json` | S9 (end-to-end binary test) |
| Non-empty findings against a real fixture tree | S10 |

If any item slips, the owning slice's PR does not merge.

## Out of scope (Phase 3)

Per RFC-32 §"Phase 3 — framework convergence (out of scope)":

- **Option A** (`specdev check --format json` emitting `ReviewFinding` JSON) — owned by RFC-28 Phase 3, already implemented; no work here.
- **Option B** (declarative `FRAME-*` rules + `scan_profile: framework` + `specdev review`) — owned by [RFC-34](rfc-34-framework-convergence.md); not required for RM-10 and explicitly does not gate RFC-32 acceptance.

Nothing in this plan should drift into Option B territory. If a sub-agent surfaces a need for framework-side enforcement during Phase 2 work, file an RFC-34 issue rather than widening scope.
