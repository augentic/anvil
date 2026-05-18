# RFC-25: Directional Adapters

> Status: Draft - Supersedes: [RFC-20 (archived)](archive/rfc-20-survey.md) and the `kind: legacy-code | documentation` discriminator in `/change:analyze`. Replaces the existing unqualified `adapter` axis with directional `source adapter` and `target adapter` roles. Compatible with: [RFC-22](rfc-22-ledger.md), [RFC-23 (archived)](archive/rfc-23-change-lifecycle.md), [RFC-24](rfc-24-omnia.md) — each gains the rename and otherwise stands.

## Abstract

Refactor Specify into a small **core** plus two directional adapter roles — **source adapters** and **target adapters**. Source adapters turn evidence (intent, documentation, legacy code, OpenAPI, …) into candidates and slice-time evidence packs. Target adapters turn Specify artifacts into runnable code (omnia, vectis, contracts, …). Both share one plugin implementation shape (manifest + briefs + optional WASI tools + resolver + cache). A single slice may bind multiple sources; slice authoring synthesises evidence from all bound sources with explicit per-requirement provenance and a `[conflict]` tag for genuine disagreements. Documentation-driven specification is a default source path, implemented through the same source-adapter contract as every other input, not a side channel.

The redesign deletes today's two-path bifurcation between documentation analysis and legacy-code survey, retires `/change:survey` and `/spec:extract` as named skills, replaces the unqualified `adapter` axis with `source adapter` and `target adapter`, and makes greenfield, migration, and mixed-evidence work three values of one parameter (the bound source set) rather than three forked code paths.

This is a ground-up redesign. **There is no backward compatibility.** Existing `project.yaml`, `registry.yaml`, `plan.yaml`, `sources.yaml`, `adapters/`, brief paths, CLI verbs, schema field names, and skill files all change shape in lockstep. The migration story is "fork the next minor, port forward, drop the old".

## Motivation

Three structural problems made the current shape brittle.

**One semantic operation, two parallel paths.** `/change:analyze` (documentation) and `/change:survey` (legacy code) answer the same question — "what slice-sized candidates exist in this source?" — and append to the same `## Candidate inventory` heading in `discovery.md`. They are not two operations; they are one operation with two evidence sources. The same duplication recurs at slice time: `/spec:define` writes artifacts from intent and docs, `/spec:extract` writes artifacts from code. Two skill families, one contract.

**Legacy migration is not core.** Deriving specs from existing source is a one-time, language-aware archaeology task that matters during migration and not afterwards. Today it lives in the framework's spine: enumeration briefs, repair loops, `surfaces.json` schema, language detection, the `legacy-code` kind discriminator, the `specify change survey` CLI verb. After a project finishes migrating, every line of this surface is dead weight that still has to be carried, taught, and tested.

**The adapter slot has no name for inputs.** Today, unqualified `adapter` names the *target* runtime (omnia, vectis, contracts). There is no symmetrical phrase for the *source* of evidence; the framework gestures at it through `kind`, through `language`, through per-language brief directories, through the `source` CLI noun on `/change:draft`. The asymmetry shows up as cognitive load on every skill author trying to add a new input.

The redesign collapses these three problems into one move: qualify adapters by direction, give source and target adapters the same implementation shape, make documentation-driven specification a first-class default source path, and put legacy-code support outside the core.

## Design

### Principles

1. **Two adapter roles, one implementation shape.** Source adapters and target adapters are distinct roles that share one plugin implementation shape (manifest + briefs + optional WASI tools + resolver + cache). Same loader, same validator, same cache layout, same `specify {source,target} {resolve,list,validate}` verb family.
2. **Core is small.** Core ships the workflow (`/spec:`*, `/change:*`), the plugin resolver, the candidate-block grammar, the `discovery.md` handshake, the default `intent` and `documentation` source adapters, and the CLI primitives. Legacy-code, contract-import, and every target adapter remain add-ons.
3. **Multi-source is the general case.** A slice's evidence is a *set* of source evidence packs. Single-source slices are a degenerate case. Greenfield, pure migration, and realistic mixed-evidence work are three values of one parameter.
4. **Provenance is mechanical.** Every requirement in `spec.md` records which source(s) supplied it. Conflicts between sources halt the slice with an explicit `[conflict]` tag rather than silent resolution.
5. **Authority is hierarchical and explicit.** When sources disagree on the same fact, a published authority hierarchy decides which evidence wins. Operators can override per-requirement; the framework never silently picks.
6. **Single writer.** The CLI is still the only writer of `plan.yaml`, `.metadata.yaml`, archive paths, `migration-log.yaml`, `discovery.md`, and the new `sources.yaml` / `targets.yaml` files. Adapters read; the CLI writes.
7. **Ground-up.** No alias keys, no compat fallbacks, no transitional schemas. The rename and restructure ship as one breaking minor.

### Vocabulary


| Term           | Meaning                                                                                                                                                                                                                    |
| -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **source adapter** | Pluggable input role. Knows how to enumerate candidates from an evidence corpus and extract per-candidate evidence packs. Examples: `intent`, `documentation`, `legacy-code-typescript`, `legacy-code-cobol`, `openapi`. |
| **target adapter** | Pluggable output role. Knows how to turn Specify artifacts into runnable code. Examples: `omnia`, `vectis`, `contracts`. Replaces today's unqualified `adapter`. |
| **plugin** | The shared implementation shape of a source adapter or target adapter. Used in schemas, the resolver, and implementation docs when speaking about both at once. |
| **evidence pack** | Persisted source evidence returned by a source adapter's `extract` capability. Structured input to slice synthesis; stores facts, provenance, paths, spans, hashes, and bounded excerpts only when explicitly allowed. |
| **candidate** | A slice-sized unit of work proposed by a source adapter's `enumerate` capability. Lives in `discovery.md` under `## Candidate inventory`. |
| **provenance** | The set of source bindings backing a single requirement in `spec.md`.                                                                                                                                                      |
| **conflict** | Two or more evidence packs disagreeing on the same fact. Surfaces as a `[conflict]` tag in the synthesised artifact. |


`provider` is reserved for Omnia DI and **not** used as an adapter-role name. `profile` is retired and **not** reintroduced. Unqualified `adapter` is removed; the public nouns are **source adapter** and **target adapter**. `plugin` remains an implementation noun for the shared manifest / resolver / cache shape.

### Adapter implementation shape

Both `source` and `target` adapters share one manifest shape. A source adapter lives at `sources/<name>/source.yaml`; a target adapter lives at `targets/<name>/target.yaml`. Schema fragments are identical except for the `axis` discriminator and the `capabilities` enum.

```yaml
# sources/<name>/source.yaml  OR  targets/<name>/target.yaml
name: legacy-code-typescript        # or: omnia
version: 1
axis: source                         # or: target
capabilities:                        # source: enumerate, extract
  - enumerate                        # target: proposal, specs, design, tasks, build, merge
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

Rules:

- `name` is kebab-case, globally unique within an axis. Names *may* collide across axes (a hypothetical `mermaid` source adapter and `mermaid` target adapter would not clash).
- `axis` is a closed enum: `source | target`. Validated at load time.
- `capabilities[]` is a closed enum, one set per axis. A source adapter declares some subset of `{enumerate, extract}`; a target adapter declares some subset of `{proposal, specs, design, tasks, build, merge}`.
- `briefs.<capability>` is the path to the markdown brief implementing that capability. Required for every declared capability.
- `tools[]` reuses the existing RFC-15 WASI tool shape. Tools materialise into `.specify/.cache/{sources,targets}/<name>/tools/` alongside briefs.
- `detect[]` is source-only. Used by the resolver when an operator binds a path without naming the source explicitly.

### Source adapter contract

A source adapter contributes two capabilities to the workflow.

**`enumerate(source-binding) → candidates`.** Called at plan time by `/change:draft`. Reads the bound evidence (a local path, a documentation file, a free-text intent string) and emits candidate blocks under `## Candidate inventory` in `discovery.md`. Output grammar is the existing candidate-block format from RFC-20 §`discovery.md`, extended with stable candidate ids and optional correlation hints.

**`extract(candidate, source-binding) → evidence-pack`.** Called at slice time by `/spec:define` for each source bound to the slice. Returns a structured evidence pack persisted under the slice before synthesis:

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

Documentation sources use the same shape. The evidence entries differ, but the workflow does not:

```yaml
# .specify/slices/<slice>/evidence/<source-key>.yaml
source: product-requirements
adapter: documentation
candidate: user-registration
authority: design-spec
evidence:
  - kind: requirement-statement
    path: docs/registration.md
    heading: Registration policy
    lines: [34, 51]
    sha256: 1f42...
    excerpt: |
      Users must register with a verified email address before they can create a workspace.
  - kind: acceptance-criterion
    path: docs/registration.md
    heading: Acceptance criteria
    lines: [62, 67]
    sha256: 7b19...
  - kind: decision-record
    path: docs/adr/004-registration.md
    title: Use external email verification
    sha256: c913...
```

The pack's `kind` enum is closed and shared across all source adapters. Initial kinds include `intent-text`, `requirement-statement`, `acceptance-criterion`, `decision-record`, `document-section`, `diagram-reference`, `contract-reference`, `code-excerpt`, `type-definition`, and `external-call`. New kinds require an RFC update. The `authority` field is the source adapter's self-classification (see §Authority hierarchy); the operator may override at slice-binding time. Evidence packs do not store raw source bodies by default: they store structured facts, relative paths, line spans, content hashes, and bounded excerpts only when the adapter contract explicitly allows them.

### Default source adapters

Core ships two source adapters by default.

| Adapter | Role | Default authority |
| ------- | ---- | ----------------- |
| `intent` | Captures operator-authored briefs, inline requirements, and explicit corrections. Used when no other source is bound, or alongside other sources when the operator adds clarifying intent. | `intent` |
| `documentation` | Captures requirements documents, design notes, proposals, RFCs, existing specs, architecture records, and other written product or technical intent. Used for the ordinary documentation-driven path into specs and design. | `design-spec` |

Both are true source adapters with manifests, `enumerate` briefs, and `extract` briefs. They are default-packaged for usability, but they do not get special workflow rules. `/change:draft` still calls `enumerate`; `/spec:define` still calls `extract`; slice synthesis still consumes evidence packs.

### Target adapter contract

A target adapter contributes briefs for some subset of `{proposal, specs, design, tasks, build, merge}`. The shape matches today's `adapter.yaml`, with three renames:

- `adapter.yaml` → `target.yaml`
- `adapters/<name>/` → `targets/<name>/`
- `specify adapter {resolve,pipeline}` → `specify target {resolve,pipeline}`

Unqualified `adapter` does not survive anywhere in the schema, on the CLI, in docs, or in skill prose. RFC-24's "adapter-gated finding" becomes "target-gated finding"; `planSlice.adapter` becomes `planSlice.target`; `Plan::resolve_adapter` becomes `Plan::resolve_target`. The renames are mechanical.

### Discovery handshake — candidate correlation

The existing rule in `/change:analyze` and `/change:survey` is "append candidate blocks; skip blocks whose `### <name>` heading already exists in `discovery.md`". The new rule is:

> When two candidate blocks have the same stable `id`, **merge** `sources` and `declared-at` into the existing block (union, sorted). When two blocks only share a heading, slug, or adapter-supplied correlation hint, record the relationship as `correlates-with` and leave the final merge to propose review.

The candidate-block grammar's `sources: [...]` field becomes load-bearing: a block with `sources: [legacy-monolith, design-doc]` declares that two source adapters corroborated the same candidate. The stable `id` is the merge key; headings and source-authored aliases are correlation hints. When a hint is wrong, the operator splits or relabels in propose.

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
| Pure greenfield | `[]` or `[intent]`             | New work driven by operator intent only. |
| Pure port       | `[<one-legacy-source>]`        | Legacy code dictates behaviour.          |
| Pure design     | `[<one-doc-source>]`           | Documentation dictates behaviour.        |
| Mixed migration | `[code-source, doc-source, …]` | Code + docs (+ specs) synthesised.       |


The slice loop does not branch on archetype. The synthesis step (§Slice authoring synthesis) handles all four uniformly.

### Slice authoring synthesis

`/spec:define` proceeds as follows when invoked for a slice with N bound sources:

1. **Resolve** the bound target adapter (one) and each bound source adapter (N).
2. **Extract** in parallel: call `extract(candidate, source-binding)` on each bound source. Each returns one persisted evidence pack.
3. **Synthesise**: invoke the Specify artifact-synthesis contract with all N evidence packs as input. Target adapters contribute their `specs` and `design` briefs where target-specific shaping is required, but the source-to-`spec.md` / `design.md` contract is uniform and Specify-owned. The briefs are written to consume pack sets, not single packs.
4. **Tag provenance**: every requirement in `specs/<crate>/spec.md` carries a `Sources:` line listing the source keys whose evidence backed it. Same for design-section blocks.
5. **Flag conflicts**: when evidence packs disagree on the same fact, the synthesis tags the requirement `[conflict]` and emits a `Conflicting evidence` block beneath it, naming each source's claim. The slice halts for operator review. `[conflict]` is a peer of today's `[unknown]` tag.
6. **Write artifacts** via the existing `/spec:define` writer; `.metadata.yaml` transitions through `defining` → `awaiting-review` as today.

The N=1 case collapses to today's single-source behaviour (no conflict possible, provenance trivially populated). The N=0 case (pure-intent slice) takes the user's free-text description through the `intent` source adapter as a synthetic evidence pack.

### Evidence authority hierarchy

When evidence packs disagree on the same fact, synthesis applies a published authority order:

1. **`intent`** — operator-stated requirements trump everything.
2. **`external-contract`** — OpenAPI, AsyncAPI, JSON Schema, signed contracts. Canonical for wire shape.
3. **`design-spec`** — architecture notes, design docs. Canonical for intent.
4. **`observed-behaviour`** — legacy code. Canonical for current implementation.

The hierarchy is *advisory for ranking*, not *silencing*. Lower-authority evidence is still recorded in `declared-at`. A conflict between two same-authority sources surfaces as `[conflict]`. A conflict between adjacent authorities (e.g. `design-spec` says X, `observed-behaviour` says Y) defaults to the higher-authority claim *and* tags the requirement `[divergence]` so the operator sees that the legacy code currently does something else.

`[divergence]` is the third evidence-tag peer (alongside `[unknown]` and `[conflict]`). It is *not* a halt; it is a signal that the slice will produce a behaviour change against the legacy baseline.

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


| Before                         | After                        |
| ------------------------------ | ---------------------------- |
| `specify adapter resolve`      | `specify target resolve`     |
| `specify adapter pipeline`     | `specify target pipeline`    |
| `adapters/<name>/adapter.yaml` | `targets/<name>/target.yaml` |
| `schemas/adapter.schema.json`  | `schemas/target.schema.json` |
| `planSlice.adapter`            | `planSlice.target`           |
| `Plan::resolve_adapter`        | `Plan::resolve_target`       |
| `Error::AdapterResolution`     | `Error::TargetResolution`    |


Additions:


| Verb                                               | Purpose                                         |
| -------------------------------------------------- | ----------------------------------------------- |
| `specify source resolve <name>`                    | Materialise a source adapter's briefs and tools. |
| `specify source list`                              | List installed source adapters.                  |
| `specify source validate <name>`                   | Validate a source adapter's manifest.            |
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


Removing the legacy-code add-on removes COBOL/TS enumeration and extraction; documentation work and greenfield work keep functioning. The framework's pure spec-driven form is the `intent` and `documentation` source adapters plus whatever target adapters are installed.

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

### Before

```text
operator runs /change:draft
  ├── if source kind == documentation: /change:analyze → candidate blocks
  ├── if source kind == legacy-code:   /change:survey  → candidate blocks
  └── propose (operator slices candidates into plan.yaml)

operator runs /spec:define <slice>
  ├── if slice has legacy source: /spec:extract → specs + design
  └── else:                       LLM writes from intent/docs → specs + design
```

### After

```text
operator runs /change:draft
  ├── for each source binding: resolve source adapter, call enumerate → candidate blocks
  │   (merge by stable id; record name/alias correlations for review)
  └── propose (operator slices candidates into plan.yaml, binds sources per slice)

operator runs /spec:define <slice>
  ├── resolve target adapter (one)
  ├── resolve source adapters (N), call extract on each → N persisted evidence packs
  ├── synthesise specs + design from evidence pack set
  │   (provenance per requirement; halt on [conflict])
  └── write artifacts
```

The named verbs `/change:analyze`, `/change:survey`, and `/spec:extract` are removed. Their behaviour lives inside source adapter briefs, invoked uniformly through the workflow.

## Implementation Plan

1. **Schemas.** Land `schemas/plugin.schema.json` (shared implementation shape), `schemas/source.schema.json` (axis-specific), `schemas/target.schema.json` (rename + axis-specific). Delete `schemas/adapter.schema.json`. Update `schemas/plan/plan.schema.json` to rename `adapter` → `target` and make `sources` a required list with min 0 entries. Update `schemas/sources/sources.schema.json` to add source-adapter binding fields.
2. **Domain rename.** Mass-rename unqualified `Adapter`* → `Target*` across today's target-runtime code in `crates/domain/`, `crates/tool/`, `crates/error/`, `src/`. Update `Error` discriminants. Update `Plan::resolve_adapter` → `Plan::resolve_target`. Land the new `Plan::resolve_sources` returning `Vec<SourceAdapter>`.
3. **Plugin loader.** New module `crates/domain/src/plugin/` containing `resolver.rs`, `cache.rs`, `manifest.rs`, `axis.rs`. Replaces `crates/domain/src/adapter/`. One loader, two axes.
4. **Default source adapters.** Ship `sources/intent/` and `sources/documentation/` in core. Each has a manifest, `briefs/enumerate.md`, and `briefs/extract.md`. `intent` enumerate emits one candidate from the operator's brief and extract emits the brief text as `kind: intent-text`; `documentation` enumerate reads bound docs and emits candidate blocks, then extract emits documentation-native evidence entries such as `requirement-statement`, `acceptance-criterion`, `decision-record`, and `document-section`.
5. **Slice synthesis.** Refactor `/spec:define` brief authoring to accept N evidence packs. New brief contract: `briefs/specs.md` and `briefs/design.md` consume an evidence-pack set, emit `Sources:` and `Status:` lines per requirement, halt with `[conflict]` on disagreement, and produce the same artifact shape whether the bound sources are `intent`, `documentation`, legacy code, contracts, or any mixture.
6. **Provenance tags.** Extend `spec.md` parser in `crates/domain/src/specs/` to require `ID:`, `Sources:`, `Status:` lines on every requirement block. Add `[conflict]` and `[divergence]` to the closed tag enum alongside `[unknown]`.
7. **Discovery handshake.** Implement stable-id merging plus reviewable name/alias correlation in the discovery writer. Add fixture and golden coverage.
8. **CLI surface.** New verbs: `specify source {resolve,list,validate}`. Rename: `specify adapter` → `specify target`. New flags: `specify plan amend <slice> --add-source <key>`, `--remove-source`. Delete: `specify change survey`.
9. **Carve out the legacy-code, contracts, and target adapters.** Move into their own repositories (or top-level directories in a monorepo). Each ships its own README, manifest, briefs, and WASI tools where applicable. The `surfaces.json` schema and repair loop move with `sources/legacy-code-`*.
10. **`discovery-summary.md` rename.** Implement the generic form. Update fixtures.
11. **Documentation rewrite.** `AGENTS.md`, `.cursor/rules/project.mdc`, `docs/` — every unqualified mention of `adapter` becomes `source adapter` or `target adapter`; every mention of `analyze`/`survey`/`extract` becomes "the source adapter's enumerate/extract capability". RFC-22, RFC-24 prose updated.
12. **Acceptance.** Cross-repo Deno suite gains: a documentation-only slice asserting standard `spec.md` / `design.md` output and `Sources: [<doc-source>]` provenance; a multi-source slice with one legacy and one doc source asserting `Sources: [...]` provenance; a `[conflict]` halt fixture; a `[divergence]` non-halt fixture; a pure-intent slice asserting trivial synthesis.

## Migration

**There is no backward compatibility.** This RFC ships as Specify 2.0.

For operators upgrading existing projects: `specify upgrade` is a one-shot migration that performs the renames against `project.yaml`, `registry.yaml`, `plan.yaml`, `sources.yaml`, `.specify/.cache/`, and `.specify/archive/`. Briefs and skills in the operator's `.cursor/plugins/` cache are re-fetched from the upgraded plugin repositories. The upgrade is a one-way door; operators who want to stay on 1.x pin their plugin and CLI versions.

For plugin authors: ship the renamed manifests and briefs against the new schemas. The old `adapter.yaml` will fail to load on 2.0. There is no graceful degradation period.

For skill authors consuming `specify` output: every JSON envelope renames `adapter` fields to `target`. Add `sources[]` consumers where slice-level evidence matters.

The justification for breaking compatibility is that the rename and restructure are inseparable. Half-renames produce a confusing transitional vocabulary that costs more, in operator and code clarity, than a clean cut.

## Alternatives Considered

**Collapse source and target into one "lens" with `axis: source | target` ([REDESIGN.md](REDESIGN.md)).** Rejected. The plugin *shape* is shared; the adapter *roles* are not. A unified name forces every sentence in docs and every error message to disambiguate, producing a permanent ambiguity tax. Two qualified adapter roles with one shared schema costs less and reads honestly.

**Keep unqualified `adapter` for the target role; introduce `source adapter` only.** Rejected on clarity grounds. `source adapter` + `adapter` is asymmetric and leaves the output side overloaded; `source adapter` + `target adapter` preserves the familiar adapter noun while making direction explicit. The rename cost is one-off; the readability gain is permanent.

**Use `provider` for source adapters.** Rejected. Omnia DI uses `provider` extensively (auth provider, storage provider, message provider). Reusing the word produces immediate collision in conversation, error messages, and search.

**Use `profile` for either axis.** Rejected. `profile` reads as configuration, not as an adapter role; the codebase already attempted this rename (`capability` → `profile`) and settled on `adapter` instead. Re-introducing it would feel like undoing a settled decision.

**Keep `/spec:extract` as a named skill, but parameterise it by source adapter.** Rejected. Extraction is one of two source-adapter capabilities; naming a skill after it gives the legacy-code path a privileged shape it does not deserve and re-creates the bifurcation. `/spec:define` is the only authoring entry point; sources are uniform inputs.

**Keep `/change:analyze` and `/change:survey` as named skills, dispatching to source adapters.** Rejected. The two names *are* the bifurcation; preserving them preserves the asymmetry. One discovery stage with source-adapter dispatch is the move.

**Per-source artifact files (`spec.<source>.md`) rather than provenance tags inside one `spec.md`.** Rejected. The operator reviews specs as one document, not as N partials. Per-source files force a manual merge step at every review and break the "one spec.md per crate" reader contract. Inline `Sources:` lines give the same audit trail without splitting the artifact.

**Author the conflict-resolution policy per-slice rather than globally.** Rejected for v1. The authority hierarchy is small and the per-requirement override (`[conflict]` halts; operator decides) is the escape hatch. Per-slice policy would be configuration nobody reads and would tempt synthesis briefs to silently apply different rules per slice.

**Allow target adapters to participate in discovery (e.g. an Omnia target adapter enumerates handlers from a baseline).** Rejected. Targets produce code; sources produce specs. A target reading baseline specs to inform discovery would re-merge the two axes. Baseline-aware planning is RFC-22's ledger territory, not a target-axis concern.

**Allow per-axis adapter id collisions (a `mermaid` source adapter and a `mermaid` target adapter).** Permitted. The resolver disambiguates by axis; the operator-facing CLI takes axis as a positional argument (`specify source resolve mermaid` vs `specify target resolve mermaid`). The cost of forcing globally-unique names across axes outweighs the small ambiguity in conversational reference.

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
2. Should `Status: divergence` halt the slice (operator must explicitly accept the behaviour change) or proceed (the synthesis already picked the higher-authority claim)? Current preference: proceed but emit a `divergence` finding in `specify slice validate`; halting is `[conflict]`-only.
3. Should `surfaces.json` move with the legacy-code adapter family or stay in core as a shared "structured evidence" schema? Current preference: move with the adapter family — it is legacy-code-shaped and other sources do not need it.
4. How should `extract` be sandboxed when a source adapter ships a WASI tool? Current preference: same posture as RFC-15 — the WASI tool runs under the existing `specify tool run` sandbox; briefs read its output.
5. Should `planSlice.sources: []` (zero-source greenfield) be permitted, or should the `intent` source always be bound implicitly? Current preference: bind `intent` implicitly so every slice has at least one evidence pack to synthesise from.
6. Should the per-requirement `Sources:` line be required for *every* requirement (including ones derived entirely from operator intent during define) or only for ones with non-trivial provenance? Current preference: required for every requirement; the operator-intent case populates `Sources: [intent]`.
7. Should authority class be operator-overridable at slice-binding time (`--add-source legacy-monolith:design-spec` to elevate code-derived evidence above its default)? Current preference: yes — operators sometimes know that a particular legacy source *is* the canonical design intent, e.g. when migrating off the only existing implementation of an external contract.
8. Should candidate-block name/alias correlations auto-merge, or only stable-id matches? Current preference: only stable-id matches auto-merge; name and alias correlations are review signals for propose.
9. Should `specify target` accept multiple targets per project for projects that produce both an Omnia service and a Vectis app from the same artifacts? Current preference: deferred — today's one-target-per-project assumption is held; a future RFC may relax it once a real bi-targeting case lands.
10. How aggressively should the carve-out happen — one adapter per repo, or one repo per family (e.g. all legacy-code languages in `specify-sources-legacy`)? Current preference: one repo per family for v1; split if individual languages grow large enough to justify their own release cadence.

## References

- [REDESIGN.md](REDESIGN.md) — historical sketch this RFC supersedes (with thanks for the "two axes, one shape" framing).
- [RFC-20: Survey-to-Plan Pipeline (archived)](archive/rfc-20-survey.md) — the survey pipeline this RFC folds into the `sources/legacy-code` adapter family.
- [RFC-21: Source Catalogue and Tier-1 Cache](rfc-21-catalogue.md) — `sources.yaml` survives; binding fields extend to record source-adapter identity.
- [RFC-22: Migration Ledger and Slice Mapping](rfc-22-ledger.md) — adapter-typed entries become target-typed; otherwise unchanged.
- [RFC-23: Change Lifecycle (archived)](archive/rfc-23-change-lifecycle.md) — the `/change:draft` → `/change:execute` → `/change:finalize` three-skill model is preserved; only the discovery stage inside `/change:draft` is restructured.
- [RFC-24: Omnia Plan Composition](rfc-24-omnia.md) — adapter-gated findings become target-gated; otherwise stands. `omnia` becomes a target adapter.
- [RFC-15: WASM Plugins (archived)](archive/rfc-15-wasm-plugins.md) — the WASI tool surface reused as the deterministic-CLI seam inside source and target adapters.
- `[specify-cli/AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md)` — exit codes and CLI contract preserved; rename surface documented there.
- `[.cursor/rules/project.mdc](../.cursor/rules/project.mdc)` — authority hierarchy at synthesis time mirrors the existing artifact authority hierarchy documented in this file.

