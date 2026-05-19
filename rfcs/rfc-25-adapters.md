# RFC-25: Directional Adapters

> Status: Draft - Supersedes: [RFC-20 (archived)](archive/rfc-20-survey.md) and the `kind: legacy-code | documentation` discriminator in `/change:analyze`. Replaces the existing unqualified `adapter` axis with directional `source adapter` and `target adapter` roles. Compatible with: [RFC-22](rfc-22-ledger.md), [RFC-23 (archived)](archive/rfc-23-change-lifecycle.md), [RFC-24](rfc-24-omnia.md) — RFC-22 gains the rename and otherwise stands; RFC-23's discovery → propose → execute → finalize seam is preserved under today's `/change:*` verb names (which [RFC-26](rfc-26-collapse.md) later retires); RFC-24 also gains the target `shape` ownership change described here.
>
> See also: [RFC-26: Workflow Collapse](rfc-26-collapse.md) — the planned follow-on which depends on this RFC's `enumerate`/`extract` symmetry.

## Abstract

Refactor Specify into a small **core** plus two directional adapter roles — **source adapters** and **target adapters**. Source adapters normalise evidence (intent, documentation, legacy code, OpenAPI, …) into two core-facing intermediate shapes: plan-time `CandidateSet`s and slice-time `EvidencePack`s. Specify core synthesises canonical artifacts from evidence packs, applying provenance, authority, `[unknown]`, `[conflict]`, and `[divergence]` rules in one place. Target adapters shape those canonical artifacts for a runtime and turn them into runnable code (omnia, vectis, contracts, …). Both adapter roles share one plugin implementation shape (manifest + briefs + optional WASI tools + resolver + cache). A single slice may bind multiple sources; slice authoring synthesises evidence from all bound sources with explicit per-requirement provenance and a `[conflict]` tag for genuine disagreements. Documentation-driven specification is a default source path, implemented through the same source-adapter contract as every other input, not a side channel. Greenfield, migration, and mixed-evidence work become three values of one parameter (the bound source set) rather than three forked code paths.

This is a ground-up redesign of the adapter axis. **There is no backward compatibility** — `project.yaml`, `registry.yaml`, `plan.yaml`, `sources.yaml`, `adapters/`, brief paths, CLI verbs, schema field names, and adapter-touching skill files all change shape in lockstep. The workflow skill family (`/change:{draft,execute,finalize}`, `/spec:{define,build,merge}`) keeps today's shapes through this RFC; their renames are RFC-26's territory.

## Motivation

Three structural problems made the current shape brittle.

**One semantic operation, two parallel paths.** `/change:analyze` (documentation) and `/change:survey` (legacy code) answer the same question — "what slice-sized candidates exist in this source?" — and append to the same `## Candidate inventory` heading in `discovery.md`. They are not two operations; they are one operation with two evidence sources. The same duplication recurs at slice time: `/spec:define` writes artifacts from intent and docs, `/spec:extract` writes artifacts from code. Two skill families, one contract.

**Legacy migration is not core.** Deriving specs from existing source is a one-time, language-aware archaeology task that matters during migration and not afterwards. Today it lives in the framework's spine: enumeration briefs, repair loops, `surfaces.json` schema, language detection, the `legacy-code` kind discriminator, the `specify change survey` CLI verb. After a project finishes migrating, every line of this surface is dead weight that still has to be carried, taught, and tested.

**The adapter slot has no name for inputs.** Today, unqualified `adapter` names the *target* runtime (omnia, vectis, contracts). There is no symmetrical phrase for the *source* of evidence; the framework gestures at it through `kind`, through `language`, through per-language brief directories, through the `source` CLI noun on `/change:draft`. The asymmetry shows up as cognitive load on every skill author trying to add a new input.

The redesign collapses these three problems into one move: qualify adapters by direction, give source and target adapters the same implementation shape, make documentation-driven specification a first-class default source path, and put legacy-code support outside the core.

## Design

### Principles

1. **Two adapter roles, one implementation shape.** Source and target adapters share one plugin shape: same loader, same validator, same cache layout, same `specify {source,target} {resolve,list,validate}` verb family.
2. **Core is small.** Core ships the workflow (`/spec:`*, `/change:*`), the plugin resolver, the candidate-block grammar, the `discovery.md` handshake, the default `intent` and `documentation` source adapters, and the CLI primitives. Legacy-code, contract-import, and every target adapter remain add-ons.
3. **Source adapters emit intermediate representations, not artifacts.** The source-axis contract has exactly two core-facing shapes: `CandidateSet` for planning and `EvidencePack` for slice synthesis. Source adapters never own `spec.md`, `design.md`, or `tasks.md`.
4. **Core owns synthesis.** Specify core turns an `EvidencePackSet` plus target shaping guidance into canonical artifacts. That keeps provenance, conflict detection, and authority handling in one place rather than redistributing them across source or target adapters.
5. **Multi-source is the general case.** A slice's evidence is a *set* of source evidence packs. Single-source slices are a degenerate case. Greenfield, pure migration, and realistic mixed-evidence work are three values of one parameter.
6. **Provenance is mechanical.** Every requirement in `spec.md` records which source(s) supplied it. Conflicts between sources halt the slice with an explicit `[conflict]` tag rather than silent resolution.
7. **Authority is hierarchical and explicit.** When sources disagree on the same fact, a published authority hierarchy decides which evidence wins. Operators can override per-requirement; the framework never silently picks.
8. **Single writer.** The CLI is still the only writer of `plan.yaml`, `.metadata.yaml`, archive paths, `migration-log.yaml`, `discovery.md`, and the new `sources.yaml` / `targets.yaml` files. Adapters read; the CLI writes.
9. **Ground-up.** No alias keys, no compat fallbacks, no transitional schemas. The rename and restructure ship as one breaking minor.

### Vocabulary


| Term           | Meaning                                                                                                                                                                                                                    |
| -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **source adapter** | Pluggable input role. Enumerates candidates from an evidence corpus and extracts per-candidate evidence packs. Examples: `intent`, `documentation`, `legacy-code-typescript`, `legacy-code-cobol`, `openapi`. |
| **target adapter** | Pluggable output role. Shapes canonical Specify artifacts for a runtime and turns them into runnable code. Examples: `omnia`, `vectis`, `contracts`. Replaces today's unqualified `adapter`. |
| **plugin** | Shared implementation shape of either adapter role. Used in schemas, the resolver, and implementation docs when speaking about both at once. |
| **candidate** / **candidate set** | A slice-sized unit of work proposed by `enumerate`, and the set returned for one source. Serialised as candidate blocks in `discovery.md` under `## Candidate inventory`. |
| **evidence pack** | Persisted source evidence returned by `extract`. Structured input to slice synthesis; stores facts, provenance, paths, spans, hashes, and bounded excerpts only when explicitly allowed. |
| **evidence pack set** | The complete set of evidence packs bound to one slice. Input to Specify-owned artifact synthesis. |
| **provenance** | The set of source bindings backing a single requirement in `spec.md`. |
| **conflict** | Two or more evidence packs disagreeing on the same fact. Surfaces as a `[conflict]` tag in the synthesised artifact. |


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
detect:                              # source adapters only — optional auto-detection
  - { kind: file-glob, pattern: "package.json" }
  - { kind: file-glob, pattern: "tsconfig.json" }
```

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
- `detect[]` is source-only. Used by the resolver when an operator binds a path without naming the source explicitly.

### Source adapter contract

A source adapter contributes two capabilities to the workflow. These capabilities return intermediate representations consumed by Specify core; they do not write final Specify artifacts.

**`enumerate(source-binding) → CandidateSet`.** Called at plan time by `/change:draft`. Reads the bound evidence (a local path, a documentation file, a free-text intent string) and emits candidate blocks under `## Candidate inventory` in `discovery.md`. Output grammar is the existing candidate-block format from RFC-20 §`discovery.md`, extended with stable candidate ids and optional correlation hints.

**`extract(candidate, source-binding) → EvidencePack`.** Called at slice time by `/spec:define` for each source bound to the slice. Returns a structured evidence pack persisted under the slice before synthesis:

```yaml
# .specify/slices/<slice>/evidence/<source-key>.yaml
source: legacy-monolith              # source-binding key
adapter: legacy-code-typescript      # source adapter name
candidate: user-registration         # candidate this pack covers
authority: observed-behaviour        # one of: intent | external-contract | design-spec | observed-behaviour
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

The pack's `kind` enum is closed and shared across all source adapters. Initial kinds: `intent-text`, `requirement-statement`, `acceptance-criterion`, `decision-record`, `document-section`, `diagram-reference`, `contract-reference`, `code-excerpt`, `type-definition`, `external-call`. New kinds require an RFC update. The `authority` field is the source adapter's self-classification (see §Authority hierarchy); the operator may override at slice-binding time. Evidence packs do not store raw source bodies by default — only structured facts, relative paths, line spans, content hashes, and bounded excerpts when the adapter contract explicitly allows them.

Packs validate against `schemas/evidence-pack.schema.json` before synthesis. The CLI writes packs; source adapters return content through briefs and WASI tools, not by touching slice paths directly.

### Default source adapters

Core ships two source adapters by default.

| Adapter | Role | Default authority |
| ------- | ---- | ----------------- |
| `intent` | Captures operator-authored briefs, inline requirements, and explicit corrections. Used when no other source is bound, or alongside other sources when the operator adds clarifying intent. | `intent` |
| `documentation` | Captures requirements documents, design notes, proposals, RFCs, existing specs, architecture records, and other written product or technical intent. Used for the ordinary documentation-driven path into specs and design. | `design-spec` |

Both are true source adapters with manifests, `enumerate` briefs, and `extract` briefs. They are default-packaged for usability, but they do not get special workflow rules. `/change:draft` still calls `enumerate`; `/spec:define` still calls `extract`; slice synthesis still consumes evidence packs.

Small point-solution work uses the same contract without forcing the operator through a heavyweight planning pass. **Specify 2.x (this RFC):** when `/spec:define` starts from inline operator intent and no accepted plan candidate exists, the workflow binds `intent` implicitly, calls `intent.enumerate` at slice time (one synthetic candidate from the operator brief), then `intent.extract`, then synthesises — enumerate still runs, but plan-time review may be absent. **Specify 3.0 ([RFC-26](rfc-26-collapse.md)):** enumerate runs only in `/spec:plan`; `/spec:refine` never invents candidates. Orphan `/spec:define` without `plan.yaml` is not supported on 3.0. The adapter call sequence is uniform — every slice runs `extract` against plan-bound sources; enumerate always precedes extract, either at plan time (3.0 and normal 2.x multi-slice work) or as a degenerate slice-time call (2.x inline-intent only).

### Target adapter contract

A target adapter contributes runtime-specific shaping and implementation behavior for canonical Specify artifacts. It does **not** own source-to-`spec.md` / `design.md` synthesis; Specify core owns that contract so multi-source provenance and conflict handling stay uniform. A target adapter may declare:

- `shape` — optional target-idiom guidance consumed by core synthesis when producing `proposal.md`, `spec.md`, `design.md`, and `tasks.md`. For Omnia this includes crate boundaries, provider patterns, handler vocabulary, and WASM constraints; for Vectis it includes Crux core and shell composition conventions.
- `build` — implementation briefs and optional tools that turn reviewed artifacts into code.
- `merge` — merge/finalisation briefs and optional tools for target-specific cleanup after a slice is built.

The manifest shape matches today's `adapter.yaml`; see §CLI surface for the full path / verb rename table.

**Pipeline verbs split by phase.** Today's `specify adapter pipeline define` drove topological artifact generation inside target adapters. That role moves to core-owned synthesis (§Synthesis contract). Target adapters retain pipeline only for implementation phases:

- `specify slice synthesize pipeline` — ordered core synthesis substeps (`proposal` → `specs` → `design` → `tasks`), optionally depending on a resolved `target.shape` brief. Replaces `specify adapter pipeline define`.
- `specify target pipeline build` — build brief topology (unchanged responsibility).
- `specify target pipeline merge` — merge brief topology (unchanged responsibility).

Unqualified `adapter` does not survive anywhere in the schema, on the CLI, in docs, or in skill prose. RFC-24's "adapter-gated finding" becomes "target-gated finding"; `planSlice.adapter` becomes `planSlice.target`; `Plan::resolve_adapter` becomes `Plan::resolve_target`. The renames are mechanical; the ownership change is not. Today target adapter briefs often own `proposal`, `specs`, `design`, and `tasks` directly. Under this RFC those artifact briefs move into Specify-owned synthesis, and target adapters supply `shape` guidance plus `build` / `merge` behavior.

### Discovery handshake — candidate correlation

The existing rule in `/change:analyze` and `/change:survey` is "append candidate blocks; skip blocks whose `### <name>` heading already exists in `discovery.md`". The new rule is:

> When two candidate blocks have the same stable `id`, **merge** `sources` and `declared-at` into the existing block (union, sorted). When two blocks only share a heading, slug, or adapter-supplied correlation hint, record the relationship as `correlates-with` and leave the final merge to propose review.

The candidate-block grammar's `sources: [...]` field becomes load-bearing: a block with `sources: [legacy-monolith, design-doc]` declares that two source adapters corroborated the same candidate. The stable `id` is the merge key; headings and source-authored aliases are correlation hints. When a hint is wrong, the operator splits or relabels in propose.

**Normative schema.** Candidate blocks in `discovery.md` validate against `schemas/discovery/candidate-block.schema.json` (extends RFC-20 with required `id`, load-bearing `sources[]`, and optional `correlates-with`). The CLI discovery writer is the parser of record; there is no separate on-disk `candidates.yaml` in v1 — `discovery.md` under `## Candidate inventory` remains the plan-time source of truth, and `specify plan add` reads blocks from there.

### `planSlice.sources` — multi-binding

`planSlice.sources` is **a list** in the schema (it already is in RFC-24 examples; this RFC makes it normative and load-bearing). A slice may bind 0..N sources:

```yaml
slices:
  - name: identity-user-registration
    target: omnia                   # renamed from `adapter`
    project: identity-svc
    sources: [legacy-monolith, design-doc, customer-api-spec]
    status: pending
```

Three archetypes follow without special-casing:


| Archetype       | `sources`                      | Meaning                                  |
| --------------- | ------------------------------ | ---------------------------------------- |
| Pure greenfield | `[intent]`                     | New work driven by operator intent only. `intent` is bound implicitly when `sources` is omitted; `[]` is normalised to `[intent]` before extraction. |
| Pure port       | `[<one-legacy-source>]`        | Legacy code dictates behaviour.          |
| Pure design     | `[<one-doc-source>]`           | Documentation dictates behaviour.        |
| Mixed migration | `[code-source, doc-source, …]` | Code + docs (+ specs) synthesised.       |


The slice loop does not branch on archetype. The synthesis step (§Slice authoring synthesis) handles all four uniformly.

### Slice authoring synthesis

`/spec:define` (2.x) and `/spec:refine` (3.0, [RFC-26](rfc-26-collapse.md)) share one pipeline when invoked for a slice with N bound sources:

1. **Resolve** the bound target adapter (one) and each bound source adapter (N).
2. **Extract** per §Extraction reliability — call `extract(candidate, source-binding)` on each bound source; persist packs under `.specify/slices/<slice>/evidence/<source-key>.yaml`.
3. **Synthesise** per §Synthesis contract — invoke core synthesis with the full `EvidencePackSet` plus optional `target.shape` guidance.
4. **Validate** — `specify slice validate` checks structural requirements (`Sources:`, `Status:`, closed tags) and emits findings for `[conflict]`, `[divergence]`, and `[unknown]`.
5. **Lifecycle** — on success without `[conflict]`, transition toward `defined` (3.0) or `awaiting-review` → `defined` (2.x interim). On `[conflict]`, park at Gate 2 per §Synthesis ↔ Gate 2 policy.

**N=1:** inter-source conflict is impossible when only one source is bound; **intra-pack** contradictions, operator authority overrides, or bad extraction can still yield `[conflict]` or `Status: conflict`. Provenance is trivially one-element (`Sources: [<key>]`).

**N=0:** normalised before extraction to `[intent]` and a synthetic candidate (2.x inline path) or forbidden on 3.0 without a plan entry.

### Evidence authority hierarchy

When evidence packs disagree on the same fact, synthesis applies a published authority order:

1. **`intent`** — operator-stated requirements trump everything.
2. **`external-contract`** — OpenAPI, AsyncAPI, JSON Schema, signed contracts. Canonical for wire shape.
3. **`design-spec`** — architecture notes, design docs. Canonical for intent.
4. **`observed-behaviour`** — legacy code. Canonical for current implementation.

The hierarchy is *advisory for ranking*, not *silencing*. Lower-authority evidence is still recorded in `declared-at`. A conflict between two same-authority sources surfaces as `[conflict]`. A conflict between adjacent authorities (e.g. `design-spec` says X, `observed-behaviour` says Y) defaults to the higher-authority claim *and* tags the requirement `[divergence]` so the operator sees that the legacy code currently does something else.

`[divergence]` is the third evidence-tag peer (alongside `[unknown]` and `[conflict]`). It is *not* a synthesis halt by itself; it is a signal that the slice will produce a behaviour change against the legacy baseline. Gate 2 behaviour for divergence is normative in §Synthesis ↔ Gate 2 policy (consumed by [RFC-26](rfc-26-collapse.md)).

**Same fact.** Two claims conflict when synthesis assigns them the same `claim-id` — a stable hash of normalised subject + predicate + object derived from evidence entry kind and fields (documented in the synthesis contract). Overlapping line spans alone are insufficient; the synthesis brief must emit explicit claim linkage.

### Synthesis ↔ Gate 2 policy

RFC-26 implements Gate 2 from this table. Specify 2.x uses `awaiting-review` where 3.0 uses `defined_provisional`.

| Synthesis outcome | `Status:` | Inline tag | Default supervised (Gate 2) | `--yes-gate2` (3.0 only) |
| ----------------- | --------- | ---------- | --------------------------- | ------------------------ |
| Clean | `agreed` | none | Auto-promote to `defined` | Auto-promote |
| Cross-authority resolved | `divergence` | `[divergence]` | **Stop** at `defined_provisional` — operator accepts behaviour change | Auto-promote with journal entry |
| Same-authority unresolved | `conflict` | `[conflict]` | **Stop** — no auto-promote | **Stop** — flag does not resolve conflicts |
| Missing evidence | `unknown` | `[unknown]` | **Stop** at `defined_provisional` | Auto-promote only if no `[conflict]` |

`--yes-gate2` skips the review *pause* when synthesis produced no `[conflict]` markers and no `Status: conflict`. It does **not** pick winners between conflicting claims.

### Per-requirement provenance and tags

`spec.md` gains a fixed-format `Sources:` line below every `ID: REQ-XXX` block:

```markdown
### Requirement: User registration accepts valid email

ID: REQ-001
Sources: [legacy-monolith, design-doc]
Status: agreed

The system accepts a registration request when the email field is RFC-5322 valid …
```

`Status:` is a closed enum: `agreed | divergence | unknown | conflict`. Together with `Sources:`, it makes the synthesis decision auditable.

`Sources:` is required on every requirement, including those derived from operator intent during define — those populate `Sources: [intent]` so the audit trail is uniform across single-source, multi-source, and intent-only slices.

### Synthesis contract

Core-owned synthesis is the single writer of `proposal.md`, `spec.md`, `design.md`, and `tasks.md`. Target adapters supply `shape` guidance only.

**Inputs**

| Input | Source | Validated by |
| ----- | ------ | ------------ |
| `EvidencePackSet` | `.specify/slices/<slice>/evidence/*.yaml` | `schemas/evidence-pack.schema.json` |
| `planSlice` bindings | `plan.yaml` entry | `schemas/plan/plan.schema.json` |
| `shape` brief | `specify target resolve` + optional `specify slice synthesize pipeline` | target manifest |

**Outputs**

| Artifact | Required sections |
| -------- | ----------------- |
| `proposal.md` | Scope, motivation (existing Specify artifact rules) |
| `specs/<crate>/spec.md` | Per requirement: `ID:`, `Sources:`, `Status:`; optional `[conflict]` / `[divergence]` / `[unknown]` |
| `design.md` | Domain model, integrations (existing rules) |
| `tasks.md` | Sequenced implementation tasks |

**Division of labour**

| Layer | Responsibility |
| ----- | -------------- |
| **Agent** | Semantic authoring from evidence packs and shape guidance (brief body under `plugins/spec/references/synthesis/`) |
| **CLI** | `specify slice synthesize pipeline` (substep order), `specify slice validate` (structure, provenance lines, tag enum), lifecycle transitions |

**Substep order.** `specify slice synthesize pipeline --change <slice-dir> --format json` returns the same topological shape today's `adapter pipeline define` returned, but every brief is core-owned and may declare a dependency on the resolved `shape` brief path. Targets do not register define-phase briefs.

**Halt rules.** Synthesis stops writing forward when any requirement would receive `Status: conflict` or an inline `[conflict]` tag; partial artifacts may remain on disk for operator inspection. `[divergence]` and `[unknown]` do not abort the synthesis pass; they affect Gate 2 per §Synthesis ↔ Gate 2 policy.

### Extraction reliability

| Rule | Behaviour |
| ---- | --------- |
| **Parallelism** | `extract` calls for distinct source bindings may run in parallel; each writes one pack file atomically via the CLI. |
| **Required sources** | Every key in `planSlice.sources` is required unless `plan.yaml` marks it `optional: true` (schema extension). |
| **Hard failure** | If any required `extract` fails, the slice stays in `defining`, no synthesis run, persisted packs from successful extracts are kept for debugging, and the CLI emits a structured error naming the failed source key. |
| **Partial packs** | Invalid packs fail validation against `evidence-pack.schema.json` before synthesis starts. |

### Lifecycle coordination (RFC-26)

Slice lifecycle on disk uses snake_case enums in `.metadata.yaml` (YAML-friendly). Display and JSON envelopes may use dotted aliases.

| 2.x (interim) | 3.0 (RFC-26) | Meaning |
| ------------- | ------------ | ------- |
| `defining` | `defining` | Extract or synthesise in progress |
| `awaiting-review` | `defined_provisional` | Gate 2 — operator review before build |
| `defined` | `defined` | Synthesis accepted; build may run |
| `built` | `built` | Implementation complete |
| `merged` | `merged` | Baseline updated |

RFC-25 lands synthesis and provenance; RFC-26 retires `awaiting-review` in favour of `defined_provisional`. See [RFC-26 §Combined lifecycle](rfc-26-collapse.md#combined-lifecycle-rfc-25--rfc-26).

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

Renames:


| Before                              | After                                      |
| ----------------------------------- | ------------------------------------------ |
| `specify adapter resolve`           | `specify target resolve`                   |
| `specify adapter pipeline define`   | `specify slice synthesize pipeline`        |
| `specify adapter pipeline build`    | `specify target pipeline build`            |
| `specify adapter pipeline merge`    | `specify target pipeline merge`            |
| `adapters/<name>/adapter.yaml`      | `targets/<name>/target.yaml`               |
| `schemas/adapter.schema.json`       | `schemas/target.schema.json`               |
| `planSlice.adapter`                 | `planSlice.target`                         |
| `Plan::resolve_adapter`             | `Plan::resolve_target`                     |
| `Error::AdapterResolution`          | `Error::TargetResolution`                  |


Additions:


| Verb                                               | Purpose                                         |
| -------------------------------------------------- | ----------------------------------------------- |
| `specify source resolve <name>`                    | Materialise a source adapter's briefs and tools. |
| `specify source list`                              | List installed source adapters.                  |
| `specify source validate <name>`                   | Validate a source adapter's manifest.            |
| `specify slice synthesize pipeline [--change <dir>]` | Core synthesis substep order (replaces define pipeline). |
| `specify plan amend <slice> --add-source <key>`    | Bind a source to a slice.                       |
| `specify plan amend <slice> --remove-source <key>` | Unbind a source.                                |


Retirements:


| Verb / Skill            | Replaced by                                                                                                                                     |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| `/change:analyze`       | One discovery stage in `/change:draft` that resolves the bound source adapter and calls `enumerate`.                                             |
| `/change:survey`        | Same — when the bound source adapter is a `legacy-code-*` flavour, `enumerate` does what `survey` does today.                                    |
| `/spec:extract`         | The bound source adapter's `extract` capability, called from `/spec:define`.                                                                     |
| `specify change survey` | Folded into the source-adapter-driven discovery stage. The bounded repair loop becomes a contract on the source adapter's `enumerate` capability. |


### Repository carve-out

**Stays in core (`augentic/specify`):**

- `/spec:{init,define,build,merge,drop}`
- `/change:{draft,execute,finalize}`
- The candidate-block grammar and `discovery.md` handshake.
- The `intent` and `documentation` source adapters, shipped as default source paths.
- The unified resolver, cache layout, and `plugin.schema.json`.
- `plugins/references/` — cross-cutting references.

**Moves to add-ons:**


| Adapter                                                         | New home                                                         | Notes                                                                                                                                    |
| --------------------------------------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| `targets/omnia`                                                 | `augentic/specify-targets-omnia` (or equivalent monorepo subdir) | Carved from `adapters/omnia/`.                                                                                                           |
| `targets/vectis`                                                | `augentic/specify-targets-vectis`                                | Carved from `adapters/vectis/`.                                                                                                          |
| `targets/contracts`                                             | `augentic/specify-targets-contracts`                             | Carved from `adapters/contracts/`.                                                                                                       |
| `sources/legacy-code-{typescript,cobol,csharp,rust,javascript}` | `augentic/specify-sources-legacy`                                | New, absorbs `plugins/change/skills/survey/briefs/enumerate/<language>.md`, the repair loop, `surfaces.json` schema, language detection. |
| `sources/openapi`, `sources/asyncapi`, `sources/json-schema`    | `augentic/specify-sources-contracts`                             | New, mirrors today's `/contract:`* import surface as evidence sources.                                                                   |

### `surfaces.json` and per-language briefs

`surfaces.json` becomes a source-adapter-internal artifact owned by `sources/legacy-code-*`. Its schema, repair loop, and validator code move out of `specify-cli` into the legacy-code adapter family (which may ship its own WASI tools). The `specify change survey` CLI verb is deleted. Per-language enumeration briefs move from `plugins/change/skills/survey/briefs/enumerate/<language>.md` to `sources/legacy-code-<language>/briefs/enumerate.md`.

### `survey.md`

Renamed `discovery-summary.md` and made generic. Sections become:

1. `# <change> discovery summary`
2. `## Summary` — counts: source / candidate / unresolved / conflict.
3. `## Source inventory` — one row per bound source: source-key, adapter, location, contribution count.
4. `## Candidate inventory` — fenced-YAML blocks per candidate, source-merged per §Discovery handshake.

Legacy-code-only columns (LOC, language, `surfaces.json` digest) populate only when the bound source adapter supplied them. The same file shape covers documentation-only, intent-only, and mixed runs.

## Workflow changes

Two forked paths collapse into one:

| Stage | Before | After |
| ----- | ------ | ----- |
| Plan time | `/change:analyze` for docs, `/change:survey` for legacy code — two skills, same output | `/change:draft` resolves every bound source adapter and calls `enumerate`; candidate blocks merge by stable id |
| Slice time | `/spec:define` for intent/docs, `/spec:extract` for legacy code — LLM in one branch, structured walker in the other | `/spec:define` resolves the target adapter (one) and source adapters (N), calls `extract` on each, then synthesises with per-requirement provenance and halts on `[conflict]` |

The named verbs `/change:analyze`, `/change:survey`, and `/spec:extract` are removed. Their behaviour lives inside source adapter briefs, invoked uniformly through the workflow.

## Implementation Plan

1. **Schemas.** Land `schemas/plugin.schema.json`, `schemas/source.schema.json`, `schemas/target.schema.json`, `schemas/evidence-pack.schema.json`, and `schemas/discovery/candidate-block.schema.json`. Delete `schemas/adapter.schema.json`. Update `schemas/plan/plan.schema.json` to rename `adapter` → `target`, make `sources` a required list with min 0 entries, and add optional `optional: true` on source bindings. Update `schemas/sources/sources.schema.json` for source-adapter identity fields.
2. **Domain rename.** Mass-rename unqualified `Adapter`* → `Target*` across today's target-runtime code in `crates/domain/`, `crates/tool/`, `crates/error/`, `src/`. Update `Error` discriminants. Update `Plan::resolve_adapter` → `Plan::resolve_target`. Land the new `Plan::resolve_sources` returning `Vec<SourceAdapter>`.
3. **Plugin loader.** New module `crates/domain/src/plugin/` containing `resolver.rs`, `cache.rs`, `manifest.rs`, `axis.rs`. Replaces `crates/domain/src/adapter/`. One loader, two axes.
4. **Default source adapters.** Ship `sources/intent/` and `sources/documentation/` in core. Each has a manifest, `briefs/enumerate.md`, and `briefs/extract.md`. `intent` enumerate emits one candidate from the operator's brief and extract emits the brief text as `kind: intent-text`; `documentation` enumerate reads bound docs and emits candidate blocks, then extract emits documentation-native evidence entries such as `requirement-statement`, `acceptance-criterion`, `decision-record`, and `document-section`.
5. **Slice synthesis.** Implement §Synthesis contract: `specify slice synthesize pipeline`, core briefs under `plugins/spec/references/synthesis/`, and `/spec:define` refactored to extract → synthesise → validate. Migrate target define briefs into core synthesis + `shape` briefs.
6. **Provenance tags.** Extend `spec.md` parser in `crates/domain/src/specs/` to require `ID:`, `Sources:`, `Status:` lines on every requirement block. Add `[conflict]` and `[divergence]` to the closed tag enum alongside `[unknown]`.
7. **Discovery handshake.** Implement stable-id merging plus reviewable name/alias correlation in the discovery writer. Add fixture and golden coverage.
8. **CLI surface.** New verbs: `specify source {resolve,list,validate}`, `specify slice synthesize pipeline`. Rename: `specify adapter resolve` → `specify target resolve`; split `specify adapter pipeline` into `specify slice synthesize pipeline` + `specify target pipeline {build,merge}`. New flags: `specify plan amend <slice> --add-source <key>`, `--remove-source`. Delete: `specify change survey`.
9. **Target brief migration.** Move today's target-owned `proposal`, `specs`, `design`, and `tasks` brief content into the core synthesis contract where it is target-neutral, and into target `shape` briefs where it is target-specific. Update RFC-24 and target skill prose to describe `shape` as guidance, not artifact ownership.
10. **Carve out the legacy-code, contracts, and target adapters.** Move into their own repositories (or top-level directories in a monorepo). Each ships its own README, manifest, briefs, and WASI tools where applicable. The `surfaces.json` schema and repair loop move with `sources/legacy-code-`*.
11. **`discovery-summary.md` rename.** Implement the generic form. Update fixtures.
12. **Documentation rewrite.** `AGENTS.md`, `.cursor/rules/project.mdc`, `docs/explanation/decision-log.md` (§Decision-log supersessions), `docs/contributing/adapter-anatomy.md` — adapter vocabulary, pipeline split, and superseded "analyze/extract split" / define-phase target ownership. RFC-22, RFC-24 prose updated.
13. **Acceptance.** Cross-repo Deno suite gains (land **before** RFC-26 collapse scenarios): documentation-only slice with `Sources:` provenance; multi-source legacy+doc slice; `[conflict]` halt; `[divergence]` with Gate-2-stop policy; pure-intent slice; target-`shape` fixture; required-source extract failure; invalid evidence-pack schema rejection.
14. **Observability ([RFC-19](rfc-19-observability.md)).** Journal events for `extract` completion per source key, synthesis completion, and `[conflict]` / `[divergence]` findings — so 2.x operators get traceability before RFC-26 Gate 2 lands.

## Migration

**There is no backward compatibility.** This RFC ships as Specify 2.0.

For operators upgrading existing projects: `specify upgrade` is a one-shot migration that performs the renames against `project.yaml`, `registry.yaml`, `plan.yaml`, `sources.yaml`, `.specify/.cache/`, and `.specify/archive/`. Briefs and skills in the operator's `.cursor/plugins/` cache are re-fetched from the upgraded plugin repositories. The upgrade is a one-way door; operators who want to stay on 1.x pin their plugin and CLI versions.

For plugin authors: ship the renamed manifests and briefs against the new schemas. The old `adapter.yaml` will fail to load on 2.0. There is no graceful degradation period.

For skill authors consuming `specify` output: every JSON envelope renames `adapter` fields to `target`. Add `sources[]` consumers where slice-level evidence matters.

The justification for breaking compatibility is that the rename and restructure are inseparable. Half-renames produce a confusing transitional vocabulary that costs more, in operator and code clarity, than a clean cut.

**Forward-compatibility with RFC-26.** [RFC-26: Workflow Collapse](rfc-26-collapse.md) ships as Specify 3.0 with a parallel hard-cut migration. The two upgrades can be sequenced with pinning; most teams should **jump 1.x → 3.0** once both RFCs land.

**Combined upgrade (1.x → 3.0).** A single `specify upgrade` target may perform, in order: adapter → source/target renames; evidence directory layout; `specify slice synthesize pipeline` migration; plan `reviewed` lifecycle; slice `defined_provisional`; plugin re-fetch (`/change:*` skills removed, `/spec:plan` + `/spec:refine` added). Operators who stop at 2.0 run adapter migration only; operators who pin 2.x skip workflow collapse until ready.

Skill authors should not invest in `/change:*` skill changes during the 2.x line.

## Alternatives Considered

**Collapse source and target into one "lens" with `axis: source | target`.** Rejected. The plugin *shape* is shared; the adapter *roles* are not. A unified name forces every sentence in docs and every error message to disambiguate, producing a permanent ambiguity tax. Two qualified adapter roles with one shared schema costs less and reads honestly.

**Keep unqualified `adapter` for the target role; introduce `source adapter` only.** Rejected on clarity grounds. `source adapter` + `adapter` is asymmetric and leaves the output side overloaded; `source adapter` + `target adapter` preserves the familiar adapter noun while making direction explicit. The rename cost is one-off; the readability gain is permanent.

**Reuse an existing noun (`provider`, `profile`) for source adapters.** Rejected. `provider` collides with Omnia DI (auth provider, storage provider, message provider) in conversation, error messages, and search. `profile` reads as configuration, not as an adapter role, and the codebase already attempted this rename once (`capability` → `profile`) before settling on `adapter`.

**Keep `/spec:extract` as a named skill, but parameterise it by source adapter.** Rejected. Extraction is one of two source-adapter capabilities; naming a skill after it gives the legacy-code path a privileged shape it does not deserve and re-creates the bifurcation. `/spec:define` is the only authoring entry point; sources are uniform inputs.

**Keep `/change:analyze` and `/change:survey` as named skills, dispatching to source adapters.** Rejected. The two names *are* the bifurcation; preserving them preserves the asymmetry. One discovery stage with source-adapter dispatch is the move.

**Per-source artifact files (`spec.<source>.md`) rather than provenance tags inside one `spec.md`.** Rejected. The operator reviews specs as one document, not as N partials. Per-source files force a manual merge step at every review and break the "one spec.md per crate" reader contract. Inline `Sources:` lines give the same audit trail without splitting the artifact.

**Author the conflict-resolution policy per-slice rather than globally.** Rejected for v1. The authority hierarchy is small and the per-requirement override (`[conflict]` halts; operator decides) is the escape hatch. Per-slice policy would be configuration nobody reads and would tempt synthesis briefs to silently apply different rules per slice.

**Allow target adapters to participate in discovery (e.g. an Omnia target adapter enumerates handlers from a baseline).** Rejected. Targets shape and implement canonical artifacts; source adapters produce planning candidates and synthesis evidence. A target reading baseline specs to inform discovery would re-merge the two axes. Baseline-aware planning is RFC-22's ledger territory, not a target-axis concern.

**Let source adapters emit `spec.md` / `design.md` directly.** Rejected. That would duplicate provenance, authority, `[unknown]`, `[conflict]`, and `[divergence]` handling across source families and make multi-source slices a merge problem between partial artifacts. Source adapters emit `CandidateSet`s and `EvidencePack`s; Specify core owns the artifact synthesis boundary.

**Keep target `proposal` / `specs` / `design` / `tasks` capabilities as artifact owners.** Rejected. Target-specific idioms matter during artifact authoring, but ownership of the canonical artifacts has to stay in core for the source-axis redesign to work. The `shape` capability preserves target guidance without making each target adapter a parallel synthesis engine.

**Allow per-axis adapter id collisions (a `mermaid` source adapter and a `mermaid` target adapter).** Permitted. The resolver disambiguates by axis; the operator-facing CLI takes axis as a positional argument (`specify source resolve mermaid` vs `specify target resolve mermaid`). The cost of forcing globally-unique names across axes outweighs the small ambiguity in conversational reference.

**Minimum viable 2.0 (synthesis still agent-only).** Rejected as the long-term shape. A 2.0 that only renames adapters without `specify slice validate` provenance checks and `evidence-pack.schema.json` would leave multi-source slices un-auditable. 2.0 may ship before RFC-26, but not without §Synthesis contract CLI validation.

## Non-Goals

- Backward compatibility with Specify 1.x manifests, schemas, verbs, or directory layouts.
- A general "plugin marketplace" or runtime plugin discovery. Source and target adapters are installed at project-init time.
- Per-handler provenance below the requirement level. `Sources:` lives on requirement blocks; finer granularity belongs in `design.md` per existing convention.
- Per-pack confidence scores. Authority is class-based; finer scoring belongs to a future RFC if operator demand emerges.
- Replacing operator review of conflicts. `[conflict]` halts the slice; operators decide. Auto-resolution heuristics are out of scope.
- Cross-repo source sharing. Each platform-repo declares its own sources via `sources.yaml`, consistent with RFC-21.
- Bidirectional adapters (an adapter that is both source and target). The axis is a discriminator, not a tag set.
- Source-adapter support for editing artifacts after slice authoring. Sources read; the workflow writes.

## Open Questions

1. Should the default `intent` and `documentation` source adapters be packaged as true plugin implementations (with `source.yaml` etc.) or hard-wired into the core CLI as built-ins? Current preference: true plugin implementations, shipped in-repo under `sources/intent/` and `sources/documentation/`, so the plugin shape has zero exceptions.
2. Should `Status: divergence` halt at Gate 2? **Resolved in §Synthesis ↔ Gate 2 policy:** supervised mode stops at `defined_provisional`; synthesis proceeds with the higher-authority claim recorded; `specify slice validate` emits a `divergence` finding.
3. Should `surfaces.json` move with the legacy-code adapter family or stay in core as a shared "structured evidence" schema? Current preference: move with the adapter family — it is legacy-code-shaped and other sources do not need it.
4. How should `extract` be sandboxed when a source adapter ships a WASI tool? Current preference: same posture as RFC-15 — the WASI tool runs under the existing `specify tool run` sandbox; briefs read its output.
5. Should authority class be operator-overridable at slice-binding time (`--add-source legacy-monolith:design-spec` to elevate code-derived evidence above its default)? Current preference: yes — operators sometimes know that a particular legacy source *is* the canonical design intent, e.g. when migrating off the only existing implementation of an external contract.
6. Should candidate-block name/alias correlations auto-merge, or only stable-id matches? Current preference: only stable-id matches auto-merge; name and alias correlations are review signals for propose.
7. Should `specify target` accept multiple targets per project for projects that produce both an Omnia service and a Vectis app from the same artifacts? Current preference: deferred — today's one-target-per-project assumption is held; a future RFC may relax it once a real bi-targeting case lands.
8. How aggressively should the carve-out happen — one adapter per repo, or one repo per family (e.g. all legacy-code languages in `specify-sources-legacy`)? Current preference: one repo per family for v1; split if individual languages grow large enough to justify their own release cadence.

## References

When this RFC lands, update [`docs/explanation/decision-log.md`](../docs/explanation/decision-log.md): the **analyze/extract split** is superseded (unified as `source.enumerate` / `source.extract`); **independently useful layers** is superseded at the verb level by [RFC-26](rfc-26-collapse.md) (on-disk `change.md` + `plan.yaml` unchanged); **CLI owns correctness** is retained and extended to synthesis structure and provenance.


- [RFC-19: Observability](rfc-19-observability.md) — journal events for extract and synthesis (implementation plan step 14).
- [RFC-20: Survey-to-Plan Pipeline (archived)](archive/rfc-20-survey.md) — the survey pipeline this RFC folds into the `sources/legacy-code` adapter family.
- [RFC-21: Source Catalogue and Tier-1 Cache](rfc-21-catalogue.md) — `sources.yaml` survives; binding fields extend to record source-adapter identity.
- [RFC-22: Migration Ledger and Slice Mapping](rfc-22-ledger.md) — adapter-typed entries become target-typed; otherwise unchanged.
- [RFC-23: Change Lifecycle (archived)](archive/rfc-23-change-lifecycle.md) — the `/change:draft` → `/change:execute` → `/change:finalize` three-skill model survives this RFC; the discovery stage inside `/change:draft` is restructured here. RFC-23's three-skill model is itself superseded by [RFC-26](rfc-26-collapse.md).
- [RFC-24: Omnia Plan Composition](rfc-24-omnia.md) — adapter-gated findings become target-gated, and Omnia artifact-authoring briefs become target `shape` guidance. `omnia` becomes a target adapter.
- [RFC-26: Workflow Collapse](rfc-26-collapse.md) — planned follow-on; collapses the `/change:*` and `/spec:*` skill families into one operator surface on top of this RFC's adapter axis.
- [RFC-15: WASM Plugins (archived)](archive/rfc-15-wasm-plugins.md) — the WASI tool surface reused as the deterministic-CLI seam inside source and target adapters.
- `[specify-cli/AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md)` — exit codes and CLI contract preserved; rename surface documented there.
- `[.cursor/rules/project.mdc](../.cursor/rules/project.mdc)` — authority hierarchy at synthesis time mirrors the existing artifact authority hierarchy documented in this file.

