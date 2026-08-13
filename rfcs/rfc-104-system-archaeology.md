# RFC-104: System Archaeology and Migration Planning

> Status: Implemented definition predecessor to [RFC-88](rfc-88-detached-changes.md) in the [Services Delivery Programme](platform.md). The definition loop through `system.wave.reviewed` holds.
>
> Depends on implemented [RFC-87](rfc-87-working-trees.md) for content-addressed trees and private workspaces used as survey `source-input`.
>
> Owns the durable definition home and the `emery system *` loop: declared boundary and coverage, source materialization with observed-tree provenance, the source WIT change (source key + prepared input on `survey` / `extract`, no `focus`), estate Evidence and correlation into the reviewed as-is model, architecture and diagram projections, dispositions, target and transition architecture, the migration plan, the immutable wave handoff, and the `system.wave.reviewed` fact. It pins no delivery CIDs; [RFC-88](rfc-88-detached-changes.md) consumes one reviewed wave and owns delivery. Consumption patches for the next RFC-88 cut live in [rfc-104-rfc-88-patches.md](rfc-104-rfc-88-patches.md).

## Intent

Make system archaeology a first-class client deliverable rather than an implicit precondition of slice authoring.

An unfamiliar enterprise system is not a list of repositories waiting to become slices. Its effective architecture spans code, interfaces, state stores, queues, scheduled work, deployment environments, runtime behaviour, operational procedures, ownership, policy, and undocumented dependencies. Some elements will change, some only constrain the change, and some will remain inaccessible or uncertain.

Emery must recover the best defensible account of that system, show the coverage and uncertainty behind it, and help practitioners turn it into a reviewed migration strategy. Only then should RFC-88 decompose one selected wave into buildable slices.

The goal is not perfect knowledge:

> **A survey is complete when every element inside a declared boundary has an explicit coverage disposition, not when Emery claims to know every fact about the estate.**

## Problem

The current source pipeline is delivery-shaped. First-party `survey` prompts cluster native surfaces into slice-sized planning leads, `extract` persists slice-local Evidence, and RFC-88 recursively partitions the result until every terminal domain is buildable. That is useful after a change boundary is understood, but it creates four failures during definition.

**System facts are forced toward slices too early.** Adapters guess at an engine noun — a slice — and merge endpoints, topics, and jobs before the engine sees them. Shared state, runtime topology, cross-repository dependencies, operational constraints, and context-only systems then have no surface to hang on, and may inform many waves without belonging to any one buildable leaf.

**Repository discovery is not system coverage.** A forge namespace does not enumerate deployed services, databases, queues, vendor integrations, environments, operational procedures, or inaccessible evidence. Omitting unmatched candidates and retaining only successful target bindings cannot support a defensible completeness claim.

**Architecture remains prose without durable authority.** Per-slice `design.md` explains implementation after decomposition. It cannot serve as the reviewed as-is model from which system context, component, deployment, data-flow, and journey diagrams are derived.

**A delivery plan is not a migration plan.** Slice dependencies do not express target architecture, transition states, state movement, coexistence, cutover, rollback, or the reason an observed legacy structure should be preserved, changed, retired, or investigated.

## Terms

- A **definition home** is the client-owned, durable root containing one system's archaeology and migration-planning artifacts. It is not a checkout registry, build workspace, or temporary change home.
- A **system boundary** is the reviewed statement of which products, journeys, environments, organizations, and evidence locations the definition engagement covers.
- A **coverage disposition** is the closed value `included | excluded | inaccessible | unsupported | unresolved` for one candidate source or system element, with a reason and authority where applicable.
- An **observed tree** is the RFC-87 tree identity (`observed-cid`, wire form `sha256:` + hex) of the concrete snapshot a survey materialized from a coverage locator, plus `observed-revision` when the origin is Git and the fetch reports one. It is survey provenance of what was read, not a delivery pin of what will be built.
- A **lead** is one adapter-native surface at the smallest unit that adapter can name from the source: an HTTP endpoint, a topic, a job, a document or top-level section, a capture handler, a screen, or an intent string. It is the unit of `extract`, not a slice and not a system-model element.
- A **system model** is the structured, evidence-linked account of the system's elements, relationships, state, constraints, and uncertainties.
- An **architecture projection** is a document or diagram derived from one exact system-model digest. The model is authority; presentation files are replaceable views.
- A **modernization disposition** is the reviewed treatment `preserve | change | retire | investigate` for observed behaviour, state, interface, or architecture.
- A **transition architecture** is one operable intermediate state between the reviewed as-is and target architectures.
- A **migration wave** is one bounded outcome in the migration plan, including its preconditions, affected and context-only elements, state movement, coexistence and cutover strategy, rollback position, acceptance, and dependencies.
- A **wave handoff** is the immutable, content-addressed projection of one migration wave and every definition identity RFC-88 must fence before delivery authoring.
- A **wave review fact** is one `system.wave.reviewed` event naming an exact handoff digest. It records an explicit operator act; it does not prove how deeply a person reviewed the material.

## Flow

The definition loop precedes the existing delivery loop for modernization:

```text
emery system survey [--dir <path>]  → pin the declared boundary; account coverage and extract;
                                      write Evidence and the as-is system model
operator reviews                    → resolve identity, topology, authority, and coverage gaps
emery system plan [--dir <path>]    → project architecture views; propose target and transition
                                      architecture plus bounded migration waves and handoffs
operator reviews                    → decide dispositions, architecture, and the first wave
emery system review [--dir <path>]  → append system.wave.reviewed over the exact handoff
emery plan author                   → --from <definition-home> --wave <id>; bind exact delivery
                                      targets and sources; decompose only that wave into slices
```

`emery system *` takes optional `--dir <path>`. The definition root is `--dir` if present, else CWD. The launcher mounts that directory as the guest `.` — no `project.yaml` walk, no `.emery/system/` probe. The guest fail-closes if `scope.yaml` is not at that root. Relative `--dir` joins the invoked directory; `--dir` is the home itself, not a search root. RFC-88's `plan author --from` names the same kind of path as a later read-only preopen; this RFC does not change that flag. Do not reuse `--project-dir`.

`emery system survey` and `emery system plan` are review-bounded stages over that root. Re-running the stage is resume: survey always re-extracts, plan always reprojects from live files. There are no checkpoint or attempt files. The two operator decisions remain separate: first review what the system is, then review how it should change.

The first pause is artifact review, not a fact. The operator may edit declared inputs and re-run `system survey`. The second pause is the same shape over plan proposals: edit dispositions, architecture, and waves, then re-run `system plan` if those inputs changed. Only `system review` mints `system.wave.reviewed`.

The definition loop may end after the survey pause or after wave review. An archaeology or readiness engagement is a legitimate completed outcome even when no delivery wave is authorized.

## Decisions

### D1 — The definition home is durable client architecture, not platform coordination

A definition home contains no product checkout and owns no product lifecycle:

```text
<system>/
  scope.yaml
  coverage.yaml
  evidence/<source>/<lead>.yaml
  system.yaml
  architecture/
    as-is.md
    target.md
    transitions/<transition>.md
    diagrams/<view>.source
    diagrams/<view>.svg
  migration.yaml
  handoffs/<digest>.yaml
  decisions/
  events/<writer>.jsonl
```

`scope.yaml` is the declared boundary. `coverage.yaml` records every candidate and disposition. An `included` row carries the source key, exact location (URL or path), and operator-declared adapter identity (a name, or an exact package pin when the operator supplies one) that `survey` / `extract` consume. After a successful included source it also carries survey-written `observed-cid` and, when applicable, `observed-revision`. After a failed included source it carries `survey-error` and does not update a prior observed tree. There is no separate `sources.yaml`. `evidence/<source>/<lead>.yaml` is the existing Evidence document; this RFC does not invent a second claim schema. `system.yaml` is the structured architecture authority: declared `identities[]`, generated `as-is`, and operator-owned `target` plus zero or more `transition-*`. `migration.yaml` records modernization dispositions, wave references to those named states, and migration waves. `handoffs/` retains every wave projection named by a review or delivery fact; a handoff is never a second editable migration plan. `decisions/<id>.yaml` is an operator-authored definition decision; it is not a product `DEC-NNNN` record.

Declared inputs are operator-editable: `scope.yaml`, coverage-row declared fields (`key`, `location`, `adapter`, `disposition`, `reason`), `decisions/`, `system.yaml` `identities[]`, and — once present — `target`, `transition-*`, and `migration.yaml`. Survey-written coverage fields are `observed-cid`, `observed-revision`, and `survey-error`. The operator creates the root and those declared files. There is no `system init` or `system amend` verb. Missing `scope.yaml` or `coverage.yaml` fails closed with a hint. First successful `system survey` writes generated layout (`evidence/`, `as-is`, `architecture/`, `events/`) into that root and does not scaffold declared inputs. A completed survey with no included Evidence still writes empty `as-is`; a size-gate stop does not.

Mixed files persist surgically: load, replace only the generated named state or the survey-owned coverage fields this run touched, then canonical YAML write. Comments and key order are not preserved; git is v1 history. Survey overwrites `evidence/` and the `as-is` named state, writes `observed-cid` / `observed-revision` onto coverage rows whose included source completed, and writes `survey-error` onto included rows whose access or adapter run failed. It does not rewrite `key`, `location`, `adapter`, `disposition`, or `reason`, and does not clear a prior `observed-cid` on failure. Plan writes `target` and any proposed `transition-*` only when `target` is absent at load (the initial architecture proposal); it never overwrites an existing named state. Later plans reproject `architecture/` and write a new `handoffs/<digest>.yaml` without deleting historical handoffs and without adding named states. Neither stage silently clobbers declared locators, identities, or recorded decisions. The engine never writes `decisions/`. The next stage is the writer gate: typed parse plus validation on `system survey`, `system plan`, or `system review`. Client git history plus digest invalidation (D10) is v1 revision retention. The archaeology package is that git tree; there is no `system archive` verb.

The definition home is retained across changes and may live in a client architecture repository. A degenerate single-product definition may live at `<product>/.emery/system/`; the operator points `--dir` at that directory or runs from it. There is no product-walk auto-discovery. RFC-88 receives the exact root and wave explicitly (`--from`).

`system *` anchoring is a launcher projection, evaluated once from argv the way `adapter add --project-dir` is. When argv is `system *`, the launcher does not call the `project.yaml` walk. It mounts `--dir` or CWD as `.`. It does not create that directory — the operator created it; missing `scope.yaml` is a guest fail-closed, not a `mkdir`. A definition-home `Layout` owns paths at that root (`events/` at `<system>/events/`, not `.emery/events/`). Adapter store, snapshots, and workspaces stay under `$EMERY_HOME`; the definition home is not a product root for cache keying. Bare coverage adapters resolve store / pull-latest. Origin trees named in `coverage.yaml` are not the `.` mount: survey materializes them into an RFC-87 workspace on the workspaces preopen.

The definition home is not the permanent platform repository rejected by RFC-88: Emery does not use it to route product workspaces, mirror product code, own repository heads, or coordinate execution. It is a durable client artifact for understanding and changing one system.

### D2 — Completeness is bounded and coverage-accounted

The operator declares the decision the survey must support and the exact locations it may inspect. v1 `scope.yaml` names that decision and the boundary's products, journeys, environments, and organizations; it does not carry locators. v1 `coverage.yaml` accepts only exact locators (URL or path). Included coverage rows also carry operator-declared adapter identities. A boundary may also name deployed services, interfaces, environments, documentation collections, runtime capture locations, infrastructure descriptions, ownership sources, and explicit exclusions as coverage candidates.

Coverage records every declared candidate. No candidate silently disappears because access was denied, an adapter could not run, or a source was excluded after review. Each receives one operator-declared coverage disposition and reason. An `included` row is the survey binding: operator-declared `key`, `location`, `adapter`, `disposition`, and `reason`. Survey may add `observed-cid` after a successful included source, `observed-revision` when the origin is Git and the fetch reports one, and `survey-error` when this run's access or adapter step failed.

`included` means Emery surveys the source at its recorded location; it does not mean every claim is known, and it does not mean this run succeeded. `inaccessible` and `unsupported` are operator-declared coverage accounting, not auto-promotions from a failed fetch. `unresolved` preserves a candidate whose identity or membership needs human judgment. An operator may move a candidate between dispositions by editing the coverage row; the next `system survey` or `system plan` validates the file, and the new coverage digest invalidates downstream model and migration projections. Survey persist loads `coverage.yaml`, patches only survey-owned fields on rows this run touched, and writes; it never rewrites declared locators or dispositions.

This RFC does not expand forge namespaces or fingerprint trees onto adapters. Completeness is coverage over declared exact locations, not a discovery engine.

This RFC does not pin delivery CIDs into the handoff. A coverage `location` remains a mutable origin locator. Every `system survey` re-resolves that locator to a concrete tree — a local path, or a fetch of a Git or HTTPS origin — prepares an RFC-87 read-only workspace, and records what it observed. `observed-cid` is the RFC-87 tree identity of that snapshot. `observed-revision` is the Git commit when the origin reports one. Those fields are generated provenance: the next successful survey overwrites them; a failed run leaves them and writes `survey-error`. They are not locators and must not be authored as the binding. RFC-88 re-resolves the same locator at bind time and pins the delivery CID; later delivery runs use that pin rather than rereading the origin. The observed CID and the delivery CID may differ if the origin moved between archaeology and binding. Snapshot objects from a survey fetch are not delivery GC roots; the workspace is discarded after extract.

The coverage summary reports the declared boundary, included evidence classes, observed trees for successful included rows, included rows that carry `survey-error` this run, inaccessible and unsupported areas, unresolved identities, and material unknowns. External claims say “surveyed within this boundary at this coverage” rather than “complete” without qualification.

### D3 — Source leads are adapter-native surfaces

RFC-104 reuses the source adapter's two operations and owns the WIT change that makes their inputs explicit. Both receive the coverage-row source key and a prepared immutable input; they never parse `plan.yaml` or recover a location from it.

- `survey(adapter, source-key, input) → Lead[]` identifies the adapter's native surfaces at their smallest stable unit.
- `extract(adapter, source-key, input, lead) → Evidence` extracts one included lead needed by the system model.

`input` is a prepared RFC-87 read-only workspace of the tree the coverage locator resolved to, or inline content. The WIT carries that tree, not the locator and not `observed-cid`. Tree identity stays engine-side on the coverage row. Survey materializes the tree before either operation runs: it does not pass a live Git URL through to the adapter and hope the guest can fetch. Inline content records `observed-cid` as the RFC-87 one-file tree of that value and has no `observed-revision`. Extract's meaning is unchanged: one terminal lead in, one Evidence document out. The Evidence document schema is unchanged; claim `path` anchors are attributable because the coverage row records the observed tree those paths were read from. RFC-88 pins delivery CIDs at bind time and does not reuse the observed CID as that pin.

The live `plan author` and `emery source survey` / `extract` path migrates onto the same WIT in this cut. The engine may still read `plan.yaml` to know which binding to prepare; adapters stop recovering location from it or from `$SOURCE_DIR`.

This RFC does not add optional parent-lead `focus` or a child-lead response shape. RFC-88 extends `survey` with those over the delivery pin. Estate survey has no parent; focused survey is a delivery-time call, not a filter on archaeology of the observed tree.

A lead is the smallest surface that adapter can name from the source: one HTTP endpoint, one topic, one job, one document or top-level section, one capture handler, one screen, or one intent string. It is not a slice. It is not a system-model element. Adapters emit what they know; they do not cluster toward an engine noun. Slice-sizing (LOC thresholds, merge-until-slice-shaped) and architecture-sizing (one lead per service, store, journey, or constraint) are both rejected. Services, stores, journeys, and ownership appear in the as-is model after correlation. Slices appear after RFC-88 groups imported surface leads. Neither is a survey guess.

The same surface-grain corpus serves both consumers. `system survey` and live `plan author` share one `survey` / `extract`; there is no definition-vs-delivery mode flag. Correlation composes many surface Evidence documents into elements and relationships. Delivery decomposition groups imported surface leads into slices. RFC-104 persists Evidence by `(source, lead)` under the definition home, independent of any change or slice. After every included source that completed `survey` this run, and before any `extract`, the engine counts those leads against a recorded engine constant beside `MAX_REPAIRS`. Exceeding it is a typed stop (`system-survey-lead-limit`): this run does not extract, does not run correlation, and does not replace `as-is`. Included rows stay `included`; recovery is D2 — the operator narrows coverage (exclude or unresolve rows) or authors another definition home for a narrower decision, then re-runs `system survey`. The engine does not partition the estate, split the correlator, or raise the ceiling from `scope.yaml` or policy. Every `system survey` that passes the gate re-extracts every included lead and overwrites that Evidence file. Extraction is never cached. A failed included source writes `survey-error` `{ kind: access | adapter, detail }` on its coverage row, leaves that source's Evidence in place, and does not update `observed-cid` / `observed-revision`. `kind: access` is a materialization or fetch failure; `kind: adapter` is a `survey` or `extract` failure after a tree was prepared. A later successful run of that source clears `survey-error`. Leads that disappear from a successful survey are deleted.

Extract of a surface records the calls, contracts, types, and excerpts that surface actually has, using the existing claim kinds, so correlation can evidence invocation, read/write, and ownership. A `POST /orders` lead that writes a store must carry that `call`; correlation may then mint an `orders-store` element. Extract must not write a second behavioural spec and hope correlation parses architecture out of `excerpt` prose.

Adapters remain source-local. They do not decide global component identity, target architecture, migration waves, or delivery boundaries. Correlation is a separate judgment over the complete extracted Evidence set, followed by deterministic validation.

RFC-88 groups the wave's imported surface leads into slices. Focused child survey runs only when an imported lead is still coarser than a buildable boundary — a monolithic document, a generated mega-handler — not to recover endpoints that estate survey already emitted. It does not repeat estate-wide extraction or reinterpret the system boundary.

### D4 — The system model separates evidence, inference, decision, and unknown

Authoritative definition files are typed, unknown-field-rejecting YAML. Digests are SHA-256 of the canonical encoding, independent of formatting, the same rule D10 already uses for a handoff. v1 specifies closed top-level shapes and provenance rules, not a per-kind attribute catalog and not a separate schema file per migration concern.

`scope.yaml` is the declared boundary:

```yaml
version: 1
id: orders
decision: Recover order-taking architecture for a first migration wave
products: [orders]
journeys: [place-order]
environments: [prod]
organizations: [commerce]
```

`coverage.yaml` is one row per declared candidate. `adapter` is required if and only if `disposition` is `included`. Failed and unsupported rows stay. `observed-cid` and `observed-revision` are absent until a successful included source writes them; `survey-error` is absent until a failed included source writes it and is removed on the next success of that source. None of those three fields is operator-declared:

```yaml
version: 1
candidates:
  - key: orders-code
    location: https://github.com/acme/orders
    adapter: typescript
    disposition: included
    reason: Primary order service repository
    observed-cid: sha256:…
    observed-revision: 7f3a9c1d2e4b
  - key: billing-code
    location: https://github.com/acme/billing
    adapter: typescript
    disposition: included
    reason: Billing service repository
    survey-error:
      kind: access
      detail: GitHub returned 404
```

`system.yaml` carries declared identities and named architecture states. `identities[]` is operator-owned from the start and may be empty. `as-is` is the recovered model `system survey` writes. `target` is the intended end state `system plan` writes when absent at load and the operator then edits. `transition-*` sections exist only when the target cannot be reached in one hop; plan proposes them only as part of that initial write. Each named state is independently digestible. Git plus digest invalidation is v1 revision history; the file does not keep an internal revision log.

```yaml
version: 1
identities:
  - id: orders
    aliases: [legacy-order-svc]
    supersedes: [order-monolith]
as-is:
  elements:
    - id: orders
      kind: service
      status: evidenced
      claims: [{ source: orders-code, id: orders.api }]
      context-only: false
  relationships:
    - id: orders-reads-store
      kind: read
      from: orders
      to: orders-store
      status: evidenced
      claims: [{ source: orders-code, id: orders.store-read }]
target: { elements: [], relationships: [] }
```

Each state carries stable elements and relationships. The initial closed element vocabulary covers systems, services or components, repositories, interfaces, data stores, queues or topics, scheduled jobs, deployment units, environments, external actors or systems, and owning groups. Relationships cover containment, deployment, invocation, publication, consumption, read, write, dependency, and ownership. Further attributes are an open map on the record; this RFC does not enumerate per-kind fields.

`status` is the closed set `evidenced | inferred | conflict | unknown | decided`. Provenance is claim refs `{ source, id }` into persisted Evidence. `evidenced` requires at least one claim that exists. `inferred` has empty `claims` and cannot become evidenced by repetition in generated prose. `conflict` retains disagreeing claims. `unknown` is a gap with empty `claims`. `decided` requires `decision: <id>` naming an existing `decisions/<id>.yaml`; correlation cannot emit it. Relationship `from` / `to` must resolve to element ids. Context-only elements and relationships are marked `context-only: true`; being relevant to a migration does not imply being modified by it.

Definition decisions are operator-authored YAML at `decisions/<id>.yaml`. They are not product Decision Records (`DEC-NNNN-*.md`). The `id` is kebab-case and equals the filename stem. Absent `decisions/` is valid. The engine never writes the directory. Digest is SHA-256 of the canonical YAML encoding, the same rule as every other definition file. Closed fields:

```yaml
version: 1
id: order-owner-authority
applies-to: [orders]
supersedes: []
context: Order records currently live in the legacy monolith store.
decision: The orders service is the system of record for order state.
consequences: Migration waves must move ownership before retiring the monolith store.
```

`applies-to` and `supersedes` are optional. `applies-to` lists element or relationship ids the persist tail stamps after writing `as-is`. Two records applying to the same id fail closed. `supersedes` records lineage; the engine does not rewrite the named files. Handoff `decisions: [{ id, digest }]` resolves only to these files.

Stable identities survive source renames and later surveys through `identities[]` aliases and supersession, not fuzzy matching hidden at projection time. Survey persist reapplies those annotations by id, then reapplies `decisions/` `applies-to` as `status: decided` plus `decision: <id>`. A vanished id becomes an explicit gap, not a silent drop. Plan and review validate the overlay; they do not stamp `as-is`. Re-run `system survey` after editing `identities[]` or `decisions/` `applies-to`.

Correlation is one judgment over the complete extracted Evidence set, the same envelope as slice synthesis (`kind: request | response`) and the same repair loop (`repaired()`, `MAX_REPAIRS`). The request lists included `(source, lead)` Evidence paths plus current `identities[]`. Those paths are surface-grain documents — one endpoint, topic, job, or equivalent each — not one document per model element. The response is the composed `as-is` element and relationship set only: many leads may evidence one service or store. Identities, `target`, `transition-*`, and `decisions/` never enter the judgment envelope.

Before that judgment, the engine counts claims across the included Evidence set against a recorded engine constant beside `MAX_REPAIRS`. Exceeding it is a typed stop (`system-correlation-claim-limit`): this run does not run correlation and does not replace `as-is`. Recovery is the same D2 narrowing as the lead-count gate. RFC-92 can measure spend later; the constants are not operator-configurable and not policy-increasable.

Zero included Evidence is a completed survey, not a failure. Persist writes `as-is: { elements: [], relationships: [] }` deterministically and skips the judgment. A model call over an empty request would hallucinate structure or fail closed. Intent-only Evidence (one intent lead, or intent plus a small constraint set) is a valid correlation request; thin `as-is` is valid. A size-gate stop is not an empty survey: it leaves prior `as-is` in place, or writes none if this was the first run.

Persist loads `system.yaml`, replaces `as-is`, reapplies `identities[]` then `decisions/` `applies-to`, and canonical-writes the file. The kernel typed-parses, rejects unknown fields, checks provenance closure and relationship endpoints, and projects `architecture/as-is.md` and diagrams from that state's digest. It is not a phase machine.

`migration.yaml` inlines `dispositions[]` (`preserve | change | retire | investigate`) and `waves[]` carrying the D9 fields. Those sub-records are not extra files; the handoff references them as `{ id, digest }`.

### D5 — Architecture documents and diagrams are projections

Each architecture document and diagram names one `system.yaml` state and the exact digest of that state. A projection may summarize:

- system context and external actors;
- component or service relationships;
- deployment and environment topology;
- state ownership and data movement;
- critical-journey sequences;
- bounded contexts and organizational ownership;
- evidence coverage, conflicts, and unknowns.

Diagram source is committed beside its rendered form. The initial rendering format is selected during implementation from a deterministic textual notation with stable identifiers and reproducible SVG output. This RFC does not pin a named drawing tool and does not make a drawing-tool binary format authoritative.

Manual architectural interpretation belongs in `decisions/` and model annotations. Editing an SVG or prose projection never changes the system model. Reprojection after a model change either updates the view or fails visibly; stale diagrams cannot silently survive as current.

### D6 — State and temporal behaviour are first-class archaeology

Stateful modernization cannot be inferred safely from request and response examples alone. State-bearing elements record, where evidence permits:

- ownership and authoritative source;
- identifiers and lifecycle;
- read and write paths;
- transaction and consistency boundaries;
- temporal, ordering, idempotency, and retention invariants;
- volume, sensitivity, residency, and recovery constraints;
- coupling to batches, operators, vendors, and deployment topology.

Missing state knowledge is an explicit model gap. A target architecture cannot move or split state unless the migration plan identifies the affected invariants and the evidence or decision that licenses the change.

Runtime captures remain valuable behavioural evidence and later protected oracles under RFC-98. They do not, by themselves, prove internal state semantics or operational safety.

### D7 — Modernization dispositions prevent accidental architecture from becoming requirements

Observed behaviour and structure describe what exists, not automatically what should survive. Before target architecture or migration waves are accepted, material observations receive a modernization disposition:

- `preserve` — required behaviour or constraint that must survive;
- `change` — intentional divergence, with the desired outcome and authority;
- `retire` — behaviour, interface, state, or component intentionally removed;
- `investigate` — insufficient evidence or authority for a responsible decision.

The disposition applies to behaviour and architecture without conflating them. Preserving a business invariant does not require preserving the legacy module, repository, database, or deployment shape that currently implements it.

RFC-98's requirement-level `[divergence]` remains the execution-time conservation mechanism. RFC-104 supplies the reviewed architectural and behavioural decision from which those requirements are later authored.

### D8 — Target architecture includes operable transition states

`architecture/target.md` projects the `target` state in `system.yaml`. It is not architecture authority. The `target` state must explain which forces, invariants, risks, and decisions justify differences from `as-is`.

Where the target cannot be reached atomically, `system.yaml` gains one or more `transition-*` states. Each must be operable and reviewable in its own right. Typical concerns include coexistence, routing, anti-corruption boundaries, data synchronization, backfill, shadow reads, dual writes, reconciliation, operational ownership, and rollback. A one-hop migration has only `as-is` and `target`.

No pattern is mandatory. Strangler replacement, re-platforming, in-place change, consolidation, and replacement are architectural options selected by practitioners against the evidence. Emery preserves the reasoning and checks the resulting plan for coherence; it does not market a proprietary migration pattern.

### D9 — A migration wave is richer than a slice dependency

Each wave in `migration.yaml` records:

- the bounded outcome and acceptance boundary;
- predecessor waves and external preconditions;
- affected, touched, and context-only system elements;
- preserved, changed, retired, and unresolved dispositions;
- target and transition architecture state before and after the wave, as named `system.yaml` states;
- state movement and reconciliation;
- coexistence, cutover, rollback, and operational-readiness requirements;
- verification and conservation expectations;
- material unknowns, commercial assumptions, and authority decisions;
- proposed delivery targets, including targets that must be created before RFC-88 authoring.

A wave may be definition, instrumentation, or evidence-collection work rather than product migration. It still becomes ordinary delivery only if selected and imported into RFC-88.

The migration plan is not a calendar, staffing plan, or fixed-price guarantee. It is the technical and evidential sequence from the reviewed current state toward the reviewed target state.

### D10 — RFC-88 consumes one reviewed wave

`emery system plan` deterministically projects each candidate delivery wave into `handoffs/<digest>.yaml`. The handoff rejects unknown fields, uses stable ordering, and is canonically digested independent of YAML formatting. It contains identities and closed references, not copied architecture prose:

```yaml
version: 1
definition: orders
scope-digest: sha256:…
coverage-digest: sha256:…
system-model-digest: sha256:…
migration-plan-digest: sha256:…
decisions-digest: sha256:…
wave:
  id: extract-orders
  digest: sha256:…
  outcome: Move order ownership behind the reviewed orders service boundary
  architecture:
    before: { id: as-is, digest: sha256:… }
    after: { id: transition-1, digest: sha256:… }
  targets:
    - id: orders-service
      locator: https://github.com/acme/orders-service
      adapter: emery:omnia@1.4.0
  evidence-scopes:
    - source: orders-code
      location: https://github.com/acme/orders
      adapter: typescript
      lead: post-orders
      evidence-digest: sha256:…
      observed-cid: sha256:…
      observed-revision: 7f3a9c1d2e4b
    - source: orders-code
      location: https://github.com/acme/orders
      adapter: typescript
      lead: orders-created
      evidence-digest: sha256:…
      observed-cid: sha256:…
      observed-revision: 7f3a9c1d2e4b
  delivery-mappings:
    - { source: orders-code, lead: post-orders, target: orders-service }
    - { source: orders-code, lead: orders-created, target: orders-service }
  affected-elements: [orders, ordering-api]
  touched-elements: [legacy-orders-store]
  context-elements: [payments, fulfilment]
  dependencies:
    - { id: establish-capture-harness, digest: sha256:… }
  preconditions:
    - { id: orders-service-repository-exists, digest: sha256:… }
  dispositions:
    - { id: orders.state-ownership, digest: sha256:… }
  state-movements:
    - { id: orders-primary-store, digest: sha256:… }
  coexistence:
    - { id: legacy-and-new-read-path, digest: sha256:… }
  cutover:
    - { id: orders-routing-switch, digest: sha256:… }
  rollback:
    - { id: restore-legacy-routing, digest: sha256:… }
  operational-readiness:
    - { id: orders-service-runbook, digest: sha256:… }
  acceptance:
    - { id: orders-cutover-accepted, digest: sha256:… }
  verification:
    - { id: orders-host-profile, digest: sha256:… }
  conservation:
    - { id: orders-critical-replays, digest: sha256:… }
  gaps:
    - { id: historical-order-retention, digest: sha256:… }
  assumptions:
    - { id: peak-order-volume, digest: sha256:… }
  decisions:
    - { id: order-owner-authority, digest: sha256:… }
```

Every `{ id, digest }` entry resolves to one canonical record in the named system model, migration plan, or `decisions/<id>.yaml`. `handoff.decisions[]` resolves only to definition-home decision files. `decisions-digest` is SHA-256 of the canonical encoding of the sorted catalogue `{ id, digest }[]` for every `decisions/<id>.yaml`, not only the wave's named refs; an absent `decisions/` is the empty list. `architecture.before` and `architecture.after` resolve only to named states in `system.yaml` (`as-is`, `target`, or a `transition-*`). `targets[]` carries the reviewed logical target, mutable origin locator, and the adapter identity the operator declared (a name or an exact package pin). `evidence-scopes[]` closes the source location, that same declared adapter identity, surface lead, extracted Evidence, and the observed tree (`observed-cid`, plus `observed-revision` when present) that produced those claims. The handoff copies adapter identity as declared; it does not resolve a name to a pin. A wave typically lists several surface leads from one source; they are a selected subset of estate-survey leads, not broad parents waiting to be split. Observed identity is archaeology provenance copied from the coverage row; it is not the delivery source pin. RFC-88 re-resolves the locator, pins the delivery CID, and fills a declared adapter name to an exact package pin at bind time; a pin already in the handoff is frozen. `delivery-mappings[]` carries reviewed source-to-target assignments without allowing RFC-88 to infer another architecture. Affected elements may experience an observable consequence, touched elements are in the delivery ownership envelope, and context elements remain read-only architectural context; none becomes a slice merely by appearing in the handoff.

The operator records review through:

```text
emery system review <wave> --handoff <sha256:…>
```

The command compare-and-sets the current scope, coverage, system-model, migration-plan, decisions, and wave digests against the handoff, then appends `system.wave.reviewed` to the definition event log. The event payload carries the handoff digest; its ordinary writer identity, and RFC-93 actor identity when available, attribute the act. It grants no product mutation authority and does not replace `plan.execute.started`. Repeating the command for an already reviewed current handoff is a read-only no-op.

The current handoff for wave `W` is the unique `handoffs/<digest>.yaml` whose `wave.id` is `W` and whose `scope-digest`, `coverage-digest`, `system-model-digest`, `migration-plan-digest`, and `decisions-digest` equal the live files. Zero matches means re-run `system plan`. Two matches fail closed. Selection never uses timestamps or “latest reviewed.” A reviewed-but-stale file remains historical; it cannot authorize the live wave.

Changing any covered definition input produces a different handoff digest. An old review fact remains historical but cannot authorize the new current wave. Reviewing the new handoff appends a new fact; it never mutates or aliases the old handoff.

`emery plan author --from <definition-home> --wave <id>` resolves that unique current handoff, requires a matching `system.wave.reviewed` fact for its digest, and verifies every referenced digest. A missing or ambiguous current projection fails closed. Authoring copies the exact handoff and review-event envelope into the change home under their content digests, records the upstream event identity, and imports only the handoff's delivery scope. Later definition drift can invalidate uncommitted work, but it cannot erase the audit record of what delivery authoring originally consumed.

The operator creates any newly decided target repositories before RFC-88 authoring. RFC-88 then re-resolves each imported locator, pins exact target and source CIDs, groups the imported surface leads into conflict domains and buildable slices, and runs focused child survey only when an imported lead is still coarser than a buildable boundary. It does not rediscover the estate, invent the target architecture, treat every system-model element as work, split endpoints that estate survey already emitted, or treat a handoff `observed-cid` as the delivery source pin.

RFC-88 binds the consumed handoff digest into its change-home artifacts and execution coverage. This RFC does not own `discovery.yaml` or `plan.yaml`. A changed covered definition input, architecture decision, or wave invalidates the current handoff; RFC-88 refuses uncommitted closed plans that consumed a now-stale digest. The RFC-88-side wording for this consumption lives in [rfc-104-rfc-88-patches.md](rfc-104-rfc-88-patches.md) until that RFC's next cut absorbs it.

A simple new-system or well-understood change still uses `emery system survey`, `emery system plan`, and `emery system review`, but its definition may be degenerate: explicit intent, constraints, one target-architecture view, and one wave, with no estate-wide source search. `as-is` may be empty (no included Evidence; survey persisted the empty state and skipped correlation) or intent-only (thin `as-is` from that Evidence). Initial `system plan`, when `target` is absent at load, is a proposal judgment — the same envelope as correlation, not a reuse of it — whose request is the live `as-is` (possibly empty), `scope.yaml`, and any included intent or constraint Evidence. Empty or thin `as-is` is a valid request; the judgment may still write `target`, optional `transition-*`, and one wave. Later plans do not run that judgment and do not add named states. Those commands write generated artifacts into the operator-created definition home and produce the immutable handoff and review fact before `plan author`; RFC-88 has no flag-only bypass. Skipping broad archaeology does not mean skipping the reviewed boundary that explains what RFC-88 is delivering.

### D11 — Human review is architectural authority

Models may inventory, correlate, explain, and propose. They cannot decide:

- whether the declared boundary is sufficient for the investment decision;
- whether two apparent elements are the same system responsibility;
- which observed behaviour is intentional;
- which state invariant may change;
- which target or transition architecture is acceptable;
- which migration wave is commercially and operationally responsible.

The first definition pause is an operator review of survey artifacts. It mints no fact. The operator may edit declared inputs — including coverage rows and `identities[]` — and re-run `system survey`. Archaeology may end here: the retained definition-home git tree is the deliverable.

The second pause is the same shape over `system plan` proposals. The operator may edit modernization dispositions, target and transition architecture, and waves, then re-run `system plan` when those inputs changed. Only `emery system review <wave> --handoff <digest>` records architectural authority for RFC-88, by appending `system.wave.reviewed` over the exact current handoff. That act is not inferred from command sequence, file presence, or elapsed time.

There is no `system init` or `system amend` verb. Declared inputs are hand-edited; the next stage validates. Generated Evidence, projections, and handoffs are engine-owned: a direct edit is staleness, not an amendment. Re-running `system survey` or `system plan` is resume. Live field-patch `emery plan amend` is unchanged by this RFC; retirement of those flags is a delivery-loop concern parked in [rfc-104-rfc-88-patches.md](rfc-104-rfc-88-patches.md).

### D12 — Accepted delivery updates the living architecture baseline

RFC-95 publication and archive outcomes identify the accepted target CIDs and wave result. A later definition update — a successor of this RFC, not a v1 DTO — reconciles those results into the system model, architecture projections, migration position, and remaining waves. v1 does not import RFC-95 outcomes.

The baseline does not claim to update itself from code alone. Product results, operational evidence, stakeholder decisions, and documentation remain distinct sources. Write-back projections must not become independent corroboration when surveyed again.

## Implementation cuts

One RFC. Three internal cuts, the same discipline [platform.md](platform.md) already requires of RFC-88. Acceptance remains the loop through `system.wave.reviewed`. Cuts control implementation risk; they are not partial public lifecycles.

1. Definition home, coverage, location materialization, observed-tree provenance, and survey/extract into definition-home Evidence, including emery-adapters surface-grain retargeting and the lead-count gate.
2. Correlation, empty-as-is persist, `system.yaml` as-is, as-is.md, and diagrams (renderer chosen in this cut).
3. `system plan` (dispositions, target/transitions, waves, canonical handoff, adapter identity copied as declared) and `system review`.

## Implementation requirements

- Add a durable definition-home `Layout` (events at `<system>/events/`, not `.emery/events/`; do not reuse `project::config::Layout`) and typed, unknown-field-rejecting DTOs for `scope.yaml`, `coverage.yaml`, `system.yaml` (`identities[]` plus named states), `migration.yaml` (inlined dispositions and waves), `decisions/<id>.yaml`, and the wave handoff. Reuse the existing Evidence document at `evidence/<source>/<lead>.yaml`. Digests are SHA-256 of the canonical YAML encoding. Do not reuse product `DEC-NNNN` Decision Records.
- Add `emery system survey`, `emery system plan`, and `emery system review` guest orchestrations plus read-only status, coverage, model, architecture, and migration projections, each taking optional `--dir`. The definition root is `--dir` if present, else CWD. Relative `--dir` joins the invoked directory. When argv is `system *`, the launcher mounts that directory as `.` and does not walk for `project.yaml` or `.emery/system/`, create the root, or key adapter cache off it. The guest fail-closes if `scope.yaml` is absent. Origin locators materialize into RFC-87 workspaces on the workspaces mount. Do not reuse `--project-dir`. Keep product build and merge operations out of the `system` surface. Do not add `system init`.
- Add `emery system review <wave> --handoff <digest>` as the sole writer of `system.wave.reviewed`; compare-and-set every handoff input before append, make same-handoff re-entry a read-only no-op, and expose the current reviewed handoff as a read-only projection. Current handoff for a wave is the unique `handoffs/<digest>.yaml` whose `wave.id` matches and whose covered input digests (scope, coverage, system-model, migration-plan, decisions) equal the live files; zero matches means re-run `system plan`, two matches fail closed, and selection never uses time. Do not add `system amend`. Declared inputs are hand-edited; `system survey`, `system plan`, and `system review` validate on load and do not overwrite declared locators, identities, or recorded decisions. Do not delete live field-patch `plan amend` flags or their amendment facts.
- Extend the closed event taxonomy and transport projection with the definition-scoped `system.wave.reviewed` payload. Reuse RFC-86's writer/sequence union semantics while keeping definition and change event roots separate.
- Change the source WIT so both `survey` and `extract` take the source key and a prepared `source-input` (RFC-87 read-only workspace or inline content). The wire carries that tree, not the locator and not `observed-cid`. Do not add `focus` or a child-lead response shape; RFC-88 owns that extension over the delivery pin. Migrate the live `plan author` and `emery source survey` / `extract` path onto the same WIT in this cut: the engine prepares input from the binding store it has (`coverage.yaml` for `system survey`, `plan.yaml` until RFC-88 for delivery survey); adapters never recover location from `plan.yaml` or `$SOURCE_DIR`. Resolve each included locator to a concrete tree (local path, or fetch a Git/HTTPS origin), prepare an RFC-87 read-only workspace as `source-input` (or pass inline content), and discard that workspace after extract. Record `observed-cid` (RFC-87 tree identity) and, when the origin is Git and reports one, `observed-revision` on the coverage row as survey-written provenance only when that included source completes; copy those fields onto each handoff `evidence-scopes[]` entry. On access or adapter failure write `survey-error` `{ kind: access | adapter, detail }`, leave that source's Evidence and prior observed tree in place, and do not rewrite `key`, `location`, `adapter`, `disposition`, or `reason`. Persist `coverage.yaml` surgically: load, patch only survey-owned fields on rows this run touched, canonical write. Do not pin the observed CID into the handoff as a delivery source identity; RFC-88 re-resolves the locator and pins the delivery CID at bind time. `extract` continues to return one Evidence document for one terminal lead. After included `survey` completes this run and before any `extract`, count those leads against a recorded engine constant beside `MAX_REPAIRS`; exceeding it is `system-survey-lead-limit` and this run does not extract, correlate, or replace `as-is`. Re-extract every included lead every survey that passes the gate; never cache Evidence. Persist a coverage row for every declared candidate; failed access and unsupported sources remain durable rows.
- In `augentic/emery-adapters`, retarget first-party `survey` / `extract` prompts in the same cut as definition-home Evidence. `survey` emits the adapter's native surfaces at their smallest stable unit and does not cluster toward slices or toward D4 element kinds. TypeScript: one lead per framework surface (HTTP endpoint, topic, job, CLI command, WS handler, outbound call site); delete the LOC collapse and same-source merge. Documentation: one lead per file or top-level section, without calling it a slice. Captures: one lead per handler directory. Intent: one lead (the intent string). Screenshots: one lead per screen. `extract` of each surface emits existing `call` / `contract` / `excerpt` / `type` claims (and adapter-native kinds such as `example`) for what that surface actually has, sufficient for correlation to evidence D4 relationships. Same prompt corpus for `system survey` and live `plan author`; no mode flag. Run `extract` into definition-home Evidence independently of slices. Focused child-lead survey is RFC-88's later extension over the delivery pin, used only when an imported lead is still coarser than a buildable boundary.
- Add one correlation judgment (`kind: request | response`, existing `repaired()` / `MAX_REPAIRS`) whose response is the `as-is` named state only. Before the judgment, count claims across the included Evidence set against a recorded engine constant beside `MAX_REPAIRS`; exceeding it is `system-correlation-claim-limit` and this run does not replace `as-is`. Zero included Evidence persists empty `as-is` deterministically and skips the judgment; intent-only Evidence is a valid request. Persist loads `system.yaml`, replaces `as-is`, reapplies `identities[]` then `decisions/` `applies-to` as `status: decided` plus `decision: <id>`, and canonical-writes. Identities, `target`, `transition-*`, and `decisions/` never enter the judgment envelope. Initial `system plan` when `target` is absent is a separate proposal judgment over live `as-is` (possibly empty), `scope.yaml`, and any included intent or constraint Evidence; it may write `target`, optional `transition-*`, and one wave. Later plans reproject architecture views and a new handoff and do not add named states. The handoff copies declared adapter identity (name or pin) and does not resolve a name to a pin. The deterministic tail checks provenance closure and relationship endpoints and projects architecture views from that state's digest.
- Add deterministic validation for provenance closure, stable element identity, relationship endpoints, coverage accounting, `survey-error` vs declared disposition, decision-record resolution and `applies-to` closure, model/projection digests, disposition authority, transition continuity, wave dependencies, state-movement declarations, and RFC-88 handoff.
- Add deterministic architecture projection from stable model ids. Generated prose and graphics remain non-authoritative views. Choose the textual diagram renderer during implementation; this RFC does not pin a named tool.
- In the same PR as the public `emery system *` surface, update `workflow.md`, `AGENTS.md`, skills, help, tutorials, and the launcher projection in `docs/contributing/cli-architecture.md` so the operator loop starts at `system survey` and `system *` mounts `--dir` or CWD. Do not rewrite those documents in this RFC. Retire in-place `plan author` estate survey when RFC-88 handoff import is authoritative.
- RFC-88, not this RFC, binds the reviewed handoff digest into change-home artifacts and plan coverage, and adds import integration coverage. RFC-103 outcome projection, when implemented, may later include definition coverage; learning remains offline and cannot rewrite the current definition.
- Add integration fixtures spanning several repositories, a shared database, asynchronous messaging, a scheduled job, runtime captures, an inaccessible source, and a target architecture that deliberately changes state ownership.

## Acceptance criteria

RFC-104 is accepted when the definition loop through `system.wave.reviewed` holds. RFC-88 import, live workflow-doc cuts, and RFC-95 write-back are programme successors, not this RFC's gate.

1. A survey over a declared multi-repository boundary records every candidate as `included | excluded | inaccessible | unsupported | unresolved`; no failed candidate disappears from the durable coverage projection. A failed included source writes `survey-error` and keeps its declared `disposition`, `reason`, and `adapter`; it does not become `inaccessible` by engine write.
2. Included code, documentation, contracts, infrastructure descriptions, and runtime captures are surveyed at their recorded URL or path: survey materializes a concrete tree, prepares an RFC-87 read-only workspace, records `observed-cid` (and `observed-revision` when applicable) on the coverage row, and produces system-level Evidence without creating a slice or target workspace. Both `survey` and `extract` receive the source key and that prepared input on the wire; they do not recover location from `plan.yaml` or receive `focus`. A lead count over the engine constant stops as `system-survey-lead-limit` before extract. The handoff copies that observed identity as provenance; RFC-88 re-resolves the locator and pins the delivery CID.
3. The system model represents repositories, runtime components, interfaces, state stores, messaging, jobs, environments, ownership, and context-only dependencies with claim-level provenance, conflicts, inference, and unknowns kept distinct. A TypeScript survey of the fixture emits one lead per framework surface, not one source-level lead and not a slice cluster; correlation still composes service and store elements from those surfaces' claims. A claim count over the engine constant stops as `system-correlation-claim-limit` without replacing `as-is`. Zero included Evidence persists empty `as-is` without a correlation call; intent-only Evidence yields thin `as-is`.
4. Re-running architecture projection over the same named-state digest produces byte-identical diagram source and equivalent rendered views. Editing a projection cannot change authority, and stale projections fail validation.
5. A stateful fixture records ownership, transaction or consistency constraints, temporal invariants, and migration risks. Captured request/response behaviour alone cannot satisfy those fields.
6. A reviewed target architecture changes a legacy state boundary without turning that boundary into a preserved requirement. The migration plan records the responsible decision, transition state, data movement, reconciliation, cutover, and rollback.
7. A migration plan can contain context-only systems and an evidence-collection wave that produce no product migration slice.
8. `system plan` produces a canonical handoff whose target, evidence-scope, delivery-mapping, affected, touched, context, dependency, precondition, disposition, state-movement, coexistence, cutover, rollback, operational-readiness, acceptance, verification, conservation, gap, assumption, and decision references all resolve against its covered definition digests. Handoff `adapter` fields equal the operator-declared name or pin; a name is not rewritten to a package pin.
9. `system review` compare-and-sets the current handoff inputs and appends one `system.wave.reviewed` fact. A stale handoff changes no state, same-handoff re-entry is a read-only no-op, and review grants no product mutation authority.
10. Changing scope, coverage, a definition decision, model identity, a material disposition, target architecture, transition architecture, or selected wave invalidates the current handoff; the old review fact remains historical only.
11. The definition loop can complete as a client-reviewable archaeology package — the retained definition-home git tree — without invoking `plan author`, preparing a product workspace, opening an execution epoch, resolving a target adapter, minting a first-review fact, or providing a `system init`, `system amend`, or `system archive` verb. Editing declared inputs and re-running `system survey` or `system plan` is the amendment and resume path; only `system review` appends `system.wave.reviewed`. A definition with no included Evidence or with intent-only Evidence completes survey (empty or thin `as-is`) and initial `system plan` may still write `target` plus one wave.
12. `cargo make ci` passes with integration coverage for bounded completeness, source-location handling, system Evidence, model correlation and validation, diagrams, state analysis, dispositions, transition planning, handoff canonicalization, review authority, and stale-review refusal.

RFC-88 accepts that `plan author --from` imports exactly one current reviewed handoff plus the matching review-event envelope, projects only that wave into delivery, and refuses uncommitted plans that consumed a now-stale digest. A successor of this RFC, after accepted RFC-95 publication, reconciles resulting CIDs into the living model without treating Emery-generated documentation as independent evidence.

## Rejected alternatives

- **Treat RFC-88 decomposition as system architecture** — conflict domains optimize one delivery wave for build and verification; they do not model context-only systems, state ownership, or transition architecture.
- **Call repository discovery complete estate discovery** — repositories are one evidence class. Runtime, state, infrastructure, interfaces, operations, ownership, and inaccessible evidence remain material.
- **Persist only successful matches** — prevents an operator or client from knowing what the survey omitted and makes completeness unauditable.
- **Let survey rewrite coverage `disposition` or `reason`** — those fields are operator-declared. A failed fetch is this-run `survey-error`, not a durable `inaccessible` claim. `adapter` is required iff `included`; flipping disposition would make the row illegal or force clearing `adapter`. Transient access failure must not stop the next survey from retrying.
- **Recover the source from `plan.yaml` or pass only a lead to `extract`** — a definition home identifies included sources by coverage-row key, and several rows may share an adapter. Both operations take the source key and the same prepared `source-input`. The live delivery path migrates onto that WIT in this cut; the engine may still read `plan.yaml` to prepare input, but adapters do not.
- **Add optional `focus` or a child-lead response to `survey` in this RFC** — estate survey has no parent lead. Focus is a delivery-time call over RFC-88's pinned CID, with a stable child-lead response, and only when an imported lead is still coarser than a buildable boundary. Landing it here either ships a no-op parameter or attributes child Evidence to the observed tree. RFC-88 already owns that extension.
- **Ask adapters to emit slice-sized units** — current TypeScript LOC collapse and same-source merge guess at an engine noun. Adapters emit the smallest surface they can name; the engine groups those leads into slices.
- **Ask adapters to emit system-model kinds** — a lead is not a service, store, journey, or constraint. Those are correlation products (and operator `identities[]`). Inventing them at survey time makes the as-is model a renamed lead inventory and dual-uses the same guess for delivery.
- **Treat a single-endpoint lead as too fine for archaeology** — correlation composes. Extract of that endpoint carries the store it writes and the topic it publishes; many such documents evidence one element. Clustering first destroys the surface identity both consumers need.
- **Give `survey` a definition-vs-delivery mode flag** — surface grain is consumer-agnostic. One prompt corpus feeds `system survey` and live `plan author`.
- **Expand forge namespaces or auto-recognize adapters during archaeology** — completeness is coverage over declared exact locations, not a discovery engine. Namespace expansion and recognition catalogs remain RFC-88 delivery-binding concerns until an engagement cannot enumerate the estate by hand. Recording an observed tree is survey provenance, not delivery binding.
- **Treat a Git URL as sufficient survey identity** — a locator is mutable. Without `observed-cid`, Evidence path anchors are unanchored, re-surveys cannot distinguish origin movement from extraction noise, and the client package cannot say which tree produced the claims. Re-fetch every survey; stamp what was seen.
- **Pin the observed survey CID as the delivery source CID in the handoff** — archaeology re-fetches a moving origin; delivery pins an exact tree. RFC-88 re-resolves at bind time. Observed and delivery CIDs may differ if the origin moved.
- **Generate diagrams directly from prose** — produces persuasive pictures with no stable element identity, provenance, or drift check. Diagrams project the system model.
- **Make diagrams authoritative** — drawing layout is presentation. Architecture authority remains structured and evidence-linked.
- **Store target or transition architecture in `migration.yaml`** — mixes what the architecture is with how a wave gets there. Named states live in `system.yaml`; waves reference them.
- **Keep an internal architecture revision log in `system.yaml`** — git plus digest invalidation is v1 history. Named states (`as-is`, `target`, `transition-*`) are concurrent models, not a version chain.
- **Round-trip `system.yaml` through the correlation answer** — the answer is `as-is` only. Persist loads the file, replaces that named state, reapplies identities and decision overlays, and canonical-writes. Comments and key order are not preserved.
- **Overwrite operator-edited `target` / `transition-*` on re-plan** — write those keys only when `target` is absent at load. Later plans reproject views and a new handoff; they do not add named states.
- **Reuse product `DEC-NNNN` Decision Records in the definition home** — those are slice-promoted baseline ADRs. Definition decisions are operator-authored YAML at `decisions/<id>.yaml` with a canonical digest. The engine never writes them.
- **Author definition decisions as Markdown** — definition digests are SHA-256 of canonical YAML, independent of formatting. A Nygard Markdown file cannot satisfy that.
- **Let correlation emit `status: decided`** — models cannot decide. The persist tail stamps `decided` from `decisions/` `applies-to` after writing `as-is`.
- **Invent a second Evidence schema for archaeology** — `extract` already returns the Evidence document. Persist it at `evidence/<source>/<lead>.yaml`.
- **Enumerate per-kind attributes or split D9 concerns into extra schema files** — closed `kind` and `status` enums plus an open attribute map are enough to run the loop. Wave sub-records stay inlined in `migration.yaml`; the handoff references them.
- **Cache or incrementally reuse Evidence across surveys** — extraction is agent-only and never memoized. Re-extract every included lead every `system survey`.
- **Fuzzy-match element identity at projection time** — stable ids plus operator-edited `identities[]` aliases and supersession. A vanished id is an explicit gap.
- **Give correlation its own phase machine** — one judgment, same envelope and repair loop as slice synthesis, then a deterministic validation/projection tail.
- **Partition correlation or raise the size ceiling from `scope.yaml` or policy** — v1 is one estate-sized judgment with engine constants beside `MAX_REPAIRS`. Recovery is D2: narrow coverage or author another definition home. RFC-92 measures spend; it does not inflate the gate.
- **Send an empty Evidence set through the correlation judgment** — persist empty `as-is` deterministically. A model call over nothing hallucinates structure or fails closed.
- **Fail closed when `as-is` is empty or intent-only** — that is the degenerate new-system path. Initial `system plan` may still write `target` and one wave.
- **Resolve adapter names to package pins when projecting the handoff** — the handoff copies what the operator declared. RFC-88 fills a name at bind time; a pin in the handoff is frozen.
- **Split the three implementation cuts into extra lifecycle RFCs** — cuts control risk inside this RFC. Acceptance remains the loop through `system.wave.reviewed`.
- **Use per-slice `design.md` as the target architecture** — target and transition architecture must constrain slice boundaries before those slices exist.
- **Rename `plan.yaml` a migration plan** — build order omits state movement, coexistence, cutover, rollback, context-only dependencies, and architecture transitions.
- **Preserve every observed behaviour or structure** — converts defects and accidental architecture into requirements and defeats modernization.
- **Let a model choose the target architecture without review** — architectural acceptability, risk, and authority are client and practitioner decisions.
- **Infer wave review from `system plan`, file presence, or later `plan author`** — projection is not an operator decision. Only an exact `system.wave.reviewed` fact makes a handoff eligible for RFC-88 import.
- **Mint a fact for the first definition pause** — archaeology is the retained files. Only wave review authorizes RFC-88 import.
- **Add `system init`** — declared inputs are operator-created. Missing `scope.yaml` / `coverage.yaml` fails closed. Survey writes generated layout into that root.
- **Add `system amend`** — declared inputs are operator-editable; the next stage validates. A typed amend surface waits until hand-edits prove unsafe. Live field-patch `plan amend` is a delivery-loop concern, not this RFC. RFC-88's `plan amend --proposal` is left to that RFC.
- **Add survey/plan checkpoint or attempt files** — re-running the stage is resume. Survey always re-extracts; plan always reprojects from live files.
- **Select the current handoff by timestamp or latest review** — current is the unique digest-named file whose wave id and covered input digests (scope, coverage, system-model, migration-plan, decisions) match the live files. A reviewed-but-stale handoff stays historical.
- **Walk for `project.yaml` or `.emery/system/` on `system *`** — one selector: `--dir` if present, else CWD. Colocated files may live at `.emery/system/`; finding them is `--dir` or `cd`. No ancestor walk for `scope.yaml`.
- **Reuse `--project-dir` as the definition-home selector** — a definition home has no `project.yaml`. RFC-88 keeps `--from` as a read-only extra preopen, not this command root.
- **Create the definition home at launch** — the operator created it. `mkdir` is `system init`.
- **Treat the definition home as a product root for adapter cache** — store, snapshots, and workspaces stay under `$EMERY_HOME`. Bare adapters resolve store / pull-latest.
- **Open coverage locators through the `.` mount** — origin trees enter through RFC-87 workspaces after host-visible materialization.
- **Hand-edit `plan.yaml` and skip to `plan execute`** — a topology or source-binding edit stales refinement. Execute is the authorization epoch, not the review gate.
- **Copy free-form architecture prose into the handoff** — duplicates authority and creates drift. The handoff carries canonical identities and digest-bound references to the reviewed definition.
- **Require perfect knowledge before any wave** — large estates never reach it. Bounded coverage, explicit unknowns, and a responsible first wave are the product.
- **Introduce a permanent product workspace registry** — the definition home stores client architecture, not product checkouts, repository heads, or execution coordination.
- **Specify change-home `discovery.yaml` / `plan.yaml` in this RFC** — those files belong to RFC-88. This RFC invalidates the current handoff; RFC-88 binds the digest.
- **Gate RFC-104 acceptance on RFC-88 import** — the definition loop is complete at `system.wave.reviewed`. Import is RFC-88's cut.
- **Rewrite `workflow.md` and skills in this RFC** — those documents change in the same PR as the public `system` surface, not before it exists.
- **Add `system archive`** — the archaeology package is the client-owned definition-home git tree.
- **Pin a named diagram renderer in this RFC** — D5 leaves the textual notation to implementation.
- **Specify RFC-95 write-back DTOs in v1** — D12 is a successor update path, not a v1 import schema.
