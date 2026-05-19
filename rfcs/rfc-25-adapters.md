# RFC-25: Directional Adapters

> Status: Draft — Supersedes [RFC-20 (archived)](archive/rfc-20-survey.md). Pairs with [RFC-26: Workflow Collapse](rfc-26-workflow.md), which depends on this RFC's `enumerate`/`extract` symmetry. Compatible with [RFC-22](rfc-22-ledger.md) and [RFC-24](rfc-24-omnia.md) (target rename + `shape` ownership); [RFC-23](archive/rfc-23-change-lifecycle.md) survives through 2.x and is retired by RFC-26.

## Abstract

Refactor Specify into a small **core** plus two directional adapter roles — **source adapters** and **target adapters**. Source adapters normalise evidence (intent, documentation, legacy code, OpenAPI, …) into two core-facing intermediate shapes: plan-time `CandidateSet`s and slice-time `EvidencePack`s. Specify core synthesises canonical artifacts from evidence packs, applying provenance and `[unknown]` / `[conflict]` rules in one place. Target adapters shape those canonical artifacts for a runtime and turn them into runnable code (omnia, vectis, contracts, …). Both adapter roles share one plugin implementation shape (manifest + briefs + optional WASI tools + resolver + cache). Documentation-driven specification is a default source path, implemented through the same source-adapter contract as every other input, not a side channel.

**v1 ships single-source slices.** `planSlice.sources` is a list in the schema and `EvidencePackSet` is the core-facing input type, but every v1 slice binds exactly one source. Multi-source synthesis is deferred — see §Non-Goals. The v1 floor still exercises every architectural seam: source adapters return `CandidateSet`s and `EvidencePack`s, core owns synthesis, target adapters shape and implement.

This is a ground-up redesign of the adapter axis. **There is no backward compatibility** — `project.yaml`, `registry.yaml`, `plan.yaml`, `sources.yaml`, `adapters/`, brief paths, CLI verbs, schema field names, and adapter-touching skill files all change shape in lockstep. The workflow skill family (`/change:{draft,execute,finalize}`, `/spec:{define,build,merge}`) keeps today's shapes through this RFC; their renames are RFC-26's territory.

## Motivation

Three structural problems made the current shape brittle.

**One semantic operation, two parallel paths.** `/change:analyze` (documentation) and `/change:survey` (legacy code) answer the same question — "what slice-sized candidates exist in this source?" — and append to the same `## Candidate inventory` heading in `discovery.md`. They are not two operations; they are one operation with two evidence sources. The same duplication recurs at slice time: `/spec:define` writes artifacts from intent and docs, `/spec:extract` writes artifacts from code. Two skill families, one contract.

**Legacy migration is not core.** Deriving specs from existing source is a one-time, language-aware archaeology task that matters during migration and not afterwards. Today it lives in the framework's spine: enumeration briefs, repair loops, `surfaces.json` schema, language detection, the `legacy-code` kind discriminator, the `specify change survey` CLI verb. After a project finishes migrating, every line of this surface is dead weight that still has to be carried, taught, and tested.

**The adapter slot has no name for inputs.** Today, unqualified `adapter` names the *target* runtime (omnia, vectis, contracts). There is no symmetrical phrase for the *source* of evidence; the framework gestures at it through `kind`, through `language`, through per-language brief directories, through the `source` CLI noun on `/change:draft`. The asymmetry shows up as cognitive load on every skill author trying to add a new input.

The redesign collapses these three problems into one move: qualify adapters by direction, give source and target adapters the same implementation shape, make documentation-driven specification a first-class default source path, and put legacy-code support outside the core.

## Design

### Principles

1. **Two adapter roles, one implementation shape.** Source and target adapters share one plugin shape: same loader, same validator, same cache layout, same `specify {source,target} resolve` verb family.
2. **Core is small.** Core ships the workflow (`/spec:`*, `/change:*`), the plugin resolver, the candidate-block grammar, the `discovery.md` handshake, the default `intent` and `documentation` source adapters, and the CLI primitives. Legacy-code, contract-import, and every target adapter remain add-ons.
3. **Source adapters emit intermediate representations, not artifacts.** The source-axis contract has exactly two core-facing shapes: `CandidateSet` for planning and `EvidencePack` for slice synthesis. Source adapters never own `spec.md`, `design.md`, or `tasks.md`.
4. **Core owns synthesis.** Specify core turns an `EvidencePackSet` plus target shaping guidance into canonical artifacts. That keeps provenance and conflict handling in one place rather than redistributing them across source or target adapters.
5. **`EvidencePackSet` is a set, of cardinality one in v1.** A slice's evidence is structurally a set of source evidence packs. v1 ships single-source slices: every `planSlice.sources` list has exactly one entry, every `EvidencePackSet` has exactly one pack. The set framing is preserved so multi-source can be added by extending synthesis, not by rewriting the contract.
6. **Provenance is mechanical.** Every requirement in `spec.md` records which source supplied it. `Sources:` lines and the closed `Status:` enum are uniform across single-source v1 and the eventual multi-source extension; the line is degenerate (one entry) in v1 rather than absent.
7. **Single writer.** The CLI is still the only writer of `plan.yaml`, `.metadata.yaml`, archive paths, `migration-log.yaml`, `discovery.md`, and the new `sources.yaml` / `targets.yaml` files. Adapters read; the CLI writes.
8. **Ground-up.** No alias keys, no compat fallbacks, no transitional schemas. The rename and restructure ship as one breaking minor.

### Vocabulary


| Term           | Meaning                                                                                                                                                                                                                    |
| -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **source adapter** | Pluggable input role. Enumerates candidates from an evidence corpus and extracts per-candidate evidence packs. Examples: `intent`, `documentation`, `legacy-code-typescript`, `legacy-code-cobol`, `openapi`. |
| **target adapter** | Pluggable output role. Shapes canonical Specify artifacts for a runtime and turns them into runnable code. Examples: `omnia`, `vectis`, `contracts`. Replaces today's unqualified `adapter`. |
| **plugin** | Shared implementation shape of either adapter role. Used in schemas, the resolver, and implementation docs when speaking about both at once. |
| **candidate** / **candidate set** | A slice-sized unit of work proposed by `enumerate`, and the set returned for one source. Serialised as candidate blocks in `discovery.md` under `## Candidate inventory`. |
| **evidence pack** | Persisted source evidence returned by `extract`. Structured input to slice synthesis; stores facts, provenance, paths, spans, hashes, and bounded excerpts only when explicitly allowed. |
| **evidence pack set** | The complete set of evidence packs bound to one slice. Input to Specify-owned artifact synthesis. |
| **provenance** | The source binding backing a single requirement in `spec.md`. (List in the schema; cardinality one in v1.) |
| **conflict** | An evidence pack containing self-inconsistent facts, or a missing fact that synthesis cannot resolve. Surfaces as a `[conflict]` tag in the synthesised artifact. (Inter-pack conflict is post-v1; see §Principles.) |


`provider` is reserved for Omnia DI and **not** used as an adapter-role name. `profile` is retired and **not** reintroduced. Unqualified `adapter` is removed; the public nouns are **source adapter** and **target adapter**. `plugin` remains an implementation noun for the shared manifest / resolver / cache shape.

### Adapter implementation shape

Both `source` and `target` adapters share one manifest shape. A source adapter lives at `sources/<name>/source.yaml`; a target adapter lives at `targets/<name>/target.yaml`. Schema fragments are identical except for the `axis` discriminator, the `capabilities` enum, and source-only detection rules.

```yaml
# sources/<name>/source.yaml
name: legacy-code-typescript
version: 1
axis: source
capabilities:
  - enumerate
  - extract
briefs:
  enumerate: briefs/enumerate.md
  extract:   briefs/extract.md
tools:                               # optional WASI tools per RFC-15
  - id: typescript-walker
    wasm: tools/walker.wasm
```

Source-adapter auto-detection from a path (`detect[]`) is deferred from v1: operators name the source explicitly at binding time (`source legacy=./repo`). The field reappears when an ergonomic shortcut for path-only binding earns its keep.

```yaml
# targets/<name>/target.yaml
name: omnia
version: 1
axis: target
capabilities:
  - shape
  - build
  - merge
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
tools:                               # optional WASI tools per RFC-15
  - id: omnia-validator
    wasm: tools/omnia-validator.wasm
```

Rules:

- `name` is kebab-case, globally unique within an axis. Names *may* collide across axes (a hypothetical `mermaid` source adapter and `mermaid` target adapter would not clash).
- `axis` is a closed enum: `source | target`. Validated at load time.
- `capabilities[]` is a closed enum, one set per axis. A source adapter declares some subset of `{enumerate, extract}`; a target adapter declares some subset of `{shape, build, merge}`.
- `briefs.<capability>` is the path to the markdown brief implementing that capability. Required for every declared capability.
- `tools[]` reuses the existing RFC-15 WASI tool shape. Tools materialise into `.specify/.cache/{sources,targets}/<name>/tools/` alongside briefs.

### Source adapter contract

A source adapter contributes two capabilities to the workflow. These capabilities return intermediate representations consumed by Specify core; they do not write final Specify artifacts.

**`enumerate(source-binding) → CandidateSet`.** Called at plan time by `/change:draft`. Reads the bound evidence (a local path, a documentation file, a free-text intent string) and emits candidate blocks under `## Candidate inventory` in `discovery.md`. Output grammar is the existing candidate-block format from RFC-20 §`discovery.md`, extended with stable candidate ids and optional correlation hints.

**`extract(candidate, source-binding) → EvidencePack`.** Called at slice time by `/spec:define` for each source bound to the slice. Returns a structured evidence pack persisted under the slice before synthesis:

```yaml
# .specify/slices/<slice>/evidence/<source-key>.yaml
source: legacy-monolith              # source-binding key
adapter: legacy-code-typescript      # source adapter name
candidate: user-registration         # candidate this pack covers
evidence:
  - kind: code-excerpt
    path: src/users/register.ts
    lines: [12, 87]
    sha256: 6c25...
    excerpt: |
      export async function registerUser(req: …) { … }
  - kind: type-definition
    name: RegisterRequest
    path: src/users/types.ts
    lines: [4, 16]
    sha256: a84d...
  - kind: external-call
    method: POST
    url: https://api.example.com/verify
    request-shape: { token: string }
    response-shape: { ok: boolean }
```

Documentation sources use the same envelope with doc-flavoured `kind:` values (`requirement-statement`, `acceptance-criterion`, `decision-record`, `document-section`, `diagram-reference`) in place of `code-excerpt` / `type-definition` / `external-call`.

The pack's `kind` enum is closed and shared across all source adapters. Initial kinds: `intent-text`, `requirement-statement`, `acceptance-criterion`, `decision-record`, `document-section`, `diagram-reference`, `contract-reference`, `code-excerpt`, `type-definition`, `external-call`. New kinds require an RFC update. Evidence packs do not store raw source bodies by default — only structured facts, relative paths, line spans, content hashes, and bounded excerpts when the adapter contract explicitly allows them.

The pack envelope reserves an optional `authority:` field for the source adapter's self-classification, populated when the multi-source extension lands. v1 synthesis ignores the field (single-source slices have no cross-source ranking to perform). The schema accepts it as an optional string so source adapters that already emit it remain valid.

Packs validate against `schemas/evidence-pack.schema.json` before synthesis. The CLI writes packs; source adapters return content through briefs and WASI tools, not by touching slice paths directly.

### Default source adapters

Core ships two source adapters by default.

| Adapter | Role |
| ------- | ---- |
| `intent` | Captures operator-authored briefs, inline requirements, and explicit corrections. Used when no other source is bound. |
| `documentation` | Captures requirements documents, design notes, proposals, RFCs, existing specs, architecture records, and other written product or technical intent. The ordinary documentation-driven path into specs and design. |

Both are true source adapters with manifests, `enumerate` briefs, and `extract` briefs. They are default-packaged for usability, but they do not get special workflow rules. `/change:draft` still calls `enumerate`; `/spec:define` still calls `extract`; slice synthesis still consumes an evidence pack.

**2.x inline-intent path:** when `/spec:define` starts without an accepted plan candidate, the workflow binds `intent` implicitly, runs a degenerate slice-time `intent.enumerate` (one synthetic candidate), then `intent.extract`, then synthesises. **3.0** ([RFC-26](rfc-26-workflow.md)) moves all enumeration to `/spec:plan`; orphan define without `plan.yaml` is not supported.

### Target adapter contract

A target adapter contributes runtime-specific shaping and implementation behavior for canonical Specify artifacts. It does **not** own source-to-`spec.md` / `design.md` synthesis; Specify core owns that contract so provenance handling stays uniform (and so the future multi-source extension lands in one place rather than across N target adapters). A target adapter may declare:

- `shape` — optional target-idiom guidance consumed by core synthesis when producing `proposal.md`, `spec.md`, `design.md`, and `tasks.md`. For Omnia this includes crate boundaries, provider patterns, handler vocabulary, and WASM constraints; for Vectis it includes Crux core and shell composition conventions.
- `build` — implementation briefs and optional tools that turn reviewed artifacts into code.
- `merge` — merge/finalisation briefs and optional tools for target-specific cleanup after a slice is built.

The manifest shape matches today's `adapter.yaml`; see §CLI surface for the full path / verb rename table.

**Pipeline verbs split by phase.** Today's `specify adapter pipeline {define,build,merge}` moves to core-owned synthesis (define phase) and hand-coded skill substeps (build/merge phase) for v1. Standalone topology verbs (`specify slice synthesize`, `specify target build`, `specify target merge`) return when a third-party target needs custom brief ordering — see [`commands.md`](commands.md).

Unqualified `adapter` does not survive anywhere in the schema, on the CLI, in docs, or in skill prose. RFC-24's "adapter-gated finding" becomes "target-gated finding"; `planSlice.adapter` becomes `planSlice.target`. Today target adapter briefs often own `proposal`, `specs`, `design`, and `tasks` directly — under this RFC those move into Specify-owned synthesis, and target adapters supply `shape` guidance plus `build` / `merge` behavior.

### Discovery handshake — candidate correlation

v1 enumeration runs one source adapter per plan; each candidate block carries a stable `id` and a `sources: [<single-key>]` declaration matching the one bound source. Re-running `enumerate` against the same source replaces blocks by stable id rather than appending duplicates (the existing "skip if heading exists" rule is dropped in favour of explicit id-based replace).

Cross-source candidate merging and `correlates-with` hints are deferred — see §Non-Goals. The schema keeps `sources` as a list and `id` as required so the extension is additive.

**Normative schema.** Candidate blocks in `discovery.md` validate against `schemas/discovery/candidate-block.schema.json` (extends RFC-20 with required `id` and required `sources[]`, cardinality one in v1). The CLI discovery writer is the parser of record; there is no separate on-disk `candidates.yaml` in v1 — `discovery.md` under `## Candidate inventory` remains the plan-time source of truth, and `specify plan add` reads blocks from there.

### `planSlice.sources` — single-binding (v1)

`planSlice.sources` is **a list** in the schema (it already is in RFC-24 examples; this RFC makes it normative). v1 requires exactly one entry; the list shape is preserved so the multi-source extension is additive, not breaking:

```yaml
slices:
  - name: identity-user-registration
    target: omnia                   # renamed from `adapter`
    project: identity-svc
    sources: [legacy-monolith]      # cardinality 1 in v1; schema permits ≥1
    status: pending
```

Three archetypes follow without special-casing:


| Archetype       | `sources`                | Meaning                                  |
| --------------- | ------------------------ | ---------------------------------------- |
| Pure greenfield | `[intent]`               | New work driven by operator intent. `intent` is bound implicitly when `sources` is omitted; `[]` is normalised to `[intent]` before extraction. |
| Pure port       | `[<one-legacy-source>]`  | Legacy code dictates behaviour.          |
| Pure design     | `[<one-doc-source>]`     | Documentation dictates behaviour.        |


Mixed-evidence slices (`[code-source, doc-source, …]`) validate against the schema but are rejected at plan-add time in v1 with a structured error pointing operators at the multi-source extension's open status. The slice loop does not branch on archetype — every v1 archetype runs the same single-source synthesis step (§Slice authoring synthesis).

### Slice authoring synthesis

`/spec:define` (2.x) and `/spec:refine` (3.0, [RFC-26](rfc-26-workflow.md)) share one pipeline:

1. **Resolve** the bound target adapter (one) and the bound source adapter (one in v1).
2. **Extract** per §Extraction reliability — call `extract(candidate, source-binding)` on the bound source; persist the pack under `.specify/slices/<slice>/evidence/<source-key>.yaml`.
3. **Synthesise** per §Synthesis contract — invoke core synthesis with the `EvidencePackSet` (cardinality 1) plus optional `target.shape` guidance.
4. **Validate** — `specify slice validate` checks structural requirements (`Sources:`, `Status:`, closed tags) and emits findings for `[conflict]` and `[unknown]`.
5. **Lifecycle** — transition to `defined`. Tags on requirements (`[conflict]` for intra-pack contradictions, `[unknown]` for missing facts) are operator-visible signals to review `spec.md` before `/spec:build`, but they do not park the slice in a separate lifecycle state.

**N=1 in v1:** `[conflict]` comes from intra-pack contradiction or unreconciled evidence; `[unknown]` covers missing facts. **N=0** normalises to `[intent]` plus a synthetic candidate on the 2.x inline path; 3.0 requires a plan entry ([RFC-26](rfc-26-workflow.md)). The `Status:` enum reserves `divergence` for the multi-source extension (§Non-Goals); v1 never writes it.

### Per-requirement provenance and tags

`spec.md` gains a fixed-format `Sources:` line below every `ID: REQ-XXX` block:

```markdown
### Requirement: User registration accepts valid email

ID: REQ-001
Sources: [legacy-monolith]
Status: agreed

The system accepts a registration request when the email field is RFC-5322 valid …
```

`Status:` is a closed enum: `agreed | unknown | conflict`. (`divergence` is reserved for multi-source synthesis; not written in v1.) Together with `Sources:`, it makes the synthesis decision auditable.

`Sources:` is required on every requirement, including those derived from operator intent during define — those populate `Sources: [intent]` so the audit trail is uniform across legacy-driven, doc-driven, and intent-only slices.

### Synthesis contract

Core-owned synthesis is the single writer of `proposal.md`, `spec.md`, `design.md`, and `tasks.md`. Target adapters supply `shape` guidance only.

**Inputs**

| Input | Source | Validated by |
| ----- | ------ | ------------ |
| `EvidencePackSet` | `.specify/slices/<slice>/evidence/*.yaml` | `schemas/evidence-pack.schema.json` |
| `planSlice` bindings | `plan.yaml` entry | `schemas/plan/plan.schema.json` |
| `shape` brief | `specify target resolve` | target manifest |

**Outputs**

| Artifact | Required sections |
| -------- | ----------------- |
| `proposal.md` | Scope, motivation (existing Specify artifact rules) |
| `specs/<crate>/spec.md` | Per requirement: `ID:`, `Sources:`, `Status:`; optional `[conflict]` / `[unknown]` |
| `design.md` | Domain model, integrations (existing rules) |
| `tasks.md` | Sequenced implementation tasks |

**Division of labour**

| Layer | Responsibility |
| ----- | -------------- |
| **Agent** | Semantic authoring from evidence packs and shape guidance (brief body under `plugins/spec/references/synthesis/`) |
| **CLI** | `specify slice validate` (structure, provenance lines, tag enum), `specify slice transition` (lifecycle stamps) |

**Substep order (v1).** Hand-coded in `/spec:define` (2.x) or `/spec:refine` (3.0): `proposal` → `specs` → `design` → `tasks`, optionally injecting `target.shape` guidance from `specify target resolve`. Every brief is core-owned; targets do not register define-phase briefs.

**Halt rules.** Synthesis writes through every requirement; `[conflict]` and `[unknown]` tags surface in `spec.md` for operator review but do not abort the pass or park the slice. Slice lifecycle is `defining → defined → built → merged` — see [RFC-26 §Combined lifecycle](rfc-26-workflow.md#combined-lifecycle-rfc-25--rfc-26).

### Extraction reliability

| Rule | Behaviour |
| ---- | --------- |
| **Serial** | v1 binds one source per slice and runs `extract` once; the pack writes atomically via the CLI. Parallel extraction across N bindings is post-v1 with the multi-source extension. |
| **Required** | The single key in `planSlice.sources` is required. An `optional: true` per-binding flag is reserved for the multi-source extension but not in the v1 schema. |
| **Hard failure** | If `extract` fails, the slice stays in `defining`, no synthesis runs, and the CLI emits a structured error naming the source key. |
| **Partial packs** | Invalid packs fail validation against `evidence-pack.schema.json` before synthesis starts. |

### Resolver and cache

A single resolver handles both axes. Caches sit under:

```text
.specify/.cache/
├── sources/
│   ├── intent/
│   ├── documentation/
│   └── legacy-code-typescript/
└── targets/
    ├── omnia/
    ├── vectis/
    └── contracts/
```

Plugin manifests, briefs, and WASI tools are materialised under their axis subdirectory. The resolver is one Rust module (`crates/domain/src/plugin/resolver.rs`) with axis-discriminated routes. Brief loading uses one code path.

### `project.yaml` shape

```yaml
# .specify/project.yaml
specify_version: 2.0.0
sources:                          # set of source adapters available in this project
  - intent                        # default source adapter
  - documentation                 # default source adapter
  - legacy-code-typescript
targets:                          # set of target adapters this project produces
  - omnia
  - contracts
hub: false
```

`sources` lists source adapters available for binding to slices, not bindings themselves. Source bindings live on individual sources (a path, a doc location, the operator's intent text) and are recorded in `sources.yaml` per [RFC-21](rfc-21-catalogue.md). `targets` lists target adapters this project can produce; per-slice targeting lives on `planSlice.target`.

The old singular `project.yaml.profile` / `project.yaml.adapter` field is gone.

### CLI surface

The full v1 CLI floor — including verbs cut from v1 and post-v1 deferrals — is enumerated in [`commands.md`](commands.md). RFC-25 axis-relevant deltas:

**Renames:** `specify adapter resolve` → `specify target resolve`; `adapters/<name>/adapter.yaml` → `targets/<name>/target.yaml`; `schemas/adapter.schema.json` → `schemas/target.schema.json`; `planSlice.adapter` → `planSlice.target`; `Plan::resolve_adapter` → `Plan::resolve_target`; `Error::AdapterResolution` → `Error::TargetResolution`. `specify adapter pipeline {define,build,merge}` retires — substep order is hand-coded in skills for v1.

**Addition (v1):** `specify source resolve <name>` — materialise a source adapter's briefs and tools.

**Retirements:**


| Verb / Skill            | Replaced by                                                                                                                                     |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `/change:analyze`       | One discovery stage in `/change:draft` that resolves the bound source adapter and calls `enumerate`.                                             |
| `/change:survey`        | Same — when the bound source adapter is a `legacy-code-*` flavour, `enumerate` does what `survey` does today.                                    |
| `/spec:extract`         | The bound source adapter's `extract` capability, called from `/spec:define`.                                                                     |
| `specify change survey` | Folded into the source-adapter-driven discovery stage. The bounded repair loop becomes a contract on the source adapter's `enumerate` capability. |


### Repository layout

v1 keeps every adapter inside the `augentic/specify` monorepo. The repository carve-out (one repo per target adapter, one repo per source-adapter family) is deferred until an individual adapter needs an independent release cadence — the structural prerequisite is the unified `plugin.schema.json` resolver and cache layout, which land in v1 regardless.

**Layout in `augentic/specify` (v1):**

- `/spec:{init,define,build,merge,drop}`, `/change:{draft,execute,finalize}` — workflow skills.
- `plugins/references/` — cross-cutting references.
- `sources/intent/`, `sources/documentation/` — default source adapters.
- `sources/legacy-code-typescript/` — the single legacy-code language v1 ships, carved from `plugins/change/skills/survey/briefs/enumerate/typescript.md` and the existing repair loop.
- `targets/omnia/`, `targets/vectis/`, `targets/contracts/` — carved from `adapters/<name>/`.

**Deferred from v1:**

- `sources/legacy-code-{cobol,csharp,rust,javascript}` — ship per language as a real consumer needs it.
- `sources/{openapi,asyncapi,json-schema}` — today's `/contract:*` import surface stays as today's skills for v1; conversion into source adapters lands with the multi-source extension when slices need to combine a contract source with operator intent.
- Splitting `targets/<name>/` and the legacy-code family out of the monorepo.

### `surfaces.json` and per-language briefs

`surfaces.json` becomes a source-adapter-internal artifact owned by `sources/legacy-code-typescript/`. Its schema, repair loop, and validator code move out of `specify-cli` into the adapter (which may ship its own WASI tools). The `specify change survey` CLI verb is deleted. The TypeScript enumeration brief moves from `plugins/change/skills/survey/briefs/enumerate/typescript.md` to `sources/legacy-code-typescript/briefs/enumerate.md`. Other languages stay parked at today's brief location until the adapter is built.

### `survey.md`

Renamed `discovery-summary.md` and made generic. Sections become:

1. `# <change> discovery summary`
2. `## Summary` — counts: candidate / unresolved.
3. `## Source inventory` — one row for the bound source: source-key, adapter, location, contribution count. (The row count grows to N with the multi-source extension.)
4. `## Candidate inventory` — fenced-YAML blocks per candidate; one block per stable id.

Legacy-code-only columns (LOC, language, `surfaces.json` digest) populate only when the bound source adapter supplied them. The same file shape covers documentation-only and intent-only runs.

## Implementation Plan

1. **Schemas.** Land `schemas/plugin.schema.json`, `schemas/source.schema.json`, `schemas/target.schema.json`, `schemas/evidence-pack.schema.json`, and `schemas/discovery/candidate-block.schema.json`. Delete `schemas/adapter.schema.json`. Update `schemas/plan/plan.schema.json` to rename `adapter` → `target` and make `sources` a required list (min 1, max 1 in v1; the upper bound relaxes with the multi-source extension). Update `schemas/sources/sources.schema.json` for source-adapter identity fields. The closed `Status:` enum is `agreed | unknown | conflict`; `divergence` is reserved but not yet emitted.
2. **Domain rename.** Mass-rename unqualified `Adapter`* → `Target*` across today's target-runtime code in `crates/domain/`, `crates/tool/`, `crates/error/`, `src/`. Update `Error` discriminants. Update `Plan::resolve_adapter` → `Plan::resolve_target`. Land the new `Plan::resolve_source` returning `SourceAdapter` (single binding in v1; switches to `Plan::resolve_sources` returning `Vec<SourceAdapter>` with the multi-source extension).
3. **Plugin loader.** New module `crates/domain/src/plugin/` containing `resolver.rs`, `cache.rs`, `manifest.rs`, `axis.rs`. Replaces `crates/domain/src/adapter/`. One loader, two axes.
4. **Default source adapters.** Ship `sources/intent/` and `sources/documentation/` in core. Each has a manifest, `briefs/enumerate.md`, and `briefs/extract.md`. `intent` enumerate emits one candidate from the operator's brief and extract emits the brief text as `kind: intent-text`; `documentation` enumerate reads bound docs and emits candidate blocks, then extract emits documentation-native evidence entries such as `requirement-statement`, `acceptance-criterion`, `decision-record`, and `document-section`.
5. **Slice synthesis.** Implement §Synthesis contract: core briefs under `plugins/spec/references/synthesis/`, `/spec:define` refactored to extract → synthesise → validate, substep order hand-coded in the skill body for v1. Migrate target define briefs into core synthesis + `shape` briefs. The `specify slice synthesize` topology verb is deferred until a third-party target needs custom brief ordering.
6. **Provenance tags.** Extend `spec.md` parser in `crates/domain/src/specs/` to require `ID:`, `Sources:`, `Status:` lines on every requirement block. Add `[conflict]` to the closed tag enum alongside the existing `[unknown]`. (`[divergence]` ships with the multi-source extension.)
7. **Discovery handshake.** Implement stable-id replace-by-id in the discovery writer (no append-and-skip behaviour, no cross-source merge). Add fixture and golden coverage for re-enumerate idempotence.
8. **CLI surface.** New v1 verb: `specify source resolve <name>` (materialises briefs + WASI tools). Rename: `specify adapter resolve` → `specify target resolve`. Retire `specify adapter pipeline {define,build,merge}` — the substep order is hand-coded in `/spec:define` (2.x) or `/spec:refine` (3.0), `/spec:build`, `/spec:merge`. Delete: `specify change survey`. See [`commands.md`](commands.md) for the full v1 floor and cut verbs.
9. **Target brief migration.** Move today's target-owned `proposal`, `specs`, `design`, and `tasks` brief content into the core synthesis contract where it is target-neutral, and into target `shape` briefs where it is target-specific. Update RFC-24 and target skill prose to describe `shape` as guidance, not artifact ownership.
10. **Documentation rewrite.** `AGENTS.md`, `.cursor/rules/project.mdc`, `docs/explanation/decision-log.md` (§Decision-log supersessions), `docs/contributing/adapter-anatomy.md` — adapter vocabulary, pipeline split, and superseded "analyze/extract split" / define-phase target ownership. RFC-22, RFC-24 prose updated.
11. **`discovery-summary.md` rename.** Implement the generic form. Update fixtures.
12. **Acceptance.** Cross-repo Deno suite gains (land **before** RFC-26 collapse scenarios): documentation-only slice with `Sources:` provenance; legacy-only slice with `Sources:` provenance; pure-intent slice; intra-pack `[conflict]` surfaced in `spec.md`; target-`shape` fixture; required-source extract failure; invalid evidence-pack schema rejection; multi-source slice request rejected at `specify plan add` with a structured error.
13. **Observability ([RFC-19](rfc-19-observability.md)).** Journal events for `extract` completion (one per slice in v1, one per source key with the multi-source extension), synthesis completion, and `[conflict]` findings — so operators get traceability without parsing skill output.

## Migration

**There is no backward compatibility.** This RFC ships as Specify 2.0.

A one-shot `migrate-to-2.0.sh` script performs mechanical renames against `project.yaml`, `registry.yaml`, `plan.yaml`, `sources.yaml`, `.specify/.cache/`, and `.specify/archive/` (`yq` + `sed`). The plugin cache re-fetches on next invocation; operators who stay on 1.x pin plugin and CLI versions. There is **no** `specify upgrade` verb — see [`commands.md`](commands.md).

Plugin authors ship renamed manifests against the new schemas; `adapter.yaml` fails to load on 2.0 with no grace period. JSON envelopes rename `adapter` → `target`; add `sources[]` consumers where slice-level evidence matters.

**RFC-26 (Specify 3.0):** most teams jump **1.x → 3.0** once both RFCs land. Combined migration (`migrate-to-3.0.sh`) and workflow-skill renames are [RFC-26 §Migration](rfc-26-workflow.md#migration). Skill authors should not invest in `/change:*` changes during the 2.x line.

## Alternatives Considered

Key rejections (full rationale in [`docs/explanation/decision-log.md`](../docs/explanation/decision-log.md) when this RFC lands):

- **One unified "lens" name** — plugin shape is shared; roles are not. Two qualified roles (`source adapter`, `target adapter`) read honestly.
- **Keep `/spec:extract`, `/change:analyze`, `/change:survey` as named skills** — the names *are* the bifurcation; one discovery stage and one authoring entry point with source-adapter dispatch is the move.
- **Per-source artifact files (`spec.<source>.md`)** — operators review one `spec.md`; inline `Sources:` lines preserve the audit trail.
- **Source adapters emit artifacts directly; targets own define-phase briefs** — duplicates provenance handling and blocks multi-source synthesis in core. Source adapters emit `CandidateSet`s and `EvidencePack`s only; targets supply `shape` + `build` + `merge`.
- **Target adapters participate in discovery** — re-merges the axes; baseline-aware planning is RFC-22's ledger territory.
- **Ship multi-source synthesis in v1** — no v1 caller exercises it; schema reserves list/set/`divergence` for an additive extension (§Non-Goals).

## Non-Goals

- Backward compatibility with Specify 1.x manifests, schemas, verbs, or directory layouts.
- A general "plugin marketplace" or runtime plugin discovery. Source and target adapters are installed at project-init time.
- Per-handler provenance below the requirement level. `Sources:` lives on requirement blocks; finer granularity belongs in `design.md` per existing convention.
- Per-pack confidence scores. Authority is class-based (when the multi-source extension lands); finer scoring belongs to a future RFC if operator demand emerges.
- Replacing operator review of conflicts. `[conflict]` tags surface in `spec.md`; operators decide. Auto-resolution heuristics are out of scope.
- Cross-repo source sharing. Each platform-repo declares its own sources via `sources.yaml`, consistent with RFC-21.
- Bidirectional adapters (an adapter that is both source and target). The axis is a discriminator, not a tag set.
- Source-adapter support for editing artifacts after slice authoring. Sources read; the workflow writes.

**Deferred from v1, reinstated when a real caller asks:**

- Multi-source slices (`planSlice.sources` cardinality > 1). v1 schema permits at most one binding; the multi-source extension brings authority hierarchy, `[divergence]` tagging, inter-pack `[conflict]` detection by `claim-id`, parallel `extract` calls, the `optional: true` per-binding flag, operator authority overrides at slice-binding time, `correlates-with` candidate correlation hints, and `specify plan amend --add-source` / `--remove-source` CLI affordances.
- Source-adapter `detect[]` auto-detection from a path.
- Repository carve-out (per-adapter or per-family repos). v1 keeps everything in `augentic/specify`.
- Legacy-code source families beyond `legacy-code-typescript`.
- Contract-source adapters (`sources/{openapi,asyncapi,json-schema}`). Today's `/contract:*` skills cover the v1 import path.

## Open Questions

1. Should the default `intent` and `documentation` source adapters be packaged as true plugin implementations (with `source.yaml` etc.) or hard-wired into the core CLI as built-ins? Current preference: true plugin implementations, shipped in-repo under `sources/intent/` and `sources/documentation/`, so the plugin shape has zero exceptions.
2. How should `extract` be sandboxed when a source adapter ships a WASI tool? Current preference: same posture as RFC-15 — the WASI tool runs under the existing `specify tool run` sandbox; briefs read its output.
3. Should `specify target` accept multiple targets per project for projects that produce both an Omnia service and a Vectis app from the same artifacts? Current preference: deferred — today's one-target-per-project assumption is held; a future RFC may relax it once a real bi-targeting case lands.

## References

- [RFC-19: Observability](rfc-19-observability.md) — journal events for extract and synthesis (implementation plan step 13).
- [RFC-20: Survey-to-Plan Pipeline (archived)](archive/rfc-20-survey.md) — the survey pipeline this RFC folds into the `sources/legacy-code` adapter family.
- [RFC-21: Source Catalogue and Tier-1 Cache](rfc-21-catalogue.md) — `sources.yaml` survives; binding fields extend to record source-adapter identity.
- [RFC-22: Migration Ledger and Slice Mapping](rfc-22-ledger.md) — adapter-typed entries become target-typed; otherwise unchanged.
- [RFC-23: Change Lifecycle (archived)](archive/rfc-23-change-lifecycle.md) — the `/change:draft` → `/change:execute` → `/change:finalize` three-skill model survives this RFC; the discovery stage inside `/change:draft` is restructured here. RFC-23's three-skill model is itself superseded by [RFC-26](rfc-26-workflow.md).
- [RFC-24: Omnia Plan Composition](rfc-24-omnia.md) — adapter-gated findings become target-gated, and Omnia artifact-authoring briefs become target `shape` guidance. `omnia` becomes a target adapter.
- [RFC-26: Workflow Collapse](rfc-26-workflow.md) — planned follow-on; collapses the `/change:*` and `/spec:*` skill families into one operator surface on top of this RFC's adapter axis.
- [RFC-15: WASM Plugins (archived)](archive/rfc-15-wasm-plugins.md) — the WASI tool surface reused as the deterministic-CLI seam inside source and target adapters.
- `[specify-cli/AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md)` — exit codes and CLI contract preserved; rename surface documented there.
- `[.cursor/rules/project.mdc](../.cursor/rules/project.mdc)` — artifact authority hierarchy. The synthesis-time authority hierarchy (`intent > external-contract > design-spec > observed-behaviour`) is designed but unimplemented in v1; it ships with the multi-source extension per §Non-Goals.

