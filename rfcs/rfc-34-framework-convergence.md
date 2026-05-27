# RFC-34: Framework Convergence — `FRAME-*` Rules and Framework Scan Profile

> Status: Draft · Depends: [RFC-28](done/rfc-28-standards-contract.md) (codex contract + `Origin` enum amendment), [RFC-32](rfc-32-standards-enforcement.md) Phase 2 (WorkspaceModel + hint interpreter) · Enables: optional follow-on to [roadmap RM-10](roadmap.md#rm-10-ci-native-standards-enforcement)

## Abstract

[RFC-32](rfc-32-standards-enforcement.md) Phase 2 ships consumer-side deterministic enforcement: `specrun review` walks a consumer project, builds a [WorkspaceModel](rfc-32-standards-enforcement.md#workspacemodel), evaluates `deterministic_hints` against applicable codex rules from `specrun codex export`, and emits `ReviewFinding` JSON. Framework-repo enforcement (this repo's own `make check`) still runs through ~30 imperative Rust predicates in `crates/authoring/src/check/`. [RFC-28](done/rfc-28-standards-contract.md) Phase 3 made those predicates emit `ReviewFinding` JSON via `specdev check --format json` but did not move them into the codex layer.

RFC-34 adds the optional **framework convergence** layer that lets new framework checks be authored as declarative codex rules instead of new Rust predicates:

1. **`FRAME-*` codex rules under `adapters/shared/codex/framework/`** — first-class first-party rules using the same hint interpreter as `UNI-*` / target-namespaced rules.
2. **Framework scan profile** — `scan_profile: framework` extractors for skills, adapters, marketplace, agent-teams symlinks, briefs.
3. **`specdev review` CLI verb** — framework-side counterpart to `specrun review`, drives the framework-profile indexer plus hint interpreter.
4. **`Origin::Framework` amendment to RFC-28** — wire-format extension so framework findings are distinguishable from `Shared` ones in `ReviewFinding` envelopes.
5. **`--include-framework` consumer opt-in** — framework rules are excluded from consumer codex exports by default; opt-in via flag.
6. **Hand-author migration cadence** — closed policy for moving imperative `Check` predicates to `FRAME-*` rules without losing CI coverage.

This RFC adds no lifecycle authority. `FRAME-*` findings may block `make check` and CI, but never transition plan entries, slices, or changes — the same contract that already governs `specrun review`.

## Motivation

After RFC-32 Phase 2 lands, the framework repository has two enforcement surfaces that share substrate but not authoring:

- Consumer projects: write a codex rule, add a `deterministic_hints` block, `specrun review` fires it.
- Framework repository: write a Rust `Check` impl, register it in `crates/authoring/src/check.rs`, ship a binary update.

Three pressure points push toward convergence:

- **Authoring asymmetry punishes framework contributors.** A first-time contributor proposing a new framework check today writes Rust + tests + a registration line + a docs entry. The consumer-side equivalent is a markdown file. Equalizing the bar is the main motivation for `FRAME-*`.
- **Imperative-predicate sprawl recreates itself.** Every new cross-file invariant grows another bespoke walk. The hint interpreter's reserved kinds (`unique`, `set-coverage`, `reference-resolves`, `set-eq`, `content-digest-eq`, …) cover the exact graph shapes most existing predicates implement; using them once on the framework side proves out the reserved kinds for the consumer side, and vice versa.
- **`specdev check --format json` already speaks `ReviewFinding`.** RFC-28 Phase 3 mapped imperative findings into the same envelope. Adding declarative findings to that envelope is the next step, not a rewrite — the wire output stays compatible whether a finding came from a `Check` impl or a `FRAME-*` rule's hint.

## Principles

1. **`FRAME-*` is additive, not a replacement.** Imperative `Check` predicates remain valid indefinitely. Migration is per-predicate and reversible until the imperative impl is deleted.
2. **One substrate, two CLIs.** `specdev review` and `specrun review` share `specify-domain::review` modules but keep separate binaries and separate failure semantics, per RFC-32 §"Framework and consumer scans share libraries, not commands."
3. **No consumer surprise.** Consumer-project `specrun review` / `specrun codex export` invocations never include `FRAME-*` rules unless `--include-framework` is set.
4. **Wire compatibility.** `Origin::Framework` is the only RFC-28 amendment. The `ReviewFinding` shape, the fingerprint algorithm, the severity enum, and the evidence union all stay as RFC-28 defined them.
5. **Parity before deletion.** An imperative `Check` only retires when an equivalent `FRAME-*` rule produces byte-identical findings against the predicate's existing golden fixtures.

## RFC-28 amendments

RFC-28 is in `rfcs/done/`. RFC-34 normatively extends it in two narrow places.

### A1 — `Origin` enum

RFC-28 §"Resolved codex export" declares the closed enum as:

```text
origin: shared | source | target | organization
```

RFC-34 widens it to:

```text
origin: shared | source | target | framework | organization
```

`Framework` is the origin assigned to rules resolved from `adapters/shared/codex/framework/` (RFC-28 §"Resolution roots" root 2, activated with pack name `framework` per §F3 below). The `schemas/codex/resolved.schema.json` enum widens in the same PR; consumer parsers that strictly type the enum see this as a non-breaking value addition because no previous value is removed or renamed.

### A2 — Origin sort order

RFC-28 line 285 declares the stable origin ordering as `target, source, shared, organization`. RFC-34 inserts `framework` between `shared` and `organization`:

```text
target → source → shared → framework → organization
```

The justification mirrors RFC-28's existing rationale: `framework` is more specific than `shared` (it scopes to the framework repository), less specific than per-adapter rules (which scope to one adapter). Placing it before `organization` keeps the closed first-party namespaces ahead of downstream-project additions.

### A3 — Consumer opt-in flag

`specrun codex export` and `specrun review` accept a new flag:

```text
--include-framework    Include FRAME-* rules from adapters/shared/codex/framework/
                       in the resolved set. Default: false (framework rules are
                       hidden from consumer exports).
```

Semantics mirror RFC-28's existing `--include-deprecated` / `--include-unmatched` flags: default exclude, opt-in include, no state. The flag and its default are pinned here so RFC-32 Phase 2 implementations may add the flag pre-emptively without an RFC-32 amendment when they ship.

A1, A2, and A3 are the only RFC-28 changes. The finding schema, fingerprint algorithm, severity enum, evidence union, and resolution-input shape are unchanged.

## Design

### F1 — Framework scan scope

Symmetric counterpart to [RFC-32 §D1](rfc-32-standards-enforcement.md#d1--consumer-scan-scope). `scan_profile: framework` walks the framework repository (`augentic/specify`) with the following defaults:

- **Roots.** `project_dir` itself (always the framework-repo root for `specdev review` invocations), plus any path explicitly named in `artifact_paths[]`.
- **Default include globs.** `adapters/**`, `plugins/**`, `docs/**`, `.cursor/**`, `rfcs/**`, `scripts/**`, `schemas/**`, `**/AGENTS.md`, `**/REVIEW.md`. Wider than the consumer profile because the framework repo's source of truth is markdown and YAML across many trees.
- **Always-ignore globs.** `target/**`, `**/node_modules/**`, `.git/**`, `dist/**`, `.specify/**` (the framework repo MAY carry a project-local `.specify/` for self-hosting; framework scans do not enter it), and every path matching the project-root `.gitignore`.
- **Symlink policy.** Framework `agent-teams.md` files symlink into `docs/reference/`. The framework profile **follows** symlinks (recording both endpoints) so review-team-protocol drift is visible. This is the opposite of the consumer profile's record-without-traverse rule and is normative. Symlink cycles abort with a `Filesystem` error.
- **Binary files.** Same NUL-byte detection as RFC-32 §D1; binary files emit `file { kind: "binary" }` facts and are skipped by `regex` hints unless the rule sets `applicability.binary: true`.
- **Encoding.** UTF-8 with U+FFFD replacement; one `index.warning` finding per non-UTF-8 file at severity `optional`.
- **Determinism.** Enumeration is sorted by project-relative path before parallel dispatch (matches RFC-32 §D1).
- **Codex parse.** Codex trees under `adapters/{shared,sources,targets}/**/codex/` are parsed using the existing RFC-28 codex parser. The framework profile additionally accepts `FRAME-*` rules at `adapters/shared/codex/framework/` per §F3.

### F2 — `specdev review` CLI surface

Add a new subcommand to the `specdev` binary that mirrors `specrun review`'s shape but defaults to the framework profile and the current directory as both codex root and scan root:

```bash
specdev review                                    # full framework scan
specdev review --rule FRAME-001                   # single-rule debug
specdev review --artifact docs/standards/style.md # narrow scope
specdev review --dump-model                       # WorkspaceModel debug
specdev review --strict-hints                     # treat reserved kinds as failures
specdev review --format json                      # CI-consumable envelope
```

**Defaults pinned for `specdev review` only** (these differ from `specrun review`):

- `--codex-root` defaults to `.` (the framework repo's own codex tree resolves shared `UNI-*` and `FRAME-*` rules without any flag).
- `--scan-profile` is hard-coded to `framework`; the flag does not exist on this verb. (A separate `specrun review --scan-profile framework` form is deliberately not introduced to avoid two ways to run the same scan; framework profile is `specdev review`'s sole reason for existing.)
- `--target` is optional and defaults to "none". A framework scan does not have a single target adapter; `applicability.adapters` filtering against framework files is rare. When supplied, the flag narrows applicability the same way it does on `specrun review`.

**Shared with `specrun review`** (per RFC-32 §D6 / §D7 / §D8 / §D9):

- The four formatters (`json`, `pretty`, `github`, `compact`) live in `specify-domain::review::diagnostics`.
- The exit-code map (RFC-32 §D8) applies verbatim; `--format json` validates against `schemas/review/review-result.schema.json` before emit (RFC-32 §D9).
- The hint evaluation order is `path-pattern → schema → regex → tool`.
- Reserved-hint policy (RFC-32 §D5) and `--strict-hints` semantics are identical.

The handler lives under `src/authoring/commands/review/{cli.rs, run.rs}` in `augentic/specify-cli` (mirroring `src/runtime/commands/review/` for `specrun review`). The `specify-authoring` crate gains a dependency on `specify-domain::review` for this verb; the existing `specdev check --format json` Phase 3a mapper is unaffected.

### F3 — `check::codex` placement and resolution

Two `check::codex` changes activate `FRAME-*` placement in the framework repo without weakening existing constraints:

1. **`CODEX_PROFILE_NAMESPACES` extension.** Map the new path `adapters/shared/codex/framework/` → `{"FRAME"}`. Owner discovery uses the same first-segment-under-`adapters/` rule already in place; the framework pack appears as a peer of `universal/` rather than as a per-adapter overlay.
2. **Placement predicate (lift, then re-apply).** RFC-28 Phase 1 step 2 added a predicate rejecting `FRAME-*` under `adapters/{sources,targets}/<name>/codex/`. That rejection stays; `FRAME-*` rules under per-adapter trees remain a `check::codex` failure. The predicate additionally REQUIRES `FRAME-*` placement under `adapters/shared/codex/framework/` (a non-`FRAME-*` rule there is rejected with the same `codex-namespace-ownership-violation` rule id).

**Resolution root activation.** RFC-28 §"Resolution roots" line 138 reserves root 2: "Shared language or artifact packs, if added later under `adapters/shared/codex/<pack>/`." RFC-34 activates root 2 with pack name `framework`. The resolver walks the new pack root immediately after `adapters/shared/codex/universal/`; rules are tagged with `origin: framework` per A1. No new root order or precedence is introduced.

**Consumer-export filtering.** `specrun codex export` filters out `origin: framework` rules unless `--include-framework` (A3) is set. `specrun review` inherits that filter from the resolver — consumer-project review runs never evaluate `FRAME-*` hints by accident.

### F4 — Per-rule applicability for framework files

Framework rules use the existing `applicability` block, with two framework-specific tokens added to the closed `applicability.artifacts` enum:

```text
artifacts:
  - skill          # plugins/**/SKILL.md frontmatter + body
  - adapter        # adapters/**/adapter.yaml manifests
  - brief          # adapters/**/briefs/*.md
  - reference      # adapters/**/references/*.md
  - codex          # codex rule files themselves
  - rfc            # rfcs/**/*.md
  - doc            # docs/**/*.md
```

These tokens compose with the existing consumer-side ones (`code`, `tests`, `contracts`, `specs`, `design`, `tasks`); a single `applicability.artifacts` entry like `[skill, adapter]` is legal. The full enum is closed; widening it is a `check::codex` schema change reviewed in the same PR as the new value.

### F5 — Migration cadence

Hand-authored only (already pinned in RFC-32 §"Phase 3 — framework convergence" Option B). RFC-34 adds the executable rule:

- An imperative `Check` impl retires only when a `FRAME-*` rule plus its hint coverage produces **byte-identical** findings against the predicate's existing golden fixtures. The parity test (`crates/authoring/tests/frame_parity_<rule>.rs`) lands in the same PR as the imperative deletion.
- Until parity is proven, both the imperative `Check` and the `FRAME-*` rule may run. Duplicate findings against the same `(rule-id, location)` are suppressed by fingerprint deduplication in `specdev review`'s envelope emission step (the existing RFC-28 fingerprint algorithm already produces identical fingerprints for identical evidence; this requires no new code).
- Migration order seeds the `High`-priority rows from RFC-32's "Predicate migration map": `adapter.*` (schema), `skill.* (frontmatter)` (schema + unique + regex), `links.*` (reference-resolves). Other rows migrate when contributor demand emerges.

### F6 — Reserved hint kind dependency

`FRAME-*` rules SHOULD prefer Phase-2-implemented hint kinds (`regex`, `path-pattern`, `schema`, `tool`) for first-wave migrations. Rules requiring reserved kinds (`unique`, `set-coverage`, `reference-resolves`, `cardinality`, `constant-eq`, `set-eq`, `content-digest-eq`, `namespace-owner`) ship paired with the interpreter implementation for that kind; the schema authoring annotation `"x-rfc32-status": "reserved"` is dropped from the kind in the same PR.

The PR pattern is therefore: kind implementation in `specify-domain::review::eval::<kind>.rs` + schema annotation removal in `crates/authoring/schemas/codex-rule.schema.json` + first FRAME-* rule using the kind + parity fixture against the retiring imperative `Check`. Reviewer can verify the full chain in one diff.

## Implementation Plan

RFC-34 lands as **five sequenced steps** merged to main in a single PR across `augentic/specify` and `augentic/specify-cli`. Steps 1–2 are pure plumbing; step 3 is the first user-visible surface; steps 4–5 prove the pattern with one real migration.

1. **Schema + predicate updates.** Add `framework` to the closed `Origin` enum in `schemas/codex/resolved.schema.json`. Extend `CODEX_PROFILE_NAMESPACES` to map `adapters/shared/codex/framework/` → `{"FRAME"}`. Update `check::codex` placement predicate to require `FRAME-*` at the new path and reject non-`FRAME-*` rules there. Add the `--include-framework` flag to `specrun codex export` (default off; no behaviour change without the flag).
2. **Framework scan profile.** Implement `scan_profile: framework` extractors under `crates/domain/src/review/index/{skill.rs, adapter.rs, marketplace.rs, agent_teams.rs, brief.rs}`. Reuse `index/files.rs`, `index/frontmatter.rs`, `index/markdown.rs`, `index/symlinks.rs` from Phase 2; symlink policy changes per §F1 (follow instead of record).
3. **`specdev review` CLI verb.** New files `src/authoring/commands/review/{cli.rs, run.rs}` in `augentic/specify-cli`. `specify-authoring` gains a `specify-domain` dependency (allowed; the binary boundary already imports `specify-domain` for the RFC-28 Phase 3 mapper). Wire export → index → eval → envelope per §F2; ship `--dump-model`, all four formatters, `--strict-hints`, and the exit-code map from RFC-32 §D8.
4. **First FRAME-* rules.** Hand-author 3–5 `FRAME-*` rules covering the High-priority migration-map rows (start with `FRAME-001` ≅ `adapter.schema`, `FRAME-002` ≅ `links.unresolved`, `FRAME-003` ≅ `skill.duplicate-name`). Each rule lands under `adapters/shared/codex/framework/` with a `## Rule` body and a `deterministic_hints` block using Phase 2 kinds.
5. **Parity tests + imperative retirement.** For each `FRAME-*` rule in step 4, land a parity fixture under `crates/authoring/tests/frame_parity_<rule>.rs`. Delete the matching imperative `Check` impl in the same PR. Update `docs/contributing/checks.md` to point at the codex rule instead of the predicate.

**Acceptance:** `cargo make ci` green; `make check` (parent repo) green; `specdev review --format json` produces a stable envelope against the framework repo with the seeded `FRAME-*` rules; consumer `specrun codex export` without `--include-framework` excludes every `FRAME-*` rule (golden test).

## Implementation Guide

Non-normative notes for the agent or contributor picking up RFC-34. The RFC body (§Design + §"Implementation Plan") is the source of truth; items here may evolve in PR review without an RFC amendment.

### Module additions

Phase 2's `crates/domain/src/review/` tree grows the framework extractors:

```text
crates/domain/src/review/index/
├── skill.rs              # plugins/**/SKILL.md frontmatter + body
├── adapter.rs            # adapters/**/adapter.yaml manifest facts
├── marketplace.rs        # .cursor-plugin/marketplace.json graph
├── agent_teams.rs        # symlink target + sha256 facts
└── brief.rs              # adapters/**/briefs/*.md sections + size
```

Existing extractors (`files.rs`, `frontmatter.rs`, `markdown.rs`, `symlinks.rs`) are reused under both profiles; `symlinks.rs` gains a `follow: bool` mode parameter so the consumer/framework split lives in one place.

### CLI surface under `specify-authoring`

```text
src/authoring/commands/review.rs          # umbrella
src/authoring/commands/review/cli.rs      # ReviewAction subcommand enum
src/authoring/commands/review/run.rs      # handler
```

Mirrors the `src/runtime/commands/review/` tree introduced by RFC-32 Phase 2. The clap-derive shape is the same as RFC-32 §"Implementation Guide" `ReviewAction` modulo the flag defaults pinned in §F2.

### Cargo dependency edges

`crates/authoring/Cargo.toml` gains `specify-domain = { workspace = true }`. This edge is permitted: the workspace graph in `specify-cli` `AGENTS.md` already has `specify-authoring` depending on `specify-domain` indirectly via the `specdev` binary's RFC-28 Phase 3 mapper. RFC-34 promotes that to a direct crate-level dependency.

### Test layout

```text
crates/authoring/tests/frame_parity_adapter_schema.rs
crates/authoring/tests/frame_parity_links_unresolved.rs
crates/authoring/tests/frame_parity_skill_duplicate_name.rs
crates/domain/tests/review_framework_indexer.rs
crates/domain/tests/fixtures/review/framework_minimal/
tests/specdev_review.rs                  # binary-level end-to-end
```

Use the existing `REGENERATE_GOLDENS` convention. The framework-minimal fixture is small (~10 files) but exercises every framework extractor at least once.

### Documentation touch-points (post-merge)

- `specify-cli` `AGENTS.md` — add the `specdev review` verb to the documentation map; list `crates/domain/src/review/index/{skill,adapter,marketplace,agent_teams,brief}.rs` under modules of note.
- `specify-cli` `docs/standards/architecture.md` — extend the workflow-domain module section with the framework-profile extractors.
- `specify` `docs/contributing/checks.md` — explain how a contributor chooses between writing a new imperative `Check` and a new `FRAME-*` rule. Default recommendation: `FRAME-*` unless the predicate needs a subprocess that cannot be modelled as a `tool` hint.
- `specify` `adapters/shared/codex/framework/README.md` — new file. Lists conventions for `FRAME-*` rule authoring (body structure, applicability tokens, hint-kind preference) and points at the migration map in RFC-32 §"Predicate migration map".

## Migration

**For framework contributors:** New checks SHOULD be authored as `FRAME-*` codex rules under `adapters/shared/codex/framework/` unless the predicate requires subprocess orchestration or stateful behaviour that the hint interpreter cannot model. The `## Rule` body is the canonical agent-readable explanation; the `deterministic_hints` block makes the rule fire under `specdev review`. Existing imperative `Check` impls remain valid; migrate only when parity is achievable.

**For consumer projects:** No change from RFC-32 Phase 2 baseline. `specrun review` and `specrun codex export` continue to exclude `FRAME-*` rules. Pass `--include-framework` only if your project deliberately wants to enforce framework-authoring rules against your own tree (rare; typically only relevant to projects that vendor parts of the framework repo).

**For RFC-28-aware tooling:** The closed `Origin` enum widens with one value (`framework`). Strict parsers MUST treat unknown enum values as a forward-compatibility extension (i.e., they should not crash on `framework` if they predate RFC-34); RFC-28 already specifies forward-compatible behaviour for closed enums in its evidence-union and severity-enum sections.

**For CI integrations consuming `specdev check --format json` (RFC-28 Phase 3):** Output is unchanged unless the integration adds `specdev review --format json` as a parallel invocation. The two envelopes are wire-compatible; aggregating them is the consumer's choice.

## Alternatives Considered

**Fold framework convergence back into RFC-32.** Rejected. RFC-32 is now `Accepted` and ships Phase 2 cleanly. Re-opening it to absorb the `Origin` amendment, framework scan-scope, and `specdev review` CLI surface would delay RM-10 and inflate RFC-32 past 750 lines covering two distinct audience scopes. The carve-out matches the precedent set by RFC-28 → RFC-32 → RFC-33.

**Run `FRAME-*` through `specrun review --scan-profile framework`.** Rejected. RFC-28 §"CLI and binary split" preserves a hard `specrun` (operator-facing) / `specdev` (contributor-facing) split. Adding a framework-profile flag to `specrun review` would erode that split for one feature and force the operator-facing binary to ship framework extractors it never needs.

**Extend `specdev check --format json` to also fire `FRAME-*` rules.** Rejected. The Phase 3a mapper translates imperative `Finding` → `ReviewFinding` and stays single-purpose; folding hint evaluation into the same handler would mix two execution models in one verb. Separate verbs (`specdev check` for imperative, `specdev review` for declarative) keep the contributor's mental model clean and let imperative retirement happen one predicate at a time without touching the other verb's handler.

**Use a different root for `FRAME-*` (e.g. `tooling/rules/`).** Rejected during RFC-32 drafting (see RFC-32 §Resolved Decisions). Keeping every codex tree under `adapters/**` reuses `check::codex` owner discovery, the resolver root walk, and the `origin:` filter without growing a second root.

**Skip `Origin::Framework`; reuse `Shared`.** Rejected. The point of `Origin` is to let consumers filter by source; collapsing `framework` into `shared` would make `--include-framework` impossible to implement without a parallel discriminant. Adding one enum value once is cheaper than two-layer discriminants forever.

**Automate `Check` → `FRAME-*` translation.** Rejected (already in RFC-32 §"Phase 3 — framework convergence"). Hand-authoring is the validation pass for whether a reserved hint kind models the predicate; an auto-porter would mechanically translate without that scrutiny and would still get the Low-priority rows (`scenarios.*`, `tools.*`) wrong.

## Non-Goals

- Mandatory migration of every imperative `Check` to `FRAME-*`. Imperative predicates may live indefinitely; RFC-34 only enables migration on demand.
- A `specdev review --scan-profile consumer` mode. The consumer profile is `specrun review`'s domain.
- New severity, fingerprint, or evidence semantics. RFC-28 + RFC-32 already cover these.
- Lifecycle authority for `FRAME-*` findings. Findings may block CI; they never transition plans, slices, or changes.
- SARIF export (deferred per RFC-32 non-goals).
- Per-source-adapter framework rules. Source adapter overlays use `SRC-*`; framework-level rules use `FRAME-*`. No cross-namespace mixing.
- A consumer `--prefer-framework` mode that auto-includes framework rules. `--include-framework` is the only opt-in surface.

## Resolved Decisions

Every design question raised while drafting RFC-34 is resolved in the body. The list below indexes the resolutions for reviewers checking that no question is parked.

- **`FRAME-*` placement** — §F3 (`adapters/shared/codex/framework/`, activates RFC-28 reserved resolution root 2 with pack name `framework`).
- **`Origin` enum widening** — §A1 + §A2 (add `framework`, sort between `shared` and `organization`).
- **Consumer opt-in flag** — §A3 (`--include-framework`, default off, mirrors `--include-deprecated`).
- **CLI surface for framework execution** — §F2 (`specdev review`, not a `specrun` flag, not a `specdev check` extension).
- **Framework scan scope** — §F1 (symmetric to RFC-32 §D1; symlinks follow instead of record; wider include globs).
- **Hint-kind preference for first-wave rules** — §F6 (Phase 2 kinds preferred; reserved kinds ship paired with their interpreter implementations).
- **Migration cadence** — §F5 (hand-authored, byte-identical parity fixture required for imperative deletion, dedup via existing fingerprint algorithm during overlap).
- **`check::codex` predicate inversion** — §F3 (placement predicate requires `FRAME-*` at the new path and rejects non-`FRAME-*` rules there, while keeping the existing rejection under per-adapter trees).
- **`applicability.artifacts` framework tokens** — §F4 (`skill`, `adapter`, `brief`, `reference`, `codex`, `rfc`, `doc` added to the closed enum).

## References

- [RFC-28: Engineering Standards — Codex Contract and Findings](done/rfc-28-standards-contract.md) — finding shape, codex resolution, the `Origin` enum widened by §A1.
- [RFC-32: Engineering Standards — Deterministic Enforcement](rfc-32-standards-enforcement.md) — Phase 2 substrate (WorkspaceModel, hint interpreter, `specrun review`).
- [RFC-33: Standards Finding Lifecycle](rfc-33-standards-finding-lifecycle.md) — baseline/suppression layer that applies equally to `specdev review` and `specrun review` envelopes.
- [RFC-5: Framework Developer Tooling](done/rfc-5-tooling.md) — `specdev` binary contract.
- [Specify Roadmap — RM-10](roadmap.md#rm-10-ci-native-standards-enforcement) — CI-native standards enforcement; RFC-34 is optional relative to RM-10.
- [Standards layer (explanation)](../docs/explanation/standards-layer.md)
- [docs/contributing/checks.md](../docs/contributing/checks.md) — receives the new "choose between `Check` and `FRAME-*`" guidance per §"Documentation touch-points".
