# RFC-34 Implementation Plan

> Companion to [rfc-34-core-rules.md](rfc-34-core-rules.md). Breaks the RFC into subagent-sized changes, with explicit dependencies and parallelism markers. Covers every reserved hint kind named in §F6 — the chassis (C1–C10) plus one card per reserved kind (C11–C18) so RFC-34 lands end-to-end rather than leaving the tail to "contributor demand."

## Scope

RFC-34 spans two repositories:

- [`augentic/specify-cli`](https://github.com/augentic/specify-cli) — Rust workspace; ships schema, CLI, indexer, predicates, and reserved hint-kind interpreters.
- [`augentic/specify`](https://github.com/augentic/specify) — framework repo; ships `CORE-*` rules under `adapters/shared/rules/core/` and contributor docs.

Each change below names the owning repo so the dispatching agent picks the right working directory.

## RFC drift to fix in passing

Three small mismatches between RFC-34's prose and the live source. The relevant card fixes each:

1. **Schema annotation key is `x-hint-status`, not `x-rfc32-status`** (RFC-34 §F6). Verified in `schemas/rules/rule.schema.json` (today's reserved kinds carry `"x-hint-status": "reserved"`). Per-kind PRs (C11–C18) drop the `x-hint-status` annotation when each kind lands.
2. **Resolver walk wiring is not in `check::rules`** (RFC-34 §F3 implies it lives next to `BUILTIN_NAMESPACES`). The actual walk lives in `crates/specify-lints/src/rules/resolve.rs` with a hardcoded `SHARED_REL = "adapters/shared/rules/universal"` constant and a parallel walk loop tagging each rule with `Origin::Shared`. C3 covers both surfaces explicitly so nothing falls through.
3. **C1 also needs to widen the closed `ruleId` regex** to allow `CORE-` ids. RFC-34 §A1 says the regex widens "in the same PR" as the new `origin` value but the C1 card here only listed enum sites. Found at C3 time (the resolver fixture failed schema validation). C3 absorbed the three-character regex addition in `schemas/rules/{rule,resolved}.schema.json` + `schemas/lint/finding.schema.json` so the chassis remained consistent; future readers of C1 should treat the regex bump as part of the same scope.
4. **C6 surfaces a latent directive-parser bug.** Wiring the declarative pass alongside the imperative pass under `specdev lint` exposed the fact that the `specify-ignore` parser accepts any non-whitespace token in the `<RULE-ID>` slot — so documentation examples like `<!-- specify-ignore: … -->` in `docs/reference/ignore-directives.md` self-triggered UNI-022 / UNI-023. The fix is a single-line guard in `crates/specify-lints/src/lint/index/ignore_directives/parse.rs` that validates the rule-id against `^[A-Z][A-Z0-9]*-[0-9]+$` at parse time (the same shape the schema's `ruleId` `$def` already enforces). Landed alongside C6 as a follow-up commit so `make check` in the framework repo stays green for C7.
5. **`applicability.artifacts` filtering inverts on the framework profile.** `crates/specify-lints/src/rules/resolve/filter.rs::artifact_dimension_matches` drops any artifact-populated rule from the resolved set when the caller passes `include_unmatched: false`. `specdev lint` and v1's framework pipeline pass `include_unmatched: false`, so a CORE-* rule that declares `applicability.artifacts: [adapter]` is silently filtered out before the evaluator runs. C7 worked around it by dropping the artifact token and narrowing via `kind: path-pattern` instead. C8 (or a chassis follow-up before C10–C18) should fix this properly by flipping `include_unmatched: true` for the framework profile or wiring artifact-kind facts off `WorkspaceModel` so the declarative token can be re-introduced. Until then, per-kind CORE-* cards (C10–C18) should mirror C7: narrow candidates with a `path-pattern` hint and leave `applicability.artifacts` unset.
6. **Adapter / source / target schemas need a framework-side mirror for `kind: schema` hints.** The CLI's `REGISTERED_SCHEMAS` table only contains rule / lint / plan / evidence / fusion / components schemas. C7 mirrored `adapter.schema.json` into the framework repo's `.cursor/schemas/` so the schema hint resolves; the same pattern is available to C8 / C10–C18 if they need `source.schema.json` / `target.schema.json` or any other manifest schema. A drift check between the CLI canonical and the `.cursor/schemas/` mirror is a candidate C9 follow-up.
7. **`CORE-*` rule bodies are themselves linted by `specdev lint`.** Two predicates fire on the rule markdown the per-kind cards author under `adapters/shared/rules/core/`: (a) `docs.specify-history-citation-in-docs` rejects any literal `RFC-<n>` citation or `rfcs/` / archival-tree path mention in a rule body — phrase rationale as "the migration-cadence fallback" rather than "per RFC-34 §F5"; and (b) `CORE-002` (`reference-resolves`) checks every relative markdown link in the body resolves on disk. Keep `CORE-*` bodies citation-free and link-clean or `make check` reds out. This bit C11's CORE-002 prose and C14's CORE-006 prose; both fixed in passing.

## Pre-flight (do once, by hand, before dispatching changes)

Confirm before kicking off any subagent:

1. **RFC-33a wave 4 is merged** in `specify-cli`. RFC-34 §F7 sizes the landing order after RFC-33a's journal emission + acceptance goldens. Look for `lint-completed` in the closed `EventKind` taxonomy at `crates/domain/src/journal.rs` (or `crates/specify-lints/`).
2. **RFC-32 Phase 2 is merged** in `specify-cli`. Look for `crates/specify-lints/src/lint/`, `schemas/lint/{finding,lint-result,workspace-model}.schema.json`, the `specrun lint` handler under `src/runtime/commands/lint/`, and `crates/specify-lints/src/lint/eval/{path_pattern,regex,schema,tool}.rs`.
3. **RFC-28 envelope is current.** Look for the existing `origin` enum in `schemas/rules/resolved.schema.json` (today: `shared | source | target | organization`).

If any prerequisite is missing, halt — RFC-34 cannot land cleanly otherwise (§F7 landing-order rule, §"Depends" frontmatter).

## Conventions for each change card

Each change below names:

- **Repo / paths touched** — where the subagent works.
- **RFC anchors** — section IDs in `rfc-34-core-rules.md` the card implements.
- **Depends on** — change IDs that must be merged first.
- **Parallel with** — change IDs that may run alongside this one.
- **Definition of done** — the literal commands or files that close the change.

Change IDs are stable (`C1`…`C18`) — reference them in subagent prompts.

## Dependency overview

```text
                ┌─────────── Pre-flight ───────────┐
                │  RFC-33a wave 4 merged           │
                │  RFC-32 Phase 2 merged           │
                └──────────────┬───────────────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
       C1 (Origin)           C2 (artifacts)         C3 (namespaces + resolver walk)
        │                      │                      │
        └──────────┬───────────┴──────────┬───────────┘
                   │                      │
                  C4 (--include-core)    C5 (framework extractors)
                   │                      │
                   │                     C6 (specdev lint CLI)
                   │                      │
                   └──────────┬───────────┘
                              │
                ┌─────────────┴──────────────┐
                │                            │
              C7 (CORE-001 rule)           C8 (parity + retire)
                │                            │
                └─────────────┬──────────────┘
                              │
                             C9 (docs)         ← chassis complete
                              │
        ┌──────────┬──────────┼──────────┬──────────┬──────────┬──────────┐
        │          │          │          │          │          │          │
      C10        C11        C12        C13        C14        C15        C16        C17        C18
      ref-       unique     set-       cardin-    constant   set-       content    namespace
      resolves              coverage   ality      -eq        eq         -digest-   -owner
                                                                        eq
                       (eight parallel per-reserved-kind PRs)
```

C1, C2, C3 are mutually independent — three subagents can run in parallel.
C4 and C5 are mutually independent once Phase 1 lands — two subagents in parallel.
C7 (rule authoring in `specify`) is independent of C6 (`specdev lint` wiring in `specify-cli`) — two subagents in parallel.
C10–C18 are mutually independent once the chassis closes — up to nine subagents in parallel.

---

## Phase 1 — Foundational schema and predicate changes

Three independent edits. Dispatch as three parallel subagents.

### C1 — `Origin` enum widening + `organization` → `unknown` rename

- **Repo:** `specify-cli`.
- **RFC anchors:** §A1, §A2, §A4.
- **Paths:**
  - `schemas/rules/resolved.schema.json` — add `core`, rename `organization` → `unknown`, update sort-order description, update the enum description ("indexer fallback…").
  - `schemas/lint/workspace-model.schema.json` — mirror the rename + description update.
  - Rust `Origin` enum (search `enum Origin` across `crates/specify-lints/`, `crates/domain/`, `crates/schema/`). Add the `Core` variant and rename `Organization` → `Unknown`.
  - `crates/specify-lints/src/lint/index/discover.rs::infer_origin` — return value renamed from `Origin::Organization` to `Origin::Unknown`.
  - `crates/specify-lints/src/rules/resolve/sort.rs` (and any other sort site) — new order `target → source → shared → core → unknown`.
- **Depends on:** Pre-flight only.
- **Parallel with:** C2, C3.
- **Notes:**
  - This is a hard wire break for parsers pinned to the literal `organization` (§A4). Land in the same chassis batch so consumer schema parsers absorb both the rename and the `core` addition in one envelope-schema bump.
  - `core` semantics (which path produces it) are wired in C3 — this card only adds the variant and sort slot.
- **Definition of done:**
  - `cargo make ci` green.
  - Schema files validate with the new enum (`cargo test -p specify-schema` or workspace tests).
  - `rg "Origin::Organization"` returns zero hits in `specify-cli`; `rg "\"organization\"" schemas/` returns zero hits except CHANGELOG / RFC quotes.

### C2 — `applicability.artifacts` framework tokens

- **Repo:** `specify-cli`.
- **RFC anchor:** §F4.
- **Paths:**
  - `schemas/rules/rule.schema.json` — extend the closed `applicability.artifacts` enum with `skill`, `adapter`, `brief`, `reference`, `codex`, `rfc`, `doc`.
  - Any Rust mirror of the enum (search `applicability` parsing under `crates/specify-lints/src/rules/parse.rs`).
- **Depends on:** Pre-flight only.
- **Parallel with:** C1, C3.
- **Definition of done:**
  - Schema validates; the existing parser accepts a fixture rule using `[skill, adapter]`.
  - `cargo make ci` green.

### C3 — `check::rules` namespaces + placement predicate + resolver walk

- **Repo:** `specify-cli`.
- **RFC anchor:** §F3.
- **Two-surface card.** §F3 names one surface (`BUILTIN_NAMESPACES`) but presumes another (the resolver walk). Both wire up the `core` pack root; doing them together keeps the activation atomic.
- **Surface 1 — validation predicate** (`crates/authoring/src/check/rules.rs`):
  - Extend `BUILTIN_NAMESPACES` with `adapters/shared/rules/core/` → `{"CORE"}`.
  - Update the placement predicate so `CORE-*` under `adapters/{sources,targets}/<name>/rules/` is still rejected, `CORE-*` under `adapters/shared/rules/core/` is **required**, and non-`CORE-*` under the new path is rejected with `codex-namespace-ownership-violation`.
- **Surface 2 — resolver walk** (`crates/specify-lints/src/rules/resolve.rs`):
  - Add a `const CORE_REL: &str = "adapters/shared/rules/core";` next to the existing `SHARED_REL`.
  - After the `shared_dir` walk loop (today around lines 182–194), add a parallel walk for `core_dir = rules_root.join(CORE_REL)` that tags each rule with `Origin::Core` (the variant added in C1).
  - Confirm `list_rule_files` and `relative_path` work unchanged for the new root.
- **Depends on:** Pre-flight only (works on a stub `Origin::Core` until C1 lands; once C1 lands, the variant resolves cleanly). In practice land after or with C1.
- **Parallel with:** C1, C2 (with the C1 caveat above — if dispatched truly in parallel, the C3 subagent should add `Origin::Core` defensively if it's not present yet).
- **Definition of done:**
  - Predicate unit tests cover all three branches (legal, illegal-foreign, illegal-non-core).
  - Resolver test against a fixture with `adapters/shared/rules/core/CORE-fixture.md` produces a `ResolvedRuleEntry` with `origin: Origin::Core`.
  - `cargo make ci` green.

---

## Phase 2 — Framework profile and CLI wiring

Two independent edits once Phase 1 lands. Dispatch C4 and C5 in parallel; C6 follows C5.

### C4 — `--include-core` flag on `specrun rules export` and `specrun lint`

- **Repo:** `specify-cli`.
- **RFC anchors:** §A3, §F3 ("Consumer-export filtering").
- **Paths:**
  - `src/runtime/commands/lint/cli.rs` — add `--include-core` (default off).
  - `src/runtime/commands/rules/` (or wherever `rules export` is wired) — add the flag.
  - `crates/specify-lints/src/rules/resolve/filter.rs` — default-exclude `Origin::Core` from resolved outputs unless the flag is set (mirrors the existing `--include-deprecated` / `--include-unmatched` filter shape).
- **Depends on:** C1 (needs `Origin::Core` to filter on), C3 (needs the resolver to actually walk the core pack root, otherwise the filter has nothing to drop).
- **Parallel with:** C5.
- **Definition of done:**
  - Golden test: `specrun rules export` (no flag) on a fixture tree with `adapters/shared/rules/core/CORE-fixture.md` excludes the rule from output.
  - With `--include-core`, the rule appears with `origin: core`.

### C5 — Framework scan profile: extractors + symlink mode

- **Repo:** `specify-cli`.
- **RFC anchors:** §F1, §"Module additions".
- **Paths (new):**
  - `crates/specify-lints/src/lint/index/skill.rs`
  - `crates/specify-lints/src/lint/index/adapter.rs`
  - `crates/specify-lints/src/lint/index/marketplace.rs`
  - `crates/specify-lints/src/lint/index/agent_teams.rs`
  - `crates/specify-lints/src/lint/index/brief.rs`
- **Paths (modified):**
  - `crates/specify-lints/src/lint/index/symlinks.rs` — gain a `follow: bool` parameter; consumer profile passes `false` (record-without-traverse), framework profile passes `true` (follow + record both endpoints).
  - Indexer dispatch (search for the consumer scan-profile match site) — branch on `scan_profile: framework` to invoke the new extractors with the §F1 include / ignore globs.
- **Depends on:** Pre-flight only (technically independent of C1–C3; can move into Phase 1 if scheduling demands).
- **Parallel with:** C4.
- **Notes:**
  - Each extractor is small — if context budget becomes tight this card can split into five subagent cards (one per extractor file), since they only share the dispatch site, not internal types.
  - Fixture: `crates/specify-lints/tests/fixtures/lint/framework_minimal/` (~10 files) exercising every extractor at least once (§"Test layout").
- **Definition of done:**
  - `crates/specify-lints/tests/lint_framework_indexer.rs` passes against the new fixture.
  - `cargo make ci` green.

### C6 — `specdev lint` CLI extension

- **Repo:** `specify-cli`.
- **RFC anchors:** §F2, §"CLI surface under `specify-authoring`".
- **Paths (new):**
  - `src/authoring/commands/lint.rs` — umbrella module.
  - `src/authoring/commands/lint/cli.rs` — `LintAction` clap-derive subcommand.
  - `src/authoring/commands/lint/run.rs` — handler.
  - `tests/specdev_lint.rs` — binary-level end-to-end test.
- **Paths (modified):**
  - `crates/authoring/Cargo.toml` — add `specify-lints.workspace = true`.
  - `src/authoring/commands.rs` — register the new subcommand.
- **Behaviour:**
  - `--rules-root` default `.`; `--scan-profile` hard-coded to `framework` (no flag); `--target` optional, default "none".
  - Wire export → index → eval → envelope.
  - Ship `--rule`, `--artifact`, `--dump-model`, `--strict-hints`, `--format {json,pretty,github,compact}`.
  - Reuse the four formatters from `specify-lints::lint::diagnostics`.
  - Apply RFC-32 §D8 exit-code map verbatim.
  - Emit exactly one `lint-completed` event per run (§F7) — payload uses `scope.target: None`, `scope.slice: None`, `scope.artifact` reflecting `--artifact <path>`, `baseline_present: false`.
  - `--format json` validates against `schemas/lint/lint-result.schema.json` before emit.
  - `CORE-*` rules are visible to the framework run by default (no `--include-core` needed on `specdev lint`; the flag is consumer-side per §A3).
- **Depends on:** C5 (needs framework extractors); C3 (resolver must emit `Origin::Core` rules so they're available to the framework run); C1 (variant must exist).
- **Parallel with:** C7.
- **Definition of done:**
  - `specdev lint --format json` produces a stable envelope against the framework repo (chassis acceptance criterion).
  - A `lint-completed` event lands in `.specify/journal.jsonl` per run (verify via integration test).
  - `cargo make ci` green.

---

## Phase 3 — First `CORE-*` rule + parity

C7 lives in the framework repo; C8 lives in the CLI repo. They are independent and can run in parallel once Phases 1 + 2 finish.

### C7 — Hand-author `CORE-001` ≅ `adapter.schema`

- **Repo:** `specify`.
- **RFC anchors:** §"Implementation Plan" step 4, §F6.
- **Paths (new):**
  - `adapters/shared/rules/core/CORE-001-adapter-schema.md` (filename follows existing `UNI-*` rule naming convention; verify by listing `adapters/shared/rules/universal/`).
  - `adapters/shared/rules/core/README.md` — conventions for `CORE-*` rule authoring (§"Documentation touch-points"). May be deferred to C9 if the agent prefers.
- **Body must include:**
  - `## Rule` body (canonical agent-readable explanation).
  - `deterministic_hints` block using the already-implemented `schema` hint kind (§F6 — chassis uses landed kinds only).
  - `applicability.artifacts: [adapter]` (one of the new tokens from C2).
- **Depends on:** C2 (token added), C3 (placement predicate accepts the path).
- **Parallel with:** C6, C8 (different repos, different surfaces).
- **Definition of done:**
  - `make check` in `specify` passes — the placement predicate accepts the rule.
  - File is referenced by the parity test landing in C8.

### C8 — Parity test + imperative retirement (`adapter` schema row)

- **Repo:** `specify-cli`.
- **RFC anchors:** §F5, §"Implementation Plan" step 5.
- **Paths (new):**
  - `crates/authoring/tests/core_parity_adapter_schema.rs` — proves byte-identical findings between the retiring imperative predicate row and `CORE-001`'s `schema` hint, against the existing predicate goldens.
- **Paths (modified):**
  - `crates/authoring/src/check/adapter.rs` — delete the schema-row impl that `CORE-001` now covers (keep other rows of `check::adapter` alive).
  - `docs/contributing/checks.md` (in `specify` repo) — point at `CORE-001` instead of the deleted predicate. May be folded into C9 if the parity author works only in `specify-cli`.
- **Behaviour:**
  - During overlap the existing fingerprint algorithm deduplicates against `(rule-id, location)` (§F5) — no new code needed.
- **Depends on:** C6 (`specdev lint` running so the parity harness can dispatch hints), C7 (rule exists at the expected path).
- **Parallel with:** C7 (after C2/C3 land — but C8's parity test imports `CORE-001`, so in practice C7 should merge first or the two land as a coordinated pair across repos).
- **Definition of done:**
  - Parity test passes byte-identically against retiring predicate's existing goldens.
  - `cargo make ci` green with the imperative row deleted.
  - `make check` (parent repo) green.

---

## Phase 4 — Documentation

### C9 — Documentation touch-points

- **Repos:** `specify-cli` + `specify`.
- **RFC anchor:** §"Documentation touch-points (post-merge)".
- **Paths:**
  - `specify-cli/AGENTS.md` — update `specdev lint` documentation map; list `crates/specify-lints/src/lint/index/{skill,adapter,marketplace,agent_teams,brief}.rs` under "Modules of note".
  - `specify-cli/docs/standards/architecture.md` — extend the standards-layer module section with the framework-profile extractors.
  - `specify/docs/contributing/checks.md` — explain when to author a new imperative `Check` versus a `CORE-*` rule; default recommendation: `CORE-*` unless the predicate needs a subprocess that cannot be modelled as a `tool` hint.
  - `specify/adapters/shared/rules/core/README.md` — if not already landed in C7, write conventions doc (body structure, applicability tokens, hint-kind preference, pointer to RFC-32 "Predicate migration map").
- **Depends on:** C1–C8 (all chassis surfaces must be settled).
- **Parallel with:** C10–C18 once Phase 3 closes — docs do not block per-kind work.
- **Definition of done:**
  - `make check` in `specify` green.
  - `cargo make ci` in `specify-cli` green.
  - `rg "specdev lint" docs/` finds the new framework-profile prose.

---

## Phase 5 — Per-reserved-hint-kind PRs (one card per kind in RFC-34 §F6)

RFC-34 §F6 lists eight reserved hint kinds (`unique`, `set-coverage`, `reference-resolves`, `cardinality`, `constant-eq`, `set-eq`, `content-digest-eq`, `namespace-owner`). RFC-34's own Implementation Plan only schedules two of them (`reference-resolves` as PR 2, `unique` as PR 3); the remaining six are deferred "as contributor demand reaches them." Per the user's instruction to cover the whole RFC end-to-end, this plan schedules all eight, one per card (C10–C18, with C12 absorbing PR 3's `unique` slot). C10–C18 are **mutually independent once C9 lands** and can dispatch to as many parallel subagents as the runtime supports.

### Shared per-kind template

Every per-kind card ships the same five-element checklist (§F6, §F5):

1. **Reserved hint kind interpreter** at `crates/specify-lints/src/lint/eval/<kind>.rs`.
2. **Schema annotation removal** — drop `"x-hint-status": "reserved"` from the kind in `schemas/rules/rule.schema.json` (RFC body says `x-rfc32-status`; live key is `x-hint-status` per "RFC drift to fix in passing" above).
3. **One seed `CORE-*` rule** under `adapters/shared/rules/core/` in the `specify` repo.
4. **Parity fixture** at `crates/authoring/tests/core_parity_<rule>.rs` proving byte-identical findings against the retiring imperative predicate's existing goldens.
5. **Imperative `Check` deletion** in the same PR.

Per-card definition of done is uniform:

- Parity test passes byte-identically against retiring predicate's existing goldens.
- `cargo make ci` green; `make check` green.
- The reserved kind's `"x-hint-status": "reserved"` annotation is gone from `schemas/rules/rule.schema.json`.

### Choosing the seed rule and retiring `Check` row

C10, C11, C12 are pinned by RFC-34 (CORE-002 = `links.unresolved`, CORE-003 = `skill.duplicate-name`, plus the implicit CORE-001 already covered in C7/C8). For C13–C18, the seed rule and the retiring `Check` row are **not** pinned by RFC-34. The subagent picks them by consulting:

1. **RFC-32 `§"Predicate migration map"`** (in `rfcs/done/rfc-32-standards-enforcement.md` in this repo, or on github at `augentic/specify-cli`). The migration map names which existing imperative predicate row each reserved kind is intended to replace.
2. **Current imperative `Check` impls** at `crates/authoring/src/check/{adapter,agent_teams,brief,docs_quality,links,plugins,prose,rules,scenarios,schema_links,skill_body,skill_frontmatter,tools}.rs`.

If RFC-32's migration map does not name a row for the kind, the card is still legal — land the kind interpreter, the schema annotation removal, and a smoke-test `CORE-*` rule against a synthetic fixture (no imperative `Check` deletion in that case). RFC-34 §F5 permits this; the imperative deletion is gated on parity, not required for the kind landing.

The candidate mappings below are non-normative starting points for the C13–C18 subagents.

### C10 — `reference-resolves` hint kind + `CORE-002` ≅ `links.unresolved`

- **Repos:** `specify-cli` + `specify`.
- **RFC anchors:** §F6 PR 2, §F5.
- **Paths (new):**
  - `crates/specify-lints/src/lint/eval/reference_resolves.rs`.
  - `adapters/shared/rules/core/CORE-002-links-unresolved.md` (in `specify`).
  - `crates/authoring/tests/core_parity_links_unresolved.rs`.
- **Paths (modified):**
  - `schemas/rules/rule.schema.json` — drop `"x-hint-status": "reserved"` from `reference-resolves`.
  - `crates/authoring/src/check/links.rs` — delete the `reference-resolves` row.
- **Depends on:** C9 (chassis fully landed).
- **Parallel with:** C11–C18.

### C11 — `unique` hint kind + `CORE-003` ≅ `skill.duplicate-name`

- **Repos:** `specify-cli` + `specify`.
- **RFC anchors:** §F6 PR 3, §F5.
- **Paths (new):**
  - `crates/specify-lints/src/lint/eval/unique.rs`.
  - `adapters/shared/rules/core/CORE-003-skill-duplicate-name.md` (in `specify`).
  - `crates/authoring/tests/core_parity_skill_duplicate_name.rs`.
- **Paths (modified):**
  - `schemas/rules/rule.schema.json` — drop `"x-hint-status": "reserved"` from `unique`.
  - `crates/authoring/src/check/skill_frontmatter.rs` — delete the duplicate-name row.
- **Depends on:** C9.
- **Parallel with:** C10, C12–C18.

### C12 — `set-coverage` hint kind + `CORE-004`

- **Repos:** `specify-cli` + `specify`.
- **RFC anchor:** §F6 (`set-coverage` listed among reserved kinds).
- **Paths (new):**
  - `crates/specify-lints/src/lint/eval/set_coverage.rs`.
  - `adapters/shared/rules/core/CORE-004-<slug>.md` — seed rule per RFC-32 "Predicate migration map".
  - `crates/authoring/tests/core_parity_<slug>.rs`.
- **Paths (modified):**
  - `schemas/rules/rule.schema.json` — drop `"x-hint-status": "reserved"` from `set-coverage`.
  - Retiring `Check` row — likely `crates/authoring/src/check/adapter.rs` or `crates/authoring/src/check/plugins.rs` if either covers "all expected members of a closed set are present" (candidate: adapter manifest `briefs.keys()` covering all `operations` for an axis). Subagent confirms against RFC-32 migration map.
- **Depends on:** C9.
- **Parallel with:** C10, C11, C13–C18.

### C13 — `cardinality` hint kind + `CORE-005`

- **Repos:** `specify-cli` + `specify`.
- **RFC anchor:** §F6.
- **Paths (new):**
  - `crates/specify-lints/src/lint/eval/cardinality.rs`.
  - `adapters/shared/rules/core/CORE-005-<slug>.md`.
  - `crates/authoring/tests/core_parity_<slug>.rs`.
- **Paths (modified):**
  - `schemas/rules/rule.schema.json` — drop `"x-hint-status": "reserved"` from `cardinality`.
  - Retiring `Check` row — likely `crates/authoring/src/check/skill_body.rs` (line-cap / size predicate at the `200/45/512` cap from skill-authoring standards) or `crates/authoring/src/check/scenarios.rs` (scenario count caps). Subagent confirms.
- **Depends on:** C9.
- **Parallel with:** C10–C12, C14–C18.

### C14 — `constant-eq` hint kind + `CORE-006`

- **Repos:** `specify-cli` + `specify`.
- **RFC anchor:** §F6.
- **Paths (new):**
  - `crates/specify-lints/src/lint/eval/constant_eq.rs`.
  - `adapters/shared/rules/core/CORE-006-<slug>.md`.
  - `crates/authoring/tests/core_parity_<slug>.rs`.
- **Paths (modified):**
  - `schemas/rules/rule.schema.json` — drop `"x-hint-status": "reserved"` from `constant-eq`.
  - Retiring `Check` row — likely a frontmatter field equality check (e.g., `crates/authoring/src/check/skill_frontmatter.rs` constant-value rows) or a manifest field check in `crates/authoring/src/check/adapter.rs`. Subagent confirms.
- **Depends on:** C9.
- **Parallel with:** C10–C13, C15–C18.

### C15 — `set-eq` hint kind + `CORE-007`

- **Repos:** `specify-cli` + `specify`.
- **RFC anchor:** §F6.
- **Paths (new):**
  - `crates/specify-lints/src/lint/eval/set_eq.rs`.
  - `adapters/shared/rules/core/CORE-007-<slug>.md`.
  - `crates/authoring/tests/core_parity_<slug>.rs`.
- **Paths (modified):**
  - `schemas/rules/rule.schema.json` — drop `"x-hint-status": "reserved"` from `set-eq`.
  - Retiring `Check` row — strong candidate: `crates/authoring/src/check/adapter.rs` `briefs.keys() == operations_for_axis(axis)` brief-completeness row (closed set equality). Subagent confirms.
- **Depends on:** C9.
- **Parallel with:** C10–C14, C16–C18.

### C16 — `content-digest-eq` hint kind + `CORE-008`

- **Repos:** `specify-cli` + `specify`.
- **RFC anchor:** §F6.
- **Paths (new):**
  - `crates/specify-lints/src/lint/eval/content_digest_eq.rs`.
  - `adapters/shared/rules/core/CORE-008-<slug>.md`.
  - `crates/authoring/tests/core_parity_<slug>.rs`.
- **Paths (modified):**
  - `schemas/rules/rule.schema.json` — drop `"x-hint-status": "reserved"` from `content-digest-eq`.
  - Retiring `Check` row — strong candidate: `crates/authoring/src/check/agent_teams.rs` symlink-target sha256 row (the framework `agent-teams.md` symlinks docs covered by §F1 "follow symlinks recording both endpoints"). Subagent confirms.
- **Depends on:** C9.
- **Parallel with:** C10–C15, C17, C18.

### C17 — `namespace-owner` hint kind + `CORE-009`

- **Repos:** `specify-cli` + `specify`.
- **RFC anchor:** §F6.
- **Paths (new):**
  - `crates/specify-lints/src/lint/eval/namespace_owner.rs`.
  - `adapters/shared/rules/core/CORE-009-<slug>.md`.
  - `crates/authoring/tests/core_parity_<slug>.rs`.
- **Paths (modified):**
  - `schemas/rules/rule.schema.json` — drop `"x-hint-status": "reserved"` from `namespace-owner`.
  - Retiring `Check` row — strong candidate: `crates/authoring/src/check/rules.rs` `BUILTIN_NAMESPACES` owner-mismatch row (the same module C3 extends). The C17 subagent must coordinate with C3's BUILTIN_NAMESPACES shape (already landed) but no merge conflict expected. Subagent confirms.
- **Depends on:** C9.
- **Parallel with:** C10–C16, C18.

### C18 — Final sweep: confirm no `x-hint-status: reserved` remain

- **Repo:** `specify-cli`.
- **Purpose:** RFC-34 §F6 implies that every reserved kind eventually lands. Once C10–C17 each drop their kind's annotation, this card verifies the cleanup is complete and adds a regression test that fails if a future kind is added without an implementation.
- **Paths (modified):**
  - `schemas/rules/rule.schema.json` — verify no `"x-hint-status": "reserved"` entries remain across the `hints[].kind` `oneOf`.
- **Paths (new, optional):**
  - `crates/specify-lints/tests/no_reserved_hint_kinds.rs` — failing-on-regression integration test that asserts every kind in the schema has a matching `eval/<kind>.rs` file. Cheap insurance against the schema and the interpreter drifting again.
- **Depends on:** C10–C17.
- **Parallel with:** None (this is the closer).
- **Definition of done:**
  - `rg "x-hint-status.*reserved" schemas/` returns zero hits.
  - `cargo make ci` green; new regression test passes.

---

## Subagent dispatch summary

| Wave | Changes | Parallelism | Owning repos |
| --- | --- | --- | --- |
| 1 | C1, C2, C3 | 3-wide parallel | `specify-cli` |
| 2 | C4, C5 | 2-wide parallel | `specify-cli` |
| 3 | C6 | sequential after C5 | `specify-cli` |
| 4 | C7, C8 | 2-wide parallel (coordinate landing) | `specify`, `specify-cli` |
| 5 | C9 | sequential after C8 | `specify-cli`, `specify` |
| 6 | C10–C17 | 8-wide parallel | `specify-cli`, `specify` |
| 7 | C18 | sequential after C10–C17 | `specify-cli` |

A single `generalPurpose` subagent can typically handle one card per dispatch — every card is bounded to one focused diff, the touched paths are enumerated, and the definition-of-done is mechanical (`cargo make ci` / `make check` plus the named golden or parity fixture).

## Cross-repo coordination notes

- Per `specify-cli/AGENTS.md` step 5, any change that touches `crates/specify-lints/src/rules/`, `crates/specify-lints/src/lint/`, `crates/schema/src/`, or `crates/domain/src/adapter/` must `rg` across **both** repos and update every hit in the same PR. C1, C3, C5, C6, and every C10–C17 tickle one or more of those modules — dispatch each subagent with explicit instruction to run the cross-repo grep.
- The `CORE-001` rule (C7) and its parity test (C8) span repos. If your subagent runtime cannot land cross-repo atomically, sequence C7 first (rule is harmless without a consumer), then C8 (parity import resolves once C7 merges). Same pattern applies to every C10–C17 pair (the `CORE-*` rule lands in `specify`; the interpreter, parity test, and imperative retirement land in `specify-cli`).
- Documentation (C9) is the chassis cleanup wave — it can be deferred until after C10–C18 ship if landing windows are tight, but RFC-34's `§"Acceptance (chassis PR)"` requires updated `docs/contributing/checks.md` for the chassis to be considered complete.

## Acceptance per RFC-34

Chassis closes (C1–C9) when, per RFC-34 §"Acceptance (chassis PR)":

- `cargo make ci` is green in `specify-cli`.
- `make check` is green in `specify`.
- `specdev lint --format json` produces a stable envelope against the framework repo with `CORE-001` seeded.
- `specrun rules export` without `--include-core` excludes every `CORE-*` rule (golden test from C4).
- A `lint-completed` event lands in `.specify/journal.jsonl` per `specdev lint` run.

Each per-kind PR (C10–C17) closes per RFC-34 §"Acceptance (per-kind PR)":

- Parity fixture passes byte-identically against the retiring predicate's existing goldens (or smoke test passes if RFC-32's migration map does not name a retiring row).
- `make check` green with the imperative `Check` row deleted (if applicable).
- The reserved hint kind's `"x-hint-status": "reserved"` annotation is gone from `schemas/rules/rule.schema.json`.

C18 closes when every reserved kind named in RFC-34 §F6 is implemented and no `"x-hint-status": "reserved"` entry remains in `schemas/rules/rule.schema.json`. At that point RFC-34 is fully implemented and may be moved to `rfcs/done/`.
