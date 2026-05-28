# RFC-34: `CORE-*` Rules

> Status: Accepted · Depends: [RFC-28](done/rfc-28-standards-contract.md) (codex contract + `Origin` enum amendment), [RFC-32](done/rfc-32-standards-enforcement.md) Phase 2 (WorkspaceModel + hint interpreter), [RFC-33a](rfc-33a-ignore-directives.md) (co-requisite — `specdev lint` inherits `status: ignored`, `disposition.directive`, and `lint-completed` verbatim per §F7) · Enables: optional follow-on to [roadmap RM-10](roadmap.md#rm-10-ci-native-standards-enforcement)

## Abstract

[RFC-32](done/rfc-32-standards-enforcement.md) Phase 2 ships consumer-side deterministic enforcement: `specrun lint` walks a consumer project, builds a [WorkspaceModel](done/rfc-32-standards-enforcement.md#workspacemodel), evaluates `deterministic_hints` against applicable rules from `specrun rules export`, and emits `LintFinding` JSON. Framework-repo enforcement (this repo's own `make check`) still runs through ~30 imperative Rust predicates in `crates/authoring/src/check/`. [RFC-28](done/rfc-28-standards-contract.md) Phase 3 made those predicates emit `LintFinding` JSON via `specdev lint --format json` but did not move them into the codex layer.

RFC-34 adds the optional **framework convergence** layer that lets new framework checks be authored as declarative rules instead of new Rust predicates:

1. `**CORE-`* rules under `adapters/shared/rules/core/`** — first-class first-party rules using the same hint interpreter as `UNI-`* / target-namespaced rules.
2. **Framework scan profile** — `scan_profile: framework` extractors for skills, adapters, marketplace, agent-teams symlinks, briefs.
3. `**specdev lint` CLI extension** — framework-side counterpart to `specrun lint`, drives the framework-profile indexer plus hint interpreter alongside existing imperative predicates.
4. `**Origin::Core` amendment to RFC-28** — wire-format extension so `Core` findings are distinguishable from `Shared` ones in `LintFinding` envelopes.
5. `**--include-core` consumer opt-in** — core rules are excluded from consumer codex exports by default; opt-in via flag.
6. **Hand-author migration cadence** — closed policy for moving imperative `Check` predicates to `CORE-`* rules without losing CI coverage.

This RFC adds no lifecycle authority. `CORE-`* findings may block `make check` and CI, but never transition plan entries, slices, or changes — the same contract that already governs `specrun lint`.

## Motivation

After RFC-32 Phase 2 lands, the framework repository has two enforcement surfaces that share substrate but not authoring:

- Consumer projects: write a rule, add a `deterministic_hints` block, `specrun lint` fires it.
- Framework repository: write a Rust `Check` impl, register it in `crates/authoring/src/check.rs`, ship a binary update.

Three pressure points push toward convergence:

- **Authoring asymmetry punishes framework contributors.** A first-time contributor proposing a new framework check today writes Rust + tests + a registration line + a docs entry. The consumer-side equivalent is a markdown file. Equalizing the bar is the main motivation for `CORE-`*.
- **Imperative-predicate sprawl recreates itself.** Every new cross-file invariant grows another bespoke walk. The hint interpreter's reserved kinds (`unique`, `set-coverage`, `reference-resolves`, `set-eq`, `content-digest-eq`, …) cover the exact graph shapes most existing predicates implement; using them once on the framework side proves out the reserved kinds for the consumer side, and vice versa.
- `**specdev lint --format json` already speaks `LintFinding`.** RFC-28 Phase 3 mapped imperative findings into the same envelope. Adding declarative findings to that envelope is the next step, not a rewrite — the wire output stays compatible whether a finding came from a `Check` impl or a `CORE-`* rule's hint.

## Principles

1. `**CORE-`* is additive, not a replacement.** Imperative `Check` predicates remain valid indefinitely. Migration is per-predicate and reversible until the imperative impl is deleted.
2. **One substrate, two CLIs.** `specdev lint` and `specrun lint` share `specify-lints::lint` modules (renamed from `specify-domain::review` by the codex→lints rename) but keep separate binaries and separate failure semantics, per RFC-32 §"Framework and consumer scans share libraries, not commands."
3. **No consumer surprise.** Consumer-project `specrun lint` / `specrun rules export` invocations never include `CORE-`* rules unless `--include-core` is set.
4. **Wire compatibility.** `Origin::Core` is the only RFC-28 amendment. The `LintFinding` shape, the fingerprint algorithm, the severity enum, and the evidence union all stay as RFC-28 defined them.
5. **Parity before deletion.** An imperative `Check` only retires when an equivalent `CORE-`* rule produces byte-identical findings against the predicate's existing golden fixtures.

## RFC-28 amendments

RFC-28 is in `rfcs/done/`. RFC-34 normatively extends it in two narrow places.

### A1 — `Origin` enum

RFC-28 §"Resolved codex export" declares the closed enum as:

```text
origin: shared | source | target | organization
```

RFC-34 widens it to:

```text
origin: shared | source | target | core | unknown
```

`Core` is the origin assigned to rules resolved from `adapters/shared/rules/core/` (RFC-28 §"Resolution roots" root 2, activated with pack name `core` per §F3 below). The `schemas/rules/resolved.schema.json` enum widens in the same PR; consumer parsers that strictly type the enum see this as a non-breaking value addition because no previous value is removed or renamed.

### A2 — Origin sort order

RFC-28 line 285 declares the stable origin ordering as `target, source, shared, organization`. RFC-34 inserts `core` between `shared` and the tail value (renamed to `unknown` per §A4):

```text
target → source → shared → core → unknown
```

The justification mirrors RFC-28's existing rationale: `core` is more specific than `shared` (it scopes to the framework repository), less specific than per-adapter rules (which scope to one adapter). Placing it before `unknown` keeps every recognized origin tier ahead of the indexer's fallback bucket.

### A3 — Consumer opt-in flag

`specrun rules export` and `specrun lint` accept a new flag:

```text
--include-core    Include CORE-* rules from adapters/shared/rules/core/
                  in the resolved set. Default: false (core rules are
                  hidden from consumer exports).
```

Semantics mirror RFC-28's existing `--include-deprecated` / `--include-unmatched` flags: default exclude, opt-in include, no state. The flag and its default are pinned here so RFC-32 Phase 2 implementations may add the flag pre-emptively without an RFC-32 amendment when they ship.

### A4 — Rename `organization` → `unknown`

RFC-28 §"Resolved codex export" declared `organization` as the fifth value in the closed `Origin` enum, "reserved for project-local overlays" (resolver root 5). Root 5 is still unimplemented, and the variant is doing a different load-bearing job today:

The consumer indexer `infer_origin` in `crates/specify-lints/src/lint/index/discover.rs` returns `Origin::Organization` for any `*.md` rule under `.specify/cache/rules/` whose path does not match the three recognized adapter shapes (`adapters/targets/<name>/rules/…`, `adapters/sources/<name>/rules/…`, `adapters/shared/rules/universal/…`). It is the indexer's fallback bucket, not the resolver's project-local overlay slot.

RFC-34 renames the variant from `organization` to `unknown` so the name matches the load-bearing job. The wire-format value becomes `unknown`; the sort position stays last so unrecognized cache files sort behind every recognized origin tier. The schema descriptions in `schemas/rules/resolved.schema.json` and `schemas/lint/workspace-model.schema.json` are updated in the same PR to read "indexer fallback for rule files whose path does not match a recognized adapter shape" instead of "reserved for project-local overlays."

When/if a future RFC activates resolver root 5, that RFC introduces a fresh variant (e.g. `overlay`) and picks its own sort position. `unknown` continues to mean "indexer fallback."

This is a hard wire break: RFC-28's forward-compatibility note covers enum *additions*, not *renames*. Strict parsers that pinned the literal value `organization` will need to update — the rename lands in the same chassis PR as RFC-34's `core` addition so consumer-side schema parsers absorb both updates in one envelope-schema bump.

A1, A2, A3, and A4 are the only RFC-28 changes. The finding schema, fingerprint algorithm, severity enum, evidence union, and resolution-input shape are unchanged.

## Design

### F1 — Framework scan scope

Symmetric counterpart to [RFC-32 §D1](done/rfc-32-standards-enforcement.md#d1--consumer-scan-scope). `scan_profile: framework` walks the framework repository (`augentic/specify`) with the following defaults:

- **Roots.** `project_dir` itself (always the framework-repo root for `specdev lint` invocations), plus any path explicitly named in `artifact_paths[]`.
- **Default include globs.** `adapters/`**, `plugins/`**, `docs/**`, `.cursor/**`, `rfcs/**`, `scripts/**`, `schemas/**`, `**/AGENTS.md`, `**/REVIEW.md`. Wider than the consumer profile because the framework repo's source of truth is markdown and YAML across many trees.
- **Always-ignore globs.** `target/`**, `**/node_modules/`**, `.git/**`, `dist/**`, `.specify/**` (the framework repo MAY carry a project-local `.specify/` for self-hosting; framework scans do not enter it), and every path matching the project-root `.gitignore`.
- **Symlink policy.** Framework `agent-teams.md` files symlink into `docs/reference/`. The framework profile **follows** symlinks (recording both endpoints) so review-team-protocol drift is visible. This is the opposite of the consumer profile's record-without-traverse rule and is normative. Symlink cycles abort with a `Filesystem` error.
- **Binary files.** Same NUL-byte detection as RFC-32 §D1; binary files emit `file { kind: "binary" }` facts and are skipped by `regex` hints unless the rule sets `applicability.binary: true`.
- **Encoding.** UTF-8 with U+FFFD replacement; one `index.warning` finding per non-UTF-8 file at severity `optional`.
- **Determinism.** Enumeration is sorted by project-relative path before parallel dispatch (matches RFC-32 §D1).
- **Codex parse.** Codex trees under `adapters/{shared,sources,targets}/**/rules/` are parsed using the existing RFC-28 codex parser. The framework profile additionally accepts `CORE-`* rules at `adapters/shared/rules/core/` per §F3.

### F2 — `specdev lint` CLI surface

Extend the `specdev lint` subcommand so it mirrors `specrun lint`'s shape while defaulting to the framework profile and the current directory as both codex root and scan root:

```bash
specdev lint                                      # full framework lint
specdev lint --rule CORE-001                     # single-rule debug
specdev lint --artifact docs/standards/style.md   # narrow scope
specdev lint --dump-model                         # WorkspaceModel debug
specdev lint --strict-hints                       # treat reserved kinds as failures
specdev lint --format json                        # CI-consumable envelope
```

**Defaults pinned for `specdev lint` only** (these differ from `specrun lint`):

- `--rules-root` defaults to `.` (the framework repo's own codex tree resolves shared `UNI-*` and `CORE-*` rules without any flag).
- `--scan-profile` is hard-coded to `framework`; the flag does not exist on this verb. (A separate `specrun lint --scan-profile framework` form is deliberately not introduced to avoid two ways to run the same scan; framework profile is `specdev lint`'s sole reason for existing.)
- `--target` is optional and defaults to "none". A framework scan does not have a single target adapter; `applicability.adapters` filtering against framework files is rare. When supplied, the flag narrows applicability the same way it does on `specrun lint`.

**Shared with `specrun lint`** (per RFC-32 §D6 / §D7 / §D8 / §D9):

- The four formatters (`json`, `pretty`, `github`, `compact`) live in `specify-lints::lint::diagnostics` (`crates/specify-lints/src/lint/diagnostics/`).
- The exit-code map (RFC-32 §D8) applies verbatim; `--format json` validates against `schemas/review/lint-result.schema.json` before emit (RFC-32 §D9).
- The hint evaluation order is `path-pattern → schema → regex → tool`.
- Reserved-hint policy (RFC-32 §D5) and `--strict-hints` semantics are identical.

The handler lives under `src/authoring/commands/lint/{cli.rs, run.rs}` in `augentic/specify-cli` (mirroring `src/runtime/commands/lint/` for `specrun lint`). The `specify-authoring` crate gains a dependency on `specify-lints` for this verb; the existing imperative `Finding` → `LintFinding` Phase 3a mapper at `src/authoring/map_finding.rs` remains part of `specdev lint --format json`.

### F3 — `check::rules` placement and resolution

Two `check::rules` changes activate `CORE-*` placement in the framework repo without weakening existing constraints:

1. **`STATIC_RULE_PROFILE_NAMESPACES` extension.** Map the new path `adapters/shared/rules/core/` → `{"CORE"}` in the static map under `crates/authoring/src/check/rules.rs` (the constant the legacy `CODEX_PROFILE_NAMESPACES` was renamed to by the codex→lints rename). Owner discovery uses the same first-segment-under-`adapters/` rule already in place; the core pack appears as a peer of `universal/` rather than as a per-adapter overlay.
2. **Placement predicate (lift, then re-apply).** RFC-28 Phase 1 step 2 added a predicate rejecting `CORE-`* under `adapters/{sources,targets}/<name>/rules/`. That rejection stays; `CORE-`* rules under per-adapter trees remain a `check::rules` failure. The predicate additionally REQUIRES `CORE-*` placement under `adapters/shared/rules/core/` (a non-`CORE-*` rule there is rejected with the same `codex-namespace-ownership-violation` rule id).

**Resolution root activation.** RFC-28 §"Resolution roots" line 138 reserves root 2: "Shared language or artifact packs, if added later under `adapters/shared/rules/<pack>/`." RFC-34 activates root 2 with pack name `core`. The resolver walks the new pack root immediately after `adapters/shared/rules/universal/`; rules are tagged with `origin: core` per A1. No new root order or precedence is introduced.

**Consumer-export filtering.** `specrun rules export` filters out `origin: core` rules unless `--include-core` (A3) is set. `specrun lint` inherits that filter from the resolver — consumer-project review runs never evaluate `CORE-`* hints by accident.

### F4 — Per-rule applicability for framework files

Core rules use the existing `applicability` block, with two framework-specific tokens added to the closed `applicability.artifacts` enum:

```text
artifacts:
  - skill          # plugins/**/SKILL.md frontmatter + body
  - adapter        # adapters/**/adapter.yaml manifests
  - brief          # adapters/**/briefs/*.md
  - reference      # adapters/**/references/*.md
  - codex          # rule files themselves
  - rfc            # rfcs/**/*.md
  - doc            # docs/**/*.md
```

These tokens compose with the existing consumer-side ones (`code`, `tests`, `contracts`, `specs`, `design`, `tasks`); a single `applicability.artifacts` entry like `[skill, adapter]` is legal. The full enum is closed; widening it is a `check::rules` schema change reviewed in the same PR as the new value.

### F5 — Migration cadence

Hand-authored only (already pinned in RFC-32 §"Phase 3 — framework convergence" Option B). RFC-34 adds the executable rule:

- An imperative `Check` impl retires only when a `CORE-*` rule plus its hint coverage produces **byte-identical** findings against the predicate's existing golden fixtures. The parity test (`crates/authoring/tests/core_parity_<rule>.rs`) lands in the same PR as the imperative deletion.
- Until parity is proven, both the imperative `Check` and the `CORE-`* rule may run. Duplicate findings against the same `(rule-id, location)` are suppressed by fingerprint deduplication in `specdev lint`'s envelope emission step (the existing RFC-28 fingerprint algorithm already produces identical fingerprints for identical evidence; this requires no new code).
- Migration order seeds the `High`-priority rows from RFC-32's "Predicate migration map": `adapter.`* (schema), `skill.* (frontmatter)` (schema + unique + regex), `links.*` (reference-resolves). Other rows migrate when contributor demand emerges.

### F6 — Reserved hint kind dependency

`CORE-*` rules SHOULD prefer Phase-2-implemented hint kinds (`regex`, `path-pattern`, `schema`, `tool`) for first-wave migrations. Rules requiring reserved kinds (`unique`, `set-coverage`, `reference-resolves`, `cardinality`, `constant-eq`, `set-eq`, `content-digest-eq`, `namespace-owner`) ship paired with the interpreter implementation for that kind; the schema authoring annotation `"x-rfc32-status": "reserved"` is dropped from the kind in the same PR.

The PR pattern is therefore: kind implementation in `crates/specify-lints/src/lint/eval/<kind>.rs` + schema annotation removal in `crates/authoring/schemas/rule.schema.json` + first CORE-* rule using the kind + parity fixture against the retiring imperative `Check`. Reviewer can verify the full chain in one diff.

### F7 — Interaction with RFC-33a

RFC-33a widens `WorkspaceModel` with `ignore_directives`, the `LintFinding` envelope with `status: ignored` / `disposition.directive` (and `false-positive`), and the closed `EventKind` taxonomy with `lint-completed`. The framework scan profile inherits all three additions verbatim — no new RFC-33a wire shape is introduced by RFC-34:

- **In-source ignore directives in framework artifacts.** The directive grammar (`specify-ignore: <RULE-ID> — <rationale>`) participates wherever the §F1 include globs walk. RFC-33a's closed comment-style list (D3) already covers the dominant framework form via `<!-- … -->` (used across `adapters/**/briefs/*.md`, `docs/**/*.md`, `rfcs/**/*.md`, `**/AGENTS.md`, `**/REVIEW.md`). No new comment style is added.
- **`UNI-022` / `UNI-023` synthetic findings.** The directive-validation pass (RFC-33a §"Implementation plan" step 4) runs identically under `specdev lint`. Directives lacking a rationale fire `UNI-022`; directives whose `rule-id` matches no finding on the target line fire `UNI-023`. Both rules resolve under the framework-repo codex tree without `--rules-root` because they live under `adapters/shared/rules/universal/`.
- **`lint-completed` journal emission.** `specdev lint` emits exactly one `lint-completed` event per run, mirroring the `specrun lint` emission point in `src/runtime/commands/lint/run.rs`. The payload uses `scope.target: None`, `scope.slice: None`, `scope.artifact` reflecting any `--artifact <path>` flag value, and the hard-coded `baseline_present: false` already used by RFC-33a producers.

**Landing order.** RFC-34 is sized to land after RFC-33a's wave 4 (journal emission + acceptance goldens) is merged in `specify-cli`, so the framework profile inherits a settled `WorkspaceModel` shape and `LintFinding` status taxonomy. If RFC-34's chassis PR lands first, the chassis MUST stub the inherited fields (`disposition: None`, no `lint-completed` emission) and a follow-on PR wires them in once RFC-33a settles; this is permitted but discouraged because it doubles the framework-side review surface.

## Implementation Plan

RFC-34 lands as a **convergence-chassis PR** across `augentic/specify-cli` and `augentic/specify` (steps 1–3 + 4a + 5a), followed by **one PR per reserved hint kind** (each carrying interpreter + schema annotation removal + one CORE-* rule + parity fixture + imperative deletion per §F6 / §F5). The chassis PR is the only one that touches the framework extractors, the `specdev lint` verb, and the `Origin` enum widening; reserved-kind PRs reuse the chassis without re-touching those surfaces. This keeps each reserved-kind PR to a single new hint kind, matching §F6's "Reviewer can verify the full chain in one diff."

**Chassis PR (steps 1–3 + 4a + 5a):**

1. **Schema + predicate updates.** Add `core` to the closed `Origin` enum in `schemas/rules/resolved.schema.json`. Extend `STATIC_RULE_PROFILE_NAMESPACES` in `crates/authoring/src/check/rules.rs` to map `adapters/shared/rules/core/` → `{"CORE"}`. Update `check::rules` placement predicate to require `CORE-*` at the new path and reject non-`CORE-*` rules there. Add the `--include-core` flag to `specrun rules export` (default off; no behaviour change without the flag).
2. **Framework scan profile.** Implement `scan_profile: framework` extractors under `crates/specify-lints/src/lint/index/{skill.rs, adapter.rs, marketplace.rs, agent_teams.rs, brief.rs}`. Reuse `index/files.rs`, `index/frontmatter.rs`, `index/markdown.rs`, `index/symlinks.rs` from Phase 2; symlink policy changes per §F1 (follow instead of record).
3. **`specdev lint` CLI extension.** Extend `src/authoring/commands/lint/{cli.rs, run.rs}` in `augentic/specify-cli`. `specify-authoring` gains a `specify-lints` dependency (allowed; the binary boundary already imports `specify-lints` for the RFC-28 Phase 3 mapper at `src/authoring/map_finding.rs`). Wire export → index → eval → envelope per §F2; ship `--dump-model`, all four formatters, `--strict-hints`, and the exit-code map from RFC-32 §D8. Emit one `lint-completed` event per run per §F7.
4. **First CORE-* rule (landed-kind only).** Hand-author `CORE-001` ≅ `adapter.schema` under `adapters/shared/rules/core/` with a `## Rule` body and a `deterministic_hints` block using the landed `schema` hint kind. The existing imperative `check::adapter` schema-row goldens are the parity fixture's input.
5. **First parity test + imperative retirement.** Land `crates/authoring/tests/core_parity_adapter_schema.rs` proving byte-identical findings versus the existing predicate goldens. Delete the matching imperative `Check::adapter` schema-row impl in the same PR. Update `docs/contributing/checks.md` to point at the rule instead of the predicate.

**Per-kind follow-on PRs.** Each reserved hint kind §F6 names ships in its own PR with the same five-element checklist (kind impl + schema annotation removal + one CORE-* rule + parity fixture + imperative deletion):

| Order (independent of each other) | Hint kind | Seed CORE rule | Imperative retired |
| --- | --- | --- | --- |
| PR 2 | `reference-resolves` | `CORE-002` ≅ `links.unresolved` | `check::links` reference-resolves row |
| PR 3 | `unique` | `CORE-003` ≅ `skill.duplicate-name` | `check::skill_frontmatter` duplicate-name row |

PR 2 and PR 3 are independent of each other once the chassis PR lands and can dispatch to separate subagents in parallel. Further reserved-kind PRs (`set-coverage`, `cardinality`, `constant-eq`, `set-eq`, `content-digest-eq`, `namespace-owner`) land in the same shape as contributor demand reaches them — RFC-34 does not pre-schedule them.

**Acceptance (chassis PR):** `cargo make ci` green; `make check` (parent repo) green; `specdev lint --format json` produces a stable envelope against the framework repo with `CORE-001` seeded; consumer `specrun rules export` without `--include-core` excludes every `CORE-*` rule (golden test); a `lint-completed` event lands in `.specify/journal.jsonl` per `specdev lint` run.

**Acceptance (per-kind PR):** parity fixture passes byte-identically against the retiring predicate's existing goldens; `make check` green with the imperative `Check` deleted; the reserved hint kind's schema annotation (`"x-rfc32-status": "reserved"`) is gone from `crates/authoring/schemas/rule.schema.json`.

## Implementation Guide

Non-normative notes for the agent or contributor picking up RFC-34. The RFC body (§Design + §"Implementation Plan") is the source of truth; items here may evolve in PR review without an RFC amendment.

### Module additions

Phase 2's `crates/specify-lints/src/lint/` tree (renamed from `crates/domain/src/review/` by the codex→lints rename) grows the framework extractors:

```text
crates/specify-lints/src/lint/index/
├── skill.rs              # plugins/**/SKILL.md frontmatter + body
├── adapter.rs            # adapters/**/adapter.yaml manifest facts
├── marketplace.rs        # .cursor-plugin/marketplace.json graph
├── agent_teams.rs        # symlink target + sha256 facts
└── brief.rs              # adapters/**/briefs/*.md sections + size
```

Existing extractors (`files.rs`, `frontmatter.rs`, `markdown.rs`, `symlinks.rs`) are reused under both profiles; `symlinks.rs` gains a `follow: bool` mode parameter so the consumer/framework split lives in one place.

### CLI surface under `specify-authoring`

```text
src/authoring/commands/lint.rs          # umbrella
src/authoring/commands/lint/cli.rs      # LintAction subcommand enum
src/authoring/commands/lint/run.rs      # handler
```

Mirrors the `src/runtime/commands/lint/` tree introduced by RFC-32 Phase 2. The clap-derive shape is the same as RFC-32 §"Implementation Guide" `LintAction` modulo the flag defaults pinned in §F2.

### Cargo dependency edges

`crates/authoring/Cargo.toml` gains `specify-lints.workspace = true`. This edge is permitted: the workspace graph in `specify-cli` `AGENTS.md` already has `specify-authoring` depending on `specify-lints` indirectly via the `specdev` binary's RFC-28 Phase 3 mapper at `src/authoring/map_finding.rs`. RFC-34 promotes that to a direct crate-level dependency. (The constraint that `specify-lints` and `specify-domain` are siblings — neither importing the other — is preserved; `specify-authoring` is allowed to depend on `specify-lints` because the standards-layer / workflow-layer split lives between the two non-authoring crates.)

### Test layout

```text
# Chassis PR
crates/authoring/tests/core_parity_adapter_schema.rs
crates/specify-lints/tests/lint_framework_indexer.rs
crates/specify-lints/tests/fixtures/lint/framework_minimal/
tests/specdev_lint.rs                    # binary-level end-to-end

# Per-kind follow-on PRs (one per reserved hint kind)
crates/authoring/tests/core_parity_links_unresolved.rs       # PR 2 (reference-resolves)
crates/authoring/tests/core_parity_skill_duplicate_name.rs   # PR 3 (unique)
```

Use the existing `REGENERATE_GOLDENS` convention. The framework-minimal fixture is small (~10 files) but exercises every framework extractor at least once.

### Documentation touch-points (post-merge)

- `specify-cli` `AGENTS.md` — update the `specdev lint` documentation map; list `crates/specify-lints/src/lint/index/{skill,adapter,marketplace,agent_teams,brief}.rs` under modules of note.
- `specify-cli` `docs/standards/architecture.md` — extend the standards-layer module section with the framework-profile extractors (the framework profile is a sibling of the consumer profile under `specify-lints`, not a `specify-domain` addition).
- `specify` `docs/contributing/checks.md` — explain how a contributor chooses between writing a new imperative `Check` and a new `CORE-*` rule. Default recommendation: `CORE-*` unless the predicate needs a subprocess that cannot be modelled as a `tool` hint.
- `specify` `adapters/shared/rules/core/README.md` — new file. Lists conventions for `CORE-*` rule authoring (body structure, applicability tokens, hint-kind preference) and points at the migration map in RFC-32 §"Predicate migration map".

## Migration

**For framework contributors:** New checks SHOULD be authored as `CORE-`* rules under `adapters/shared/rules/core/` unless the predicate requires subprocess orchestration or stateful behaviour that the hint interpreter cannot model. The `## Rule` body is the canonical agent-readable explanation; the `deterministic_hints` block makes the rule fire under `specdev lint`. Existing imperative `Check` impls remain valid; migrate only when parity is achievable.

**For consumer projects:** No change from RFC-32 Phase 2 baseline. `specrun lint` and `specrun rules export` continue to exclude `CORE-`* rules. Pass `--include-core` only if your project deliberately wants to enforce framework-authoring rules against your own tree (rare; typically only relevant to projects that vendor parts of the framework repo).

**For RFC-28-aware tooling:** The closed `Origin` enum widens with one value (`core`). Strict parsers MUST treat unknown enum values as a forward-compatibility extension (i.e., they should not crash on `core` if they predate RFC-34); RFC-28 already specifies forward-compatible behaviour for closed enums in its evidence-union and severity-enum sections.

**For CI integrations consuming `specdev lint --format json` (RFC-28 Phase 3):** Output remains wire-compatible as RFC-34 adds declarative `CORE-`* findings to the same envelope. Integrations that already consume `LintFinding` can aggregate imperative and declarative findings without changing schema handling.

## Alternatives Considered

**Fold framework convergence back into RFC-32.** Rejected. RFC-32 is now `Accepted` and ships Phase 2 cleanly. Re-opening it to absorb the `Origin` amendment, framework scan-scope, and `specdev lint` framework-rule extension would delay RM-10 and inflate RFC-32 past 750 lines covering two distinct audience scopes. The carve-out matches the precedent set by RFC-28 → RFC-32 → RFC-33a / RFC-33b.

**Run `CORE-`* through `specrun lint --scan-profile framework`.** Rejected. RFC-28 §"CLI and binary split" preserves a hard `specrun` (operator-facing) / `specdev` (contributor-facing) split. Adding a framework-profile flag to `specrun lint` would erode that split for one feature and force the operator-facing binary to ship framework extractors it never needs.

**Keep separate imperative and declarative framework verbs.** Rejected. The framework-facing command should match the consumer-facing `specrun lint` vocabulary, and RFC-28 already made imperative authoring findings speak the `LintFinding` envelope. A single `specdev lint` surface keeps contributors focused on "framework lint" while implementation can still keep imperative predicates and declarative `CORE-`* evaluation as separate internal stages.

**Use a different root for `CORE-`* (e.g. `tooling/rules/`).** Rejected during RFC-32 drafting (see RFC-32 §Resolved Decisions). Keeping every codex tree under `adapters/`** reuses `check::rules` owner discovery, the resolver root walk, and the `origin:` filter without growing a second root.

**Skip `Origin::Core`; reuse `Shared`.** Rejected. The point of `Origin` is to let consumers filter by source; collapsing `core` into `shared` would make `--include-core` impossible to implement without a parallel discriminant. Adding one enum value once is cheaper than two-layer discriminants forever.

*Automate `Check` → `CORE-` translation.** Rejected (already in RFC-32 §"Phase 3 — framework convergence"). Hand-authoring is the validation pass for whether a reserved hint kind models the predicate; an auto-porter would mechanically translate without that scrutiny and would still get the Low-priority rows (`scenarios.`*, `tools.`*) wrong.

## Non-Goals

- Mandatory migration of every imperative `Check` to `CORE-*`. Imperative predicates may live indefinitely; RFC-34 only enables migration on demand.
- A `specdev lint --scan-profile consumer` mode. The consumer profile is `specrun lint`'s domain.
- New severity, fingerprint, or evidence semantics. RFC-28 + RFC-32 already cover these.
- Lifecycle authority for `CORE-*` findings. Findings may block CI; they never transition plans, slices, or changes.
- SARIF export (deferred per RFC-32 non-goals).
- Per-source-adapter framework rules. Source adapter overlays use `SRC-*`; framework-level rules use `CORE-*`. No cross-namespace mixing.
- A consumer `--prefer-core` mode that auto-includes core rules. `--include-core` is the only opt-in surface.

## Resolved Decisions

Every design question raised while drafting RFC-34 is resolved in the body. The list below indexes the resolutions for reviewers checking that no question is parked.

- `**CORE-*` placement** — §F3 (`adapters/shared/rules/core/`, activates RFC-28 reserved resolution root 2 with pack name `core`).
- `**Origin` enum widening** — §A1 + §A2 (add `core`, sort between `shared` and the tail value).
- `**organization` → `unknown` rename** — §A4 (variant renamed so the name matches its load-bearing job as the indexer fallback bucket; reserved-overlay slot vacated and available for a fresh variant from a future RFC).
- **Consumer opt-in flag** — §A3 (`--include-core`, default off, mirrors `--include-deprecated`).
- **CLI surface for framework execution** — §F2 (`specdev lint`, not a `specrun` flag, not a separate declarative-only verb).
- **Framework scan scope** — §F1 (symmetric to RFC-32 §D1; symlinks follow instead of record; wider include globs).
- **Hint-kind preference for first-wave rules** — §F6 (Phase 2 kinds preferred; reserved kinds ship paired with their interpreter implementations).
- **Migration cadence** — §F5 (hand-authored, byte-identical parity fixture required for imperative deletion, dedup via existing fingerprint algorithm during overlap).
- `**check::rules` predicate inversion** — §F3 (placement predicate requires `CORE-`* at the new path and rejects non-`CORE-`* rules there, while keeping the existing rejection under per-adapter trees).
- `**applicability.artifacts` framework tokens** — §F4 (`skill`, `adapter`, `brief`, `reference`, `codex`, `rfc`, `doc` added to the closed enum).
- **First-wave PR sequencing** — §"Implementation Plan" (one convergence-chassis PR ships steps 1–3 + `CORE-001` using the landed `schema` hint kind; reserved-kind PRs ship per §F6's pairing rule, one new hint kind per PR, dispatchable in parallel after the chassis lands).
- **RFC-33a interaction** — §F7 (framework profile inherits `ignore_directives`, `status: ignored` / `disposition.directive`, and the `lint-completed` journal event verbatim; `specdev lint` emits exactly one `lint-completed` event per run; landing order is sized to follow RFC-33a wave 4).

## References

- [RFC-28: Engineering Standards — Codex Contract and Findings](done/rfc-28-standards-contract.md) — finding shape, codex resolution, the `Origin` enum widened by §A1.
- [RFC-32: Engineering Standards — Deterministic Enforcement](done/rfc-32-standards-enforcement.md) — Phase 2 substrate (WorkspaceModel, hint interpreter, `specrun lint`).
- [RFC-33a: Standards Ignore Directives](rfc-33a-ignore-directives.md) — in-source ignore directive + telemetry layer that applies equally to `specdev lint` and `specrun lint` envelopes.
- [RFC-33b: Standards Baseline](future/rfc-33b-standards-baseline.md) — deferred cross-run baseline + diff layer; lands when the trigger conditions in RFC-33b §"Trigger conditions" are met.
- [RFC-5: Framework Developer Tooling](done/rfc-5-tooling.md) — `specdev` binary contract.
- [Specify Roadmap — RM-10](roadmap.md#rm-10-ci-native-standards-enforcement) — CI-native standards enforcement; RFC-34 is optional relative to RM-10.
- [Standards layer (explanation)](../docs/explanation/standards-layer.md)
- [docs/contributing/checks.md](../docs/contributing/checks.md) — receives the new "choose between `Check` and `CORE-`*" guidance per §"Documentation touch-points".

