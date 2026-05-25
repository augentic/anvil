# RFC-31: WorkspaceModel and Declarative Rule Execution

> Status: Draft · Depends: [RFC-28](rfc-28-codex-rules.md), [RFC-5](done/rfc-5-tooling.md) · Enables: [roadmap RM-10](roadmap.md#rm-10-ci-native-specify-review), [RFC-18](future/rfc-18-slm.md) · Optional follow-on: framework-repo convergence (Phase 3)

## Abstract

[RFC-28](rfc-28-codex-rules.md) defines the **contract layer** for Specify review: resolved codex export, stable `rule-id`s, structured review findings, and `deterministic_hints` as declarative metadata. It deliberately does not implement scanners, hint execution, or a unified extraction pipeline.

This RFC defines the **execution layer** that consumes that contract:

1. **WorkspaceModel** — a deterministic, versioned snapshot of project facts extracted once per scan (files, frontmatter, links, skills, adapters, symlinks, manifest edges).
2. **Hint interpreter** — a closed evaluator for codex `deterministic_hints` against the model and raw artifact bytes.
3. **`specify review` deterministic core** — the first consumer-project scanner that resolves applicable rules via RFC-28 export, evaluates hints, and emits RFC-28 review findings.
4. **Optional Phase 3** — converge framework-repo `tooling check` toward the same finding shape and, where worthwhile, the same declarative rule format; imperative checks may remain indefinitely.

The design separates **extraction** (imperative, shared library code) from **policy** (declarative codex rules and hint kinds). Cross-file invariants become graph queries over WorkspaceModel rather than bespoke walks in every check module.

## Motivation

Today enforcement is split across three shapes:

| Surface | Input | Rule form | Output |
| --- | --- | --- | --- |
| `tooling check` | Framework repo (`plugins/`, `adapters/`, `docs/`) | ~30 imperative Rust `Check` predicates | Ad hoc `Finding { rule_id, message, location }` |
| Codex markdown | Adapter trees + shared universal rules | Human prose + frontmatter hints (shape-only in RFC-28) | None until a scanner exists |
| Target review briefs | Generated artifacts | Agent judgment + optional `rule_id` in `REVIEW.md` | Human markdown |

Each surface re-implements walking, parsing, and linking. Cross-file rules — duplicate skill names, marketplace drift, unresolved directives, variable coverage — embed graph logic inside individual modules. [RFC-5](done/rfc-5-tooling.md) accepted that split as the right day-one tradeoff; RFC-28 adds the finding contract without fixing the duplication.

RM-10 (`specify review`) needs a scanner substrate. Without WorkspaceModel, every deterministic codex rule would require a one-off Rust predicate, recreating the `tooling check` sprawl on consumer projects. Without a shared finding shape at the framework boundary, CI annotations and dashboards would continue to treat framework checks and review findings as unrelated formats.

### What this RFC does not repeat

- **RFC-28** owns rule resolution, finding schema, namespace ownership, and `specify codex export`.
- **RFC-5** owns the framework dev-tooling binary, schema-first authoring, and the current imperative `check` modules until Phase 3 optionally migrates them.
- **RFC-4 Option 2/3** (typed skill manifests / Rust DSL) remains an alternative way to make structural skill rules declarative by changing the artifact shape. Phase 3 here does not require that migration.

## Principles

1. **Extract once, evaluate many.** Parsers and walkers live in the indexer; rules query the snapshot.
2. **RFC-28 findings are the wire format.** Deterministic producers emit `ReviewFinding` JSON; markdown and terminal views are presentation.
3. **Hint kinds stay closed.** New kinds require schema and interpreter changes; no arbitrary embedded scripts in rule files.
4. **Scanner ≠ resolver.** `specify codex export` answers which rules apply; `specify review` answers which hints fire.
5. **Framework and consumer scans share libraries, not commands.** `tooling check` and `specify review` keep separate CLIs, inputs, and failure semantics per RFC-28.
6. **Phase 3 is optional.** Imperative `tooling check` may remain the framework gate indefinitely if migration cost outweighs benefit.
7. **No lifecycle authority in review.** Findings may block CI; they never transition plan entries, slices, or changes.

## Design

### Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│ RFC-28 (prerequisite)                                           │
│   specify codex export → ResolvedCodex + ReviewFinding schema   │
│   shared CodexRule parser in specify-domain                     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│ Phase 2 — WorkspaceModel + hint interpreter (this RFC, core)    │
│                                                                 │
│  ┌──────────────┐    ┌─────────────────┐    ┌───────────────┐ │
│  │ Indexer      │───▶│ WorkspaceModel  │───▶│ Hint runner   │ │
│  │ (imperative) │    │ (versioned JSON)│    │ (declarative) │ │
│  └──────────────┘    └─────────────────┘    └───────────────┘ │
│         ▲                                              │        │
│         │                                              ▼        │
│  consumer project                              ReviewFinding[]  │
│  artifact paths                                specify review   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (optional)
┌─────────────────────────────────────────────────────────────────┐
│ Phase 3 — framework convergence (optional)                      │
│   tooling check → same ReviewFinding mapper and/or FRAME rules  │
└─────────────────────────────────────────────────────────────────┘
```

### Phasing

| Phase | Owner | Scope | Required? |
| --- | --- | --- | --- |
| **1** | RFC-28 | Finding schema, `specify codex export`, shared codex parser, hint shape validation | Yes — prerequisite |
| **2** | RFC-31 | WorkspaceModel, hint interpreter, `specify review` deterministic MVP | Yes — this RFC |
| **3** | RFC-31 | Framework `tooling check` convergence: shared finding output, selective declarative migration | No — operator choice |

Phase 2 must not block on Phase 3. Phase 3 must not block RM-10.

### WorkspaceModel

The model is a deterministic JSON document produced by the indexer. It is an internal execution artifact, not an operator-facing Specify artifact. Version field `version: 1` is required; breaking indexer output bumps the version.

#### Extraction inputs

| Field | Meaning |
| --- | --- |
| `project_dir` | Scan root (consumer project or framework repo) |
| `scan_profile` | `consumer` or `framework` — controls which extractors run |
| `artifact_paths[]` | Optional narrow list; default is profile-specific full scan |
| `languages[]` | Optional tokens supplied by caller or inferred from paths |

#### Core entity families (v1)

Facts are normalized relations, not nested domain objects. Examples:

| Family | Fact shape | Source extractors |
| --- | --- | --- |
| `file` | `{ path, kind, language?, sha256? }` | Filesystem walk with profile globs |
| `frontmatter` | `{ path, schema_id?, fields }` | Markdown `---` extraction + YAML parse |
| `markdown_section` | `{ path, level, title, line_start, line_end, body_line_count }` | Markdown structure pass |
| `markdown_link` | `{ from_path, to_raw, line, resolves? }` | Link scan with fence/comment stripping |
| `symlink` | `{ path, target, broken }` | Filesystem metadata |
| `skill` | `{ name, path, plugin, frontmatter_ref }` | `plugins/**/SKILL.md` |
| `adapter_manifest` | `{ axis, name, path, version }` | `adapters/{sources,targets}/**/adapter.yaml` |
| `marketplace_entry` | `{ plugin, path_in_manifest }` | `.cursor-plugin/marketplace.json` |
| `codex_rule` | `{ rule_id, path, origin, frontmatter_ref }` | Codex trees (reuse RFC-28 parser) |
| `text_match` | `{ path, line, column, pattern_id }` | Precomputed regex index (optional optimization) |

Extractors may run only under `scan_profile: framework` (marketplace, skill graph, brief size) or under both profiles (files, frontmatter, links).

#### Stability

- Entity and edge ordering is byte-stable for fixture tests.
- Paths are project-relative with forward slashes.
- The model may be written to a temp file or emitted on stdout as `specify review --dump-model` for debugging; it is not persisted under `.specify/` by default.

#### Schema location

Add `specify-cli/schemas/review/workspace-model.schema.json` and matching DTOs in `specify-domain` (or a small `specify-review` crate if dependency direction requires it). The schema documents v1 fact families; it does not attempt to encode every future extractor.

### Deterministic hints

RFC-28 defines the authoring surface:

```yaml
deterministic_hints:
  - kind: regex
    value: "https?://"
    description: Literal URL in generated code.
```

RFC-31 extends the **closed** `kind` enum for execution. RFC-28 validates shape only; this RFC owns interpreter semantics.

#### Hint kinds — Phase 2 (implement)

| Kind | Evaluates against | Purpose |
| --- | --- | --- |
| `regex` | Raw file bytes (per applicability path filter) | Line/column findings for pattern hits |
| `path-pattern` | `file.path` glob | Narrow scan targets before other hints |
| `schema` | Parsed JSON/YAML value or extracted frontmatter | Structural validation via JSON Schema ref |
| `tool` | External command | Delegate to declared WASI/host tool (`specify tool run …`) |

#### Hint kinds — reserved (schema may list; interpreter returns `unsupported` until implemented)

| Kind | Evaluates against | Maps from today's `tooling check` |
| --- | --- | --- |
| `unique` | WorkspaceModel collection + field | `skill.duplicate-name`, `codex.duplicate-rule-id` |
| `reference-resolves` | `markdown_link.resolves == false` | `links.unresolved`, `links.broken-reference` |
| `set-coverage` | defined vs used symbol sets | `skill.variable-coverage` |
| `cardinality` | counted collection size | `skill.invalid-critical-path` (5–7 steps) |
| `constant-eq` | cross-artifact constant paths | `prose.numeric-cap-exceeded` cap sync |
| `set-eq` | two model collections | `plugins.marketplace-drift` |
| `content-digest-eq` | file sha256 vs expected | `agent_teams` canonical SHA check |
| `namespace-owner` | codex id prefix vs tree owner | `codex.namespace-ownership-violation` |

Reserved kinds are documented in the schema with `"x-rfc31-status": "reserved"` so RFC-28 exporters and tooling validators accept files that declare future hints without executing them.

#### Evaluation algorithm

For each applicable rule from `specify codex export`:

1. Filter hints by implied artifact paths and languages.
2. Run `path-pattern` hints first to build the candidate file set.
3. For each candidate file, run `schema` and `regex` hints.
4. Run `tool` hints last (subprocess boundary).
5. Map each hit to a `ReviewFinding` with `rule-id`, `source: deterministic`, stable `fingerprint`, and bounded `evidence`.

Hybrid and model-assisted rules (`review_mode: hybrid | model-assisted`) are exported but not evaluated by the Phase 2 deterministic core unless a hint explicitly matches.

### `specify review` (Phase 2 CLI)

Add the consumer-project scanner RM-10 depends on:

```bash
specify review --target omnia --format json
specify review --slice billing-export --format json
specify review --artifact crates/billing/src/lib.rs --format json
specify review --dump-model --format json   # debug: emit WorkspaceModel only
```

Behavior:

1. Resolve target adapter (and optional source adapters) from project context or flags.
2. Call `specify codex export` internally with matching `--artifact` / `--language` filters.
3. Build WorkspaceModel for the scan profile `consumer`.
4. Evaluate deterministic hints for applicable rules with `review_mode: deterministic | hybrid`.
5. Emit the RFC-28 review result envelope with byte-stable finding order.

Phase 2 scope is **deterministic hints only**. Model-assisted findings remain the responsibility of target adapter review briefs until a later RFC wires LLM producers into the same schema.

Exit codes follow the operator CLI table: validation/findings failure → `2`; infrastructure error → `1`.

### Relationship to RFC-28

| Concern | RFC-28 | RFC-31 |
| --- | --- | --- |
| `rule-id` namespaces | Defines and exports | Consumes; does not mint new consumer-facing namespaces |
| Finding JSON schema | Defines | Produces |
| `deterministic_hints` shape | Validates | Executes |
| `specify codex export` | Implements | Calls |
| `specify review` | Non-goal | Implements deterministic core |
| Shared codex parser | Implements in `specify-domain` | Reuses in indexer |

RFC-28 should reserve extensibility in the codex authoring schema for additional hint kinds without implementing them. See [RFC-28 §Deterministic hints extensibility](rfc-28-codex-rules.md#deterministic-hints-extensibility).

### Relationship to framework tooling (Phase 3 — optional)

Phase 3 converges framework enforcement toward the same substrate without merging commands.

#### Option A — finding shape only (lowest cost)

Keep imperative `tooling check` predicates. Add a mapper from today's `Finding` to RFC-28 `ReviewFinding` and optional `tooling check --format json`. Rule ids stay as today (`skill.duplicate-name`, `links.unresolved`, …). No codex migration.

#### Option B — declarative framework rules (higher cost)

Introduce first-party framework policy files under `tooling/rules/` using the same codex markdown shape but a dedicated namespace:

| Namespace | Owner |
| --- | --- |
| `FRAME-*` | Framework-repo checks not tied to a consumer target adapter |

`FRAME-*` rules use the same hint interpreter and WorkspaceModel with `scan_profile: framework`. Imperative checks retire only when a declarative rule plus extractor coverage replaces them. Until then, both may run; duplicate coverage is a migration smell, not a CI failure.

#### Option C — defer indefinitely

Leave `tooling check` unchanged. RM-10 and framework CI remain separate surfaces sharing only `specify-domain` parsers. This is valid if Phase 3 cost exceeds benefit.

**Recommendation:** ship Phase 2 first; adopt Option A when RM-10 needs unified CI annotations; adopt Option B only for predicates that clearly map to reserved hint kinds.

### Predicate migration map (Phase 3 reference)

Reference mapping from current `tooling/src/check/` predicates to declarative kinds. Not a commitment to migrate every row.

| Current rule id prefix | Declarative kind(s) | Phase 3 priority |
| --- | --- | --- |
| `adapter.*` | `schema` | High — already schema-shaped |
| `skill.*` (frontmatter) | `schema`, `unique`, grammar as `regex` | High |
| `skill.*` (body) | `cardinality`, `regex`, `set-coverage` | Medium |
| `links.*` | `reference-resolves` | High |
| `codex.*` | `schema`, `namespace-owner`, `unique` | Medium — shape checks may stay in tooling |
| `plugins.*` | `set-eq`, symlink facts | Medium |
| `prose.*` | `regex`, `constant-eq` | Medium |
| `scenarios.*` | `schema`, custom trace freshness | Low — may stay imperative |
| `tools.*` | `tool`, `constant-eq` | Low |

Predicates invoking subprocesses (`specify source resolve`, declared-tool equivalence) remain `kind: tool` or imperative orchestration.

### Library layout

Prefer `specify-domain` for shared types and parsers already used by RFC-28. Add execution modules behind a feature flag if dependency weight matters:

```text
specify-domain (or specify-review)
├── codex/          # RFC-28: parse, resolve, export DTOs
├── review/
│   ├── finding.rs  # RFC-28: ReviewFinding, envelope
│   ├── model.rs    # RFC-31: WorkspaceModel DTOs
│   ├── index/      # RFC-31: profile-specific extractors
│   └── eval/       # RFC-31: hint interpreter
```

Framework repo `tooling/` depends on `specify-domain` for parsers (already true per RFC-5). Phase 3 Option B may add a thin `tooling` subcommand `tooling review` that runs the framework profile locally — **not** required for Phase 2.

## Implementation Plan

### Phase 2 (required)

1. **Schemas.** Add `workspace-model.schema.json`; extend codex authoring schema with reserved hint kinds (documented, not executed in RFC-28).
2. **Indexer.** Implement `scan_profile: consumer` extractors: files, frontmatter, markdown links, basic sections. Reuse RFC-28 codex parser for any codex paths in consumer overlays.
3. **Hint interpreter.** Implement `regex`, `path-pattern`, `schema`, `tool` with golden fixtures per kind.
4. **`specify review`.** Wire export → index → eval → envelope; add `--dump-model` for debugging.
5. **Acceptance.** Golden tests: resolved rules + sample crate tree → stable findings JSON; fingerprint stability; evidence size cap enforcement from RFC-28.
6. **Roadmap RM-10.** Update review briefs to reference deterministic findings alongside human `REVIEW.md`.

### Phase 3 (optional)

1. **Finding mapper.** `tooling check --format json` emitting RFC-28 findings (Option A).
2. **Framework profile.** Extend indexer for marketplace, skills, briefs, agent-teams (Option B).
3. **FRAME rules.** Port high-priority predicates from the migration map; delete imperative code only when fixture parity is proven.
4. **Diagnostics UX.** Pretty, GitHub, and compact formatters shared between `specify review` and `tooling check` (optional; may follow [augentic/lints](https://github.com/augentic/lints) patterns without coupling repos).

## Migration

**For operator projects:** `specify review` is additive. Existing target adapter review briefs and `REVIEW.md` remain authoritative for model-assisted judgment until operators opt into CI gates on JSON findings.

**For adapter authors:** Add `deterministic_hints` to codex rules that should fire in CI without waiting for custom Rust scanners. Hints must use Phase 2 kinds only until reserved kinds are implemented.

**For framework contributors:** No change until Phase 3. `make check` continues to run imperative predicates.

**For CLI maintainers:** Keep indexer and interpreter free of lifecycle transitions. Do not persist WorkspaceModel under `.specify/` without a separate RFC.

## Alternatives Considered

**Fold into RFC-28.** Rejected. RFC-28's value is landing the contract and export before any scanner exists. Combining execution would delay RM-10 and blur resolver vs scanner boundaries.

**Datalog (Soufflé / embedded) as the rule language.** Rejected for v1. Powerful for graph invariants but poor contributor ergonomics for a closed first-party rule set. Reserved hint kinds cover the same invariants with a smaller vocabulary; revisit if FRAME rule count exceeds ~100 graph-heavy rules.

**Semgrep / ast-grep as the engine.** Rejected as primary. Useful for `regex` and language-specific paths inside hint values, but does not unify markdown frontmatter, marketplace graphs, or RFC-28 finding emission without a Specify-owned model layer.

**Replace markdown skills with RFC-4 manifests first.** Rejected as prerequisite. Typed manifests reduce the need for markdown extractors but are independent; WorkspaceModel must scan today's repos.

**Single command (`specify check`) for framework and consumer.** Rejected. RFC-5 and RFC-28 preserve separate audiences and failure semantics.

## Non-Goals

- Model-assisted or hybrid review execution (agents remain in target adapter briefs for Phase 2).
- Implementing every reserved hint kind in the first Phase 2 PR.
- Mandatory Phase 3 migration of `tooling check`.
- SARIF output (may follow as an export adapter).
- Persisting WorkspaceModel as a Specify artifact.
- Auto-opening slices or mutating lifecycle state from findings.
- RFC-4 Option 2/3 skill manifest or DSL work.

## Open Questions

1. Should `FRAME-*` live in `tooling/rules/` or `adapters/shared/codex/framework/`? Current preference: `tooling/rules/` to keep consumer codex trees clean; export treats them as `origin: framework`.
2. Should the indexer run extractors in parallel (rayon) from day one? Current preference: yes for file walks; sequential is acceptable for v1 if fixture runtime stays under RFC-5's full-scan budget on CI.
3. Should `specify review` evaluate reserved hints as no-ops or hard-fail? Current preference: no-op with a single summary warning when `--verbose`; hard-fail only with `--strict-hints`.
4. Where should shared diagnostic formatters live — `specify-domain`, a new `specify-diagnostics` crate, or duplicated thin wrappers? Current preference: `specify-domain` until formatters pull in heavy deps.
5. Is Phase 3 Option B worth automating from existing `Check` impls, or hand-authored FRAME rules only? Current preference: hand-authored only; migration is deliberate.

## References

- [RFC-28: Codex Resolution and Structured Review Findings](rfc-28-codex-rules.md)
- [RFC-5: Framework Developer Tooling](done/rfc-5-tooling.md)
- [RFC-4: Type-Safe Skill Expression](future/rfc-4-dsl.md)
- [Specify Roadmap — RM-10](roadmap.md#rm-10-ci-native-specify-review)
- [docs/contributing/checks.md](../docs/contributing/checks.md)
- [tooling/src/check/mod.rs](../tooling/src/check/mod.rs) — current imperative predicate registry
- [augentic/lints](https://github.com/augentic/lints) — reference for diagnostics UX in Phase 3
