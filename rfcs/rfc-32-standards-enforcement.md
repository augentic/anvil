# RFC-32: Engineering Standards — Deterministic Enforcement

> Status: Accepted · Depends: [RFC-28](done/rfc-28-standards-contract.md), [RFC-5](done/rfc-5-tooling.md) · Enables: [roadmap RM-10](roadmap.md#rm-10-ci-native-standards-enforcement), [RFC-18](future/rfc-18-slm.md) · Optional follow-on: framework-repo convergence — see [RFC-34](rfc-34-framework-convergence.md)

## Abstract

[RFC-28](done/rfc-28-standards-contract.md) defines the **standards contract layer**: resolved codex export, stable `rule-id`s, structured review findings, and `deterministic_hints` as declarative metadata. It deliberately does not implement scanners, hint execution, or a unified extraction pipeline.

This RFC defines the **standards enforcement layer** that consumes that contract:

1. **WorkspaceModel** — a deterministic, versioned snapshot of project facts extracted once per scan (files, frontmatter, links, skills, adapters, symlinks, manifest edges).
2. **Hint interpreter** — a closed evaluator for codex `deterministic_hints` against the model and raw artifact bytes.
3. **`specrun review` deterministic core** — the first consumer-project **standards scanner** that resolves applicable rules via RFC-28 export, evaluates hints, and emits RFC-28 review findings.
4. **Optional Phase 3 (split)** — [RFC-28](done/rfc-28-standards-contract.md) **Phase 3** converges `specdev check` to the same `ReviewFinding` shape (Option A); this RFC retains optional declarative `FRAME-*` migration (Option B); imperative checks may remain indefinitely.

The design separates **extraction** (imperative, shared library code) from **policy** (declarative codex rules and hint kinds). Cross-file invariants become graph queries over WorkspaceModel rather than bespoke walks in every check module.

## Motivation

Today enforcement is split across three shapes:


| Surface              | Input                                             | Rule form                                              | Output                                          |
| -------------------- | ------------------------------------------------- | ------------------------------------------------------ | ----------------------------------------------- |
| `specdev check`      | Framework repo (`plugins/`, `adapters/`, `docs/`) | ~30 imperative Rust `Check` predicates                 | Ad hoc `Finding { rule_id, message, location }` |
| Codex markdown       | Adapter trees + shared universal rules            | Human prose + frontmatter hints (shape-only in RFC-28) | None until a scanner exists                     |
| Target review briefs | Generated artifacts                               | Agent judgment + optional `rule_id` in `REVIEW.md`     | Human markdown                                  |


Each surface re-implements walking, parsing, and linking. Cross-file rules — duplicate skill names, marketplace drift, unresolved directives, variable coverage — embed graph logic inside individual modules. [RFC-5](done/rfc-5-tooling.md) accepted that split as the right day-one tradeoff; RFC-28 adds the finding contract without fixing the duplication.

RM-10 (CI-native **standards enforcement** via `specrun review`) needs a scanner substrate. Without WorkspaceModel, every deterministic codex rule would require a one-off Rust predicate, recreating the `specdev check` sprawl on consumer projects. Without a shared finding shape at the framework boundary, CI annotations and dashboards would continue to treat framework checks and review findings as unrelated formats.

### What this RFC does not repeat

- **RFC-28** owns rule resolution, finding schema, namespace ownership, and `specrun codex export`.
- **RFC-5** owns the framework dev-tooling binary, schema-first authoring, and the current imperative `check` modules until Phase 3 optionally migrates them.
- **RFC-4 Option 2/3** (typed skill manifests / Rust DSL) remains an alternative way to make structural skill rules declarative by changing the artifact shape. Phase 3 here does not require that migration.

## Principles

1. **Extract once, evaluate many.** Parsers and walkers live in the indexer; rules query the snapshot.
2. **RFC-28 findings are the wire format.** Deterministic producers emit `ReviewFinding` JSON; markdown and terminal views are presentation.
3. **Hint kinds stay closed.** New kinds require schema and interpreter changes; no arbitrary embedded scripts in rule files.
4. **Scanner ≠ resolver.** `specrun codex export` answers which standards apply; `specrun review` answers which hints fire.
5. **Framework and consumer scans share libraries, not commands.** `specdev check` and `specrun review` keep separate CLIs, inputs, and failure semantics per RFC-28.
6. **Phase 3 is optional.** Imperative `specdev check` may remain the framework gate indefinitely if migration cost outweighs benefit.
7. **No lifecycle authority in review.** Findings may block CI; they never transition plan entries, slices, or changes.

## Design

### Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│ RFC-28 (prerequisite)                                           │
│   specrun codex export → ResolvedCodex + ReviewFinding schema   │
│   shared CodexRule parser in specify-codex (see §Library layout)│
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
│  artifact paths                                specrun review   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ (optional)
┌─────────────────────────────────────────────────────────────────┐
│ Phase 3a — RFC-28 Phase 3: specdev check --format json             │
│ Phase 3b (optional) — FRAME rules + framework WorkspaceModel    │
└─────────────────────────────────────────────────────────────────┘
```

### Phasing


| Phase  | Owner                                     | Scope                                                                              | Required?            |
| ------ | ----------------------------------------- | ---------------------------------------------------------------------------------- | -------------------- |
| **1**  | RFC-28 Phases 1–2                         | Finding schema, `specrun codex export`, shared codex parser, hint shape validation | Yes — prerequisite   |
| **2**  | RFC-32                                    | WorkspaceModel, hint interpreter, `specrun review` deterministic MVP               | Yes — this RFC       |
| **3a** | RFC-28 Phase 3                            | Framework `specdev check --format json` → `ReviewFinding` (Option A)               | Yes — same train     |
| **3b** | [RFC-34](rfc-34-framework-convergence.md) | Declarative `FRAME-*` rules + framework scan profile + `specdev review` (Option B) | No — operator choice |


Phase 2 must not block on Phase 3a or 3b. Phase 3a (RFC-28 Phase 3) must not block Phase 2 or RM-10. Phase 3b (RFC-34) must not block RM-10 and must not block RFC-32 acceptance.

### WorkspaceModel

The model is a deterministic JSON document produced by the indexer. It is an internal execution artifact, not an operator-facing Specify artifact. Version field `version: 1` is required; breaking indexer output bumps the version.

#### Extraction inputs


| Field              | Meaning                                                     |
| ------------------ | ----------------------------------------------------------- |
| `project_dir`      | Scan root (consumer project or framework repo)              |
| `scan_profile`     | `consumer` or `framework` — controls which extractors run   |
| `artifact_paths[]` | Optional narrow list; default is profile-specific full scan |
| `languages[]`      | Optional tokens supplied by caller or inferred from paths   |


#### Core entity families (v1)

Facts are normalized relations, not nested domain objects. Examples:


| Family              | Fact shape                                                      | Source extractors                               |
| ------------------- | --------------------------------------------------------------- | ----------------------------------------------- |
| `file`              | `{ path, kind, language?, sha256? }`                            | Filesystem walk with profile globs              |
| `frontmatter`       | `{ path, schema_id?, fields }`                                  | Markdown `---` extraction + YAML parse          |
| `markdown_section`  | `{ path, level, title, line_start, line_end, body_line_count }` | Markdown structure pass                         |
| `markdown_link`     | `{ from_path, to_raw, line, resolves? }`                        | Link scan with fence/comment stripping          |
| `symlink`           | `{ path, target, broken }`                                      | Filesystem metadata                             |
| `skill`             | `{ name, path, plugin, frontmatter_ref }`                       | `plugins/**/SKILL.md`                           |
| `adapter_manifest`  | `{ axis, name, path, version }`                                 | `adapters/{sources,targets}/**/adapter.yaml`    |
| `marketplace_entry` | `{ plugin, path_in_manifest }`                                  | `.cursor-plugin/marketplace.json`               |
| `codex_rule`        | `{ rule_id, path, origin, frontmatter_ref }`                    | Codex trees (reuse RFC-28 parser)               |
| `text_match`        | `{ path, line, column, pattern_id }`                            | Precomputed regex index (optional optimization) |


Extractors may run only under `scan_profile: framework` (marketplace, skill graph, brief size) or under both profiles (files, frontmatter, links).

#### Stability

- Entity and edge ordering is byte-stable for fixture tests.
- Paths are project-relative with forward slashes.
- The model may be written to a temp file or emitted on stdout as `specrun review --dump-model` for debugging; it is not persisted under `.specify/` by default.

#### Schema location

Add `specify-cli/schemas/review/workspace-model.schema.json` (embedded by `specify-schema`) and matching DTOs in `specify-codex` (under `crates/codex/src/review/`, mirroring the existing codex precedent from RFC-28 — both modules co-locate in `specify-codex` per §"Library layout"). The schema documents v1 fact families; it does not attempt to encode every future extractor.

#### Persistence and query (v1 decision)

WorkspaceModel is an internal execution artifact in v1: produced fresh per `specrun review` invocation, kept in memory, optionally dumped to stdout via `--dump-model` for debugging. It is **not** persisted under `.specify/` by default and is **not** an operator-facing Specify artifact.

Two adjacent surfaces are deliberately *reserved* (not implemented) so the public surface does not have to widen when consumers downstream of `specrun review` need the same extract:

1. **Persistent cache path.** `.specify/cache/workspace-model.v1.json` is reserved as the future v2 persistence location. v1 does not write it. The `version: 1` discriminant on WorkspaceModel pins the model shape exactly as it pins persisted-file shapes elsewhere in Specify; promoting v1 to a persisted cache is a separate RFC, not a re-architecture.
2. **Read-only query verb.** `specrun model query <selector>` is reserved as a future read-only CLI verb so [roadmap RM-13](roadmap.md#rm-13-read-oriented-specify-mcp-server) (read-oriented MCP) and [roadmap RM-21](roadmap.md#rm-21-adapter-ecosystem-operating-model) (catalogue ecosystem) consumers do not fork a second extractor a year from now. v1 ships `--dump-model` only; the named query verb lands when one of those consumers needs it.

Both reservations are shape-level: the cache filename and the CLI verb appear in the v1 schema and `specrun --help` documentation as "reserved" entries with no implementation behind them. Persistence and query land together or not at all, in their own RFC.

#### Performance — parallelism and incrementality

The indexer is the hot path: every `specrun review` invocation walks a consumer project tree, parses frontmatter, scans links, and (eventually) hashes files. v1 makes two related decisions:

- **Parallelism.** File walks and per-file extractors (frontmatter, markdown sections, regex scans) run in parallel via `rayon` from day one. Order-dependent steps (cross-file edges, marketplace graph, codex-rule discovery) run sequentially after the parallel pass. Output ordering is byte-stable regardless of thread scheduling because entity and edge collections are sorted before envelope emission per §"Stability".
- **Incrementality (reserved).** v1 re-extracts the full WorkspaceModel on every invocation. `.specify/cache/index.v1.json` is reserved for a per-file content-hash cache (path → sha256 → cached facts) so v2 can skip unchanged files. v1 does not write the file; consumers MUST NOT depend on it existing. The cache and the WorkspaceModel-persistence cache above share the same `version` discriminant and land together in their follow-on RFC, because invalidating one without the other produces silent drift.

Sequential v1 is acceptable as a fallback only if the parallel implementation cannot meet RFC-5's full-scan budget on CI fixtures; in that case it is a v1 implementation choice, not a contract.

### Deterministic hints

RFC-28 defines the authoring surface:

```yaml
deterministic_hints:
  - kind: regex
    value: "https?://"
    description: Literal URL in generated code.
```

RFC-32 extends the **closed** `kind` enum for execution. RFC-28 validates shape only; this RFC owns interpreter semantics.

#### Hint kinds — Phase 2 (implement)


| Kind           | Evaluates against                               | Purpose                                                         |
| -------------- | ----------------------------------------------- | --------------------------------------------------------------- |
| `regex`        | Raw file bytes (per applicability path filter)  | Line/column findings for pattern hits                           |
| `path-pattern` | `file.path` glob                                | Narrow scan targets before other hints                          |
| `schema`       | Parsed JSON/YAML value or extracted frontmatter | Structural validation via JSON Schema ref                       |
| `tool`         | Declared WASI tool                              | Delegate to a project-declared WASI tool via `specrun tool run` |


#### Hint kinds — reserved (schema may list; interpreter returns `unsupported` until implemented)


| Kind                 | Evaluates against                 | Maps from today's `specdev check`                 |
| -------------------- | --------------------------------- | ------------------------------------------------- |
| `unique`             | WorkspaceModel collection + field | `skill.duplicate-name`, `codex.duplicate-rule-id` |
| `reference-resolves` | `markdown_link.resolves == false` | `links.unresolved`, `links.broken-reference`      |
| `set-coverage`       | defined vs used symbol sets       | `skill.variable-coverage`                         |
| `cardinality`        | counted collection size           | `skill.invalid-critical-path` (5–7 steps)         |
| `constant-eq`        | cross-artifact constant paths     | `prose.numeric-cap-exceeded` cap sync             |
| `set-eq`             | two model collections             | `plugins.marketplace-drift`                       |
| `content-digest-eq`  | file sha256 vs expected           | `agent_teams` canonical SHA check                 |
| `namespace-owner`    | codex id prefix vs tree owner     | `codex.namespace-ownership-violation`             |


Reserved kinds are documented in the schema with `"x-rfc32-status": "reserved"` (matching RFC-28's annotation) so RFC-28 exporters and tooling validators accept files that declare future hints without executing them.

#### Evaluation algorithm

For each applicable rule from `specrun codex export`:

1. Filter hints by implied artifact paths and languages.
2. Run `path-pattern` hints first to build the candidate file set.
3. For each candidate file, run `schema` and `regex` hints.
4. Run `tool` hints last (subprocess boundary).
5. Map each hit to a `ReviewFinding` with `rule-id`, `source: deterministic`, stable `fingerprint`, and bounded `evidence`.

Hybrid and model-assisted rules (`review_mode: hybrid | model-assisted`) are exported but not evaluated by the Phase 2 deterministic core unless a hint explicitly matches.

### `specrun review` (Phase 2 CLI)

Add the consumer-project **standards scanner** RM-10 depends on:

```bash
specrun review --target omnia --format json
specrun review --slice billing-export --format json
specrun review --artifact crates/billing/src/lib.rs --format json
specrun review --dump-model --format json   # debug: emit WorkspaceModel only
```

Behavior:

1. Resolve target adapter (and optional source adapters) from project context or flags.
2. Call `specrun codex export` internally with matching `--artifact` / `--language` filters.
3. Build WorkspaceModel for the scan profile `consumer`.
4. Evaluate deterministic hints for applicable rules with `review_mode: deterministic | hybrid`.
5. Emit the RFC-28 review result envelope with byte-stable finding order.

Phase 2 scope is **deterministic hints only**. Model-assisted findings remain the responsibility of target adapter review briefs until a later RFC wires LLM producers into the same schema.

Exit codes follow the operator CLI table: validation/findings failure → `2`; infrastructure error → `1`.

### Relationship to RFC-28


| Concern                     | RFC-28                                                                 | RFC-32                                                 |
| --------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------ |
| `rule-id` namespaces        | Defines and exports                                                    | Consumes; does not mint new consumer-facing namespaces |
| Finding JSON schema         | Defines                                                                | Produces                                               |
| `deterministic_hints` shape | Validates                                                              | Executes                                               |
| `specrun codex export`      | Implements                                                             | Calls                                                  |
| `specrun review`            | Non-goal                                                               | Implements deterministic core                          |
| Shared codex parser         | Implements (relocates to `specify-codex` per RFC-32 §"Library layout") | Reuses in indexer                                      |


RFC-28 should reserve extensibility in the codex authoring schema for additional hint kinds without implementing them. See [RFC-28 §Deterministic hints extensibility](done/rfc-28-standards-contract.md#deterministic-hints-extensibility).

### Relationship to framework tooling (Phase 3 — split)

Phase 3 converges framework enforcement toward the same substrate without merging commands.

#### Option A — finding shape only (RFC-28 Phase 3)

Owned by [RFC-28](done/rfc-28-standards-contract.md) Phase 3, not this RFC. Keep imperative `specdev check` predicates. Add a mapper from today's `Finding` to RFC-28 `ReviewFinding` and `specdev check --format json`. Rule ids stay as today (`skill.duplicate-name`, `links.unresolved`, …). No codex migration.

#### Option B — declarative framework rules (higher cost)

Introduce first-party framework policy files under `adapters/shared/codex/framework/` (sibling of `adapters/shared/codex/universal/`) using the same codex markdown shape but a dedicated namespace:


| Namespace | Owner                                                       |
| --------- | ----------------------------------------------------------- |
| `FRAME-`* | Framework-repo checks not tied to a consumer target adapter |


`FRAME-*` rules use the same hint interpreter and WorkspaceModel with `scan_profile: framework`. Imperative checks retire only when a declarative rule plus extractor coverage replaces them. Until then, both may run; duplicate coverage is a migration smell, not a CI failure.

Full normative contract for Option B (framework scan-scope, the `specdev review` verb, the `Origin::Framework` amendment to RFC-28, the `--include-framework` consumer opt-in, and the migration cadence) lives in [RFC-34](rfc-34-framework-convergence.md). This RFC stops at acknowledging Option B as the long-term direction; nothing about RFC-32 Phase 2 acceptance depends on RFC-34 landing.

#### Option C — defer indefinitely

Leave `specdev check` unchanged. RM-10 and framework CI remain separate surfaces sharing only `specify-codex` parsers (per §"Library layout"). This is valid if Phase 3 cost exceeds benefit.

**Recommendation:** ship RFC-28 Phases 1–2, then RFC-32 Phase 2 (`specrun review`); RFC-28 Phase 3 ships in the same PR as Phases 1–2 when unified CI annotations for framework checks are needed; adopt Option B only for predicates that clearly map to reserved hint kinds.

### Predicate migration map (Phase 3 reference)

Reference mapping from the current `crates/authoring/src/check/` predicates (in `augentic/specify-cli`) to declarative kinds. Not a commitment to migrate every row.


| Current rule id prefix  | Declarative kind(s)                    | Phase 3 priority                                      |
| ----------------------- | -------------------------------------- | ----------------------------------------------------- |
| `adapter.`*             | `schema`                               | High — already schema-shaped                          |
| `skill.*` (frontmatter) | `schema`, `unique`, grammar as `regex` | High                                                  |
| `skill.*` (body)        | `cardinality`, `regex`, `set-coverage` | Medium                                                |
| `links.*`               | `reference-resolves`                   | High                                                  |
| `codex.*`               | `schema`, `namespace-owner`, `unique`  | Medium — shape checks may stay in `crates/authoring/` |
| `plugins.*`             | `set-eq`, symlink facts                | Medium                                                |
| `prose.*`               | `regex`, `constant-eq`                 | Medium                                                |
| `scenarios.*`           | `schema`, custom trace freshness       | Low — may stay imperative                             |
| `tools.*`               | `tool`, `constant-eq`                  | Low                                                   |


Predicates invoking subprocesses (`specify source resolve`, declared-tool equivalence) remain `kind: tool` or imperative orchestration.

### Library layout

This RFC introduces two new workspace crates — `specify-codex` (the standards layer) and `specify-schema` (shared JSON-Schema plumbing) — and amends RFC-28's placement of the codex parser accordingly. The crate graph becomes:

```text
specify-error ─┬──> specify-tool ──────────────────────┐
               │                                       │
               └──> specify-schema ──┬──> specify-codex ┐
                                     │                  ├──> specify (root binary)
                                     └──> specify-domain ┘
```

- **`specify-schema` (new leaf).** Owns every embedded JSON Schema constant — workflow (`PLAN_JSON_SCHEMA`, `EVIDENCE_JSON_SCHEMA`, `FUSION_JSON_SCHEMA`, `COMPONENTS_JSON_SCHEMA`), codex (`CODEX_RULE_JSON_SCHEMA`, `RESOLVED_CODEX_JSON_SCHEMA`), and review (`REVIEW_FINDING_JSON_SCHEMA`, `WORKSPACE_MODEL_JSON_SCHEMA`, `REVIEW_RESULT_JSON_SCHEMA`) — plus the generic JSON-Schema plumbing both consumers share (`compile_schema`, `validate_value`, `validate_serialisable`, `validation_error_detail`, `child_pointer`, `read_yaml_as_json`). Depends only on `specify-error` plus `jsonschema` / `serde_json` / `serde_saphyr` / `serde`. Total surface is ~150 LoC; the crate exists to eliminate the only honest duplication between `specify-codex` and `specify-domain` rather than to host elaborate machinery.

- **`specify-codex` (new, this RFC).** The standards layer covering RFC-28 (codex types, parser, resolver, finding wire shape, finding validator) and RFC-32 (WorkspaceModel, indexer, hint interpreter, diagnostic formatters, `specrun review` runner). Depends on `specify-error`, `specify-tool` (for the `kind: tool` hint), and `specify-schema`. Does **not** depend on `specify-domain` — the §"Principles" rule that review carries no lifecycle authority becomes a type-system invariant rather than a coding convention. The crate is named `specify-codex` because operators reach for "codex" as the umbrella noun for everything in this layer; throughout this RFC "codex" is shorthand for the standards surface as a whole (codex authoring **and** review enforcement), not the authoring half alone.

- **`specify-domain` (existing, reshaped).** Workflow only: slice, change, spec, task, adapter, registry, config, merge, validate, init, evidence, journal, discovery, design_system. Loses its `codex/` module and the codex/review schema constants from `schema.rs`; gains a dependency on `specify-schema` so the workflow validators (`validate_plan`, `validate_evidence_dir`, `validate_components_yaml`) keep working. Its `Cargo.toml` description ("slice, change, spec, task, adapter, registry, config, merge, validate, init") finally matches what's in the crate.

- **Sibling, not parent.** `specify-codex` and `specify-domain` are siblings — neither imports the other. The root `specify` binary wires them together for `specrun review` (consumes both: `specify-codex` for the scanner, `specify-domain` for project / slice context resolution). If a future workflow validator ever needs to mint a `ReviewFinding` directly, `specify-domain` gains a dependency on `specify-codex` at that point — the leaf-→-root order still holds. v1 does not need this and the sibling shape keeps the workflow crate strictly independent of the standards crate.

- **`specify-authoring` picks up `specify-codex`.** The `specdev` predicate crate (`crates/authoring/`) currently depends only on `specify-error`. It gains a dependency on `specify-codex` so codex-frontmatter predicates can consume `CODEX_RULE_JSON_SCHEMA` and the typed `CodexRule` DTO directly. This is the precondition for the §"Eliminates the vendored codex-rule schema" cleanup below.

The internal module shape of `specify-codex` preserves the RFC-28 vs RFC-32 split so the two RFC bodies remain navigable from the source tree:

```text
crates/codex/src/         (specify-codex)
├── lib.rs
├── codex.rs              # RFC-28 umbrella + DTOs (CodexRule, ResolvedCodex, ReviewFinding, …)
├── codex/
│   ├── parse.rs
│   ├── resolve.rs        # plus resolve/{filter,sort}.rs
│   ├── finding.rs        # finding validator (CH-16)
│   └── fingerprint.rs
├── review.rs             # RFC-32 umbrella
└── review/
    ├── model.rs          # WorkspaceModel DTOs
    ├── index.rs          # umbrella for extractors
    ├── index/
    │   ├── files.rs      # filesystem walk + profile globs (§D1)
    │   ├── frontmatter.rs
    │   ├── markdown.rs   # sections + links (fence-aware)
    │   ├── symlinks.rs
    │   └── codex.rs      # reuses crate::codex::parse
    ├── eval.rs           # umbrella for hint interpreter
    ├── eval/
    │   ├── path_pattern.rs
    │   ├── schema.rs
    │   ├── regex.rs
    │   └── tool.rs
    └── diagnostics.rs    # umbrella for formatters
crates/codex/src/review/diagnostics/
├── json.rs
├── pretty.rs
├── github.rs
└── compact.rs
```

`scan_profile: framework` extractors (skill, adapter, marketplace, agent-teams) land here when [RFC-34](rfc-34-framework-convergence.md) ships — they live in `specify-codex` for the same reason consumer extractors do (the standards surface owns review code).

`specify-schema` is intentionally flat:

```text
crates/schema/src/         (specify-schema)
├── lib.rs                 # re-exports
├── constants.rs           # every embedded schema (include_str! one-liners)
└── validate.rs            # compile_schema, validate_value, helpers
```

#### Diagnostic formatters

`ReviewFinding` rendering lives in `specify-codex::review::diagnostics` so `specrun review` (Phase 2) and `specdev check --format json` (RFC-28 Phase 3) share one set of formatters and cannot drift. v1 implements:

- `pretty` — terminal output with severity colour and source location;
- `github` — `::error file=…,line=…,title=…::…` workflow annotation format;
- `compact` — one finding per line, suitable for `grep` and PR-bot consumption;
- `json` — the envelope shape RFC-28 defines verbatim.

Heavy presentation dependencies (e.g. syntax highlighting, full markdown rendering) do not belong in `specify-codex` and are out of scope for v1; if they land later they live behind a feature flag or in a separate `specify-diagnostics` crate that depends on `specify-codex`, not the other way around. The `specdev` binary imports `diagnostics` directly for Phase 3; it does not duplicate formatter code at the binary boundary.

### Eliminates the vendored codex-rule schema

The §"Library layout" split lifts the constraint that forced the codex-rule schema to be vendored in two places. Before this RFC:

- `crates/authoring/schemas/codex-rule.schema.json` is the authoring source-of-truth that `specdev check` codex-frontmatter predicates read.
- `specify-cli/schemas/codex/codex-rule.schema.json` is the vendored runtime copy that `specify-domain` embeds via `include_str!` for the codex resolver and DTO round-trip tests.
- `scripts/sync-codex-schema.sh` is the only sanctioned way to refresh the vendored copy (a plain `cp src dst`).
- `codex.schema-drift` (CH-09, `crates/authoring/src/check/codex_schema_drift.rs`) computes SHA-256 over both files and fails CI on drift.

The duplication exists *because* `specify-authoring` deliberately does not depend on `specify-domain` — the predicate crate's charter is to stay lightweight and not drag in workflow types (slice, change, journal, merge). With `specify-codex` as a lightweight sibling crate (no workflow deps), `specify-authoring` gains a dependency on `specify-codex` and consumes the canonical schema constant directly. The vendoring goes away:

1. The canonical schema lives at `specify-cli/schemas/codex/codex-rule.schema.json` and is embedded once in `specify-schema` via `include_str!`.
2. `specify-codex` (the codex resolver) reads `specify_schema::CODEX_RULE_JSON_SCHEMA`.
3. `specify-authoring` (the codex-frontmatter predicate) reads `specify_schema::CODEX_RULE_JSON_SCHEMA` via its new `specify-codex` dependency.
4. `crates/authoring/schemas/codex-rule.schema.json` is deleted.
5. `scripts/sync-codex-schema.sh` is deleted.
6. `crates/authoring/src/check/codex_schema_drift.rs`, its test (`crates/authoring/tests/check_codex_schema_drift.rs`), and the `codex.schema-drift` rule id are deleted.
7. The schema's `$id` URL is corrected to match its on-disk location (today it points at an aspirational `schemas/authoring/` path that does not exist on disk).

This cleanup is bundled into the same implementation PR as the standards-layer split because the justification for the duplication evaporates at the moment `specify-codex` lands — leaving the vendoring in place after the split would ship a confused intermediate state on `main`.

The same logic does not apply to the other authoring-only schemas in `crates/authoring/schemas/` (`marketplace.schema.json`, `scenario.schema.json`, `skill.schema.json`) — those are not duplicated anywhere and stay put.

### Phase 2 normative decisions

A consolidated checklist of pre-flight contracts that Phase 2 implementations MUST honour. These resolve open spots in §"Deterministic hints", §"`specrun review`", and §"Diagnostic formatters" so an implementing agent can start without reopening design.

#### D1 — Consumer scan scope

`scan_profile: consumer` walks `project_dir` with the following defaults:

- **Roots.** `project_dir` itself, plus any path explicitly named in `artifact_paths[]`. Symlinks are recorded as `symlink` facts but **not** traversed.
- **Default include globs.** `**/*.{md,yaml,yml,json,toml,rs,swift,kt,kts,gradle,ts,tsx,js,jsx,py,sql}` plus every path under `.specify/**` (the slice tree).
- **Always-ignore globs.** `target/**`, `**/node_modules/**`, `.git/**`, `dist/**`, `build/**`, `out/**`, `**/.DS_Store`, and every path matching the project-root `.gitignore` (parsed with the `ignore` crate).
- **Binary files.** Files whose first 8 KiB contain a NUL byte are recorded as `file { kind: "binary" }` with no further extraction. Hints with `kind: regex` skip binary files unless the rule's frontmatter sets `applicability.binary: true` (reserved; rejected as `unsupported` until §"Reserved kinds" lands).
- **Encoding.** UTF-8 with U+FFFD replacement on invalid sequences; an `index.warning` finding is emitted once per non-UTF-8 file (severity `optional`).
- **Determinism.** File enumeration is sorted by project-relative path before parallel dispatch. Each extractor sorts its own output before merge into the model envelope so the JSON serialization is byte-stable irrespective of thread scheduling.

`scan_profile: framework` is out of scope for Phase 2 (it lands with Phase 3b).

#### D2 — `--slice` and `--artifact` scope semantics

The two narrowing flags compose with `--target` rather than replacing it:

| Flag                             | Effect on extraction inputs                                                                                                                                                                     |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| _(none)_                         | `artifact_paths[]` is empty; the indexer performs a full consumer scan under D1.                                                                                                                |
| `--slice <name>`                 | `artifact_paths[]` is populated with every path listed under the slice's `tasks.md` *Touches* / *Produces* sections plus `.specify/slices/<name>/**`. Hints still scan within those paths only. |
| `--artifact <path>` (repeatable) | Each `<path>` is appended to `artifact_paths[]` verbatim. Globs are expanded against the D1 enumeration so ignored paths stay ignored.                                                          |
| `--slice` + `--artifact`         | Union of the two sets.                                                                                                                                                                          |

`specrun codex export` is called with the same `--artifact` / `--language` filters so resolved rule set and scan set agree.

#### D3 — `schema` hint value format

`value` is a string that selects a JSON Schema. The interpreter accepts exactly two shapes; everything else is an `unsupported` finding:

1. **Registered schema id.** A bare token resolvable against the schemas the CLI ships (`schemas/codex/resolved.schema.json`, `schemas/review/finding.schema.json`, `schemas/review/workspace-model.schema.json`, the per-axis adapter manifests, …). The token matches the schema's `$id` final path segment, e.g. `value: codex-rule`.
2. **Project-relative `$ref`.** A `./` or `../`-prefixed path to a JSON Schema file under the project tree, resolved against `project_dir`. External `http(s)://` refs are rejected to keep `specrun review` offline and reproducible.

The hint applies to the parsed JSON / YAML body of the candidate file or, if the hint frontmatter sets `target: frontmatter`, to the extracted frontmatter document only. Schema validation errors map to one `ReviewFinding` per failing keyword, with `evidence.json_pointer` recording the failing location.

#### D4 — `tool` hint wire contract

`tool` hints invoke **declared WASI tools only** — no host commands, no shell expansion, no environment leakage. The interpreter calls the same code path `specrun tool run <name>` uses today:

- `value` is the declared tool name (matched against the project's `tools.yaml` / adapter-declared tools).
- Optional `args:` array supplies positional arguments, expanded against a closed set of placeholders: `{artifact}`, `{project_dir}`, `{rule_id}`. Any other `{…}` placeholder rejects as `unsupported`.
- The tool's structured output (RFC-28 `ReviewFinding[]` envelope, or a single-finding shape with `rule_id`, `severity`, `evidence`) is folded into the scan result. Non-zero exit with no findings emits one `tool.invocation-failed` finding with the tool's stderr captured in `evidence.text` (bounded by the RFC-28 evidence cap).
- Tools that are not declared by the project, or that the project's `tools.yaml` does not grant `review` capability, surface as `tool.undeclared` findings (severity `important`).

Phase 2 ships this for at least the `contract` tool so OpenAPI / JSON Schema validation runs through the same review surface as deterministic hints.

#### D5 — Reserved hint policy

- **Default behaviour.** Reserved kinds (anything listed under §"Hint kinds — reserved" above) evaluate as no-ops. The runner emits **one** summary `review.reserved-hint-skipped` finding per scan (severity `optional`) listing the affected `(rule_id, hint_index)` pairs so operators can see which rules carry hints awaiting implementation.
- **Strict mode.** `specrun review --strict-hints` upgrades the summary to severity `important` and contributes to exit-code `2`. Useful in CI for adapter authors who want a forcing function before reserved kinds ship.
- **Single rule id across modes.** Strict mode emits the same `review.reserved-hint-skipped` rule id as default mode (only `severity` differs); producers MUST NOT mint a separate rule id for strict mode so dashboards aggregate cleanly across strict and non-strict runs.
- **No silent skip.** Reserved hints are never silently dropped — at minimum the summary finding fires so the absence of expected findings is observable.

#### D6 — Formatter scope in Phase 2

`specrun review --format <name>` accepts the closed set `{ json, pretty, github, compact }` from day one. All four formatters live in `specify-codex::review::diagnostics` (§"Diagnostic formatters") and ship together in Phase 2 step 4. JSON is the wire contract and the only format whose shape is governed by RFC-28; the other three are presentation layers that render the same envelope. There is no JSON-only intermediate release.

#### D7 — `specrun review` codex-root resolution

The runner resolves the codex tree in the following order (first hit wins):

1. `--codex-root <path>` (explicit flag).
2. `$CODEX_ROOT` environment variable (matches `specrun codex export`).
3. `.specify/cache/codex/` if present (populated by `specrun init` and refreshed by `specrun workspace sync`).
4. The shared codex tree bundled with the CLI install (resolved relative to the binary, mirroring how schemas are located).

Resolution failure (no source found) is `Error::Validation` with exit code `2` and a hint pointing at `specrun init` / `--codex-root`. Consumer projects without a vendored codex but with network access do **not** fetch on demand — that contract belongs to `specrun init`, not `specrun review`.

#### D8 — `specrun review` exit-code map

Pins the closed mapping from runtime failure modes to `Exit::from(&Error)` so CI scripts, JSON envelope consumers, and reviewing agents can switch on a stable surface. Producers MUST NOT introduce new exit codes; new failure modes route through the existing `Error` variants:

| Source error                                                         | `Error` variant                                          | Exit |
| -------------------------------------------------------------------- | -------------------------------------------------------- | ---- |
| Findings present (`summary.critical + important > 0`)                | `Validation`                                             | 2    |
| Strict reserved hint (`--strict-hints`, per §D5)                     | `Validation { rule_id: "review-unsupported-hint-kind" }` | 2    |
| Codex export resolution failure                                      | passthrough from `specrun codex export` resolver mapping | 1/2  |
| WorkspaceModel I/O (filesystem walk, frontmatter read, link resolve) | `Filesystem { op: "review-index" }`                      | 1    |
| `tool` hint subprocess failure (per §D4)                             | `Tool { … }` (passthrough from `specrun tool run`)       | 1    |

The `Filesystem { op }` discriminant and the `rule_id` strings appear verbatim in `--format json` output (kebab-case wire form); changing either is a wire break and requires an envelope version bump per §D9.

#### D9 — Review-result envelope schema

The `specrun review --format json` envelope (`{ version, summary, findings }` per RFC-28 §"Review result envelope") is validated against `schemas/review/review-result.schema.json` before emit. RFC-32 owns this schema; RFC-28 owns the inner `ReviewFinding[]` element shape via `schemas/review/finding.schema.json` (referenced by `$ref`). The `version` field is the discriminant any future RM-11 / RM-14 consumer keys on; widening the envelope (new top-level fields, new `summary` keys) is a `version: 2` change and requires updating both schemas and at least one round-trip consumer fixture in the same PR.

## Implementation Plan

### Phase 2 (required)

1. **Crates and schemas.** Introduce `specify-schema` (constants + JSON-Schema plumbing) and `specify-codex` (codex + review modules) per §"Library layout"; relocate `crates/domain/src/codex/` to `crates/codex/src/codex/`; move the embedded schema constants and validator helpers from `crates/domain/src/schema.rs` to `crates/schema/src/`; add `workspace-model.schema.json` and `review-result.schema.json` (per §D9) under `specify-cli/schemas/review/`; extend codex authoring schema with reserved hint kinds (documented, not executed in RFC-28); execute the §"Eliminates the vendored codex-rule schema" cleanup (delete `crates/authoring/schemas/codex-rule.schema.json`, `scripts/sync-codex-schema.sh`, and the `codex.schema-drift` predicate + test).
2. **Indexer.** Implement `scan_profile: consumer` extractors per §D1 (consumer scan scope): files, frontmatter, markdown links, basic sections, symlinks, binary detection. Reuse RFC-28 codex parser for any codex paths in consumer overlays.
3. **Hint interpreter.** Implement `regex`, `path-pattern`, `schema` (per §D3), `tool` (per §D4) with golden fixtures per kind; reserved kinds emit the §D5 summary finding. Within a scan, evaluate hints in the order `path-pattern` → `schema` → `regex` → `tool` so subprocess hints run only against the candidate set that survived the cheaper filters (mirrors §"Evaluation algorithm").
4. **`specrun review`.** Wire export → index → eval → envelope per §D2 / §D7 (scope flags and codex-root resolution); ship `--dump-model` and all four formatters from §D6 (`json`, `pretty`, `github`, `compact`); add `--strict-hints` for §D5; map failure modes to exit codes per §D8 and validate the JSON envelope against the §D9 schema before emit.
5. **Acceptance.** Golden tests: resolved rules + sample crate tree → stable findings JSON; fingerprint stability; evidence size cap enforcement from RFC-28; one fixture per Phase 2 hint kind plus one for each formatter; `--dump-model` schema-validates against `workspace-model.schema.json`.
6. **Seed policy.** Land `deterministic_hints` on at least one shared `UNI-*` rule (e.g. `UNI-014` URL-in-generated-code via `kind: regex`) and one target-namespaced rule so acceptance has non-empty findings against a fixture tree. Without this step the scanner ships but emits zero findings on real projects.
7. **Roadmap RM-10.** Update review briefs to reference deterministic standards findings alongside human `REVIEW.md`.

### Phase 3 — framework convergence (out of scope)

Phase 3 splits into two surfaces, both **out of scope for this RFC**:

- **Option A — finding-shape mapper** (`specdev check --format json` emitting `ReviewFinding` JSON from existing imperative predicates): owned by [RFC-28 Phase 3](done/rfc-28-standards-contract.md#phase-3--framework-finding-export-specdev), already implemented.
- **Option B — declarative `FRAME-*` rules + framework scan profile + `specdev review` verb**: owned by [RFC-34](rfc-34-framework-convergence.md). RFC-34 carries the full normative contract (framework scan-scope, the `Origin::Framework` amendment to RFC-28, consumer opt-in flag, migration cadence, parity-fixture rule).

Nothing about RFC-32 Phase 2 acceptance depends on RFC-34. RM-10 (CI-native consumer-project standards enforcement) ships on Phase 2 alone.

## Implementation Guide

Non-normative notes for the agent or contributor picking up Phase 2. These are *implementation suggestions*, not contract — the RFC body (§"Design", §"Phase 2 normative decisions") is the source of truth. Items here may evolve in PR review without an RFC amendment.

### Module layout under `crates/codex/src/`

Mirror the existing `crates/domain/src/codex.rs` + `codex/` umbrella precedent (no `mod.rs` outside `tests/`, per `specify-cli` coding standards) inside the new `specify-codex` crate. The full tree is shown in §"Library layout"; the salient rule is that `codex.rs` (RFC-28 surface) and `review.rs` (RFC-32 surface) are sibling umbrellas, each owning a subdirectory of submodules:

```text
crates/codex/src/codex/
├── parse.rs
├── resolve.rs                              # + resolve/{filter,sort}.rs
├── finding.rs                              # CH-16 finding validator
└── fingerprint.rs

crates/codex/src/review/
├── model.rs                                # WorkspaceModel DTOs + version discriminant
├── index/
│   ├── files.rs                            # filesystem walk, profile globs (per §D1)
│   ├── frontmatter.rs                      # markdown --- block extractor
│   ├── markdown.rs                         # sections + links (fence-aware)
│   ├── symlinks.rs
│   └── codex.rs                            # reuses crate::codex::parse
├── eval/
│   ├── path_pattern.rs
│   ├── schema.rs
│   ├── regex.rs
│   └── tool.rs
└── diagnostics/
    ├── json.rs
    ├── pretty.rs
    ├── github.rs
    └── compact.rs
```

`scan_profile: framework` extractors (skill, adapter, marketplace, agent-teams) land under `review/index/` when [RFC-34](rfc-34-framework-convergence.md) ships.

`specify-schema` stays flat — `constants.rs` for the `include_str!` one-liners, `validate.rs` for the helpers, `lib.rs` re-exports both.

### Cargo dependency additions

The new crates land their own `[dependencies]` blocks; only `rayon` and `ignore` are net-new to the workspace:

- **`specify-schema/Cargo.toml`** — `specify-error.workspace = true`, `jsonschema.workspace = true`, `serde.workspace = true`, `serde_json.workspace = true`, `serde-saphyr.workspace = true`. All pre-existing workspace deps.
- **`specify-codex/Cargo.toml`** — `specify-error.workspace = true`, `specify-tool.workspace = true`, `specify-schema.workspace = true`, plus the moved-out parsing/regex/glob deps (`serde`, `serde_json`, `serde-saphyr`, `regex`, `glob`, `thiserror`, `jiff`, `petgraph`, `semver`, `strum`, `tempfile`) and the two net-new entries:
  - `rayon` — the §"Performance — parallelism and incrementality" parallel pass over per-file extractors.
  - `ignore` — `.gitignore`-aware filesystem walk used by §D1's always-ignore globs.
- **`specify-domain/Cargo.toml`** — gains `specify-schema.workspace = true` (for the workflow validators); loses the codex/review schema constants and the JSON-Schema plumbing (both move to `specify-schema`).
- **`specify-authoring/Cargo.toml`** — gains `specify-codex.workspace = true` (precondition for §"Eliminates the vendored codex-rule schema").

Both `rayon` and `ignore` are low-risk in `cargo deny` terms (already in adjacent Rust tooling crates). Defer pinning specific versions until the PR; the workspace pattern is `package.workspace = true`.

### `ReviewAction` CLI shape

A clap-derive sketch for the new `Commands::Review` arm in `src/runtime/cli.rs`, mirroring the existing `CodexAction` precedent in `src/runtime/commands/codex/cli.rs`:

```rust
#[derive(Subcommand)]
pub enum ReviewAction {
    /// Resolve applicable codex rules, build a WorkspaceModel,
    /// evaluate deterministic hints, and emit the RFC-28 review
    /// envelope (RFC-32 §"`specrun review` (Phase 2 CLI)").
    Run {
        #[arg(long)] codex_root: Option<PathBuf>,
        #[arg(long)] target: String,
        #[arg(long = "source", value_name = "NAME")] sources: Vec<String>,
        #[arg(long)] slice: Option<String>,
        #[arg(long = "artifact", value_name = "PATH")] artifacts: Vec<PathBuf>,
        #[arg(long = "language", value_name = "TOKEN")] languages: Vec<String>,
        #[arg(long)] dump_model: bool,
        #[arg(long)] strict_hints: bool,
        #[arg(long, default_value = ".")] project_dir: PathBuf,
    },
}
```

Files (matching the codex command tree):

```text
src/runtime/commands/review.rs              # umbrella
src/runtime/commands/review/cli.rs          # ReviewAction subcommand enum
src/runtime/commands/review/run.rs          # handler: export → index → eval → envelope
```

### Hint interpreter function signature

A reasonable starting shape for each kind's evaluator, kept identical so the runner can dispatch through a closed match:

```rust
pub(crate) fn evaluate(
    rule: &ResolvedRule,
    hint: &DeterministicHint,
    model: &WorkspaceModel,
    project_dir: &Path,
) -> Result<Vec<ReviewFinding>, HintError>;
```

`HintError` is a closed `thiserror` enum mapping to the §D8 exit-code table at the handler boundary.

### Test layout

Golden tests live alongside the new `crates/codex/tests/` integration tree, one fixture per Phase 2 hint kind plus an end-to-end runner:

```text
crates/codex/tests/review_indexer_consumer.rs
crates/codex/tests/review_hint_regex.rs
crates/codex/tests/review_hint_path_pattern.rs
crates/codex/tests/review_hint_schema.rs
crates/codex/tests/review_hint_tool.rs
crates/codex/tests/review_dump_model.rs
crates/codex/tests/fixtures/review/minimal/      # shared minimal fixture tree
tests/review_run.rs                              # binary-level end-to-end
```

`crates/schema/tests/` carries the schema-compile smoke tests that live in `crates/domain/src/schema.rs::tests` today (one `#[test]` per embedded constant, plus the RFC-28 example round-trip tests).

Use the existing `REGENERATE_GOLDENS` convention from `docs/standards/testing.md`. A single shared minimal fixture is cheaper than parallel per-kind fixtures and forces hints to compose cleanly.

### Documentation touch-points (post-merge)

After the implementation PR lands, these follow-up docs need editing — none of these block Phase 2 itself and none belong in this RFC body:

- `specify-cli` `AGENTS.md` — replace the `crates/domain/src/codex/` "Modules of note" row with one row for `crates/codex/src/codex/` and one for `crates/codex/src/review/`; add a row for `crates/schema/`; update the §"Crate graph" diagram to show `specify-schema` as a leaf and `specify-codex` as a sibling of `specify-domain`; update the §"When working in this repo" cross-repo `rg` rule to name `crates/codex/src/codex/` and `crates/codex/src/review/`; add a documentation-map row pointing at this RFC.
- `specify-cli` `docs/standards/architecture.md` — extend the workspace-layout section with `specify-codex` and `specify-schema`; add the crate-graph diagram from §"Library layout"; note the standards-layer-vs-workflow split.
- `specify-cli` `DECISIONS.md` — record the standards-layer split (new `specify-codex` and `specify-schema` crates, sibling shape, no workflow-→-standards dependency) and the vendored-codex-rule-schema removal as standing decisions.
- `specify-cli` `crates/authoring/` — note the new `specify-codex` dependency picked up to eliminate the vendored codex-rule schema; remove any prose that mentions the `codex.schema-drift` predicate or the sync script.
- `specify` `docs/contributing/checks.md` — note `specrun review` as the consumer-project counterpart to `specdev check --format json`; remove the `codex.schema-drift` (CH-09) entry now that the predicate is deleted.
- `specify` `docs/explanation/standards-layer.md` — replace references to "shared codex parser in specify-domain" with `specify-codex`; document the new crate split as the type-system enforcement of the "no lifecycle authority in review" rule.

## Migration

**For operator projects:** `specrun review` is additive. Existing target adapter review briefs and `REVIEW.md` remain authoritative for model-assisted **standards judgment** until operators opt into CI gates on JSON findings.

**For adapter authors:** Add `deterministic_hints` to codex rules that should fire in CI without waiting for custom Rust scanners. Hints must use Phase 2 kinds only until reserved kinds are implemented.

**For codex rule authors:** File location implies the owning adapter — a rule under `adapters/targets/omnia/codex/` is already scoped to Omnia. Omit `applicability.adapters` unless it narrows further (e.g. to `omnia@v2`). The `check::codex` predicate that lints redundant `applicability.adapters` declarations is deferred until the first redundant declaration appears in the tree; the authoring rule is documented here and in `docs/contributing/checks.md` so contributors can find it by reading rather than by tripping a check. (Belongs to `specify-authoring` when implemented, not the runtime resolver.)

**For framework contributors:** No change from RFC-32 alone. RFC-28 Phase 3 already ships `specdev check --format json` (imperative findings in `ReviewFinding` shape); declarative `FRAME-*` rules and the `specdev review` verb arrive with [RFC-34](rfc-34-framework-convergence.md). `make check` continues to run imperative predicates throughout.

**For CLI maintainers:** Keep indexer and interpreter free of lifecycle transitions. Do not persist WorkspaceModel under `.specify/` without a separate RFC.

## Alternatives Considered

**Kept codex types in `specify-domain` (the original RFC-32 v1 design).** Rejected on second pass. The original design co-located standards code with workflow code on the basis that splitting added a crate without delivering a concrete win. In practice the split delivers three: (1) `specify-authoring` can consume the canonical codex schema directly, eliminating the `crates/authoring/schemas/codex-rule.schema.json` vendoring + `scripts/sync-codex-schema.sh` + `codex.schema-drift` predicate (CH-09) trio; (2) the §"Principles" rule that review carries no lifecycle authority becomes a type-system invariant rather than a coding convention; (3) `specify-domain` shrinks to the workflow-only crate its `Cargo.toml` description has always claimed it is. See §"Library layout" and §"Eliminates the vendored codex-rule schema".

**Bundled the JSON-Schema plumbing into `specify-codex` or `specify-domain` rather than a third crate.** Rejected. Both crates need the helpers (`compile_schema`, `validate_value`, …) and the embedded schema constants. Folding them into `specify-codex` forces `specify-domain` to depend on `specify-codex` for `validate_plan` / `validate_evidence_dir` — wrong direction. Folding them into `specify-domain` forces `specify-codex` to depend on `specify-domain` — also wrong, and inverts the standards-vs-workflow separation. `specify-schema` is a thin leaf crate (~150 LoC) that owns the schema constants and the validator helpers; both `specify-codex` and `specify-domain` depend on it. The duplication-vs-coupling tradeoff (duplicate the helpers into both crates and avoid a new crate entirely) was considered and rejected because the workspace already has two concrete consumers from day one and `specify-authoring` would become a third the moment a codex-frontmatter predicate ports to declarative form.

**Fold into RFC-28.** Rejected. RFC-28's value is landing the contract and export before any scanner exists. Combining execution would delay RM-10 and blur resolver vs scanner boundaries.

**Datalog (Soufflé / embedded) as the rule language.** Rejected for v1. Powerful for graph invariants but poor contributor ergonomics for a closed first-party rule set. Reserved hint kinds cover the same invariants with a smaller vocabulary; revisit if FRAME rule count exceeds ~100 graph-heavy rules.

**Semgrep / ast-grep as the engine.** Rejected as primary. Useful for `regex` and language-specific paths inside hint values, but does not unify markdown frontmatter, marketplace graphs, or RFC-28 finding emission without a Specify-owned model layer.

**Replace markdown skills with RFC-4 manifests first.** Rejected as prerequisite. Typed manifests reduce the need for markdown extractors but are independent; WorkspaceModel must scan today's repos.

**Single command (`specify check`) for framework and consumer.** Rejected. RFC-5 and RFC-28 preserve separate audiences and failure semantics.

## Non-Goals

- Model-assisted or hybrid review execution (agents remain in target adapter briefs for Phase 2).
- Implementing every reserved hint kind in the first Phase 2 PR.
- Declarative `FRAME-*` rules, the framework scan profile, the `specdev review` verb, and the `Origin::Framework` enum widening (all owned by [RFC-34](rfc-34-framework-convergence.md); not required for RM-10).
- SARIF output (may follow as an export adapter).
- Persisting WorkspaceModel as a Specify artifact.
- Auto-opening slices or mutating lifecycle state from findings.
- RFC-4 Option 2/3 skill manifest or DSL work.

## Resolved Decisions

Every question raised during drafting is resolved in the body. The list below indexes the resolutions for reviewers checking that no design question is parked.

- **Standards-layer crate split** — §"Library layout" (new `specify-codex` crate sibling to `specify-domain`; new leaf `specify-schema` crate owning embedded schema constants and JSON-Schema plumbing; neither standards crate depends on workflow code).
- **Vendored codex-rule schema removal** — §"Eliminates the vendored codex-rule schema" (deletes `crates/authoring/schemas/codex-rule.schema.json`, `scripts/sync-codex-schema.sh`, and the `codex.schema-drift` CH-09 predicate; the standards-layer split makes the workaround unnecessary; bundled into the same implementation PR as the split).
- **Indexer parallelism + incrementality** — §"Performance — parallelism and incrementality" (rayon from day one, sequential cross-file pass, `.specify/cache/index.v1.json` reserved).
- **Shared diagnostic formatter location** — §"Diagnostic formatters" (`specify-codex::review::diagnostics`).
- **WorkspaceModel persistence and query surface** — §"Persistence and query (v1 decision)" (in-memory only; cache path and `specrun model query` reserved).
- **Phase 2 normative contract** — §"Phase 2 normative decisions" D1–D9 (scan scope, scope-flag composition, schema/tool hint payloads, reserved-hint policy, formatter scope, codex-root resolution, exit-code map, envelope schema).
- **`FRAME-*` placement, framework scan profile, `specdev review` CLI surface, `Origin::Framework` enum widening, consumer opt-in flag, `Check` → `FRAME-*` migration cadence** — carved out as [RFC-34](rfc-34-framework-convergence.md). This RFC commits only to the high-level direction; the wire and CLI contracts live in RFC-34 to keep RFC-32 acceptance focused on consumer-side enforcement.
- **Redundant `applicability.adapters` lint** — Migration §"For codex rule authors" (predicate deferred until first redundant declaration; inference rule documented as authoring guidance in this RFC and in `docs/contributing/checks.md`).

## References

- [RFC-28: Engineering Standards — Codex Contract and Findings](done/rfc-28-standards-contract.md)
- [Standards layer (explanation)](../docs/explanation/standards-layer.md)
- [RFC-5: Framework Developer Tooling](done/rfc-5-tooling.md)
- [RFC-4: Type-Safe Skill Expression](future/rfc-4-dsl.md)
- [Specify Roadmap — RM-10](roadmap.md#rm-10-ci-native-standards-enforcement)
- [docs/contributing/checks.md](../docs/contributing/checks.md)
- [`specify-cli` `crates/authoring/src/check.rs`](https://github.com/augentic/specify-cli/blob/main/crates/authoring/src/check.rs) — current imperative predicate registry
- [augentic/lints](https://github.com/augentic/lints) — reference for diagnostics UX in Phase 3

