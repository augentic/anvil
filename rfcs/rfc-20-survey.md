# RFC-20 Survey to Plan

> Status: Alternative Draft - Compare with [RFC-20](rfc-20-survey.md) - Depends: [RFC-13](archive/rfc-13-extensibility.md), [RFC-23](archive/rfc-23-change-lifecycle.md)

## Abstract

Introduce a mechanical source-survey stage inside `/change:draft` so Specify can turn legacy code into a reviewable migration plan. The legacy input may be one large monolith, many repositories, or a mix of both. The survey decomposes code from the outside in: externally observable surfaces first, the source files those surfaces touch second, capability-shaped slice candidates last.

The goal is not to extract full specs. The goal is to answer one planning question before `propose` runs:

> What are the smallest coherent business capabilities we can migrate, and in what order?

This RFC focusses on the `/change:draft` analysis process. Detailed schemas, detector catalogues, and future reconciliation features are deliberately secondary.

## Motivation

`/change:draft` already knows how to author `plan.yaml` through a brief pipeline: discovery, optional workspace sync, propose, optional assignment, validate, and hand-off. What it does not yet have is a reliable decomposition step for legacy code.

Without that step:

- A 100k LOC monolith reaches planning as one oversized input.
- A fleet of legacy repositories reaches planning as many disconnected inputs.
- Capability boundaries are inferred directly from code organization, which risks rebuilding the legacy architecture in the target system.
- Cross-repo flows such as publisher/subscriber pairs or service-to-service HTTP calls must be stitched together by hand.
- `propose` has to negotiate slice boundaries and plan entries at the same time.

The missing primitive is a source survey: a deterministic analysis pass that turns legacy code into small, capability-shaped candidates before `propose` asks the operator to accept, edit, or reject plan entries.

## Core Idea

Legacy code should be decomposed by externally visible behavior, not internal structure.

A surface is an observable entry point or contract edge: an HTTP route, message publication, message subscription, scheduled job, WebSocket handler, UI route, CLI command, or outbound service call. Surfaces are useful because they describe what the system promises to the outside world. Source modules describe how the legacy system happened to implement those promises.

For every legacy source, the survey records:

- The surfaces the source exposes or consumes.
- The handler or call site for each surface.
- The source files reached from that handler or call site.
- Evidence explaining how the surface was found.

Then `/change:survey` composes all sources together, clusters related surfaces into business capabilities, sizes each capability candidate, and emits the ordered candidate set consumed by `propose`.

## `/change:draft` Analysis Flow

After this RFC, the planning pipeline inside `/change:draft` has one extra analysis step:

- **Pre-flight** — confirm the command can run in the current project, validate arguments, and fail early before any plan files are written.
- **Brief scaffold** — create the draft change workspace and deterministic brief structure that later stages append to.
- **Registry validate** — check project and capability registry state so planning uses known targets and declared capabilities.
- **Discovery** — gather planning-level source facts and documentation hints before slice candidates are proposed.
- **Workspace sync, when multi-repo** — refresh the workspace inventory so repository assignments and target projects reflect the current registry.
- **Source survey** — mechanically decompose legacy code into surfaces, code footprints, and capability-sized candidates.
- **Propose** — turn accepted capability candidates into operator-reviewable plan entries.
- **Assignment, when multi-repo** — attach accepted plan entries to the projects or repositories that should own the work.
- **Plan validate** — run the canonical plan validation before handing the draft back to the operator.
- **Hand-off** — stop after producing the reviewed planning artifacts and leave execution to `/change:execute`.

Only the middle of the pipeline changes. The initial scaffold, single-writer rule for `plan.yaml`, final `specify plan validate`, and operator hand-off remain unchanged.

### Step 1: Collect Inputs

`specify change draft` records the change and its inputs. A source may be:

- `legacy-code`: a local path or materialized clone of an application, service, package, or repository.
- `documentation`: architecture notes, API docs, runbooks, or other prose.

The same flow covers one source and many sources. A monolith is simply one `legacy-code` source. A distributed legacy estate is many `legacy-code` sources plus any documentation inputs.

### Step 2: Analyze Each Input

The discovery brief still invokes `/change:analyze` once per input.

For `documentation`, `/change:analyze` behaves as it does today: it extracts planning-level capability hints into `discovery.md`.

For `legacy-code`, `/change:analyze` becomes mechanical. It does not infer capability summaries directly. Instead, it invokes `specify survey` and writes two sidecars under the plan working directory:

```text
.specify/plans/<change>/analyze/<source-key>/metadata.json
.specify/plans/<change>/analyze/<source-key>/surfaces.json
```

`metadata.json` records coarse source facts such as language, LOC, module count, and top-level modules. `surfaces.json` records the source's externally observable surfaces and their code footprints (see [Artifacts](#artifacts)).

This split is the key simplification: plan-time code analysis first produces structural evidence, not slice decisions.

Before invoking survey, the discovery brief writes the `## Capability inventory` heading wrapper into `discovery.md` exactly once. Survey appends capability blocks under that heading; the brief never re-emits it.

### Step 3: Pass 1 — Structural Decomposition (mechanical, top-down)

After all inputs have been analyzed, `/change:survey` builds a decomposition DAG with five node kinds:


| Kind            | Level | Sized as                                              | Slice candidate? |
| --------------- | ----- | ----------------------------------------------------- | ---------------- |
| `root`          | 0     | union of all source children                          | no               |
| `source`        | 1     | union of surface-group children                       | no               |
| `surface-group` | 2     | union of contained surface `touches`                  | no               |
| `surface`       | 3     | union of handler `touches`                            | no               |
| `capability`    | leaf  | dedup union of `touches` across participating sources | yes              |


Pass 1 only descends into each source independently. There is no Pass 1 cross-source decomposition; cross-source pairing happens in Pass 2 against normalized identifiers, never against source code.

Cuts are tried in priority order; the first that applies wins:

1. **Size check.** XS or S → stop, leaf candidate.
2. **Source split.** At root, always cut on `<source-key>`.
3. **Framework module boundary.** Nest `@Module`, Rails engine, Spring `@Configuration`, Phoenix context, etc., when surfaces partition cleanly.
4. **URI / topic / channel prefix.** Group by longest common prefix (`/users/`*, `user.*`); cut where distinct prefixes have low `touches` overlap.
5. **Worker pool / scheduled-job batch.** Workers and jobs sharing a topic or schedule prefix form their own group.
6. **Surface enumeration.** Each surface group ends at its `surfaces.json` constituents.

Structural depth is capped at **6 per source**. An M+ surface-group still M+ after 6 levels is fatal and forces operator intervention. If no signal cleanly partitions an M+ node before the cap, record `unresolved: true` on that surface-group and emit the DAG with it marked.

### Step 4: Pass 2 — Capability Clustering (semantic, bottom-up)

Once Pass 1 ends at surfaces, cluster surfaces into capability leaves. Inputs:

- All `surfaces.json` files, intra-source first.
- `discovery.md` capability hints from documentation inputs.
- `<plan-dir>/identifier-aliases.yaml` (operator) plus per-capability alias bundles, normalized as in [Identifier Normalization](#identifier-normalization).

Clustering evidence, in priority order:

1. **Shared `touches` overlap (≥ 50%)** within a source — the scattered-within-source case.
2. **Documentation grouping.** When documentation explicitly groups surfaces under one capability heading, that grouping is authoritative even if identifiers do not match mechanically.
3. **Cross-source contract edges**, matched on normalized `identifier`:
  - **Pub/sub pairing.** `message-pub` in source A + `message-sub` in source B sharing the normalized identifier → one cross-source leaf. **Publisher's source is canonical owner**; subscribers join.
  - **HTTP contract pairing.** `external-call-out` in source A whose normalized identifier matches an `http-route` in source B → one cross-source leaf. **Route owner is canonical**; caller depends on it.
  - **WebSocket contract pairing.** `external-call-out` (channel kind) matching a `ws-handler` → one cross-source leaf. **Handler owner is canonical**.
4. **Worker-pool / topic-prefix affinity** that survives surface-group boundaries.

Cluster outcomes:

- **Span more than one surface-group within a source** → `cross_module: true`.
- **Span more than one source** → `cross_source: true`. Surface ids are namespaced `<source-key>:<surface-id>` so the same identifier from two repos remains distinguishable. The two flags are not exclusive.
- `**depends_on` / `depends_on_by`** derive from contract edges (canonical owner → consumer). When producer and consumer end up in the same leaf, no edge is emitted — the dependency is internal.
- **Subscriber surface with no in-scope publisher** → record as a `consumes-external` annotation on its single-source leaf, not an `unresolved`.
- **Ambiguous match** (multiple plausible cross-source pairings after normalization, or an alias-resolved pair the operator has not confirmed) → leaf is `unresolved: true` with the candidate set listed verbatim. Survey never invents fictitious cross-source pairs.

Pass 2 has no depth cap; it is a single pass over the surface set.

### Step 5: Size And Order Candidates

Each candidate is sized using a framework-pinned T-shirt rubric over **production LOC** (excluding tests, generated code, vendored deps, blank lines, comment-only lines):


| Size | Production LOC | Planning meaning                         |
| ---- | -------------- | ---------------------------------------- |
| XS   | `< 200`        | Smaller than a normal slice; acceptable. |
| S    | `200-1499`     | Slice-sized; acceptable.                 |
| M    | `1500-4999`    | Too large; split or mark unresolved.     |
| L    | `5000-19999`   | Too large; split or mark unresolved.     |
| XL   | `>= 20000`     | Too large; split or mark unresolved.     |


For a cross-source capability, LOC is the **deduplicated union of `touches` across every participating source**.

The invariant is simple: `propose` should receive XS/S candidates or explicit unresolved items. It should not receive an unsliced monolith or an undifferentiated repo fleet.

Ordering comes from `depends_on`. Independent candidates may appear at the same order level and migrate in parallel.

### Step 6: Hand Candidates To Propose

Survey appends capability blocks to the `## Capability inventory` heading the discovery brief wrote in Step 2. Propose remains the only stage that asks the operator to accept, edit, reject, or abort plan entries. Every accepted entry is still written through `specify plan add`.

Survey produces candidates; propose produces `plan.yaml`.

## Identifier Normalization

Cross-source matching keys on a canonicalised form of `surfaces[].identifier`. Original identifiers are preserved verbatim on every surface; the normalized form is only the matching key.

Framework defaults — explicit, identical for every capability:


| Surface kind                                        | Default canonicalisation                                                                                                                                                                                              |
| --------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `http-route`, `ui-route`                            | Lowercase host/path, strip trailing slash, fold path-parameter syntax (`{id}` ≡ `:id` ≡ `<id>`). Strip configured version prefixes (`/v1`, `/v2`, …) **only when** `http: { strip_version_prefix: true }` is enabled. |
| `message-pub`, `message-sub`, `ws-handler`          | Case-fold, unify dot/dash/underscore separators (`user.created` ≡ `user-created` ≡ `user_created`). Strip configured environment prefixes (`prod.`, `staging.`) **only when** listed.                                 |
| `cli-command`, `scheduled-job`, `external-call-out` | Lowercase identifier; otherwise verbatim.                                                                                                                                                                             |


After framework canonicalisation, alias bundles merge with strict precedence:

> **operator (`<plan-dir>/identifier-aliases.yaml`) > capability (`briefs/<cap>/identifier-aliases.yaml`) > framework default**

Alias schema:

```yaml
aliases:
  - kind: message-pub
    group: [user.created, users.created, user-created]
http:
  strip_version_prefix: true
```

Aliases inside a `group` are bidirectional. Any alias whose `kind` fails the closed `surface kind` enum check **fails the survey**.

Aliases are a review mechanism, not a guess. Survey marks ambiguous matches `unresolved` until the operator confirms the equivalence.

## Artifacts

### `surfaces.json`

One file per `legacy-code` source. Byte-stable, validated before write.

Conceptual shape:

```json
{
  "version": 1,
  "source_key": "legacy-monolith",
  "language": "typescript",
  "framework_signatures": ["express", "bullmq"],
  "surfaces": [
    {
      "id": "http-post-users",
      "kind": "http-route",
      "identifier": "POST /users",
      "handler": "src/auth/register.ts:registerUser",
      "touches": [
        "src/auth/register.ts",
        "src/notifications/email.ts",
        "src/users/repository.ts"
      ],
      "evidence": "Express route registered in src/server.ts"
    }
  ]
}
```

All fields are required. `version` is `1`; bumps go through an RFC update. `surfaces[]` is sorted by `id`; `touches` is sorted alphabetically; `framework_signatures` is sorted alphabetically. No timestamps, no absolute paths, no host-state leaks.

The surface kind enum is closed in v1:

`http-route`, `message-pub`, `message-sub`, `ws-handler`, `scheduled-job`, `cli-command`, `ui-route`, `external-call-out`.

Unknown kinds fail validation. Extensions require an RFC update so capabilities do not drift into incompatible vocabularies.

### `survey.md`

One file per change. Required sections, in order:

1. `Summary` — source / surface / candidate / unresolved counts.
2. `Source inventory` — one row per input source.
3. `DAG` — root → source → surface-group → surface → capability.
4. `Capability candidates` — proposed slice-sized leaves.
5. `Unresolved` — ambiguous or oversized items requiring operator input.
6. `Migration order` — topological sort over `depends_on`.

Within each node block, fields appear in fixed order so re-runs diff cleanly:

> `kind`, `sources`, `target_project`, `handler`, `touches`, `surfaces`, `cross_module`, `cross_source`, `evidence`, `children`, `depends_on`, `depends_on_by`, `unresolved`

Omit fields that don't apply to the node's kind.

Example capability leaf:

```markdown
### identity.user-registration [S, 1094 LOC]

- kind: capability
- sources: [legacy-monolith, legacy-workers]
- target_project: identity-svc
- surfaces: [legacy-monolith:http-post-users, legacy-monolith:message-pub-user-created, legacy-workers:message-sub-user-created]
- cross_module: true
- cross_source: true
- evidence: pub/sub pairing on normalized identifier `user.created`; legacy-monolith is canonical owner; touched files overlap on user repository and email verification
- depends_on: [shared-validation]
```

Re-running on unchanged inputs (including aliases) produces byte-identical `survey.md`.

### `identifier-aliases.yaml`

Operator-authored, tracked alongside the change. See [Identifier Normalization](#identifier-normalization) for schema, precedence, and validation.

## Mechanical Scanner

The CLI scanner invoked by `/change:analyze legacy-code`:

```text
specify survey <source-path> --source-key <key> --format json --out <path>
```

It owns mechanical work only:

- Detect framework signatures.
- Enumerate surfaces.
- Resolve handlers and call sites where static analysis can do so.
- Record touched files.
- Validate and write `surfaces.json`.

The scanner does not call an LLM, infer capabilities, or write `plan.yaml`. If no detector applies, it exits non-zero with discriminant `surface-scan-no-detectors-registered` and writes no partial output.

Capability-owned detector packages add framework support over time. v1 only needs enough detectors to prove the flow on the first supported stack; unsupported stacks fall back to manual source scoping until a detector exists.

## Skill Responsibility Split


| Component                       | Responsibility                                                                                                                            |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| `/change:analyze documentation` | Extract capability hints from prose into `discovery.md`.                                                                                  |
| `/change:analyze legacy-code`   | Run the mechanical scanner; write `metadata.json` + `surfaces.json`; do not infer capabilities.                                           |
| `specify survey`                | Deterministically enumerate surfaces for one source.                                                                                      |
| `/change:survey`                | Compose all sources, run Pass 1 + Pass 2, size candidates, write `survey.md`, append capability blocks under the discovery-owned heading. |
| `propose` brief                 | Ask the operator to accept/edit/reject candidates and write accepted plan entries through `specify plan add`.                             |


This split keeps expensive semantic judgement out of per-source analysis and lets cross-source clustering happen with the full system in view.

## Routing Hint Precedence

When assignment infers a target project for a survey-leaf slice, signals are consulted in strict order:

1. **Survey leaf `target_project`** — inherited from the nearest ancestor that carries one (typically a documentation hint, or the canonical owner on a cross-source leaf). Surfaced verbatim in the assignment table's `Rationale` column.
2. Description match (today's primary signal).
3. Baseline spec affinity (today's secondary signal).
4. Capability compatibility (today's tiebreaker).
5. Ambiguity → human.

Cross-source leaves carry the `target_project` of the canonical owner: publisher for pub/sub, route owner for HTTP, handler owner for WebSocket.

## Single-Source And Multi-Source Behavior

The algorithm is identical in both cases.

For a monolith, survey usually finds cross-module candidates: one capability implemented across several internal folders, workers, or packages.

For a repo fleet, survey can also find cross-source candidates: one capability implemented by multiple deployable systems connected by HTTP, messages, jobs, or shared external contracts.

The source count changes the breadth of the graph, not the planning model.

## Brownfield Behavior (v1)

When a target workspace already has `.specify/specs/` baselines, survey treats baseline projects as **opaque routing targets** consumed by existing assignment logic. It does **not** read baselines to flag delta-target opportunities; that is a propose-time concern with no concrete first user. See [Out Of Scope](#out-of-scope).

## Guardrails

- Survey is plan-time decomposition, not spec extraction. Full `spec.md` and `design.md` authoring still happens per slice through `/spec:define`, delegating to `/spec:extract` when legacy code is the source.
- Survey never writes `plan.yaml`. Only `specify change draft`, `specify plan add`, and `specify plan amend` write plan state.
- Legacy module boundaries are evidence, not authority. They may help find surfaces and code footprints, but they do not define slices.
- Unknown surface kinds, malformed sidecars, and aliases failing the closed-kind check fail closed.
- Outputs are byte-stable on unchanged inputs: fixed field order, sorted lists, no timestamps, no absolute paths, no host-specific state.
- Ambiguity is explicit. Survey emits `unresolved` candidates rather than inventing aliases or silently merging unrelated surfaces.

## Implementation Plan

1. Add the `surfaces.json` and `identifier-aliases.yaml` schemas + validators (closed-kind enforcement on alias `kind`).
2. Add `specify survey` with a stub detector registry, deterministic output, validation before write, and the `surface-scan-no-detectors-registered` exit when no detector applies.
3. Land the framework identifier canonicaliser inside `/change:survey` with the rules in [Identifier Normalization](#identifier-normalization). Fixtures cover the canonical-form table and the operator > capability > framework alias-merge precedence.
4. Land first mechanical detectors for the initial supported stack (Express, NestJS, BullMQ).
5. Rewrite `/change:analyze legacy-code` to write `metadata.json` and `surfaces.json` only.
6. Extend the discovery brief to write the `## Capability inventory` heading wrapper before invoking survey.
7. Add `/change:survey` with Pass 1 (priority-ordered cuts, depth cap 6/source) and Pass 2 (canonicalised cross-source pairing with canonical-owner rules and `consumes-external` annotation for unpaired subscribers). Wire it between workspace sync and propose.
8. Update `assignment.md` for the precedence in [Routing Hint Precedence](#routing-hint-precedence).
9. Acceptance fixtures: single-source L monolith with one cross-module capability; multi-source change with **≥ 3 source-keys** producing at least one cross-source capability and one `unresolved` leaf resolved by adding to `identifier-aliases.yaml` and re-running survey; greenfield documentation-only pass-through; root-already-S no-op.
10. Tutorials: monolith decomposition, legacy-fleet decomposition (with one alias-resolved `unresolved`), update `legacy-migration-at-scale.md`.

## Migration

This is a plan-time behavioral change for legacy-code inputs.

**For operators.** `/change:analyze legacy-code` no longer infers capability summaries directly into `discovery.md`. Instead it writes `metadata.json` + `surfaces.json` sidecars, and `/change:survey` owns capability clustering and writes the candidate inventory for propose. In-flight plans do not need conversion — re-running `/change:draft` for a legacy-code change regenerates plan-time scratch artifacts in the new shape. Multi-source changes get cross-source clustering automatically; ambiguous identifiers surface as `unresolved` with the candidate set listed, and the operator extends `<plan-dir>/identifier-aliases.yaml` and re-runs.

**For capability authors.** Move the `legacy-code` clustering content out of `plugins/change/skills/draft/briefs/<cap>/analyze.md` into `plugins/change/skills/survey/briefs/<cap>/cluster.md`. `analyze.md` retains only the `documentation` branch. Register surface detectors under `plugins/change/skills/survey/briefs/<cap>/detectors/` (mechanical AST/regex only in v1). Capability-owned alias overrides live at `plugins/change/skills/survey/briefs/<cap>/identifier-aliases.yaml` and merge against framework defaults per the precedence above.

**For skill authors consuming planning artifacts.** New artifacts: `surfaces.json` per source under `<plan-dir>/analyze/<source-key>/`, and `survey.md` under `<plan-dir>/`. Both schemas pinned, byte-stable. The `## Capability inventory` heading in `discovery.md` is now authored by survey for legacy-code inputs; the block shape is unchanged.

Documentation-only changes continue to work. They may pass through `/change:survey` as a no-op or skip it entirely (see Open Question 5).

## Non-Goals

- Extracting full specs from legacy code during draft.
- Replacing the propose accept/edit/reject loop.
- Durable source catalogues or cross-change source caches; those belong to RFC-21.
- A migration ledger or cumulative mapping of migrated surfaces; those belong to RFC-22.
- Brownfield reconciliation against existing `.specify/specs/` baselines.
- LLM fallback detectors for unsupported frameworks.
- A standalone sizing command outside the survey flow.
- Everything in [Out Of Scope](#out-of-scope).

## Out Of Scope

Each item below was considered for v1 and deferred. Re-open triggers are concrete so the bar for adding them back is clear.


| Item                                                                                                     | Re-open when                                                                                                                                         |
| -------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `domain-model` as a third closed-enum kind on `/change:analyze` (structured bounded-context import)      | An operator wants a structured context-map workflow, or documentation analyze repeatedly fails to surface bounded-context attribution routing needs. |
| `synthesize` brief and `## Reconciliation` section in `discovery.md`                                     | Propose repeatedly drafts slices that ignore documented-but-uncoded capabilities, or `domain-model` lands and produces a third corpus to reconcile.  |
| `specify plan size` standalone CLI verb                                                                  | Operators report wanting LOC audits outside a draft run (slice review, candidate spot-check).                                                        |
| Per-capability `cut.md` brief separate from `cluster.md`                                                 | A capability author writes a Pass 1 refinement that materially exceeds half a page inside `cluster.md`.                                              |
| Per-capability `sizing.toml` overrides (tighten LOC rubric, add aggregate/endpoint counts)               | A capability demonstrates LOC-only sizing produces persistently wrong slices in operator review.                                                     |
| LLM-fallback detector contract and `--fallback-llm` flag                                                 | A real legacy stack outside the mechanical-detector envelope reaches the planning pipeline.                                                          |
| Brownfield reconciliation against `.specify/specs/` baselines (read baselines for delta-target flagging) | Brownfield-only changes reach the pipeline frequently enough that propose's missing delta-target awareness becomes a recurring complaint.            |
| Surface `confidence` field (graded high/medium/low)                                                      | The LLM-fallback contract lands; the field then differentiates mechanical from probabilistic detection.                                              |


## Open Questions

1. Is the S-size ceiling of `1499` production LOC a good default, or should it start lower?
2. Should `specify survey` live as a top-level verb, or under `specify change survey` to make its plan-time role clearer?
3. Should `survey.md` have a machine-readable JSON sibling in v1, or wait for a downstream consumer?
4. Default identifier-normalisation aggressiveness for HTTP version prefixes — opt-in is currently safer (a `/v1` ↔ `/v2` accidental pairing is the exact case operators want kept distinct), but an opt-out default may be better in single-product fleets.
5. Should documentation-only changes invoke `/change:survey` in pass-through mode for a uniform pipeline, or skip it to reduce ceremony?

## References

- [RFC-13: Extensibility](archive/rfc-13-extensibility.md)
- [RFC-20: Survey-to-Plan Pipeline](rfc-20-survey.md)
- [RFC-21: Source Catalogue and Tier-1 Cache](rfc-21-catalogue.md)
- [RFC-22: Migration Ledger](rfc-22-ledger.md)
- [RFC-23: Change Lifecycle](archive/rfc-23-change-lifecycle.md)
- `[/change:draft` SKILL.md](../plugins/change/skills/draft/SKILL.md)
- `[/change:analyze` SKILL.md](../plugins/change/skills/analyze/SKILL.md)

