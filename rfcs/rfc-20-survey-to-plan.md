# RFC-20: Survey-to-Plan Pipeline

> Status: Draft - Depends: [RFC-3a](archive/rfc-3a-monoliths.md), [RFC-3b](archive/rfc-3b-platform.md), [RFC-9](archive/rfc-9-platform.md), [RFC-13](archive/rfc-13-extensibility.md)

## Abstract

Extend the `/change:plan` brief pipeline so it can ingest **architectural inputs as first-class structured data**, **synthesise** them against per-source analysis, and produce **machine-readable cross-source survey outputs** that drive slice decomposition and project routing.

Concretely, this RFC adds:

1. A new closed-enum kind `domain-model` for `/spec:analyze`, with a pinned schema for bounded contexts, aggregates, ownership, and routing hints.
2. A new `survey` brief, run between discovery (3a) and propose (3c), that fans `/spec:analyze` across many inputs in parallel and emits `survey.md` — a synthesised, byte-stable inventory of source structure, capability overlaps, domain-model alignment, and consolidation/split candidates.
3. A new `synthesise` brief, run between survey and propose, that reconciles documented capabilities against discovered code capabilities and emits a `## Reconciliation` block in `discovery.md`.

The plan skill's five-step loop, the single-writer invariant for `plan.yaml`, and the closed-kind enum's strict validation posture are preserved. The four scenarios this RFC unblocks — single-repo legacy migration, multi-repo legacy migration, greenfield multi-repo, and brownfield multi-repo — share the same pipeline; only the inputs differ.

## Motivation

The framework already supports per-source capability extraction (`/spec:analyze`), per-input dispatch (the discovery brief), and multi-project routing (assignment + greenfield-registry-bootstrap). What is missing is the connective tissue between *what the architecture says should exist* and *what the planning loop actually proposes*:

- **No structured architectural input.** Domain models, context maps, EventStorming exports, and design docs all land as opaque `kind: documentation`. Bounded contexts, aggregates, and ownership are not extracted into discrete, queryable form. The greenfield-registry-bootstrap clustering algorithm therefore cannot key on architectural intent — it is driven by capability inference alone.
- **No cross-source synthesis stage.** `/spec:analyze` is per-source by design. Today's discovery brief loops over inputs sequentially and merges by capability name, but it does not reason about *cross-source overlap*, *consolidation candidates*, or *split candidates*. That reasoning falls to the operator reading `discovery.md` line by line.
- **No reconciliation step.** When the docs claim capability X exists and the code shows capability Y, today's pipeline appends both with their respective confidences and lets the operator notice the mismatch in the propose review. There is no first-class reconciliation block that flags "documented but not in code", "in code but undocumented", or "evidence concordance".
- **Scenario coverage is uneven.** Single-source migrations work cleanly. Multi-source migrations (80+ repos) work mechanically but degrade as N grows because each plan is sequential, idempotent only at the per-source level, and lacks a synthesised view. Greenfield multi-repo planning lacks any structured architectural anchor, so the topology proposal is implicit in capability clustering.

The framework already pins a clean separation between *plan-time, shallow* (analyze) and *define-time, deep* (extract) work ([`legacy-migration-at-scale.md`](../docs/tutorials/legacy-migration-at-scale.md)). This RFC adds an analogous separation between *per-input* and *cross-input* reasoning at plan time, and it adds the missing structured input type.

## Design

### Principles

1. **Capability-owned, framework-shaped.** The new kind, survey, and synthesise briefs are framework concerns; per-capability prompts (clustering algorithms, reconciliation heuristics) live in `plugins/change/skills/plan/briefs/<capability>/`.
2. **Closed enums stay closed.** Adding `domain-model` to the kind enum is an explicit change, not an open extension point. Unknown kinds remain a hard exit.
3. **Idempotency is non-negotiable.** Every new artifact (`survey.md`, the `## Reconciliation` block, the `## Domain model` section) must be byte-stable on unchanged inputs, with sorted ordering and no host-state leaks.
4. **Read-only with respect to `plan.yaml`.** Survey and synthesise emit Markdown under `.specify/plans/<change>/`; neither writes to `plan.yaml`. The propose brief still owns slice creation through `specify change plan add`.
5. **Composition only.** Where possible, new behaviour is a brief layered on existing skill primitives. New CLI verbs are introduced only when no existing primitive fits.
6. **One change at a time.** This RFC does not introduce multi-plan output, parallel changes, or cross-change state — those are RFC-21 concerns.

### `domain-model` as a third closed-enum kind for `/spec:analyze`

Today the kind enum is `{legacy-code, documentation}` ([`/spec:analyze` SKILL.md](../plugins/spec/skills/analyze/SKILL.md)). This RFC adds `domain-model`. The enum becomes `{legacy-code, documentation, domain-model}` and remains hard-closed; unknown values are still a non-zero exit before any partial write.

The new kind branches `/spec:analyze` into a third path that:

- Reads a structured YAML or JSON document (the domain model) at `$INPUT_PATH`.
- Validates it against the schema below.
- Emits two pieces of output:
  1. A `## Domain model` section appended to `<output-dir>/discovery.md` (under the existing `## Capability inventory` wrapper) — see *Discovery section shape* below.
  2. A structural sidecar at `<plan-dir>/analyze/<source-key>/domain-model.json` — the parsed, byte-canonicalised model, ready for downstream briefs to consume without re-parsing the source.

The clustering and extraction prompts for the new branch live alongside the existing two in `plugins/change/skills/plan/briefs/<capability>/analyze.md`, so capability-specific name-mapping conventions (e.g., kebab-cased crate names per capability) stay capability-owned.

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

| Field                                    | Required | Notes                                                                                                                                                                                                     |
| ---------------------------------------- | -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `version`                                | yes      | `1` only; future bumps go through an RFC update.                                                                                                                                                          |
| `bounded_contexts[].name`                | yes      | Kebab-case, unique within the document.                                                                                                                                                                   |
| `bounded_contexts[].description`         | yes      | Single-line free text.                                                                                                                                                                                    |
| `bounded_contexts[].aggregates`          | no       | Kebab-case; sorted alphabetically.                                                                                                                                                                        |
| `bounded_contexts[].owners`              | no       | Kebab-case team identifiers; sorted.                                                                                                                                                                      |
| `bounded_contexts[].target_project`      | no       | Kebab-case; matches `registry.yaml:projects[].name` when the registry exists. Routing hint, not a binding.                                                                                                |
| `bounded_contexts[].sources`             | no       | Kebab-case source-keys; matches the `--source <key>=…` namespace (or, with RFC-21, a key in `sources.yaml`).                                                                                              |
| `bounded_contexts[].ubiquitous_language` | no       | Optional glossary; surfaces in propose for naming consistency checks.                                                                                                                                     |
| `relationships[].pattern`                | no       | Closed enum: `customer-supplier`, `partnership`, `shared-kernel`, `conformist`, `anti-corruption-layer`, `published-language`, `open-host-service`, `separate-ways`. Standard DDD context-map vocabulary. |

The schema rejects unknown top-level keys, unknown bounded-context keys, and unknown relationship patterns. Validation is performed by a small library extension to the existing `specify-validate` crate; invalid input fails before any write.

### Discovery section shape

`/spec:analyze --kind domain-model` appends one well-defined block per bounded context to `discovery.md`, alphabetically sorted by `name`, under a stable `## Domain model` heading. The discovery brief writes the heading once before invoking analyze (analogous to the existing `## Capability inventory` wrapper).

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

### The `survey` brief

A new brief at `plugins/change/skills/plan/survey.md` runs as **step 3(b.5)** in the plan-skill loop — between sync-peers (3b) and propose (3c). For single-source single-repo runs, the survey brief is a no-op; it returns immediately and writes no `survey.md`. The trigger is **two or more `/spec:analyze` invocations during step 3(a)** *or* **a domain model present in any of the inputs**.

The brief is read-only with respect to `plan.yaml`. It reads:

- `discovery.md` (the canonical capability inventory, plus the new `## Domain model` section if present);
- every `<plan-dir>/analyze/<key>/metadata.json` (per-source structural facts);
- every `<plan-dir>/analyze/<key>/domain-model.json` (parsed bounded-context blocks);
- `workspace.md` (when present — multi-project registries only);
- `registry.yaml` (for target project names and descriptions).

It writes a single artifact: `<plan-dir>/survey.md`. The shape is pinned for idempotency:

```markdown
# Survey — <change-name>

## Source inventory

| Key         | Kind          | Language   | LOC    | Modules | Top-level                        | Capabilities |
| ----------- | ------------- | ---------- | ------ | ------- | -------------------------------- | ------------ |
| legacy-a    | legacy-code   | typescript | 87,312 | 42      | src/auth, src/billing, src/users | 4            |
| legacy-b    | legacy-code   | typescript | 12,403 | 9       | src/notify                       | 1            |
| arch.md     | documentation | —          | —      | —       | —                                | 5            |
| domain.yaml | domain-model  | —          | —      | —       | —                                | 6 contexts   |

## Cross-source capability overlap

| Capability        | Sources              | Confidence | Pattern                 | Recommendation                                          |
| ----------------- | -------------------- | ---------- | ----------------------- | ------------------------------------------------------- |
| user-registration | [legacy-a, legacy-b] | medium     | candidate-consolidation | propose single slice with sources: [legacy-a, legacy-b] |
| email-dispatch    | [legacy-b]           | high       | one-to-one              | propose single slice with sources: [legacy-b]           |

## Domain-model alignment

| Bounded context | Documented capabilities                 | Source capabilities                     | Proposed target | Confidence |
| --------------- | --------------------------------------- | --------------------------------------- | --------------- | ---------- |
| billing         | [invoicing, dunning]                    | [invoicing, dunning, refunds]           | billing-svc     | high       |
| identity        | [user-registration, email-verification] | [user-registration, email-verification] | identity-svc    | high       |

## Mapping recommendations

- legacy-a ∪ legacy-b → identity-svc (consolidation; rationale: `user-registration` and `email-verification` co-occur across both).
- legacy-a → billing-svc + identity-svc (split; rationale: bounded contexts `billing` and `identity` both present in `legacy-a`).
```

Idempotency rules mirror `discovery.md`: alphabetical sort within every list/table, no timestamps, no absolute paths, no run IDs, no host-state leaks. Re-running survey on unchanged inputs produces a byte-identical `survey.md`.

The detailed cross-source clustering, overlap detection, and recommendation algorithms are **capability-owned** and live in `plugins/change/skills/plan/briefs/<capability>/survey.md`. The framework brief pins:

- the input set;
- the table headings and column order;
- the recommendation vocabulary (`one-to-one`, `candidate-consolidation`, `candidate-split`, `greenfield`);
- the byte-stable output contract.

### The `synthesise` brief

A new brief at `plugins/change/skills/plan/synthesise.md` runs as **step 3(b.6)** — immediately after survey, before propose. Its job is to reconcile *what the docs and domain model say* against *what the code shows*, and to emit a `## Reconciliation` block appended to `discovery.md`.

Inputs:

- `discovery.md` (capability blocks, possibly tagged with `<!-- source-key: <k> -->`);
- the `## Domain model` section if present;
- per-source `domain-model.json` and `metadata.json` sidecars.

Output: a single appended `## Reconciliation` section in `discovery.md`, structured as follows:

```markdown
## Reconciliation

### Documented but not found in code

- `tax-calculation` (sources: [arch.md]) — no source capability with this name; flag for greenfield slice.

### Found in code but not documented

- `legacy-promo-engine` (sources: [legacy-a]) — no documentation reference; flag for review.

### Confidence concordance

- `user-registration` — code: medium, docs: high → resolved confidence: high.
- `email-dispatch` — code: high, docs: high → confirmed.

### Domain-model alignment

- `billing` bounded context aligns with capabilities [invoicing, dunning, refunds] in source `legacy-a`. The domain model lists [invoicing, dunning]; `refunds` is undocumented.
```

Each subsection is omitted if empty. Within each subsection, entries are sorted alphabetically by their leading identifier. The reconciliation block is written once per run; an `extend` mode rewrites it from current state.

The reconciliation prompt is **capability-owned** and lives in `plugins/change/skills/plan/briefs/<capability>/synthesise.md`. The framework brief pins the section structure and idempotency rules.

### Pipeline ordering

The plan skill's brief pipeline (steps 3a–3d) becomes:

| Step       | Today                                                                | After RFC-20                                                                                                  |
| ---------- | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| 3(a)       | Discovery → `/spec:analyze` per input → `discovery.md`               | unchanged; new `domain-model` kind dispatches a third branch                                                  |
| 3(b)       | Sync peers (multi-repo only) → `workspace.md`                        | unchanged                                                                                                     |
| **3(b.5)** | —                                                                    | **Survey → `survey.md`** (when ≥2 inputs *or* domain model present)                                           |
| **3(b.6)** | —                                                                    | **Synthesise → appends `## Reconciliation` to `discovery.md`**                                                |
| 3(c)       | Propose → `specify change plan add` per accepted slice               | unchanged contract; propose brief gains read access to `survey.md`                                            |
| 3(d)       | Assignment (multi-repo only) → `specify change plan amend --project` | unchanged contract; assignment brief gains read access to `survey.md` and `## Domain model` for routing hints |

Steps 1, 2, 4 (validate), and 5 (hand-off) are unchanged. Re-entry of the orchestration mode treats the new artifacts as part of the plan-time scratch; they live under `.specify/plans/<change>/` and are swept by `specify change plan archive`.

### Routing hint precedence

When assignment infers a target project for a plan entry, the new precedence is:

1. **Explicit `target_project` in a domain-model bounded context** that matches the entry's bounded-context tag (when survey could attribute the entry to one context). Hard hint.
2. **Survey's mapping recommendation** for the entry's source(s). Strong hint.
3. **Description match** (today's primary signal). Existing.
4. **Baseline spec affinity** (today's secondary signal). Existing.
5. **Capability compatibility** (today's tiebreaker). Existing.
6. **Ambiguity → human.** Existing.

The first two hints are surfaced in the assignment table's `Rationale` column verbatim, so the operator can audit the routing decision against the architectural intent.

### CLI surface

This RFC adds **no new CLI verbs**. All existing primitives compose:

- `/spec:analyze` gains a third kind branch — same positional arity (`<input-path> <output-dir> <kind> [source-key]`).
- `specify change plan create` is unchanged.
- `specify change plan add` is unchanged.
- `specify change plan validate` is unchanged.

Skills change:

- `/spec:analyze` SKILL.md updates: add the third kind to the closed enum table, add a `## Domain model` output contract section, add the structural sidecar shape for `domain-model.json`, update the guardrail list.
- `/change:plan` SKILL.md updates: add steps 3(b.5) and 3(b.6) to the Critical Path, add references to `survey.md` and `synthesise.md`.
- New brief siblings: `plugins/change/skills/plan/survey.md`, `plugins/change/skills/plan/synthesise.md`.
- New per-capability brief siblings: `plugins/change/skills/plan/briefs/<cap>/survey.md`, `plugins/change/skills/plan/briefs/<cap>/synthesise.md`.

### Scenario coverage

| Scenario                          | Pre-RFC-20                                                 | Post-RFC-20                                                                                                                                                                     |
| --------------------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Legacy monolith → multi-target | Discovery clusters; topology proposal is code-driven only. | Domain model can drive the `## Proposed registry topology` block; survey identifies splits within the monolith; synthesise reconciles docs vs code.                             |
| 2. Multi-repo legacy migration    | Sequential per-source analyze; no cross-source view.       | Survey synthesises across N sources; consolidation/split candidates surface explicitly; domain model anchors target-project routing. (Scale and durability deferred to RFC-21.) |
| 3. Greenfield multi-repo          | Topology proposal driven by capability clustering of docs. | Domain model directly seeds the topology; bounded contexts → projects mapping is explicit and reviewable.                                                                       |
| 4. Brownfield multi-repo          | Routing via `workspace.md` baseline affinity.              | Domain-model `target_project` hints take precedence; survey aligns new capabilities with bounded contexts before assignment.                                                    |

## Implementation Plan

1. **Schema and validator.** Add `specify-cli/schemas/domain-model/schema.json` and `specify-cli/schemas/domain-model/README.md`. Extend `specify-validate` with a domain-model validator. Add unit tests covering required fields, kebab-case constraints, the relationship-pattern enum, and `additionalProperties: false`.
2. **Closed-enum extension in `/spec:analyze`.** Update the SKILL.md kind enum, the per-capability `analyze.md` brief structure, and the `--kind` validation in any helper that enforces the enum. Land first as a stub branch that errors with `domain-model-not-yet-implemented` so operators get a stable diagnostic.
3. **Domain-model branch in the per-capability brief.** Implement the Omnia per-capability variant first (`plugins/change/skills/plan/briefs/omnia/analyze.md`), with fixtures under `plugins/change/skills/plan/briefs/omnia/fixtures/analyze/domain-model/`. Vectis variant follows.
4. **Discovery section shape.** Extend the discovery brief and the `omnia/discovery.md` brief to write the `## Domain model` heading wrapper. Pin a fixture for the byte-stable shape.
5. **Survey brief.** Land `plugins/change/skills/plan/survey.md` (framework-level) and `plugins/change/skills/plan/briefs/omnia/survey.md` (capability-owned). Land fixtures: single-source no-op, multi-source consolidation, multi-source split, mixed-input with domain model.
6. **Synthesise brief.** Land `plugins/change/skills/plan/synthesise.md` and `plugins/change/skills/plan/briefs/omnia/synthesise.md`. Fixtures: docs-only, code-only, mixed concordance.
7. **Pipeline wiring.** Update `/change:plan` SKILL.md and `references/runbook.md` to add steps 3(b.5) and 3(b.6). Update orchestration verb-hygiene table.
8. **Routing-hint precedence.** Update `assignment.md` to document the new precedence order and the `target_project` hint surface. Land fixtures showing routing rationale text.
9. **Tutorials.** Add `docs/tutorials/domain-model-driven-greenfield.md` (Scenario 3) and update `docs/tutorials/legacy-migration-at-scale.md` to walk through survey + synthesise.
10. **Acceptance.** Extend the cross-repo Deno acceptance suite with a domain-model-driven greenfield fixture and a multi-source consolidation fixture.

## Migration

This RFC is **additive**. Every existing `/spec:analyze` invocation, `discovery.md`, and `plan.yaml` continues to work without change.

For operators:

- Continue using `kind: documentation` for unstructured architecture docs. Promote to `kind: domain-model` only when the input is a structured YAML/JSON document conforming to the schema.
- A run that supplies a domain model gains a `## Domain model` section in `discovery.md` and a `survey.md`; nothing else changes on disk.
- The propose phase still drives accept/edit/reject per slice — the new survey recommendations are advisory.

For capability authors:

- Add a `## Domain model` branch to the per-capability `analyze.md` brief.
- Add a `survey.md` and `synthesise.md` brief under the capability's brief directory. Reference fixtures in the framework brief.
- Existing single-source single-repo flows do not require capability-side changes; survey is a no-op there.

For skill authors consuming planning artifacts:

- `survey.md` is a new artifact under `.specify/plans/<change>/`; the schema is pinned in this RFC. Treat its byte-stable contract the same as `discovery.md`.
- The `## Reconciliation` section is appended to `discovery.md`; existing parsers must tolerate the new section.

There is **no breaking change** to the closed-kind enum's validation behaviour: unknown kinds are still hard exits. Adding `domain-model` is a deliberate, RFC-driven change to the enum.

## Alternatives Considered

**Land domain models as opaque `documentation` and infer structure from prose.** Rejected. Loses the schema's audit value, defeats `target_project` routing hints, and mixes structured architectural intent with free-form prose in the same `## Capability inventory` block.

**Add the survey logic to `discovery.md` directly.** Rejected. Discovery is per-input by design; smearing cross-input synthesis into it conflates two responsibilities and breaks the per-source idempotency contract. Survey is a separate brief precisely so cross-source reasoning has its own byte-stable output.

**Make `survey.md` a sibling of `plan.yaml` rather than `.specify/plans/<change>/`.** Rejected. The survey is per-change scratch and should archive with the rest of the plan-time tier-1 state. Promoting it to a top-level artifact would also imply cross-change durability that this RFC does not provide (RFC-21 handles durable cross-change state separately).

**Promote `survey` to a top-level `/spec:survey` skill in this RFC.** Deferred. The brief-first approach lets us validate the artifact shape and capability-owned algorithms before committing to a slash-command surface. A future RFC can promote it once demand is clear.

**Use `tracks: capabilities` in the survey brief frontmatter (per the brief schema).** Considered and rejected for v1. The survey iterates over sources and capabilities at a higher level than the brief schema's `tracks` field, which today targets per-task progress reporting. Revisit if the brief framework gains higher-level tracking.

**Embed the `## Reconciliation` block in `survey.md` instead of `discovery.md`.** Rejected. Reconciliation is the natural closing section of the discovery inventory — both belong to the same logical document. Survey is structurally different (cross-source, recommendation-oriented).

**Add a CLI verb `specify change plan survey`.** Rejected for v1. Brief-driven composition reuses the existing plan-skill orchestration shell; a CLI verb would invent new state-transition ownership outside the single-writer invariant.

## Non-Goals

- Cross-change durable state (covered by RFC-21).
- A source-repo catalogue (covered by RFC-21).
- Tier-1 clone caching beyond the current per-change scope (covered by RFC-21).
- A `mapping` field on plan slices (covered by RFC-21).
- Replacing the propose accept/edit/reject loop with automated decisions.
- Replacing operator review of `discovery.md` with model-driven judgement.
- A general "context map import" workflow from external DDD tools (out of scope; the schema is small enough for hand authoring or a thin importer in a future RFC).
- Runtime enforcement of bounded-context boundaries in generated code (a runtime concern, not a planning concern).
- Multi-plan output or parallel changes.

## Open Questions

1. Should `domain-model` be a third kind on `/spec:analyze`, or a sibling positional (`/spec:domain-model`)? The closed-enum approach is more conservative; a sibling skill would offer cleaner schema-validation diagnostics. Current preference: closed-enum extension, since it preserves the per-input dispatch contract.
2. Should `survey.md` be a no-op for single-source runs, or always emit a (trivial) survey for consistency? Current preference: no-op, with a one-line `Survey skipped — single source, no domain model.` comment in `discovery.md` for audit.
3. Should the relationship-pattern enum be the full DDD context-map vocabulary (eight values) or a reduced set? Current preference: full vocabulary, since the domain-model document is small and its consumers can ignore patterns they do not care about.
4. Should the domain-model document live alongside `change.md` (per-change) or under `.specify/` (per-platform)? Current preference: per-change input via `--source <key>=<path>:domain-model`, leaving the operator free to keep a durable copy elsewhere.
5. Should the per-capability `survey.md` brief be required, or should the framework provide a default capability-agnostic implementation? Current preference: capability-agnostic default; capabilities override only when their conventions diverge meaningfully.
6. Should the `## Reconciliation` section gate propose, or remain advisory? Current preference: advisory; gating would couple two phases that we have deliberately separated.
7. How should `--dry-run` behave for the new briefs? Current preference: print survey and reconciliation previews to stdout; do not write files under `.specify/plans/`.

## References

- [RFC-3a: Monoliths](archive/rfc-3a-monoliths.md) — the per-source vs per-slice analyze/extract split this RFC builds on.
- [RFC-3b: Platform](archive/rfc-3b-platform.md) — assignment, registry, and one-slice-one-project invariant.
- [RFC-9: Platform](archive/rfc-9-platform.md) — orchestration umbrella and shape inference.
- [RFC-13: Extensibility](archive/rfc-13-extensibility.md) — capability-owned briefs and pipeline composition.
- [`/spec:analyze` SKILL.md](../plugins/spec/skills/analyze/SKILL.md) — the per-source analyze contract this RFC extends.
- [`/change:plan` SKILL.md](../plugins/change/skills/plan/SKILL.md) — the plan-skill loop this RFC inserts steps 3(b.5) and 3(b.6) into.
- [`docs/explanation/workspace-tiers.md`](../docs/explanation/workspace-tiers.md) — the tier-1 vs tier-2 boundary survey/synthesise sit inside.
- [`docs/tutorials/legacy-migration-at-scale.md`](../docs/tutorials/legacy-migration-at-scale.md) — the canonical Scenario 1+2 walkthrough this RFC updates.