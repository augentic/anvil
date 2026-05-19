# RFC-25: Directional Adapters

> Status: Draft — Supersedes [RFC-20 (archived)](archive/rfc-20-survey.md). Ships in lockstep with [RFC-26: Workflow Collapse](rfc-26-workflow.md) as Specify 3.0 — RFC-26 depends on this RFC's `enumerate`/`extract` symmetry, and the two land together with one migration script. Compatible with [RFC-22](rfc-22-ledger.md) and [RFC-24](rfc-24-omnia.md) (target rename + `shape` ownership); [RFC-23](archive/rfc-23-change-lifecycle.md) is retired by RFC-26 in this same release.

## Abstract

Refactor Specify into a small **core** plus two directional adapter roles — **source adapters** and **target adapters**. Source adapters normalise evidence (intent, documentation, legacy code, OpenAPI, …) into two core-facing intermediate shapes: plan-time `CandidateSet`s and slice-time `EvidencePack`s. Specify core synthesises canonical artifacts from evidence packs, applying provenance, the authority hierarchy, and `[unknown]` / `[conflict]` / `[divergence]` rules in one place. Target adapters shape those canonical artifacts for a runtime and turn them into runnable code (omnia, vectis, contracts, …). Both adapter roles share one plugin implementation shape (manifest + briefs + optional WASI tools + resolver + cache). Documentation-driven specification is a default source path, implemented through the same source-adapter contract as every other input, not a side channel.

**v1 ships multi-source synthesis.** `planSlice.sources` is a list with cardinality ≥ 1; `EvidencePackSet` always contains at least one pack. Pure greenfield, pure port, and pure design slices are degenerate sets of cardinality one; combined-evidence slices — most notably legacy code + surrounding design documentation during a migration — are first-class. Authority hierarchy, `[divergence]` tagging, inter-pack `[conflict]` detection, the per-binding `optional:` flag, and `correlates-with` candidate correlation hints all ship in v1. The set framing keeps later additions (parallel extract, per-claim authority overrides) additive rather than reshape-the-contract.

This is a ground-up redesign of the adapter axis. **There is no backward compatibility** — `project.yaml`, `registry.yaml`, `plan.yaml`, `sources.yaml`, `adapters/`, brief paths, CLI verbs, schema field names, and adapter-touching skill files all change shape in lockstep. The workflow skill family (`/change:{draft,execute,finalize}`, `/spec:{define,build,merge}`) is renamed and collapsed by [RFC-26](rfc-26-workflow.md), which ships alongside this RFC in the same Specify 3.0 release; this RFC's scope is the adapter axis, RFC-26's scope is the operator-facing workflow.

## Motivation

Three structural problems made the current shape brittle.

**One semantic operation, two parallel paths.** `/change:analyze` (documentation) and `/change:survey` (legacy code) answer the same question — "what slice-sized candidates exist in this source?" — and append to the same `## Candidate inventory` heading in `discovery.md`. They are not two operations; they are one operation with two evidence sources. The same duplication recurs at slice time: `/spec:define` writes artifacts from intent and docs, `/spec:extract` writes artifacts from code. Two skill families, one contract.

**Legacy migration is not core.** Deriving specs from existing source is a one-time, language-aware archaeology task that matters during migration and not afterwards. Today it lives in the framework's spine: enumeration briefs, repair loops, `surfaces.json` schema, language detection, the `legacy-code` kind discriminator, the `specify change survey` CLI verb. After a project finishes migrating, every line of this surface is dead weight that still has to be carried, taught, and tested.

**The adapter slot has no name for inputs.** Today, unqualified `adapter` names the *target* runtime (omnia, vectis, contracts). There is no symmetrical phrase for the *source* of evidence; the framework gestures at it through `kind`, through `language`, through per-language brief directories, through the `source` CLI noun on `/change:draft`. The asymmetry shows up as cognitive load on every skill author trying to add a new input.

The redesign collapses these three problems into one move: qualify adapters by direction, give source and target adapters the same implementation shape, make documentation-driven specification a first-class default source path, and put legacy-code support outside the core.

## Design

### Principles

1. **Two adapter roles, one implementation shape.** Source and target adapters share one shape: same loader, same validator, same cache layout, same `specify {source,target} resolve` verb family.
2. **Core is small.** Core ships the workflow (`/spec:`*, `/change:`*), the plugin resolver, the candidate-block grammar, the `discovery.md` handshake, the default `intent` and `documentation` source adapters, and the CLI primitives. Legacy-code, contract-import, and every target adapter remain add-ons.
3. **Source adapters emit intermediate representations, not artifacts.** The source-axis contract has exactly two core-facing shapes: `CandidateSet` for planning and `EvidencePack` for slice synthesis. Source adapters never own `spec.md`, `design.md`, or `tasks.md`.
4. **Core owns synthesis.** Specify core turns an `EvidencePackSet` plus target shaping guidance into canonical artifacts. That keeps provenance and conflict handling in one place rather than redistributing them across source or target adapters.
5. **`EvidencePackSet` is a set, cardinality ≥ 1.** A slice's evidence is structurally a set of source evidence packs. Pure-greenfield, pure-port, and pure-design slices land at cardinality one; combined-evidence slices (e.g. `legacy-code-typescript` + `documentation`) are normal at cardinality two or more. Synthesis is one operation that handles both shapes.
6. **Provenance is mechanical.** Every requirement in `spec.md` records which sources supplied it. `Sources:` lines list every contributing source key; the closed `Status:` enum (`agreed | unknown | conflict | divergence`) records how synthesis resolved them.
7. **Single writer.** The CLI is still the only writer of `plan.yaml`, `.metadata.yaml`, archive paths, `migration-log.yaml`, `discovery.md`, and the new `sources.yaml` / `targets.yaml` files. Adapters read; the CLI writes.
8. **Ground-up.** No alias keys, no compat fallbacks, no transitional schemas. The rename and restructure ship as one breaking minor.

### Vocabulary


| Term                              | Meaning                                                                                                                                                                                                              |
| --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **source adapter**                | Pluggable input role. Enumerates candidates from an evidence corpus and extracts per-candidate evidence packs. Examples: `intent`, `documentation`, `legacy-code-typescript`, `legacy-code-cobol`, `openapi`.        |
| **target adapter**                | Pluggable output role. Shapes canonical Specify artifacts for a runtime and turns them into runnable code. Examples: `omnia`, `vectis`, `contracts`. Replaces today's unqualified `adapter`.                         |
| **plugin**                        | Shared implementation shape of either adapter role. Used in schemas, the resolver, and implementation docs when speaking about both at once.                                                                         |
| **candidate** / **candidate set** | A slice-sized unit of work proposed by `enumerate`, and the set returned for one source. Serialised as candidate blocks in `discovery.md` under `## Candidate inventory`.                                            |
| **evidence pack**                 | Persisted source evidence returned by `extract`. Structured input to slice synthesis; stores facts, provenance, paths, spans, hashes, and bounded excerpts only when explicitly allowed.                             |
| **evidence pack set**             | The complete set of evidence packs bound to one slice. Input to Specify-owned artifact synthesis.                                                                                                                    |
| **provenance**                    | The set of source bindings backing a single requirement in `spec.md`. List in the schema; one or more entries.                                                                                                       |
| **conflict**                      | Disagreement on the same claim that the authority hierarchy cannot resolve — either intra-pack (one pack contradicts itself) or inter-pack at the same authority class. Surfaces as a `[conflict]` tag.              |
| **divergence**                    | Disagreement on the same claim across packs that the authority hierarchy *can* resolve. Synthesis writes the authority winner as the operative requirement and preserves the loser inline. Surfaces as a `[divergence]` tag. |
| **authority**                     | Per-pack classification drawn from the closed enum `intent \| external-contract \| design-spec \| observed-behaviour`. Used by synthesis to rank disagreeing packs.                                                  |


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

`**enumerate(source-binding) → CandidateSet`.** Called at plan time by `/spec:plan` ([RFC-26](rfc-26-workflow.md)). Reads the bound evidence (a local path, a documentation file, a free-text intent string) and emits candidate blocks under `## Candidate inventory` in `discovery.md`. Output grammar is the existing candidate-block format from RFC-20 §`discovery.md`, extended with stable candidate ids and optional correlation hints.

`**extract(candidate, source-binding) → EvidencePack`.** Called at slice time by `/spec:refine` ([RFC-26](rfc-26-workflow.md)) for each source bound to the slice. Returns a structured evidence pack persisted under the slice before synthesis:

```yaml
# .specify/slices/<slice>/evidence/<source-key>.yaml
source: legacy-monolith              # source-binding key
adapter: legacy-code-typescript      # source adapter name
authority: observed-behaviour        # source-adapter classification (§Synthesis contract)
candidate: user-registration         # candidate this pack covers
evidence:
  - kind: code-excerpt
    claim-id: users.register.email-validation
    path: src/users/register.ts
    lines: [12, 87]
    sha256: 6c25...
    excerpt: |
      export async function registerUser(req: …) { … }
  - kind: type-definition
    claim-id: users.types.RegisterRequest
    name: RegisterRequest
    path: src/users/types.ts
    lines: [4, 16]
    sha256: a84d...
  - kind: external-call
    claim-id: users.register.verify-call
    method: POST
    url: https://api.example.com/verify
    request-shape: { token: string }
    response-shape: { ok: boolean }
```

Documentation sources use the same envelope with doc-flavoured `kind:` values in place of `code-excerpt` / `type-definition` / `external-call`. `requirement-statement` is reserved for normative product or system behaviour ("must", "shall", "the system ..."). `acceptance-criterion` is reserved for directly testable examples, bullets, or Given/When/Then-style checks. `decision-record` captures rationale or explicit trade-offs. `document-section` captures supporting context that may inform `proposal.md` or `design.md` but is not itself a requirement. `diagram-reference` points at an external figure or image when the source document relies on it.

The pack's `kind` enum is closed and shared across all source adapters. Initial kinds: `intent-text`, `requirement-statement`, `acceptance-criterion`, `decision-record`, `document-section`, `diagram-reference`, `contract-reference`, `code-excerpt`, `type-definition`, `external-call`. New kinds require an RFC update. Evidence packs do not store raw source bodies by default — only structured facts, relative paths, line spans, content hashes, and bounded excerpts when the adapter contract explicitly allows them.

**Documentation extraction example.** Given this input document:

```markdown
# Password reset

The account service should let a registered user request a password reset link by email.

Acceptance:
- Unknown email addresses receive the same outward response as known users.
- Reset links expire after 30 minutes.

Decision: use the existing transactional email provider rather than introducing a new notification service.
```

`documentation.extract` returns one evidence pack:

```yaml
source: product-notes
adapter: documentation
authority: design-spec
candidate: password-reset
evidence:
  - kind: requirement-statement
    claim-id: password-reset.request
    path: docs/account.md
    lines: [3, 3]
    sha256: 4f39...
    statement: "The account service should let a registered user request a password reset link by email."
  - kind: acceptance-criterion
    claim-id: password-reset.response-privacy
    path: docs/account.md
    lines: [6, 6]
    sha256: 91aa...
    criterion: "Unknown email addresses receive the same outward response as known users."
  - kind: acceptance-criterion
    claim-id: password-reset.expiry
    path: docs/account.md
    lines: [7, 7]
    sha256: 0d8b...
    criterion: "Reset links expire after 30 minutes."
  - kind: decision-record
    path: docs/account.md
    lines: [9, 9]
    sha256: f6c2...
    decision: "Use the existing transactional email provider rather than introducing a new notification service."
```

Core synthesis turns that pack into normal Specify artifacts. The resulting `spec.md` carries provenance on every requirement:

```markdown
### Requirement: Password reset request

ID: REQ-001
Sources: [product-notes]
Status: agreed

The system lets a registered user request a password reset link by email.

### Requirement: Password reset response privacy

ID: REQ-002
Sources: [product-notes]
Status: agreed

The system returns the same outward response for known and unknown email addresses.

### Requirement: Password reset expiry

ID: REQ-003
Sources: [product-notes]
Status: agreed

The system expires password reset links after 30 minutes.
```

**Authority and claim-id.** Each pack carries a top-level `authority:` field — the source adapter's self-classification, drawn from the closed enum `intent | external-contract | design-spec | observed-behaviour`. Adapters may set it explicitly per-pack; absent an explicit value the pack inherits the adapter manifest's `default-authority`. Synthesis uses authority to rank claims when packs disagree (§Synthesis contract).

Each evidence entry that asserts a claim about behaviour (`requirement-statement`, `acceptance-criterion`, `code-excerpt` of executable behaviour, `external-call`, `type-definition`, `contract-reference`) carries an optional `claim-id:` — a stable, adapter-controlled identifier for the claim. When two packs emit entries with the same `claim-id`, synthesis fuses them into a single requirement and applies authority resolution. When `claim-id` is absent, synthesis falls back to semantic correlation across packs. Adapters that can produce stable identifiers — `legacy-code-*` keyed by symbol path, `documentation` keyed by heading slug — should do so; deterministic fusion produces stable golden output.

Packs validate against `schemas/evidence-pack.schema.json` before synthesis. The CLI writes packs; source adapters return content through briefs and WASI tools, not by touching slice paths directly.

### Default source adapters

Core ships two source adapters by default.


| Adapter         | `default-authority` | Role                                                                                                                                                                                                               |
| --------------- | ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `intent`        | `intent`            | Captures operator-authored briefs, inline requirements, and explicit corrections. Used when no other source is bound, or to override other sources at slice authoring time.                                       |
| `documentation` | `design-spec`       | Captures requirements documents, design notes, proposals, RFCs, existing specs, architecture records, and other written product or technical intent. The ordinary documentation-driven path into specs and design. |


Both are true source adapters with manifests, `enumerate` briefs, and `extract` briefs. They are default-packaged for usability, but they do not get special workflow rules. `/spec:plan` ([RFC-26](rfc-26-workflow.md)) calls `enumerate`; `/spec:refine` calls `extract`; slice synthesis consumes an evidence pack.

There is no orphan-define path: every slice originates from a `plan.yaml` entry, even N=1 greenfield work, which `/spec:plan` handles via a degenerate `intent.enumerate` (one synthetic candidate). See [RFC-26 §Planning at every scale](rfc-26-workflow.md#planning-at-every-scale).

### Target adapter contract

A target adapter contributes runtime-specific shaping and implementation behavior for canonical Specify artifacts. It does **not** own source-to-`spec.md` / `design.md` synthesis; Specify core owns that contract so provenance handling and multi-pack authority resolution stay uniform across every target rather than reimplemented N times. A target adapter may declare:

- `shape` — optional target-idiom guidance consumed by core synthesis when producing `proposal.md`, `spec.md`, `design.md`, and `tasks.md`. For Omnia this includes crate boundaries, provider patterns, handler vocabulary, and WASM constraints; for Vectis it includes Crux core and shell composition conventions.
- `build` — implementation briefs and optional tools that turn reviewed artifacts into code.
- `merge` — merge/finalisation briefs and optional tools for target-specific cleanup after a slice is built.

The manifest shape matches today's `adapter.yaml`; see §CLI surface for the full path / verb rename table.

**Pipeline verbs split by phase.** Today's `specify adapter pipeline {define,build,merge}` moves to core-owned synthesis (define phase) and hand-coded skill substeps (build/merge phase) for v1. Standalone topology verbs (`specify slice synthesize`, `specify target build`, `specify target merge`) return when a third-party target needs custom brief ordering — see `[commands.md](commands.md)`.

Unqualified `adapter` does not survive anywhere in the schema, on the CLI, in docs, or in skill prose. RFC-24's "adapter-gated finding" becomes "target-gated finding"; `planSlice.adapter` becomes `planSlice.target`. Today target adapter briefs often own `proposal`, `specs`, `design`, and `tasks` directly — under this RFC those move into Specify-owned synthesis, and target adapters supply `shape` guidance plus `build` / `merge` behavior.

### Discovery handshake — candidate correlation

v1 enumeration runs every bound source adapter for a change. Each candidate block carries a stable `id`, a `sources:` list naming the adapter(s) that contributed the candidate, and an optional `correlates-with:` list referencing other candidate ids that synthesis should treat as the same slice-sized unit of work.

When two source adapters independently surface the same behavioural slice — typically a documentation candidate and a legacy-code candidate that describe the same feature — the operator merges them at propose time by adding `correlates-with: [<other-id>]` to one block, or by accepting a synthesis-suggested correlation. The merged candidate's `sources:` list carries every contributing key; `specify plan add` writes a single slice with the combined source bindings.

Re-running `enumerate` against the same source replaces blocks by stable id rather than appending duplicates (the "skip if heading exists" rule is dropped in favour of explicit id-based replace). Re-running `enumerate` against a different source appends new candidates with their own ids; correlation is an explicit operator action at propose time, not an automatic merge.

**Normative schema.** Candidate blocks in `discovery.md` validate against `schemas/discovery/candidate-block.schema.json` (extends RFC-20 with required `id`, required `sources[]`, and optional `correlates-with[]`). The CLI discovery writer is the parser of record; there is no separate on-disk `candidates.yaml` in v1 — `discovery.md` under `## Candidate inventory` remains the plan-time source of truth, and `specify plan add` reads blocks from there.

### `planSlice.sources` — one or more bindings

`planSlice.sources` is **a list** of one or more source-binding keys; cardinality ≥ 1, no upper bound:

```yaml
slices:
  - name: identity-user-registration
    target: omnia                                # renamed from `adapter`
    project: identity-svc
    sources: [legacy-monolith, identity-design-notes]
    status: pending
```

Four archetypes follow without special-casing:


| Archetype          | `sources`                              | Meaning                                                                                                                                         |
| ------------------ | -------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Pure greenfield    | `[intent]`                             | New work driven by operator intent. `intent` is bound implicitly when `sources` is omitted; `[]` is normalised to `[intent]` before extraction. |
| Pure port          | `[<one-legacy-source>]`                | Legacy code dictates behaviour.                                                                                                                 |
| Pure design        | `[<one-doc-source>]`                   | Documentation dictates behaviour.                                                                                                               |
| Combined evidence  | `[<doc-source>, <legacy-source>, …]`   | Multiple sources contribute; authority hierarchy resolves disagreements (§Synthesis contract).                                                  |


The combined-evidence archetype is the canonical migration shape: documentation captures intent, legacy code captures observed behaviour, and synthesis fuses them with authority resolution. Every archetype runs the same multi-source synthesis step (§Slice authoring synthesis); pure cases are simply degenerate sets of cardinality one.

Schema constraints that `specify plan add` enforces in v1:

- At most one `intent` binding per slice (free-text operator content has no corpus to enumerate from).
- At most one binding per source key per slice.
- At least one binding total — empty `sources: []` is normalised to `[intent]`.

### Slice authoring synthesis

`/spec:refine` ([RFC-26](rfc-26-workflow.md)) runs one pipeline:

1. **Resolve** the bound target adapter (one) and every bound source adapter (one or more).
2. **Extract** per §Extraction reliability — call `extract(candidate, source-binding)` on each bound source serially in `planSlice.sources` declaration order; persist each pack under `.specify/slices/<slice>/evidence/<source-key>.yaml`.
3. **Synthesise** per §Synthesis contract — invoke core synthesis with the `EvidencePackSet` (cardinality ≥ 1) plus optional `target.shape` guidance. Synthesis applies the authority hierarchy across packs.
4. **Validate** — `specify slice validate` checks structural requirements (`Sources:`, `Status:`, closed tags) and emits findings for `[conflict]`, `[divergence]`, and `[unknown]`.
5. **Lifecycle** — transition to `defined`. Tags on requirements are operator-visible signals to review `spec.md` before `/spec:build`, but they do not park the slice in a separate lifecycle state.

**Tag semantics:**

- `[unknown]` — no pack supplies the fact.
- `[conflict]` — packs disagree at the same authority class, or a single pack contains an internal contradiction. Operator must reconcile before build is meaningful, but the slice is not parked.
- `[divergence]` — packs disagree across authority classes. Synthesis records the authority winner as the operative requirement and preserves the loser as inline commentary so the operator can override if the hierarchy chose wrong.

**N=0** normalises to `[intent]` plus a synthetic candidate written by `/spec:plan` ([RFC-26](rfc-26-workflow.md) §Planning at every scale); every slice has a plan entry.

### Per-requirement provenance and tags

`spec.md` gains a fixed-format `Sources:` and `Status:` line below every `ID: REQ-XXX` block. Single-source case:

```markdown
### Requirement: User registration accepts valid email

ID: REQ-001
Sources: [legacy-monolith]
Status: agreed

The system accepts a registration request when the email field is RFC-5322 valid …
```

Combined-evidence case where two packs agree:

```markdown
### Requirement: User registration accepts valid email

ID: REQ-001
Sources: [identity-design-notes, legacy-monolith]
Status: agreed

The system accepts a registration request when the email field is RFC-5322 valid …
```

`[divergence]` example — docs and legacy code disagreed and authority resolved in favour of `design-spec` over `observed-behaviour`:

```markdown
### Requirement: Reset link expiry [divergence]

ID: REQ-007
Sources: [identity-design-notes, legacy-monolith]
Status: divergence

The system expires password reset links after 30 minutes. (from identity-design-notes; design-spec)

Note: legacy-monolith observed 24-hour expiry; the design-spec authority overrides. Operator review recommended.
```

`Status:` is a closed enum: `agreed | unknown | conflict | divergence`. Together with `Sources:`, it makes the synthesis decision auditable.

`Sources:` is required on every requirement and lists every contributing source key in the order synthesis honoured them (highest authority first). Intent-only slices populate `Sources: [intent]` so the audit trail is uniform across pure and combined-evidence cases.

### Synthesis contract

Core-owned synthesis is the single writer of `proposal.md`, `spec.md`, `design.md`, and `tasks.md`. Target adapters supply `shape` guidance only.

**Inputs**


| Input                | Source                                    | Validated by                        |
| -------------------- | ----------------------------------------- | ----------------------------------- |
| `EvidencePackSet`    | `.specify/slices/<slice>/evidence/*.yaml` | `schemas/evidence-pack.schema.json` |
| `planSlice` bindings | `plan.yaml` entry                         | `schemas/plan/plan.schema.json`     |
| `shape` brief        | `specify target resolve`                  | target manifest                     |


**Outputs**


| Artifact                | Required sections                                                                  |
| ----------------------- | ---------------------------------------------------------------------------------- |
| `proposal.md`           | Scope, motivation (existing Specify artifact rules)                                |
| `specs/<crate>/spec.md` | Per requirement: `ID:`, `Sources:`, `Status:`; optional `[conflict]` / `[divergence]` / `[unknown]` |
| `design.md`             | Domain model, integrations (existing rules)                                        |
| `tasks.md`              | Sequenced implementation tasks                                                     |


**Division of labour**


| Layer     | Responsibility                                                                                                    |
| --------- | ----------------------------------------------------------------------------------------------------------------- |
| **Agent** | Semantic authoring from evidence packs and shape guidance (brief body under `plugins/spec/references/synthesis/`) |
| **CLI**   | `specify slice validate` (structure, provenance lines, tag enum), `specify slice transition` (lifecycle stamps)   |


**Authority hierarchy.** When two or more packs supply the same claim — correlated by `claim-id` when both packs emit one, semantically by synthesis otherwise — synthesis ranks them by authority class:

1. `intent` — operator's stated intent overrides everything; the operator is the source of truth at slice authoring time.
2. `external-contract` — published API contracts, OpenAPI specs, regulatory requirements; binding on the system.
3. `design-spec` — internal design documents, RFCs, architecture decisions; authoritative for behaviour the code has not yet realised.
4. `observed-behaviour` — facts read from legacy code; authoritative for what the system *does*, not what it *should* do.

Resolution rules:

| Pack agreement | Authority winner | Output |
|---|---|---|
| One pack supplies claim | n/a | `Status: agreed`, `Sources:` lists the one key. |
| Multiple packs agree on the value | n/a | `Status: agreed`, `Sources:` lists every contributing key (highest authority first). |
| Multiple packs disagree, single highest authority | The highest-ranked pack | `Status: divergence`, requirement tagged `[divergence]`, winner's value written, loser preserved as inline commentary, `Sources:` lists every contributing key. |
| Multiple packs disagree, two or more share the highest authority | None | `Status: conflict`, requirement tagged `[conflict]`, both values preserved as inline commentary; operator reconciles by editing the slice or amending source bindings. |
| No pack supplies the claim | n/a | `Status: unknown`, requirement body tagged `[unknown]`. |

**Substep order (v1).** Hand-coded in `/spec:refine`: `proposal` → `specs` → `design` → `tasks`, optionally injecting `target.shape` guidance from `specify target resolve`. Every brief is core-owned; targets do not register define-phase briefs.

**Halt rules.** Synthesis writes through every requirement; `[conflict]`, `[divergence]`, and `[unknown]` tags surface in `spec.md` for operator review but do not abort the pass or park the slice. Slice lifecycle is `defining → defined → built → merged` — see [RFC-26 §Combined lifecycle](rfc-26-workflow.md#combined-lifecycle-rfc-25--rfc-26).

### Extraction reliability


| Rule              | Behaviour                                                                                                                                                                                                                                |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Order**         | `extract` runs serially in `planSlice.sources` declaration order. Serial keeps results deterministic for goldens and gives operators a predictable failure mode. Parallel extraction returns when a real consumer asks for it.            |
| **Required**      | Every key in `planSlice.sources` is required by default. A per-binding `optional: true` flag (declared on `sources.yaml`, consumed by `specify plan add`) marks a binding whose `extract` is allowed to return an empty pack — or to fail — without halting synthesis. |
| **Hard failure**  | If a required `extract` fails, the slice stays in `defining`, no synthesis runs, and the CLI emits a structured error naming the source key.                                                                                              |
| **Soft failure**  | Optional bindings whose `extract` fails emit a warning, persist no pack, and synthesis proceeds with the remaining packs. Resulting requirements that would otherwise have been agreed across packs may downgrade to `Status: agreed` against the surviving source. |
| **Empty pack**    | A pack containing zero evidence entries is valid and contributes nothing to synthesis; downstream tags reflect what other packs supplied (or `[unknown]` if no pack covered the requirement).                                              |
| **Partial packs** | Invalid packs fail validation against `evidence-pack.schema.json` before synthesis starts.                                                                                                                                                |


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
specify_version: 3.0.0
sources:                          # set of source adapters available in this project
  - intent                        # default source adapter
  - documentation                 # default source adapter
  - legacy-code-typescript
target: omnia                     # single target adapter this project produces in v1
hub: false
```

`sources` lists source adapters available for binding to slices, not bindings themselves. Source bindings live on individual sources (a path, a doc location, the operator's intent text) and are recorded in `sources.yaml` per [RFC-21](rfc-21-catalogue.md). `target` names the single target adapter this project produces in v1. `planSlice.target` remains explicit for hub plans and future multi-target projects, but v1 validation requires it to match the resolved project's `project.yaml.target`.

The old singular `project.yaml.profile` / `project.yaml.adapter` field is gone.

### CLI surface

The full v1 CLI floor — including verbs cut from v1 and post-v1 deferrals — is enumerated in `[commands.md](commands.md)`. RFC-25 axis-relevant deltas:

**Renames:** `specify adapter resolve` → `specify target resolve`; `adapters/<name>/adapter.yaml` → `targets/<name>/target.yaml`; `schemas/adapter.schema.json` → `schemas/target.schema.json`; `planSlice.adapter` → `planSlice.target`; `Plan::resolve_adapter` → `Plan::resolve_target`; `Error::AdapterResolution` → `Error::TargetResolution`. `Plan::resolve_source` returns `Vec<SourceAdapter>` (one or more bindings). `specify adapter pipeline {define,build,merge}` retires — substep order is hand-coded in skills for v1.

**Additions (v1):**

- `specify source resolve <name>` — materialise a source adapter's briefs and tools.
- `specify plan amend --add-source <key>` / `--remove-source <key>` — rebind sources on an existing slice. Permitted while `plan.lifecycle ≤ reviewed` and the slice's per-entry lifecycle is `pending`; rebinding an already-extracted slice requires the operator to drop and re-define.

**Retirements:**


| Verb / Skill            | Replaced by                                                                                                                                       |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/change:analyze`       | One discovery stage in `/spec:plan` ([RFC-26](rfc-26-workflow.md)) that resolves the bound source adapter and calls `enumerate`.                  |
| `/change:survey`        | Same — when the bound source adapter is a `legacy-code-`* flavour, `enumerate` does what `survey` does today.                                     |
| `/spec:extract`         | The bound source adapter's `extract` capability, called from `/spec:refine` ([RFC-26](rfc-26-workflow.md)).                                       |
| `specify change survey` | Folded into the source-adapter-driven discovery stage. The bounded repair loop becomes a contract on the source adapter's `enumerate` capability. |


### Repository layout

v1 keeps every adapter inside the `augentic/specify` monorepo. The repository carve-out (one repo per target adapter, one repo per source-adapter family) is deferred until an individual adapter needs an independent release cadence — the structural prerequisite is the unified `plugin.schema.json` resolver and cache layout, which land in v1 regardless.

**Layout in `augentic/specify` (v1):**

- `/spec:{init,plan,refine,execute,build,merge,finalize,drop}` — workflow skills ([RFC-26](rfc-26-workflow.md) operator surface).
- `plugins/references/` — cross-cutting references.
- `sources/intent/`, `sources/documentation/` — default source adapters.
- `sources/legacy-code-typescript/` — the single legacy-code language v1 ships, carved from `plugins/change/skills/survey/briefs/enumerate/typescript.md` and the existing repair loop.
- `targets/omnia/`, `targets/vectis/`, `targets/contracts/` — carved from `adapters/<name>/`.

**Deferred from v1:**

- `sources/legacy-code-{cobol,csharp,rust,javascript}` — ship per language as a real consumer needs it.
- `sources/{openapi,asyncapi,json-schema}` — today's `/contract:`* import surface stays as today's skills for v1; conversion into source adapters lands when a real consumer needs contract evidence packs alongside `intent` or `legacy-code-*` in a single slice.
- Splitting `targets/<name>/` and the legacy-code family out of the monorepo.

### `surfaces.json` and per-language briefs

`surfaces.json` becomes a source-adapter-internal artifact owned by `sources/legacy-code-typescript/`. Its schema, repair loop, and validator code move out of `specify-cli` into the adapter (which may ship its own WASI tools). The `specify change survey` CLI verb is deleted. The TypeScript enumeration brief moves from `plugins/change/skills/survey/briefs/enumerate/typescript.md` to `sources/legacy-code-typescript/briefs/enumerate.md`. Other languages stay parked at today's brief location until the adapter is built.

### `survey.md`

Renamed `discovery-summary.md` and made generic. Sections become:

1. `# <change> discovery summary`
2. `## Summary` — counts: candidate / unresolved.
3. `## Source inventory` — one row per bound source: source-key, adapter, location, contribution count.
4. `## Candidate inventory` — fenced-YAML blocks per candidate; one block per stable id.

Legacy-code-only columns (LOC, language, `surfaces.json` digest) populate only when the bound source adapter supplied them. The same file shape covers documentation-only and intent-only runs.

## Implementation Plan

1. **Schemas.** Land `schemas/plugin.schema.json`, `schemas/source.schema.json`, `schemas/target.schema.json`, `schemas/evidence-pack.schema.json`, and `schemas/discovery/candidate-block.schema.json`. Delete `schemas/adapter.schema.json`. Update `schemas/plan/plan.schema.json` to rename `adapter` → `target` and make `sources` a required list (min 1, no upper bound). Update `schemas/sources/sources.schema.json` for source-adapter identity fields and the per-binding `optional: true` flag. The closed `Status:` enum is `agreed | unknown | conflict | divergence`. `evidence-pack.schema.json` requires top-level `authority:` (closed enum `intent | external-contract | design-spec | observed-behaviour`) with adapter-manifest defaults, and accepts optional `claim-id:` on every claim-shaped evidence entry. `candidate-block.schema.json` carries required `id`, required `sources[]`, and optional `correlates-with[]`. Source manifests gain `default-authority:`.
2. **Domain rename.** Mass-rename unqualified `Adapter`* → `Target`* across today's target-runtime code in `crates/domain/`, `crates/tool/`, `crates/error/`, `src/`. Update `Error` discriminants. Update `Plan::resolve_adapter` → `Plan::resolve_target`. Land `Plan::resolve_sources` returning `Vec<SourceAdapter>` (one or more bindings).
3. **Plugin loader.** New module `crates/domain/src/plugin/` containing `resolver.rs`, `cache.rs`, `manifest.rs`, `axis.rs`. Replaces `crates/domain/src/adapter/`. One loader, two axes.
4. **Default source adapters.** Ship `sources/intent/` and `sources/documentation/` in core, each with a manifest declaring `default-authority` (`intent` and `design-spec` respectively), `briefs/enumerate.md`, and `briefs/extract.md`. `intent` enumerate emits one candidate from the operator's brief and extract emits the brief text as `kind: intent-text`; `documentation` enumerate reads bound docs and emits candidate blocks, then extract emits documentation-native evidence entries (`requirement-statement`, `acceptance-criterion`, `decision-record`, `document-section`) with `claim-id` keyed by heading slug + position.
5. **Slice synthesis.** Implement §Synthesis contract: core briefs under `plugins/spec/references/synthesis/`, `/spec:refine` ([RFC-26](rfc-26-workflow.md)) implements extract-per-binding → synthesise → validate, substep order hand-coded in the skill body for v1. Synthesis applies the authority hierarchy across packs, fuses claims by `claim-id` where present, falls back to semantic correlation otherwise, and emits `[conflict]` / `[divergence]` / `[unknown]` tags per §Synthesis contract resolution rules. Migrate target define briefs into core synthesis + `shape` briefs. The `specify slice synthesize` topology verb is deferred until a third-party target needs custom brief ordering.
6. **Provenance tags.** Extend `spec.md` parser in `crates/domain/src/specs/` to require `ID:`, `Sources:`, `Status:` lines on every requirement block. The closed tag enum is `[conflict] | [divergence] | [unknown]`; the closed `Status:` enum is `agreed | unknown | conflict | divergence`. `Sources:` parses as a list (length ≥ 1).
7. **Discovery handshake.** Implement stable-id replace-by-id in the discovery writer; emit and parse `correlates-with:` on candidate blocks; teach `specify plan add` to fold correlated candidates into one slice with combined `sources:`. Add fixture and golden coverage for re-enumerate idempotence and cross-source correlation.
8. **CLI surface.** New v1 verbs: `specify source resolve <name>` (materialises briefs + WASI tools); `specify plan amend --add-source <key>` / `--remove-source <key>` (slice rebinding). Rename: `specify adapter resolve` → `specify target resolve`. Retire `specify adapter pipeline {define,build,merge}` — the substep order is hand-coded in `/spec:refine`, `/spec:build`, `/spec:merge` ([RFC-26](rfc-26-workflow.md)). Delete: `specify change survey`. See `[commands.md](commands.md)` for the full v1 floor and cut verbs.
9. **Target brief migration.** Move today's target-owned `proposal`, `specs`, `design`, and `tasks` brief content into the core synthesis contract where it is target-neutral, and into target `shape` briefs where it is target-specific. Update RFC-24 and target skill prose to describe `shape` as guidance, not artifact ownership.
10. **Documentation rewrite.** `AGENTS.md`, `.cursor/rules/project.mdc`, `docs/explanation/decision-log.md` (§Decision-log supersessions), `docs/contributing/adapter-anatomy.md` — adapter vocabulary, pipeline split, and superseded "analyze/extract split" / define-phase target ownership. RFC-22, RFC-24 prose updated. `.cursor/rules/project.mdc` §authority hierarchy promoted from documentation principle to synthesis-time enforcement note.
11. **`discovery-summary.md` rename.** Implement the generic form. Update fixtures.
12. **Acceptance.** Cross-repo Deno suite gains (land **before** RFC-26 collapse scenarios):
    - documentation-only slice with `Sources: [<doc-key>]` provenance;
    - legacy-only slice with `Sources: [<legacy-key>]` provenance;
    - pure-intent slice with `Sources: [intent]`;
    - **combined-evidence slice (`legacy-code-typescript` + `documentation`)** — packs agree on most claims, `Sources:` carries both keys, `claim-id` correlation produces deterministic fusion;
    - **`[divergence]` from authority resolution** — combined-evidence slice where docs and code disagree at different authority classes; design-spec wins, observed-behaviour preserved as inline commentary;
    - **`[conflict]` from same-authority disagreement** — two `documentation` sources disagree on the same claim; both values preserved, requirement tagged `[conflict]`;
    - intra-pack `[conflict]` surfaced in `spec.md`;
    - target-`shape` fixture;
    - required-source extract failure halts the slice;
    - **optional-source extract failure proceeds with remaining packs** and warns;
    - invalid evidence-pack schema rejection;
    - **`correlates-with` propose-time merge** — two adapters surface the same candidate, operator merges, `specify plan add` writes one slice with combined sources.
13. **Observability ([RFC-19](rfc-19-observability.md)).** Journal events for `extract` completion (one per source key per slice), synthesis completion, and `[conflict]` / `[divergence]` / `[unknown]` findings — so operators get traceability without parsing skill output.

## Migration

**There is no backward compatibility.** This RFC ships as part of Specify 3.0 alongside [RFC-26](rfc-26-workflow.md); the two land together with a single release tag and a single migration script.

The adapter-axis half of the migration is covered by [RFC-26 §Migration](rfc-26-workflow.md#migration)'s `migrate-to-3.0.sh`, which performs mechanical renames against `project.yaml`, `registry.yaml`, `plan.yaml`, `sources.yaml`, `.specify/.cache/`, and `.specify/archive/` (`yq` + `sed`) in addition to RFC-26's skill-directory moves and `reviewed` lifecycle addition. The plugin cache re-fetches on next invocation; operators who stay on 1.x pin plugin and CLI versions. There is **no** `specify upgrade` verb — see `[commands.md](commands.md)`.

Plugin authors ship renamed manifests against the new schemas; `adapter.yaml` fails to load on 3.0 with no grace period. JSON envelopes rename `adapter` → `target`; add `sources[]` consumers where slice-level evidence matters.

There is no 2.x intermediate release. The earlier draft sequenced RFC-25 as 2.0 and RFC-26 as 3.0 with a parallel preview channel for early adopters; that sequencing carried doubled migration scripts, a `specify-3.0-preview` marketplace tag, and a "do not invest in `/change:`* changes during 2.x" caveat that pointed at zero productive lifetime. Single-release collapse removes all three. The intra-release ordering is preserved: RFC-25 §Implementation Plan steps 1–13 precede the RFC-26 workflow collapse inside the same 3.0 release.

## Alternatives Considered

Key rejections (full rationale in `[docs/explanation/decision-log.md](../docs/explanation/decision-log.md)` when this RFC lands):

- **One unified "lens" name** — plugin shape is shared; roles are not. Two qualified roles (`source adapter`, `target adapter`) read honestly.
- **Keep `/spec:extract`, `/change:analyze`, `/change:survey` as named skills** — the names *are* the bifurcation; one discovery stage and one authoring entry point with source-adapter dispatch is the move.
- **Per-source artifact files (`spec.<source>.md`)** — operators review one `spec.md`; inline `Sources:` lines preserve the audit trail.
- **Source adapters emit artifacts directly; targets own define-phase briefs** — duplicates provenance handling and blocks multi-source synthesis in core. Source adapters emit `CandidateSet`s and `EvidencePack`s only; targets supply `shape` + `build` + `merge`.
- **Target adapters participate in discovery** — re-merges the axes; baseline-aware planning is RFC-22's ledger territory.
- **Defer multi-source synthesis to a follow-on release** — earlier draft scoped v1 to single-source on the assumption that no v1 caller exercises multi-source. Rejected on reread: legacy-migration changes routinely combine code with surrounding design documentation, and forcing that shape into a single-source approximation throws away the most natural source of `Status: agreed` cross-corroboration. Multi-source ships in v1.

## Non-Goals

- Backward compatibility with Specify 1.x manifests, schemas, verbs, or directory layouts.
- A general "plugin marketplace" or runtime plugin discovery. Source and target adapters are installed at project-init time.
- Per-handler provenance below the requirement level. `Sources:` lives on requirement blocks; finer granularity belongs in `design.md` per existing convention.
- Per-pack confidence scores. Authority is class-based; finer scoring belongs to a future RFC if operator demand emerges.
- Replacing operator review of conflicts. `[conflict]` tags surface in `spec.md`; operators decide. Auto-resolution heuristics are out of scope.
- Cross-repo source sharing. Each platform-repo declares its own sources via `sources.yaml`, consistent with RFC-21.
- Bidirectional adapters (an adapter that is both source and target). The axis is a discriminator, not a tag set.
- Source-adapter support for editing artifacts after slice authoring. Sources read; the workflow writes.

**Deferred from v1, reinstated when a real caller asks:**

- Source-adapter `detect[]` auto-detection from a path.
- Repository carve-out (per-adapter or per-family repos). v1 keeps everything in `augentic/specify`.
- Legacy-code source families beyond `legacy-code-typescript`.
- Contract-source adapters (`sources/{openapi,asyncapi,json-schema}`). Today's `/contract:`* skills cover the v1 import path.
- Parallel `extract` across bindings. v1 runs serially in `planSlice.sources` declaration order — see §Extraction reliability.
- Per-slice and per-claim operator authority overrides (`plan.yaml.slices[].authority-override`). v1 uses adapter-class defaults; per-slice overrides return when a real ambiguity surfaces and editing `spec.md` after `[divergence]` is no longer enough.
- Multi-target projects. v1 project configuration has exactly one `target`; a future RFC may relax this when a real bi-targeting case lands.

## Open Questions

1. Should the default `intent` and `documentation` source adapters be packaged as true plugin implementations (with `source.yaml` etc.) or hard-wired into the core CLI as built-ins? Current preference: true plugin implementations, shipped in-repo under `sources/intent/` and `sources/documentation/`, so the plugin shape has zero exceptions.
2. How should `extract` be sandboxed when a source adapter ships a WASI tool? Current preference: same posture as RFC-15 — the WASI tool runs under the existing `specify tool run` sandbox; briefs read its output.
3. **Per-pack `authority:` requiredness.** Adapters declare `default-authority` in their manifest (e.g. `legacy-code-* → observed-behaviour`, `documentation → design-spec`); per-pack `authority:` overrides the default. Should v1 require explicit `authority:` on every emitted pack, or treat the manifest default as load-bearing? Current preference: manifest default is the contract; explicit per-pack `authority:` is reserved for adapters that classify sub-corpus content (e.g. an OpenAPI source emitting both `external-contract` operation packs and `design-spec` description packs once `sources/openapi` ships).
4. **`claim-id` requiredness for documentation.** Source adapters that can produce stable identifiers should emit `claim-id`; absent that, synthesis correlates semantically. Should v1 require `claim-id` on `requirement-statement` and `acceptance-criterion` from `documentation`, given that heading-slug + position identifiers are mechanical? Current preference: yes for `documentation`'s claim-shaped kinds, optional for everything else; semantic-correlation refinement is a follow-up.
5. **Operator override seam for `[divergence]`.** When the operator believes synthesis chose the wrong authority winner, the v1 path is "edit `spec.md`, swap the requirement body, downgrade the `[divergence]` tag manually." Per-slice and per-claim authority overrides return with a real consumer ask — see §Non-Goals.

## References

- [RFC-19: Observability](rfc-19-observability.md) — journal events for extract and synthesis (implementation plan step 13).
- [RFC-20: Survey-to-Plan Pipeline (archived)](archive/rfc-20-survey.md) — the survey pipeline this RFC folds into the `sources/legacy-code` adapter family.
- [RFC-21: Source Catalogue and Tier-1 Cache](rfc-21-catalogue.md) — `sources.yaml` survives; binding fields extend to record source-adapter identity.
- [RFC-22: Migration Ledger and Slice Mapping](rfc-22-ledger.md) — adapter-typed entries become target-typed; otherwise unchanged.
- [RFC-23: Change Lifecycle (archived)](archive/rfc-23-change-lifecycle.md) — the `/change:draft` → `/change:execute` → `/change:finalize` three-skill model is superseded by [RFC-26](rfc-26-workflow.md) in the same 3.0 release; the discovery stage inside what was `/change:draft` is restructured here and consumed by `/spec:plan`.
- [RFC-24: Omnia Plan Composition](rfc-24-omnia.md) — adapter-gated findings become target-gated, and Omnia artifact-authoring briefs become target `shape` guidance. `omnia` becomes a target adapter.
- [RFC-26: Workflow Collapse](rfc-26-workflow.md) — planned follow-on; collapses the `/change:*` and `/spec:*` skill families into one operator surface on top of this RFC's adapter axis.
- [RFC-15: WASM Plugins (archived)](archive/rfc-15-wasm-plugins.md) — the WASI tool surface reused as the deterministic-CLI seam inside source and target adapters.
- `[specify-cli/AGENTS.md](https://github.com/augentic/specify-cli/blob/main/AGENTS.md)` — exit codes and CLI contract preserved; rename surface documented there.
- `[.cursor/rules/project.mdc](../.cursor/rules/project.mdc)` — artifact authority hierarchy. The synthesis-time authority hierarchy (`intent > external-contract > design-spec > observed-behaviour`) ships in v1 — see §Synthesis contract.

