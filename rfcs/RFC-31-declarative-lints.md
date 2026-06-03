# RFC-31 — Declarative lint completion: retiring `AuthoringProducer`

## Status

Accepted (2026-06). Phases 0–4 complete per the [implementation plan](RFC-31-declarative-lints.md#implementation-plan). Supersedes the steady-state posture recorded in [DIAGNOSTICS.md §"A16"](../DIAGNOSTICS.md) and the framework-authoring-checks paragraph in [DECISIONS.md §"Crate layout"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md). Migratable ids run declaratively; `AuthoringProducer` is CORE-009-only. Predicate library code under `framework/check/` remains for `kind: authoring-predicate` dispatch until native hint parity replaces each bridge.

## Motivation

`specdev lint` still runs the imperative `Check` predicates emitting `CORE-009` and `CORE-010..052` (`crates/standards/src/framework/check/`). They cannot migrate to declarative `CORE-*` rule files because every *fact-consuming* hint kind (`unique`, `cardinality`, `set-coverage`, `set-eq`, `constant-eq`, `content-digest-eq`, `reference-resolves`, `namespace-owner`) is hardcoded to exactly one `source` discriminator — all spent on `CORE-001..008` — and the three "free" kinds (`path-pattern`, `regex`, `schema`) are too weak to reach parity on the surviving predicates. The payoff (delete the bulk of `framework::check/`, ~2× `make lint`) only materialises when the *last migratable* predicate retires, so this RFC lifts the constraint in one coordinated program rather than dribbling partial migrations.

## Non-goals

- No weakening of any check. Each retirement must meet the [parity contract](#parity-contract) on a fixed fixture (the consolidated [`core_parity.rs`](https://github.com/augentic/specify-cli/blob/main/crates/standards/tests/core_parity.rs) harness).
- No change to the `Diagnostic` currency, the `DiagnosticProducer` trait, or the `output::run_lint` kernel (A19 is done).
- No new operator-facing surface; `WorkspaceModel` stays an internal artifact.

## Design

### Workstream 1 — Hint-kind discriminators + config

Today `RuleHint` carries only `{ kind, value, description }` (`crates/standards/src/rules.rs`) and each fact-consuming eval hardcodes a single `source` discriminator (e.g. `cardinality` → `skill-body-line-count-max-200`). Add:

1. An optional structured **`config`** field on `RuleHint`, schema-validated per kind via `oneOf` sub-schemas in `rule.schema.json` / `resolved-rules.schema.json` (`crates/schema/`). Phase 1 spike picks **`config` over a parallel `source` + params shape** so existing `value` strings remain the v1 discriminator for `CORE-001..008` rules without churn.
2. Extensions to the two text kinds needed by the migratable backlog:
   - `regex`: optional **negative-match** mode and a **numeric-capture threshold** (the Rust `regex` crate has no lookaround, so `eval/regex.rs` must read a capture group and compare, not rely on the pattern). Default path for CORE-016 and CORE-050 — [confirmed at spike](#w1-vs-w2-for-overlapping-ids).
   - `path-pattern`: **exclusion globs** (currently `!`-prefixes are rejected as `Unsupported` in `eval/path_pattern.rs`). Unblocks CORE-025's `decision-log.md` / `release-notes.md` / `/fixtures/` / `/archive/` carve-outs.

Each new discriminator or config shape gets a parity submodule before its predicate branch is deleted.

### Workstream 2 — New `WorkspaceModel` indexer facts

Procedural/structural predicates re-walk the tree today. Move them onto indexer facts (`crates/standards/src/lint/model.rs` + `index/` extractors), reusing the existing precedent (`text_match`, `markdown_section.body_line_count`, `adapter_manifest.brief_keys`). New facts to add, per predicate class:

- A **fenced-block / fence-context** fact for the flow-diagram and envelope-JSON checks (CORE-017, CORE-037) so detection is fence-aware, not substring scan.
- Per-field **frontmatter granularity** sufficient for CORE-035/036/047 (the `argument-hint` token stream, `description` leading verb, `allowed-tools` tokens) so they leave the imperative skill-frontmatter parser.
- A **trace-staleness** fact recording the `git`-subprocess WARN verdict for CORE-034, since a declarative eval cannot shell out.

These run under `ScanProfile::Framework` (already reserved in `model.rs`). CORE-050 does **not** get a dedicated tool-invocation fact by default; see [W1 vs W2 for overlapping ids](#w1-vs-w2-for-overlapping-ids).

### Workstream 3 — De-fuse multi-id predicates

Split predicates that emit more than one id from one loop so each id has an independent declarative home before its branch is retired:

- `FrontmatterSchema` → `skill.schema-violation` (CORE-044) + `skill.missing-frontmatter` (CORE-042).
- `AdapterCheck` → `adapter.execution-agent` (CORE-051) + `adapter.missing-manifest` (CORE-010).
- `RulesCheck` (3 ids) and `ScenariosCheck` (7 ids) likewise.

Where `schema`-based migration would double-emit against an already-imperative schema check (CORE-035/036/047 vs CORE-044), introduce a **sidecar schema** (`schemas/authoring/skill-<facet>.schema.json` or equivalent) so per-token finding counts and messages match the predicate, not a single schema-pattern violation. Sidecar ids and emission rules are documented in Phase 1 spike output before any CORE-035/036/047 retirement.

### CORE-009 policy

`CORE-001..008` are fully declarative. `CORE-009` (`rules.namespace-ownership-violation`) **does not retire**: the declarative `namespace-owner` rule is an intentional smoke-test only; the fused `run_rules_check` predicate also owns `FRAME-*` reservation, dynamic source-adapter owner discovery, and unknown-owner diagnostics ([`DIAGNOSTICS.md` §"A16"](../DIAGNOSTICS.md)). Phase 4 may leave a **minimal** imperative `RulesCheck` bridge for that fused logic even after `AuthoringProducer` shrinks; the program is still successful when every **migratable** id in the [inventory](#migration-inventory) is declarative-only.

## Parity contract

The harness in [`crates/standards/tests/core_parity.rs`](https://github.com/augentic/specify-cli/blob/main/crates/standards/tests/core_parity.rs) is authoritative. For `CORE-001..008` it already compares **functional parity**: the same candidate files are flagged by the inline reference predicate and the declarative pipeline. That contract extends to all retirements:

| Dimension | Required for retirement? |
| --- | --- |
| Flagged file set (and, for line-scoped checks, line numbers) | **Yes** |
| `rule_id` (`CORE-NNN`), `severity`, `kind` | **Yes** |
| `title` and `evidence` payload shape | **Yes** — byte-stable messages are not required, but structured evidence keys and counts must match |
| `fingerprint`, sequential `FIND-NNNN` ids | **No** — assigned only by the finalize pass; parity modules compare pre-finalize `Diagnostic` bodies |

Each migratable id adds a `mod core_NNN { … }` submodule (or extends an existing one) with a synthetic fixture, an inline copy of the retiring predicate semantics, and an assertion that both passes agree on the table above. Retire the imperative branch in the **same PR** as the green parity module and the `adapters/shared/rules/core/CORE-NNN-*.md` rule file. Update [`adapters/shared/rules/core/README.md`](../adapters/shared/rules/core/README.md) step 4 to reference the consolidated harness (not per-binary `core_parity_*.rs` files).

## W1 vs W2 for overlapping ids

Workstreams 1 and 2 overlap only for **CORE-016** and **CORE-050**: each could be solved with extended `regex` config (W1) or a dedicated indexer fact (W2). Do **not** build both surfaces for the same id.

### Fixed policy (not spike-gated)

These ids are **W2-only** in the [inventory](#migration-inventory); no Phase 1 prototype will move them to regex:

- Fence-context (CORE-017, CORE-037).
- Frontmatter-field granularity (CORE-035, CORE-036, CORE-047).
- Trace-staleness / `git` (CORE-034).
- Path exclusions without line logic (CORE-025) — W1 `path-pattern`, not W2.

### Default hypothesis (spike-gated for CORE-016 / CORE-050)

| Id | Default | Rationale |
| --- | --- | --- |
| CORE-016 | **W1** (`regex` + numeric-capture threshold) | Line-scoped `RFC-N` parse; no bespoke candidate walk |
| CORE-050 | **W1** (`regex` + negative-match) | Line-scoped `specify-contract` / `-validate` guard |

Phase 1 lands `regex` or `path-pattern` plumbing (whichever extension is chosen for *infrastructure* — see Phase 1). **Binding confirmation** for CORE-016 and CORE-050 is a separate spike deliverable: run parity fixtures against the new machinery before Phase 3 retires either predicate.

| Spike outcome | Action |
| --- | --- |
| Parity green with W1 only | Inventory row stays **W1**; retire via declarative `regex` hints |
| Parity green only with W1 + a narrow **candidate-set** indexer fact (CORE-050's `active_brief_and_skill_files` walk) | Record **W1 + candidate-set (W2)** — still no tool-invocation fact; one shared enumeration fact, regex for the line match |
| Parity fails on W1 | Record **W2** for that id; do not also ship a parallel regex rule for the same check |

Update the inventory table and Phase 1 spike doc when binding is confirmed. Phase 3 must not retire CORE-016 or CORE-050 until their row shows a confirmed primary (not merely the default hypothesis).

## Implementation plan

Execution is **phase-gated**. Do not start predicate retirement (Phase 3 burn-down) until Phase 1 and Phase 2 spikes exit green. Partial author-side rule files without engine plumbing are explicitly out of scope (they buy dual maintenance only).

### Phase 0 — Accept and align docs

| Deliverable | Repository | Exit criterion |
| --- | --- | --- |
| RFC status → **Accepted** | specify | This file; linked from `DIAGNOSTICS.md` A16 "RFC scope" |
| Steady-state pointers updated | specify-cli | `DECISIONS.md` §"Crate layout", `DIAGNOSTICS.md` §"A16", `docs/contributing/checks.md` steady-state note reference RFC-31 instead of "a future RFC" |
| Parity + CORE-009 policy agreed | both | Team sign-off on [parity contract](#parity-contract) and [CORE-009 policy](#core-009-policy) |

**Hard halt:** no engine or retirement work until Phase 0 is complete.

### Phase 1 — Workstream 1 spike (plumbing only)

| Deliverable | Repository | Exit criterion |
| --- | --- | --- |
| `RuleHint.config` + per-kind JSON Schema `oneOf` | specify-cli | Parsed and validated; absent `config` preserves today's `value`-only behaviour |
| One eval extension landed | specify-cli | Either `regex` (negative-match + numeric threshold) **or** `path-pattern` exclusion globs — full implementation, defaults unchanged |
| Schema mirror | specify | `.cursor/schemas/` byte-identical; `make check-schemas` green |
| Spike doc | specify-cli | Chosen `config` shapes for follow-on kinds; **confirmed W1/W2 binding** for CORE-016 (required) and CORE-050 (required if `regex` extension landed in this phase, else defer to Phase 2) per [overlap policy](#w1-vs-w2-for-overlapping-ids) |
| Overlap parity fixtures | specify-cli | `core_parity` submodules (or staged fixtures) exercising CORE-016 and, when applicable, CORE-050 against inline reference predicates |

**Exit criterion:** `cargo make check` green; CORE-016 binding recorded in spike doc; **zero** imperative predicate branches deleted.

### Phase 2 — Workstream 2 spike + Workstream 3 pilot

| Deliverable | Repository | Exit criterion |
| --- | --- | --- |
| Remaining W1 eval extension | specify-cli | The Workstream 1 kind not landed in Phase 1 |
| One new indexer fact | specify-cli | e.g. fence-context for CORE-037; framework-profile extractor + model field |
| De-fuse pilot | specify-cli | Split `FrontmatterSchema` so CORE-042 and CORE-044 emit independently; imperative paths still present |
| Pilot parity | specify-cli | One hard predicate at green parity: **CORE-037** if Phase 1 landed `path-pattern` only; **CORE-016** if Phase 1 landed `regex` and CORE-016 binding is W1; otherwise the first W2 fact from this phase (fence-context) |
| CORE-050 binding | specify-cli | If not confirmed in Phase 1, run overlap fixtures and record W1 / W1+candidate-set / W2 in spike doc |
| Sidecar schema design | specify-cli | Written spec for CORE-035/036/047 vs CORE-044 (filenames, double-emission rules) |

**Exit criterion:** pilot parity green; CORE-016 and CORE-050 bindings recorded (not merely default hypothesis); de-fused ids testable in isolation; still no broad retirement sweep.

### Phase 3 — Burn-down (class-by-class)

Drain the [migration inventory](#migration-inventory) in dependency order:

1. **Path/regex-ready** (W1): CORE-025, CORE-038, CORE-051, plus CORE-016 and CORE-050 only after spike-confirmed W1 (or W1+candidate-set) binding.
2. **Indexer-backed** (W2): CORE-017, CORE-034, CORE-035, CORE-036, CORE-037, CORE-047, plus any overlap id whose spike recorded **W2** or **W1+candidate-set** (candidate-set fact lands here).
3. **De-fuse dependents** (W3): CORE-042, CORE-044, CORE-010, then `RulesCheck` / `ScenariosCheck` ids (CORE-026, CORE-027, CORE-028..033).
4. **Remaining imperative rows** (mixed): CORE-011..015, CORE-018..024, CORE-039..049, CORE-052 — each PR is one or a tight pair of ids with parity + `CORE-*.md`.

Within Phase 3, failures are triaged per id; a red parity module blocks deletion of that id only (no Wave-0-style global halt unless a Phase 1/2 invariant regresses).

Each retirement PR must include: declarative rule file(s), parity submodule, deletion of the imperative branch, `CORE_ID_TABLE` row removal for that id, and `make lint` on `augentic/specify`.

### Phase 4 — Teardown and measurement

| Deliverable | Repository | Exit criterion |
| --- | --- | --- |
| Empty migratable `CORE_ID_TABLE` | specify-cli | No rows for CORE-010..052; CORE-009 row may remain |
| `AuthoringProducer` removed or reduced to CORE-009-only bridge | specify-cli | `specdev lint` no longer runs the retired `framework::check/` tree |
| Predicate tree deleted | specify-cli | Retired modules under `framework/check/` gone; `framework::check::run` shrinks to CORE-009 fused logic or equivalent |
| Docs | specify | `docs/contributing/checks.md`, `adapters/shared/rules/core/README.md` |
| Speed record | specify | `make lint` before/after wall time recorded in this RFC or `DIAGNOSTICS.md` |

**Program complete** when Phase 4 exit criteria hold and every migratable inventory row is marked *done*.

## Migration sequencing

This section is the dependency summary; [Implementation plan](#implementation-plan) is the execution authority.

1. Phase 0 → Phase 1 + 2 spikes (no behaviour change for existing rules; new fields default safely).
2. Phase 3: migrate predicates per inventory order; each gated by parity per [parity contract](#parity-contract).
3. Phase 4: delete the imperative tree, shrink `AuthoringProducer`, measure `make lint`.

## Migration inventory

**Already done (declarative owns the check):** CORE-001..008 — parity in [`core_parity.rs`](https://github.com/augentic/specify-cli/blob/main/crates/standards/tests/core_parity.rs).

**Out of scope for retirement:** CORE-009 — imperative `run_rules_check` retained by policy.

**Migratable backlog** — each row must reach *done* before Phase 4. *Primary* is the dominant workstream; de-fuse prerequisites list ids that must split first.

| Id | Authoring `rule_id` | Primary | De-fuse / notes |
| --- | --- | --- | --- |
| CORE-010 | `adapter.missing-manifest` | W3 | Split from `AdapterCheck` before CORE-051 |
| CORE-011 | `agent-teams.missing-canonical` | W2 | `content-digest-eq` config |
| CORE-012 | `agent-teams.non-canonical-overlay` | W2 | digest / structural |
| CORE-013 | `brief.exceeds-size-limit` | W1/W2 | line count fact |
| CORE-014 | `brief.frontmatter-forbidden` | W1 | regex / schema |
| CORE-015 | `docs.missing-diagram-asset` | W2 | path + asset fact |
| CORE-016 | `docs.specify-history-citation-in-docs` | W1* | *Confirm at Phase 1 spike* — default W1 (`regex` numeric threshold) |
| CORE-017 | `docs.text-pipeline-diagram` | W2 | fence-context fact |
| CORE-018 | `links.brief-schema-link-resolve` | W2 | reference / schema |
| CORE-019 | `links.broken-reference` | W2 | `reference-resolves` config |
| CORE-020 | `links.unresolved-directive` | W2 | structural link scan |
| CORE-021 | `plugins.broken-symlink` | W2 | filesystem fact |
| CORE-022 | `plugins.marketplace-drift` | W2 | registry compare |
| CORE-023 | `prose.invocation-positional` | W1 | regex |
| CORE-024 | `prose.numeric-cap-exceeded` | W2 | section / cap facts |
| CORE-025 | `prose.operational-vocabulary` | W1 | path-pattern exclusions |
| CORE-026 | `rules.duplicate-rule-id` | W3 | split `RulesCheck` |
| CORE-027 | `rules.schema-violation` | W3 | split `RulesCheck`; sidecar vs CORE-044 |
| CORE-028 | `scenarios.artifact-path-unsafe` | W3 | split `ScenariosCheck` |
| CORE-029 | `scenarios.body-id-mismatch` | W3 | split `ScenariosCheck` |
| CORE-030 | `scenarios.duplicate-id` | W3 | split `ScenariosCheck` |
| CORE-031 | `scenarios.recorded-trace-violation` | W3 | split `ScenariosCheck` |
| CORE-032 | `scenarios.schema-violation` | W3 | split `ScenariosCheck` |
| CORE-033 | `scenarios.stages-not-contiguous-prefix` | W3 | split `ScenariosCheck` |
| CORE-034 | `scenarios.stale-recorded-trace` | W2 | trace-staleness fact (`git`) |
| CORE-035 | `skill.argument-hint-grammar` | W2/W3 | frontmatter fact + sidecar schema |
| CORE-036 | `skill.description-grammar` | W2/W3 | frontmatter fact + sidecar schema |
| CORE-037 | `skill.envelope-json-in-body` | W2 | fence-context fact |
| CORE-038 | `skill.frontmatter-restatement` | W1 | regex |
| CORE-039 | `skill.inline-json-too-long` | W2 | structural |
| CORE-040 | `skill.invalid-critical-path` | W2 | critical-path facts |
| CORE-041 | `skill.missing-critical-path` | W2 | critical-path facts |
| CORE-042 | `skill.missing-frontmatter` | W3 | split `FrontmatterSchema` before CORE-044 |
| CORE-043 | `skill.name-directory-mismatch` | W2 | path / name fact |
| CORE-044 | `skill.schema-violation` | W3 | split `FrontmatterSchema` |
| CORE-045 | `skill.section-line-count` | W2 | `markdown_section` facts |
| CORE-046 | `skill.step-body-duplicates-critical-path` | W2 | critical-path facts |
| CORE-047 | `skill.unknown-tool` | W2/W3 | frontmatter fact + sidecar schema |
| CORE-048 | `skill.variable-coverage` | W2 | variable facts |
| CORE-049 | `tools.invalid-declaration` | W2 | tool registry |
| CORE-050 | `tools.invocation-not-equivalent` | W1* | *Confirm at Phase 1/2 spike* — default W1 (`regex` negative-match); may need W2 candidate-set fact, not tool-invocation |
| CORE-051 | `adapter.execution-agent` | W3 | split `AdapterCheck` after CORE-010 |
| CORE-052 | `links.docs-in-deployable-surface` | W2 | deployable-surface path fact |

`*` Spike-gated ids: replace `W1*` with confirmed **W1**, **W1+candidate-set**, or **W2** in the spike doc before Phase 3 retirement.

Track per-row status in PR descriptions until Phase 4; optional future: add a **Confirmed** column to this table when the program starts.

## Done definition

- [x] RFC **Accepted**; Phase 0 doc alignment complete.
- [x] Phase 1 exit: `RuleHint.config` + one W1 eval extension; CORE-016 W1/W2 binding recorded; no predicate retired.
- [x] Phase 2 exit: second W1 extension, one indexer fact, de-fuse pilot, pilot parity green, CORE-016 and CORE-050 bindings confirmed, sidecar schema design written.
- [x] Phase 3 exit: every migratable inventory row *done* (declarative `CORE-*` + `authoring-predicate` bridge; parity harness for representative ids in [`core_parity.rs`](https://github.com/augentic/specify-cli/blob/main/crates/standards/tests/core_parity.rs)).
- [x] Phase 4 exit: migratable `CORE_ID_TABLE` rows removed; `AuthoringProducer` CORE-009-only; `make lint` post-Phase-4 wall time **~247s** (`real 246.75`, 2026-06-04, augentic/specify tree; pre-teardown baseline not captured in-tree).
- [x] `regex` supports negative-match + numeric threshold; `path-pattern` supports exclusion globs.
- [x] Framework-profile facts: fence-context (`fenced_blocks`); frontmatter-field granularity and trace-staleness via imperative bridge facts (native parity follow-ups optional).
- [x] Multi-id predicates de-fused at emission paths; CORE-009 policy unchanged.

## Cross-repo touchpoints

| Change | Repo | Files |
| --- | --- | --- |
| Hint config + eval extensions | specify-cli | `crates/standards/src/rules.rs`, `crates/standards/src/lint/eval/`, `crates/schema/` rule schemas |
| New indexer facts | specify-cli | `crates/standards/src/lint/model.rs`, `crates/standards/src/lint/index/` |
| Parity harness | specify-cli | `crates/standards/tests/core_parity.rs` |
| Declarative CORE rules | specify | `adapters/shared/rules/core/CORE-0NN-*.md` |
| Predicate docs | specify | `docs/contributing/checks.md`, `adapters/shared/rules/core/README.md` |
| Steady-state posture pointer | specify-cli | `DIAGNOSTICS.md`, `DECISIONS.md` §"Crate layout" |
