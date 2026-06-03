# RFC-31 — Declarative lint completion: retiring `AuthoringProducer`

## Status

Proposed. Supersedes the steady-state posture recorded in
[DIAGNOSTICS.md §"A16"](../DIAGNOSTICS.md) and the framework-authoring-checks
paragraph in [DECISIONS.md §"Crate layout"](../DECISIONS.md). Until this RFC
lands, that posture stands: do not retire predicates or remove
`AuthoringProducer` by weakening checks.

## Motivation

`specdev lint` still runs the imperative `Check` predicates emitting
`CORE-009..051` (`crates/standards/src/framework/check/`). They cannot migrate to
declarative `CORE-*` rule files because every *fact-consuming* hint kind
(`unique`, `cardinality`, `set-coverage`, `set-eq`, `constant-eq`,
`content-digest-eq`, `reference-resolves`, `namespace-owner`) is hardcoded to
exactly one `source` discriminator — all spent on `CORE-001..009` — and the three
"free" kinds (`path-pattern`, `regex`, `schema`) are too weak to reach parity on
the surviving predicates. The payoff (delete `AuthoringProducer`, ~2× `make lint`)
only materialises at the *last* retirement, so this RFC lifts the constraint in
one coordinated change rather than dribbling partial migrations.

## Non-goals

- No weakening of any check. Each retirement must reach byte-parity with its
  predicate on a fixed fixture (the existing `core_parity_*.rs` harness pattern).
- No change to the `Diagnostic` currency, the `DiagnosticProducer` trait, or the
  `output::run_lint` kernel (A19 is done).
- No new operator-facing surface; `WorkspaceModel` stays an internal artifact.

## Design

### Workstream 1 — Hint-kind discriminators + config

Today `DeterministicHint` carries only `{ kind, value, description }`
(`crates/standards/src/rules.rs`) and each fact-consuming eval hardcodes a single
`source` discriminator (e.g. `cardinality` → `skill-body-line-count-max-200`).
Add:

1. An optional structured **`config`** (or `source` discriminator + params) field
   on `DeterministicHint`, schema-validated per kind, so a second metric/policy is
   expressible without new Rust. The `rule.schema.json` /
   `resolved-rules.schema.json` in `crates/schema/` gain the per-kind config
   sub-schemas.
2. Extensions to the two text kinds that block CORE-016/025/050:
   - `regex`: optional **negative-match** mode and a **numeric-capture threshold**
     (the Rust `regex` crate has no lookaround, so `eval/regex.rs` must read a
     capture group and compare, not rely on the pattern). Unblocks CORE-016
     (`RFC-N` where `N < 100`) and CORE-050 (`specify-contract` not followed by
     `-validate`).
   - `path-pattern`: **exclusion globs** (currently `!`-prefixes are rejected as
     `Unsupported` in `eval/path_pattern.rs`). Unblocks CORE-025's
     `decision-log.md` / `release-notes.md` / `/fixtures/` / `/archive/` carve-outs.

Each new discriminator gets a parity fixture before its predicate branch is
deleted.

### Workstream 2 — New `WorkspaceModel` indexer facts

Procedural/structural predicates re-walk the tree today. Move them onto indexer
facts (`crates/standards/src/lint/model.rs` + `index/` extractors), reusing the
existing precedent (`text_match`, `markdown_section.body_line_count`,
`adapter_manifest.brief_keys`). New facts to add, per predicate class:

- A **fenced-block / fence-context** fact for the `text` flow-diagram and
  envelope-JSON checks (so detection is fence-aware, not substring scan).
- A **tool-invocation** fact carrying matched helper + trailing context for the
  CORE-050 negative-lookahead, sourced once instead of re-reading files.
- Per-field **frontmatter granularity** sufficient for CORE-035/036/047 (the
  `argument-hint` token stream, `description` leading verb, `allowed-tools`
  tokens) so they leave the imperative skill-frontmatter parser.
- The `git`-subprocess **trace-staleness** WARN (CORE-034) needs a fact that
  records the staleness verdict, since a declarative eval cannot shell out.

These run under `ScanProfile::Framework` (already reserved in `model.rs`).

### Workstream 3 — De-fuse multi-id predicates

Split predicates that emit more than one id from one loop so each id has an
independent declarative home before its branch is retired:

- `FrontmatterSchema` → `skill.schema-violation` (CORE-044) +
  `skill.missing-frontmatter` (CORE-042).
- `AdapterCheck` → `adapter.execution-agent` (CORE-051) +
  `adapter.missing-manifest` (CORE-010).
- `RulesCheck` (3 ids) and `ScenariosCheck` (7 ids) likewise.

Where `schema`-based migration would double-emit against an already-imperative
schema check (CORE-035/036/047 vs CORE-044), introduce a **sidecar schema** so the
per-token finding counts and messages match the predicate, not a single
schema-pattern violation.

## Migration sequencing

1. Land Workstream 1 + 2 plumbing (no behaviour change; new fields default to the
   current single discriminator).
2. Migrate predicates class-by-class, each gated by a `core_parity_*.rs` fixture
   asserting identical `Diagnostic` output (id, location, evidence, severity).
3. Delete the imperative predicate only once its parity test is green and its id
   has a declarative `CORE-*` rule file in the framework repo
   (`adapters/shared/rules/core/`).
4. When the **last** predicate retires: remove `AuthoringProducer`, the
   `framework::check::` predicate tree, and `CORE_ID_TABLE`; `specdev lint`
   becomes a thin framework-profile wrapper over the same declarative pipeline as
   `specrun lint`.

## Decision: regex-config vs indexer-fact overlap

Workstreams 1 and 2 overlap for CORE-016/050: each can be solved *either* by
extending `regex` (W1) *or* by adding a dedicated indexer fact (W2). To avoid
building both surfaces, pick one per predicate up front:

- **Regex-config (W1)** for pure-text numeric/negative cases (CORE-016, CORE-050).
- **Indexer facts (W2)** only for the genuinely structural cases (fence-context,
  frontmatter granularity, trace-staleness).

## Done definition

- [ ] `DeterministicHint` carries per-kind config; `regex` supports negative-match
      + numeric threshold; `path-pattern` supports exclusion globs.
- [ ] New framework-profile `WorkspaceModel` facts land for fence-context,
      tool-invocation, frontmatter-field granularity, and trace-staleness.
- [ ] All multi-id predicates de-fused; each `CORE-NNN` has an independent home.
- [ ] Every `CORE-010..051` predicate retired at parity; `CORE_ID_TABLE` empty.
- [ ] `AuthoringProducer` removed; `make lint` speedup measured and recorded.

## Cross-repo touchpoints

| Change | Repo | Files |
| --- | --- | --- |
| Hint config + eval extensions | specify-cli | `crates/standards/src/rules.rs`, `crates/standards/src/lint/eval/`, `crates/schema/` rule schemas |
| New indexer facts | specify-cli | `crates/standards/src/lint/model.rs`, `crates/standards/src/lint/index/` |
| Declarative CORE rules | specify | `adapters/shared/rules/core/CORE-0NN-*.md` |
| Predicate docs | specify | `docs/contributing/checks.md` |
| Steady-state posture pointer | specify-cli | `DIAGNOSTICS.md`, `DECISIONS.md` §"Crate layout" |
