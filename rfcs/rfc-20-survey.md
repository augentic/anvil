# RFC-20: Survey-to-Plan Pipeline

> Status: Draft - Depends: [RFC-3a](archive/rfc-3a-monoliths.md), [RFC-3b](archive/rfc-3b-platform.md), [RFC-9](archive/rfc-9-platform.md), [RFC-13](archive/rfc-13-extensibility.md)

## Abstract

Extend the `/change:plan` brief pipeline so that the operator can produce a **migration plan** for a legacy system — single or multi-source — by progressively decomposing the system into coherent chunks until every leaf is **slice-sized (T-shirt: Small)** and ready to feed `/spec:propose`. The surveyor's job is **not** to extract specs from legacy code; it is to understand the system *sufficiently* to plan the migration. Spec authorship remains `/spec:extract` (at define-time) and `/spec:define` (for greenfield).

Concretely, this RFC adds:

1. A new closed-enum kind `domain-model` for `/change:analyze`, with a pinned schema for bounded contexts, aggregates, ownership, and routing hints. This is the top-down architectural anchor for decomposition.
2. A pinned **T-shirt sizing rubric** for plan-time chunks — LOC-based defaults at the framework level, optionally tightened per capability.
3. A new `survey` brief, run between sync-workspace (3b) and propose (3c), that performs an **iterative top-down decomposition** of the system into a DAG of chunks, halting at slice-sized leaves. It emits `survey.md` — a byte-stable representation of the DAG plus per-node sizing, evidence, and routing metadata.
4. A new `synthesise` brief, run after survey, that reconciles documented capabilities against the survey's leaves and appends a `## Reconciliation` block to `discovery.md`.

The plan skill's five-step loop, the single-writer invariant for `plan.yaml`, and the closed-kind enum's strict validation posture are preserved. `/change:analyze` remains a one-shot fan-out per source — the survey brief consumes its inventory and structural metadata, not the source code itself. The four scenarios this RFC unblocks — single-repo legacy migration, multi-repo legacy migration, greenfield multi-repo, and brownfield multi-repo — share the same pipeline; only the inputs differ.

## Motivation

The framework already supports per-source capability extraction (`/change:analyze`), per-input dispatch (the discovery brief), and multi-project routing (assignment + greenfield-registry-bootstrap). What is missing is **the decomposition step that turns "here is a 100k LOC monolith" into "here are the 30 slice-sized chunks to migrate, in dependency order"**.

- **No structured architectural input.** Domain models, context maps, EventStorming exports, and design docs all land as opaque `kind: documentation`. Bounded contexts, aggregates, and ownership are not extracted into discrete, queryable form. The greenfield-registry-bootstrap clustering algorithm therefore cannot key on architectural intent — it is driven by capability inference alone.
- **No decomposition primitive.** Today's discovery brief loops over inputs sequentially and merges by capability name. There is no representation of *the system as a graph of chunks at different granularities*, and no notion of when a chunk is "small enough to migrate." That reasoning falls entirely to the operator reading `discovery.md` line by line.
- **No explicit stopping criterion.** Propose accepts whatever slices the operator and brief negotiate, but there is no upstream contract that says "every input to propose is already at slice scale." That contract is what makes a decomposition useful — and is what lets propose stop second-guessing slice boundaries.
- **No reconciliation step.** When the docs claim capability X exists and the code shows capability Y, today's pipeline appends both with their respective confidences and lets the operator notice the mismatch in the propose review. There is no first-class reconciliation block that flags "documented but not in code", "in code but undocumented", or "evidence concordance".
- **Scenario coverage is uneven.** Single-source migrations work cleanly when the source is already small. Larger sources and multi-source migrations (80+ repos) degrade because each plan is sequential, idempotent only at the per-source level, and lacks a structured decomposition.

The framework already pins a clean separation between *plan-time, shallow* (analyze) and *define-time, deep* (extract) work ([`legacy-migration-at-scale.md`](../docs/tutorials/legacy-migration-at-scale.md)). This RFC adds the missing **decomposition** primitive on top of analyze, and adds the structured architectural input that anchors it.

## Design

### Principles

1. **Migration planning, not spec extraction.** The surveyor reads enough of the system to decompose it; it does not extract aggregate-level domain logic. Specs remain the responsibility of `/spec:extract` and `/spec:define`, invoked at slice-implementation time.
2. **Top-down decomposition with an explicit stopping criterion.** The system is decomposed iteratively, level by level, until every leaf is T-shirt: Small (or smaller). Decomposition halts at the first level that satisfies the rubric — there is no over-decomposition.
3. **DAG, not tree.** Shared subcomponents (a common auth library, a shared schema crate) appear once and are referenced from multiple parents. The migration order falls out of the DAG's topological sort.
4. **Capability-owned, framework-shaped.** The new kind, the survey brief, and the synthesise brief are framework concerns; per-capability prompts (clustering heuristics, refined sizing rubrics, reconciliation algorithms) live in `plugins/change/skills/plan/briefs/<capability>/`.
5. **Closed enums stay closed.** Adding `domain-model` to the kind enum is an explicit change, not an open extension point. Unknown kinds remain a hard exit.
6. **Idempotency is non-negotiable.** Every new artifact (`survey.md`, `## Reconciliation`, `## Domain model`) must be byte-stable on unchanged inputs, with sorted ordering, fixed field order, and no host-state leaks.
7. **Read-only with respect to `plan.yaml`.** Survey and synthesise emit Markdown under `.specify/plans/<change>/`; neither writes to `plan.yaml`. The propose brief still owns slice creation through `specify change plan add`.
8. **Composition only.** Where possible, new behaviour is a brief layered on existing skill primitives. New CLI verbs are introduced only when no existing primitive fits.
9. **One change at a time.** Multi-plan output, parallel changes, and cross-change state are RFC-21 (catalogue + cache) and RFC-22 (ledger + mapping) concerns.

### `domain-model` as a third closed-enum kind for `/change:analyze`

Today the kind enum is `{legacy-code, documentation}` ([`/change:analyze` SKILL.md](../plugins/change/skills/analyze/SKILL.md)). This RFC adds `domain-model`. The enum becomes `{legacy-code, documentation, domain-model}` and remains hard-closed; unknown values are still a non-zero exit before any partial write.

The new kind branches `/change:analyze` into a third path that:

- Reads a structured YAML or JSON document (the domain model) at `$INPUT_PATH`.
- Validates it against the schema below.
- Emits two pieces of output:
  1. A `## Domain model` section appended to `<output-dir>/discovery.md` (under the existing `## Capability inventory` wrapper) — see *Discovery section shape* below.
  2. A structural sidecar at `<plan-dir>/analyze/<source-key>/domain-model.json` — the parsed, byte-canonicalised model, ready for the survey brief to consume without re-parsing the source.

The clustering and extraction prompts for the new branch live alongside the existing two in `plugins/change/skills/plan/briefs/<capability>/analyze.md`, so capability-specific name-mapping conventions stay capability-owned.

### `domain-model` schema

A new sibling JSON Schema lives at `specify-cli/schemas/domain-model/schema.json`. The shape:

```yaml
version: 1
bounded_contexts:
  - name: billing
    description: Subscription billing, invoicing, and dunning.
    aggregates: [invoice, subscription, payment-method]
    owners: [team-billing]
    target_project: billing-svc       # optional routing hint
    sources: [legacy-billing]         # optional source-key hint
    ubiquitous_language:
      - term: invoice
        definition: A monetary obligation issued to a customer.
relationships:
  - upstream: billing
    downstream: notifications
    pattern: customer-supplier        # closed enum
```

Schema rules (`additionalProperties: false` everywhere, mirroring `plan.schema.json` and `registry.yaml` posture):

| Field | Required | Notes |
| --- | --- | --- |
| `version` | yes | `1` only; future bumps go through an RFC update. |
| `bounded_contexts[].name` | yes | Kebab-case, unique within the document. |
| `bounded_contexts[].description` | yes | Single-line free text. |
| `bounded_contexts[].aggregates` | no | Kebab-case; sorted alphabetically. |
| `bounded_contexts[].owners` | no | Kebab-case team identifiers; sorted. |
| `bounded_contexts[].target_project` | no | Kebab-case; matches `registry.yaml:projects[].name` when the registry exists. Routing hint, not a binding. |
| `bounded_contexts[].sources` | no | Kebab-case source-keys; matches the `--source <key>=…` namespace (or, with RFC-21, a key in `sources.yaml`). |
| `bounded_contexts[].ubiquitous_language` | no | Optional glossary; surfaces in propose for naming consistency checks. |
| `relationships[].pattern` | no | Closed enum: `customer-supplier`, `partnership`, `shared-kernel`, `conformist`, `anti-corruption-layer`, `published-language`, `open-host-service`, `separate-ways`. Standard DDD context-map vocabulary. |

The schema rejects unknown top-level keys, unknown bounded-context keys, and unknown relationship patterns. Validation is performed by a small library extension to the existing `specify-validate` crate; invalid input fails before any write.

### Sizing rubric

Every node in the decomposition DAG carries a T-shirt size computed from a framework-pinned rubric. The default thresholds are LOC-based on **production source only**, excluding tests, generated code, vendored dependencies, third-party imports, blank lines, and comment-only lines:

| Size | Production LOC | Plan-time meaning |
| --- | --- | --- |
| XS | `< 200` | Already slice-sized; promote directly. Often already extracted or trivially mappable. |
| **S** | `200–999` | **Slice-sized. STOP — emit as a leaf.** |
| M | `1000–4999` | Decompose further. |
| L | `5000–19999` | Decompose further. |
| XL | `≥ 20000` | Decompose further; expect ≥ 2 levels of decomposition remaining. |

A node is a **slice-candidate leaf** iff its size is XS or S. Internal (non-leaf) nodes are M or larger. The stopping criterion is uniform across the DAG: any node that measures XS or S becomes a leaf, regardless of depth.

Per-capability briefs MAY refine the rubric with additional constraints — for example, "Omnia slices are S iff LOC < 1000 *and* aggregate count ≤ 1 *and* external endpoint count ≤ 3." Refinements may only *tighten* the framework rubric (a node the framework calls S may be re-classified M by the capability), never loosen it. This guarantees that propose can rely on the leaf invariant: every input it receives is at most S by both framework and capability measures.

A small CLI helper `specify change plan size --path <dir> [--capability <name>]` computes the production-LOC count using a pinned set of language-aware include/exclude globs (framework defaults at `specify-cli/sizing.toml`; per-capability overrides at `plugins/change/skills/plan/briefs/<cap>/sizing.toml`). The survey loop calls it under the hood; operators can call it directly to audit sizing decisions. The globs and LOC counter are deterministic — re-running on unchanged inputs yields identical counts.

### Survey topology: the decomposition DAG

The system being surveyed is modelled as a directed acyclic graph:

- **Root**: a synthetic node representing the entire scope of the change (all sources, combined).
- **Internal nodes**: chunks at intermediate granularity (subsystem, module-cluster, package, folder). Sized M, L, or XL.
- **Leaf nodes**: slice-candidates. Sized XS or S.
- **Edges**: parent-child decomposition (`contains`).
- **Cross-edges**: shared-dependency back-references (`depends_on`). When two parents both contain the same chunk, the chunk appears once with two `depends_on_by` parents — it is **not** duplicated.

Each node carries:

| Attribute | Notes |
| --- | --- |
| `id` | Kebab-case, unique within the survey. Hierarchical (`identity.user-registration`) for readability; not parsed structurally. |
| `name` | Short human label. |
| `size` | `xs | s | m | l | xl`. |
| `loc` | Production LOC count, integer. |
| `sources` | `[source-key, ...]` the node spans. Multi-source nodes are first-class. |
| `bounded_context` | Optional, present when the node aligns with a documented bounded context. |
| `target_project` | Optional routing hint, inherited from the nearest ancestor (or bounded context) that carries one. |
| `capabilities` | `[capability-name, ...]` from the per-source analyze inventory that fall within this node. |
| `evidence` | Short prose explaining why this cut was chosen (e.g., "module boundary `src/billing/`", "bounded context `billing`", "import-graph cluster"). |
| `children` | `[node-id, ...]`, empty for leaves. |
| `depends_on` | `[node-id, ...]` cross-edges to shared chunks. |
| `depends_on_by` | `[node-id, ...]` reverse cross-edges. Populated by the framework from `depends_on`; emitted for shared leaves so the migration-order section is self-contained. |
| `unresolved` | `true` when no signal cleanly applies and the node still measures M or larger. Operator review required. |

Edges within each list are sorted by child `id` for byte-stability; nodes are emitted in topological order (root first, then BFS by depth, alphabetical within each depth band).

### Decomposition strategy

At each node, the surveyor decides whether to stop or to cut. The decision is driven, in priority order, by:

1. **Size check.** If the node is XS or S, stop. It is a leaf.
2. **Domain-model alignment.** If a documented bounded context covers exactly this node's source range, cut along its aggregates. Bounded contexts are the strongest cut signal because they encode explicit architectural intent.
3. **Structural boundaries.** If the node spans clearly separable top-level modules (top-level folders, crates, packages with low cross-import density), cut along those boundaries.
4. **Capability clusters.** Use the per-source analyze capability inventory: capabilities that co-occur within a module cluster but have low cross-module ties become natural cut candidates.
5. **Operator escape.** If no signal cleanly applies and the node is still M or larger, the surveyor records `unresolved: true` and defers the cut to operator review. The DAG is still emitted with the unresolved node marked.

Decomposition halts when every reachable leaf from the root is XS, S, or marked `unresolved`. The framework caps depth at **6**; an M+ node that remains M+ after 6 levels of cuts is fatal and triggers operator intervention. The cap exists to bound runtime and to flag pathological inputs early, not as a hard architectural limit.

### The `survey` brief

A new brief at `plugins/change/skills/plan/survey.md` runs as **step 3(b.5)** in the plan-skill loop — between sync-workspace (3b) and propose (3c). For inputs whose composite root is already XS or S (very small single-source migrations), the survey brief emits a one-node DAG and a one-line `Survey: root is already slice-sized (S, 743 LOC).` summary; everything else gets a full decomposition.

The brief is read-only with respect to `plan.yaml`. It reads:

- `discovery.md` (the canonical capability inventory, plus the `## Domain model` section if present);
- every `<plan-dir>/analyze/<key>/metadata.json` (per-source structural facts: module tree, import graph, LOC by path);
- every `<plan-dir>/analyze/<key>/domain-model.json` (parsed bounded-context blocks);
- `workspace.md` (when present — multi-project registries only);
- `registry.yaml` (for target project names and descriptions).

It writes a single artifact: `<plan-dir>/survey.md`. The shape is pinned for idempotency.

#### `survey.md` shape

```markdown
# Survey — <change-name>

## Summary

- Sources: 2 (`legacy-a`, `legacy-b`)
- Root size: XL (99,715 LOC across 2 sources)
- Decomposition depth: 3
- Leaves: 14 (12 S, 2 XS)
- Unresolved nodes: 0

## Source inventory

| Key | Kind | Language | LOC | Modules | Top-level | Capabilities |
| --- | --- | --- | --- | --- | --- | --- |
| legacy-a | legacy-code | typescript | 87,312 | 42 | src/auth, src/billing, src/users | 4 |
| legacy-b | legacy-code | typescript | 12,403 | 9 | src/auth, src/notify | 1 |
| arch.md | documentation | — | — | — | — | 5 |
| domain.yaml | domain-model | — | — | — | — | 3 contexts |

## DAG

### root [XL · 99,715 LOC]

- sources: [legacy-a, legacy-b]
- evidence: composite root spanning all sources in scope
- children: [billing, identity, notifications, shared-auth-lib]

### identity [M · 4,231 LOC]

- sources: [legacy-a, legacy-b]
- bounded_context: identity
- target_project: identity-svc
- capabilities: [email-verification, password-reset, user-registration]
- evidence: bounded context `identity` (domain-model) covers `legacy-a/src/auth/` and `legacy-b/src/auth/`
- children: [identity.email-verification, identity.password-reset, identity.user-registration]
- depends_on: [shared-auth-lib]

### identity.user-registration [S · 612 LOC] *(leaf)*

- sources: [legacy-a, legacy-b]
- bounded_context: identity
- target_project: identity-svc
- capabilities: [user-registration]
- evidence: capability `user-registration` co-occurs in legacy-a/src/auth/register.ts and legacy-b/src/auth/register.ts; consolidation candidate

### identity.email-verification [S · 487 LOC] *(leaf)*

- sources: [legacy-a]
- bounded_context: identity
- target_project: identity-svc
- capabilities: [email-verification]
- evidence: module `legacy-a/src/auth/verify/` is structurally isolated and within size

... (further sections, sorted alphabetically by id within each depth band) ...

### shared-auth-lib [XS · 184 LOC] *(leaf, shared)*

- sources: [legacy-a, legacy-b]
- target_project: identity-svc
- capabilities: [jwt-signing, token-validation]
- evidence: shared dependency referenced by [billing, identity]; consolidate as utility crate
- depends_on_by: [billing, identity]

## Migration order

Topological sort of leaves, dependencies-first:

1. shared-auth-lib (XS · identity-svc)
2. identity.user-registration (S · identity-svc)
3. identity.email-verification (S · identity-svc)
4. identity.password-reset (S · identity-svc)
5. billing.invoicing (S · billing-svc)
6. ...

## Slice candidates

| Leaf id | Size | LOC | Sources | Target project | Bounded context |
| --- | --- | --- | --- | --- | --- |
| billing.invoicing | S | 921 | [legacy-a] | billing-svc | billing |
| identity.email-verification | S | 487 | [legacy-a] | identity-svc | identity |
| identity.user-registration | S | 612 | [legacy-a, legacy-b] | identity-svc | identity |
| shared-auth-lib | XS | 184 | [legacy-a, legacy-b] | identity-svc | — |
| ... | | | | | |
```

Idempotency rules mirror `discovery.md`: alphabetical sort within every list/table, no timestamps, no absolute paths, no run IDs, no host-state leaks. Re-running survey on unchanged inputs produces a byte-identical `survey.md`. Field order inside each node block is fixed: `sources`, `bounded_context`, `target_project`, `capabilities`, `evidence`, `children`, `depends_on`, `depends_on_by`, `unresolved`.

The detailed clustering, cut-selection, and dependency-extraction algorithms are **capability-owned** and live in `plugins/change/skills/plan/briefs/<capability>/survey.md`. The framework brief pins:

- the input set;
- the sizing rubric (LOC defaults; capability refinements layer on);
- the node attributes and their field order;
- the decomposition strategy priority list;
- the byte-stable output contract.

### The `synthesise` brief

A new brief at `plugins/change/skills/plan/synthesise.md` runs as **step 3(b.6)** — immediately after survey, before propose. Its job is to reconcile *what the docs and domain model say* against *what the decomposition revealed*, and to emit a `## Reconciliation` block appended to `discovery.md`.

Reconciliation operates at the **leaf level** of the survey DAG. This is a deliberate choice: reconciling at intermediate nodes inflates noise (the same mismatch echoes up the tree), while leaf-level reconciliation surfaces concrete actionable mismatches per slice candidate.

Inputs:

- `discovery.md` (capability blocks, possibly tagged with `<!-- source-key: <k> -->`);
- the `## Domain model` section if present;
- `survey.md` (the DAG, particularly the leaf inventory);
- per-source `domain-model.json` and `metadata.json` sidecars.

Output: a single appended `## Reconciliation` section in `discovery.md`:

```markdown
## Reconciliation

### Documented but not found in code

- `tax-calculation` (sources: [arch.md]) — no leaf in the survey DAG covers this capability; flag for greenfield slice in propose.

### Found in code but not documented

- `legacy-promo-engine` (sources: [legacy-a], leaf: `billing.promo-engine`) — no documentation reference; flag for operator review.

### Confidence concordance

- `email-dispatch` (leaf: `notifications.email-dispatch`) — code: high, docs: high → confirmed.
- `user-registration` (leaf: `identity.user-registration`) — code: medium, docs: high → resolved confidence: high.

### Domain-model alignment

- `billing` bounded context resolves to leaves [billing.dunning, billing.invoicing, billing.refunds]. The domain model lists [invoicing, dunning]; `refunds` is undocumented.
- `identity` bounded context resolves to leaves [identity.email-verification, identity.password-reset, identity.user-registration]. All documented.

### Unresolved survey nodes

- (none)
```

Each subsection is omitted if empty. Within each subsection, entries are sorted alphabetically by their leading identifier. The reconciliation block is written once per run; an `extend` mode rewrites it from current state.

The reconciliation prompt is **capability-owned** and lives in `plugins/change/skills/plan/briefs/<capability>/synthesise.md`. The framework brief pins the section structure and idempotency rules.

### Discovery section shape

`/change:analyze --kind domain-model` appends one well-defined block per bounded context to `discovery.md`, alphabetically sorted by `name`, under a stable `## Domain model` heading. The discovery brief writes the heading once before invoking analyze (analogous to the existing `## Capability inventory` wrapper).

````markdown
## Domain model

<!-- source-key: <k> -->
### bounded-context: billing

```yaml
description: Subscription billing, invoicing, and dunning.
aggregates: [invoice, payment-method, subscription]
owners: [team-billing]
target_project: billing-svc
sources: [legacy-billing]
relationships:
  - downstream: notifications
    pattern: customer-supplier
```
````

Field order inside the YAML block is fixed: `description`, `aggregates`, `owners`, `target_project`, `sources`, `relationships`. Lists are sorted alphabetically. The `relationships` array within a context lists only the relationships *originating* from that context (deduplication by upstream).

### Pipeline ordering

The plan skill's brief pipeline (steps 3a–3d) becomes:

| Step | Today | After RFC-20 |
| --- | --- | --- |
| 3(a) | Discovery → `/change:analyze` per input → `discovery.md` | unchanged contract; new `domain-model` kind dispatches a third branch |
| 3(b) | Sync workspace (multi-repo only) → `workspace.md` | unchanged |
| **3(b.5)** | — | **Survey → top-down DAG decomposition → `survey.md`** |
| **3(b.6)** | — | **Synthesise → appends `## Reconciliation` to `discovery.md`, keyed by survey leaves** |
| 3(c) | Propose → `specify change plan add` per accepted slice | propose brief consumes `survey.md` leaves as the candidate slice set |
| 3(d) | Assignment (multi-repo only) → `specify change plan amend --project` | unchanged contract; assignment reads `target_project` hints from survey nodes |

Steps 1, 2, 4 (validate), and 5 (hand-off) are unchanged. Re-entry of the orchestration mode treats the new artifacts as part of the plan-time scratch; they live under `.specify/plans/<change>/` and are swept by `specify change plan archive`.

`/change:analyze` remains a single fan-out at 3(a). The survey brief does **not** re-invoke analyze at deeper levels. If a particular cut requires finer-grained capability inventory than the level-0 analyze produced, the surveyor records `unresolved: true` on the parent node and defers to the operator; re-running analyze with refined `--source` scoping is an operator decision, not a survey-internal loop. This preserves analyze's per-source idempotency contract.

### Routing hint precedence

When assignment infers a target project for a plan entry (a slice fed from a survey leaf), the new precedence is:

1. **Survey leaf's `target_project`** — inherited from the nearest ancestor (or bounded context) that carries one. The hardest hint; surfaced verbatim in the assignment table's `Rationale` column.
2. **Explicit `target_project` in a domain-model bounded context** that the leaf does not yet have attributed (rare; happens when the leaf spans contexts). Strong hint.
3. **Description match** (today's primary signal). Existing.
4. **Baseline spec affinity** (today's secondary signal). Existing.
5. **Capability compatibility** (today's tiebreaker). Existing.
6. **Ambiguity → human.** Existing.

The first two hints are surfaced in the assignment table's `Rationale` column verbatim, so the operator can audit the routing decision against the architectural intent.

### CLI surface

This RFC adds **no new top-level CLI verbs**. All existing primitives compose:

- `/change:analyze` gains a third kind branch — same positional arity (`<input-path> <output-dir> <kind> [source-key]`).
- `specify change create` (the merged brief + plan scaffold) is unchanged by this RFC.
- `specify change plan add` is unchanged.
- `specify change plan validate` is unchanged.

One new internal subcommand under the existing `plan` namespace:

- `specify change plan size --path <dir> [--capability <name>]` — production-LOC counter with per-capability include/exclude globs. Used by the survey loop; also useful for operators auditing sizing decisions. Pure read-only LOC counting, no state.

Skills change:

- `/change:analyze` SKILL.md updates: add the third kind to the closed enum table, add a `## Domain model` output contract section, add the structural sidecar shape for `domain-model.json`, update the guardrail list.
- `/change:plan` SKILL.md updates: add steps 3(b.5) and 3(b.6) to the Critical Path, add references to `survey.md` and `synthesise.md`, document the sizing rubric.
- New brief siblings: `plugins/change/skills/plan/survey.md`, `plugins/change/skills/plan/synthesise.md`.
- New per-capability brief siblings: `plugins/change/skills/plan/briefs/<cap>/survey.md`, `plugins/change/skills/plan/briefs/<cap>/synthesise.md`, optional `plugins/change/skills/plan/briefs/<cap>/sizing.toml`.

### Scenario coverage

| Scenario | Pre-RFC-20 | Post-RFC-20 |
| --- | --- | --- |
| 1. Legacy monolith → multi-target | Discovery clusters; topology proposal is code-driven only; operator decomposes by hand. | DAG decomposition produces slice-sized leaves directly; domain model drives bounded-context cuts; synthesise reconciles docs vs code per leaf. |
| 2. Multi-repo legacy migration | Sequential per-source analyze; no cross-source view; no migration order. | Survey synthesises across N sources, identifies shared dependencies, produces a topological migration order; consolidation/split candidates are explicit leaves. (Catalogue and tier-1 cache deferred to RFC-21; cross-change ledger and `mapping` field deferred to RFC-22.) |
| 3. Greenfield multi-repo | Topology proposal driven by capability clustering of docs. | Domain model seeds the DAG root; bounded contexts → projects mapping is explicit; the DAG is sparse but the same shape. |
| 4. Brownfield multi-repo | Routing via `workspace.md` baseline affinity. | Survey leaves carry inherited `target_project` hints; domain-model overrides cleanly when bounded contexts cross baseline boundaries. |

## Implementation Plan

1. **Schema and validator.** Add `specify-cli/schemas/domain-model/schema.json` and `specify-cli/schemas/domain-model/README.md`. Extend `specify-validate` with a domain-model validator. Add unit tests covering required fields, kebab-case constraints, the relationship-pattern enum, and `additionalProperties: false`.
2. **Sizing helper.** Add `specify change plan size` CLI subcommand with framework-default include/exclude globs at `specify-cli/sizing.toml` and a `--capability` flag that loads `briefs/<cap>/sizing.toml`. Pin the LOC counter algorithm (line-based, language-aware blank/comment skipping). v1 languages: TypeScript, JavaScript, Python, Rust, Go. Add fixtures asserting deterministic counts on sample trees.
3. **Closed-enum extension in `/change:analyze`.** Update the SKILL.md kind enum, the per-capability `analyze.md` brief structure, and the `--kind` validation in any helper that enforces the enum. Land first as a stub branch that errors with `domain-model-not-yet-implemented` so operators get a stable diagnostic.
4. **Domain-model branch in the per-capability brief.** Implement the Omnia per-capability variant first (`plugins/change/skills/plan/briefs/omnia/analyze.md`), with fixtures under `plugins/change/skills/plan/briefs/omnia/fixtures/analyze/domain-model/`. Vectis variant follows.
5. **Discovery section shape.** Extend the discovery brief and the `omnia/discovery.md` brief to write the `## Domain model` heading wrapper. Pin a fixture for the byte-stable shape.
6. **Survey brief.** Land `plugins/change/skills/plan/survey.md` (framework-level) with the DAG schema, decomposition strategy priority list, and output shape. Land `plugins/change/skills/plan/briefs/omnia/survey.md` (capability-owned) with the Omnia clustering heuristics. Fixtures: root-already-S no-op, single-source XL decomposition, multi-source consolidation, multi-source split with shared dependency, mixed-input with domain model, unresolved-node escape.
7. **Synthesise brief.** Land `plugins/change/skills/plan/synthesise.md` and `plugins/change/skills/plan/briefs/omnia/synthesise.md`. Fixtures: docs-only, code-only, mixed concordance, unresolved-node carry-through.
8. **Pipeline wiring.** Update `/change:plan` SKILL.md and `references/runbook.md` to add steps 3(b.5) and 3(b.6). Update orchestration verb-hygiene table. Document the sizing rubric and the leaf-invariant contract between survey and propose.
9. **Routing-hint precedence.** Update `assignment.md` to document the new precedence order and the survey-leaf `target_project` surface. Land fixtures showing routing rationale text quoting survey nodes.
10. **Tutorials.** Add `docs/tutorials/decomposing-a-monolith.md` walking Scenarios 1 and 2 through the survey DAG. Update `docs/tutorials/legacy-migration-at-scale.md` to cite the survey brief instead of describing manual decomposition.
11. **Acceptance.** Extend the cross-repo Deno acceptance suite with a domain-model-driven greenfield fixture, a multi-source consolidation fixture with shared dependency, and an XL monolith decomposition fixture.

## Migration

This RFC is **additive**. Every existing `/change:analyze` invocation, `discovery.md`, and `plan.yaml` continues to work without change.

For operators:

- Continue using `kind: documentation` for unstructured architecture docs. Promote to `kind: domain-model` only when the input is a structured YAML/JSON document conforming to the schema.
- A run with a non-trivial codebase now produces a `survey.md` automatically. For very small single-source migrations, `survey.md` is a one-node DAG with a one-line summary.
- The propose phase still drives accept/edit/reject per slice — survey emits *candidates*, not commitments.
- The new sizing rubric makes "is this slice too big?" a first-class check rather than operator intuition. Use `specify change plan size` to audit any node by hand.

For capability authors:

- Add a `## Domain model` branch to the per-capability `analyze.md` brief.
- Add a `survey.md` and `synthesise.md` brief under the capability's brief directory; reference the framework fixtures.
- Optionally add a `sizing.toml` to tighten the framework default sizing rubric (e.g., layer aggregate-count or endpoint-count constraints on top of LOC).

For skill authors consuming planning artifacts:

- `survey.md` is a new artifact under `.specify/plans/<change>/`; the schema is pinned in this RFC. Treat its byte-stable contract the same as `discovery.md`.
- The `## Reconciliation` section is appended to `discovery.md`; existing parsers must tolerate the new section.

There is **no breaking change** to the closed-kind enum's validation behaviour: unknown kinds are still hard exits. Adding `domain-model` is a deliberate, RFC-driven change to the enum.

## Alternatives Considered

**Keep survey as a single-pass cross-source synthesis step (flat tables of source inventory, capability overlap, and mapping recommendations).** Rejected. Flat tables conflate "what's in the system" with "how do we slice it" and leave the operator to decompose by hand. The DAG decomposition makes the decomposition itself the deliverable, with an explicit stopping criterion that propose can rely on.

**Decompose into a tree, not a DAG.** Rejected. Real legacy systems have shared subcomponents (auth libraries, schema crates, utility packages) that genuinely belong to multiple parents. Forcing a tree shape either duplicates these subcomponents (inflating LOC counts and migration effort estimates) or arbitrarily attributes them to one parent (losing migration-order information).

**Recursive `/change:analyze` calls driven by the surveyor (analyze-inside-survey).** Rejected for v1. It would couple two skills tightly, blur the per-source idempotency contract, and complicate caching. The level-0 analyze fan-out plus structural metadata is sufficient evidence for the DAG decomposition in the common case; the `unresolved` escape covers the rest.

**Capability-defined sizing rubrics with no framework default.** Rejected. Without a framework anchor, different capabilities produce non-comparable DAGs, and propose cannot rely on the "every input is at most S" invariant. Capabilities may *tighten* the framework rubric, not replace it.

**Non-LOC sizing primitive (aggregates, endpoints, story-points).** Considered. LOC is a coarse proxy but is universally measurable, deterministic, and language-agnostic enough to anchor the rubric. Capability-side refinements (`sizing.toml`) are the place to layer richer signals. Revisit only if LOC produces persistently wrong sizing across capabilities.

**Land domain models as opaque `documentation` and infer structure from prose.** Rejected. Loses the schema's audit value, defeats `target_project` routing hints, and mixes structured architectural intent with free-form prose in the same `## Capability inventory` block.

**Add the survey logic to `discovery.md` directly.** Rejected. Discovery is per-input by design; smearing decomposition into it conflates two responsibilities and breaks the per-source idempotency contract.

**Make `survey.md` a sibling of `plan.yaml` rather than `.specify/plans/<change>/`.** Rejected. The survey is per-change scratch and should archive with the rest of the plan-time tier-1 state. Promoting it to a top-level artifact would imply cross-change durability that this RFC does not provide (RFC-21 adds the durable source catalogue and tier-1 cache; RFC-22 adds the cumulative migration ledger).

**Promote `survey` to a top-level `/spec:survey` skill in this RFC.** Deferred. The brief-first approach lets us validate the artifact shape and capability-owned algorithms before committing to a slash-command surface. A future RFC can promote it once demand is clear.

**Embed the `## Reconciliation` block in `survey.md` instead of `discovery.md`.** Rejected. Reconciliation is the natural closing section of the discovery inventory — both belong to the same logical document. Survey is structurally different (DAG, decomposition-oriented).

**Reconcile at every internal node, not just leaves.** Rejected. The same mismatch echoes up the tree, inflating noise. Leaf-level reconciliation surfaces concrete actionable mismatches per slice candidate; internal-node reconciliation can be added later if operators report missing context.

**Add a CLI verb `specify change plan survey`.** Rejected for v1. Brief-driven composition reuses the existing plan-skill orchestration shell; a CLI verb would invent new state-transition ownership outside the single-writer invariant. The `specify change plan size` helper is a different shape — pure read-only LOC counting, no state.

## Non-Goals

- **Spec extraction from legacy code.** Survey is migration-planning, not spec authorship. `/spec:extract` and `/spec:define` remain the spec entry points, invoked at slice-implementation time. The DAG records *which* aggregates live where; it does not capture their behaviours, invariants, or schemas.
- A source-repo catalogue (covered by RFC-21).
- Tier-1 clone caching beyond the current per-change scope (covered by RFC-21).
- Cross-change durable state — the migration ledger (covered by RFC-22).
- A `mapping` field on plan slices (covered by RFC-22).
- Replacing the propose accept/edit/reject loop with automated decisions. Survey emits candidates; the operator commits them.
- Replacing operator review of `discovery.md` with model-driven judgement.
- A general "context map import" workflow from external DDD tools (out of scope; the schema is small enough for hand authoring or a thin importer in a future RFC).
- Runtime enforcement of bounded-context boundaries in generated code (a runtime concern, not a planning concern).
- Multi-plan output or parallel changes.

## Open Questions

1. **Capability-specific sizing refinements.** The framework rubric is LOC-only. Concrete capabilities (Omnia, Vectis) may want to layer aggregate-count or endpoint-count constraints. Should `briefs/<cap>/sizing.toml` be required for every capability, or only when refinement is needed? Current preference: optional; the framework LOC default applies when the file is absent.
2. **LOC threshold calibration.** S = 200–999 LOC of production source is the proposed stopping band. Is `<1000` the right ceiling for "small enough to migrate as one slice" across the capabilities we ship? Current preference: yes, with refinement via per-capability `sizing.toml` when one capability's "small" reliably runs hotter or cooler.
3. **LOC counter language coverage.** v1 supports TypeScript, JavaScript, Python, Rust, and Go. Additional languages land as the counter is extended; survey can degrade to raw line counts with a `language-not-supported` evidence tag. Should the surveyor refuse to size unsupported-language nodes, or fall back to raw counts with a warning? Current preference: fall back, mark `evidence: language-not-supported, raw-LOC`.
4. **Maximum decomposition depth.** Currently capped at 6. Is that the right ceiling for 100k+ LOC monoliths, or do we need it configurable? Current preference: fixed at 6, with the `unresolved` escape catching pathological cases.
5. **Per-source vs whole-system root.** The DAG root spans all sources in scope. Should multi-source migrations get one root per source plus a synthetic super-root, or a single composite root? Current preference: single composite root, with `sources` per node disambiguating attribution.
6. **Shared dependencies — cross-edge vs duplicated leaf.** When `shared-auth-lib` is depended on by two subsystems, this RFC picks DAG (one leaf, two `depends_on_by`). Revisit if the operator burden of tracking shared leaves outweighs the migration-order benefit.
7. **Should `survey.md` emit JSON alongside Markdown for tooling?** Current preference: defer. The Markdown DAG is parseable enough for v1; a JSON sidecar can land in a follow-up if downstream skills demand it.
8. **Reconciliation at internal nodes vs leaves only.** Current preference: leaves only. Internal-node reconciliation inflates noise. Revisit if operators report missing context.
9. **`--dry-run` behaviour for the new briefs.** Current preference: print survey DAG summary and reconciliation previews to stdout; do not write files under `.specify/plans/`.

## References

- [RFC-3a: Monoliths](archive/rfc-3a-monoliths.md) — the per-source vs per-slice analyze/extract split this RFC builds on.
- [RFC-3b: Platform](archive/rfc-3b-platform.md) — assignment, registry, and one-slice-one-project invariant.
- [RFC-9: Platform](archive/rfc-9-platform.md) — orchestration umbrella and shape inference.
- [RFC-13: Extensibility](archive/rfc-13-extensibility.md) — capability-owned briefs and pipeline composition.
- [`/change:analyze` SKILL.md](../plugins/change/skills/analyze/SKILL.md) — the per-source analyze contract this RFC extends.
- [`/change:plan` SKILL.md](../plugins/change/skills/plan/SKILL.md) — the plan-skill loop this RFC inserts steps 3(b.5) and 3(b.6) into.
- [`docs/explanation/workspace-tiers.md`](../docs/explanation/workspace-tiers.md) — the tier-1 vs tier-2 boundary survey/synthesise sit inside.
- [`docs/tutorials/legacy-migration-at-scale.md`](../docs/tutorials/legacy-migration-at-scale.md) — the canonical Scenario 1+2 walkthrough this RFC updates.
