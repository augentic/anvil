# RFC-104: System Archaeology and Migration Planning

> Status: Active product predecessor to [RFC-88](rfc-88-detached-changes.md) in the [Services Delivery Programme](platform.md)
>
> Owns: the durable definition home; the declared system boundary and coverage record; immutable source acquisition; system-wide Evidence extraction and correlation; the reviewed as-is architecture model; architecture-document and diagram projections; modernization dispositions; target and transition architecture; the migration plan; the immutable wave handoff; and its explicit review fact before RFC-88 import.
>
> Builds on [RFC-86](rfc-86-change-facts.md)'s per-writer event and lossless-union shape, [RFC-87](rfc-87-working-trees.md)'s content-addressed values for immutable source inputs, and the existing source-adapter `survey` / `extract` contract. It adds `system.wave.reviewed` to the closed event taxonomy for definition-home writers without making that fact a delivery authorization epoch. [RFC-88](rfc-88-detached-changes.md) consumes one reviewed migration wave and owns delivery decomposition, execution, and accepted target CIDs. [RFC-94](rfc-94-target-readiness.md) assesses whether selected delivery targets can execute that wave. [RFC-98](rfc-98-behavioural-conservation.md) verifies preserved behaviour against protected captures.
>
> This RFC does not add another agent scheduler, target build phase, publication path, or repository registry. It productizes the definition and architecture work Propellerhead needs before responsible modernization delivery begins.

## Intent

Make system archaeology a first-class client deliverable rather than an implicit precondition of slice authoring.

An unfamiliar enterprise system is not a list of repositories waiting to become slices. Its effective architecture spans code, interfaces, state stores, queues, scheduled work, deployment environments, runtime behaviour, operational procedures, ownership, policy, and undocumented dependencies. Some elements will change, some only constrain the change, and some will remain inaccessible or uncertain.

Emery must recover the best defensible account of that system, show the coverage and uncertainty behind it, and help practitioners turn it into a reviewed migration strategy. Only then should RFC-88 decompose one selected wave into buildable slices.

The goal is not perfect knowledge:

> **A survey is complete when every element inside a declared boundary has an explicit coverage disposition, not when Emery claims to know every fact about the estate.**

## Problem

The current source pipeline is delivery-shaped. `survey` emits planning leads, `extract` persists slice-local Evidence, and RFC-88 recursively partitions the result until every terminal domain is buildable. That is useful after a change boundary is understood, but it creates four failures during definition.

**System facts are forced toward slices too early.** Shared state, runtime topology, cross-repository dependencies, operational constraints, and context-only systems may inform many waves without belonging to any one buildable leaf.

**Repository discovery is not system coverage.** A forge namespace does not enumerate deployed services, databases, queues, vendor integrations, environments, operational procedures, or inaccessible evidence. Omitting unmatched candidates and retaining only successful target bindings cannot support a defensible completeness claim.

**Architecture remains prose without durable authority.** Per-slice `design.md` explains implementation after decomposition. It cannot serve as the reviewed as-is model from which system context, component, deployment, data-flow, and journey diagrams are derived.

**A delivery plan is not a migration plan.** Slice dependencies do not express target architecture, transition states, state movement, coexistence, cutover, rollback, or the reason an observed legacy structure should be preserved, changed, retired, or investigated.

## Terms

- A **definition home** is the client-owned, durable root containing one system's archaeology and migration-planning artifacts. It is not a checkout registry, build workspace, or temporary change home.
- A **system boundary** is the reviewed statement of which products, journeys, environments, organizations, and evidence locations the definition engagement covers.
- A **coverage disposition** is the closed value `included | excluded | inaccessible | unsupported | unresolved` for one candidate source or system element, with a reason and authority where applicable.
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
emery system survey  → pin the declared boundary; inventory and extract;
                       write coverage, Evidence, and the as-is system model
operator reviews     → resolve identity, topology, authority, and coverage gaps
emery system plan    → project architecture views; propose target and transition
                       architecture plus bounded migration waves and handoffs
operator reviews     → decide dispositions, architecture, and the first wave
emery system review  → append system.wave.reviewed over the exact handoff
emery plan author    → --from <definition-home> --wave <id>; bind exact delivery
                       targets and sources; decompose only that wave into slices
```

`emery system survey` and `emery system plan` are resumable, review-bounded stages over the same definition home. Exact transport grammar may use a detached root flag rather than the illustrative positional form, but the two operator decisions remain separate: first review what the system is, then review how it should change.

The definition loop may end after either review. An archaeology or readiness engagement is a legitimate completed outcome even when no delivery wave is authorized.

## Decisions

### D1 — The definition home is durable client architecture, not platform coordination

A definition home contains no product checkout and owns no product lifecycle:

```text
<system>/
  scope.yaml
  coverage.yaml
  sources.yaml
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

`scope.yaml` is the reviewed boundary. `coverage.yaml` records every candidate and disposition. `sources.yaml` pins the included source values, origins, CIDs, and exact adapter identities. `system.yaml` is the structured architecture authority. `migration.yaml` records reviewed modernization dispositions, target and transition architecture references, and migration waves. `handoffs/` retains every wave projection named by a review or delivery fact; a handoff is never a second editable migration plan.

The definition home is retained across changes and may live in a client architecture repository. A degenerate single-product definition may live at `<product>/.emery/system/`; a detached multi-repository definition uses its own operator-selected root. In either case, RFC-88 receives the exact root and wave explicitly.

The definition home is not the permanent platform repository rejected by RFC-88: Emery does not use it to route product workspaces, mirror product code, own repository heads, or coordinate execution. It is a durable client artifact for understanding and changing one system.

### D2 — Completeness is bounded and coverage-accounted

The operator declares the decision the survey must support and the boundary it may inspect. A boundary may name repository namespaces and exact repositories, deployed services, interfaces, environments, documentation collections, runtime capture locations, infrastructure descriptions, ownership sources, and explicit exclusions.

Discovery records every candidate it considered, including failed and unsupported candidates. No candidate silently disappears because adapter recognition failed, access was denied, a repository lacked Emery metadata, or a source exceeded a read budget. Each receives one coverage disposition and reason.

`included` means Emery acquired and pinned the source; it does not mean every claim is known. `unresolved` preserves a candidate whose identity or membership needs human judgment. An operator decision may move a candidate between dispositions, and the new coverage digest invalidates downstream model and migration projections.

The coverage summary reports the declared boundary, included evidence classes, inaccessible and unsupported areas, unresolved identities, and material unknowns. External claims say “surveyed within this boundary at this coverage” rather than “complete” without qualification.

### D3 — Source leads are evidence scopes before they are delivery scopes

RFC-104 reuses the source adapter's two operations:

- `survey(Source) → Lead[]` identifies stable, source-local evidence scopes;
- `extract(Lead, Source) → Evidence` extracts every included lead needed by the system model.

A lead is not promised to be a slice. It may represent a service, interface, state store, job, critical journey, architectural document, runtime scenario, or cross-cutting constraint. RFC-104 persists Evidence by `(source, lead)` under the definition home, independent of any change or slice.

Adapters remain source-local. They do not decide global component identity, target architecture, migration waves, or delivery boundaries. Correlation is a separate judgment over the complete extracted Evidence set, followed by deterministic validation.

RFC-88 may request focused child leads when one reviewed wave needs more delivery detail, but it does not repeat estate-wide extraction or reinterpret the system boundary.

### D4 — The system model separates evidence, inference, decision, and unknown

`system.yaml` carries stable elements and relationships. The initial closed element vocabulary covers systems, services or components, repositories, interfaces, data stores, queues or topics, scheduled jobs, deployment units, environments, external actors or systems, and owning groups. Relationships cover containment, deployment, invocation, publication, consumption, read, write, dependency, and ownership.

Every asserted attribute or relationship carries provenance to contributing Evidence claims. Model-assisted correlation may propose identity and relationships, but an unsupported proposal remains `inferred`; it cannot become evidenced by repetition in generated prose. Disagreement remains a conflict, missing information remains unknown, and reviewed stakeholder decisions name their authority.

The model also records context-only elements and relationships. Being relevant to a migration does not imply being modified by it.

Stable identities survive source renames and later surveys through reviewed aliases and supersession, not fuzzy matching hidden at projection time.

### D5 — Architecture documents and diagrams are projections

The as-is architecture document and every diagram name the exact system-model digest they project. A projection may summarize:

- system context and external actors;
- component or service relationships;
- deployment and environment topology;
- state ownership and data movement;
- critical-journey sequences;
- bounded contexts and organizational ownership;
- evidence coverage, conflicts, and unknowns.

Diagram source is committed beside its rendered form. The initial rendering format is selected during implementation from a deterministic textual notation with stable identifiers and reproducible SVG output; this RFC does not make a drawing-tool binary format authoritative.

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

`architecture/target.md` and its model section describe the reviewed intended architecture, not an unconstrained model proposal. It must explain which forces, invariants, risks, and decisions justify differences from the as-is model.

Where the target cannot be reached atomically, the migration plan records one or more transition architectures. Each transition must be operable and reviewable in its own right. Typical concerns include coexistence, routing, anti-corruption boundaries, data synchronization, backfill, shadow reads, dual writes, reconciliation, operational ownership, and rollback.

No pattern is mandatory. Strangler replacement, re-platforming, in-place change, consolidation, and replacement are architectural options selected by practitioners against the evidence. Emery preserves the reasoning and checks the resulting plan for coherence; it does not market a proprietary migration pattern.

### D9 — A migration wave is richer than a slice dependency

Each wave in `migration.yaml` records:

- the bounded outcome and acceptance boundary;
- predecessor waves and external preconditions;
- affected, touched, and context-only system elements;
- preserved, changed, retired, and unresolved dispositions;
- target and transition architecture state before and after the wave;
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
sources-digest: sha256:…
system-model-digest: sha256:…
migration-plan-digest: sha256:…
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
      source-cid: sha256:…
      adapter: emery:typescript@1.2.0
      lead: orders-api
      evidence-digest: sha256:…
  delivery-mappings:
    - { source: orders-code, lead: orders-api, target: orders-service }
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

Every `{ id, digest }` entry resolves to one canonical record in the named system model or migration plan. `targets[]` carries the reviewed logical target, mutable origin locator, and exact adapter requirement; RFC-88 resolves the locator to an exact Git revision and CID. `evidence-scopes[]` closes the source value, adapter identity, lead, and extracted Evidence that may inform delivery. `delivery-mappings[]` carries reviewed source-to-target assignments without allowing RFC-88 to infer another architecture. Affected elements may experience an observable consequence, touched elements are in the delivery ownership envelope, and context elements remain read-only architectural context; none becomes a slice merely by appearing in the handoff.

The operator records review through:

```text
emery system review <wave> --handoff <sha256:…>
```

The command compare-and-sets the current scope, coverage, sources, system-model, migration-plan, and wave digests against the handoff, then appends `system.wave.reviewed` to the definition event log. The event payload carries the handoff digest; its ordinary writer identity, and RFC-93 actor identity when available, attribute the act. It grants no product mutation authority and does not replace `plan.execute.started`. Repeating the command for an already reviewed current handoff is a read-only no-op.

Changing any covered definition input produces a different handoff digest. An old review fact remains historical but cannot authorize the new current wave. Reviewing the new handoff appends a new fact; it never mutates or aliases the old handoff.

`emery plan author --from <definition-home> --wave <id>` resolves the single current handoff projection for that wave, requires a matching `system.wave.reviewed` fact, and verifies every referenced digest. A missing or ambiguous current projection fails closed rather than selecting by timestamp. Authoring copies the exact handoff and review-event envelope into the change home under their content digests, records the upstream event identity, and imports only the handoff's delivery scope. Later definition drift can invalidate uncommitted work, but it cannot erase the audit record of what delivery authoring originally consumed.

The operator creates any newly decided target repositories before RFC-88 authoring. RFC-88 then pins exact target and source CIDs, obtains any focused delivery leads, and decomposes the wave into conflict domains and buildable slices. It does not rediscover the estate, invent the target architecture, or treat every system-model element as work.

`discovery.yaml` and `plan.yaml` carry the handoff digest plus its definition, system-model, migration-plan, and wave identities. The plan digest transitively binds the handoff into execution coverage. A changed covered definition input, architecture decision, or wave invalidates the handoff and every uncommitted closed plan that consumed it.

A simple new-system or well-understood change still uses `emery system survey`, `emery system plan`, and `emery system review`, but its definition may be degenerate: explicit intent, constraints, one target-architecture view, and one wave, with no estate-wide source search. Those commands create the ordinary definition home, immutable handoff, and review fact before `plan author`; RFC-88 has no flag-only bypass. Skipping broad archaeology does not mean skipping the reviewed boundary that explains what RFC-88 is delivering.

### D11 — Human review is architectural authority

Models may inventory, correlate, explain, and propose. They cannot decide:

- whether the declared boundary is sufficient for the investment decision;
- whether two apparent elements are the same system responsibility;
- which observed behaviour is intentional;
- which state invariant may change;
- which target or transition architecture is acceptable;
- which migration wave is commercially and operationally responsible.

The two definition review seams record decisions and authority; they are not inferred from command sequence or elapsed time. An automation may prepare projections, but no generated target architecture or migration wave becomes an RFC-88 input without an exact `system.wave.reviewed` fact over its handoff.

### D12 — Accepted delivery updates the living architecture baseline

RFC-95 publication and archive outcomes identify the accepted target CIDs and wave result. A subsequent definition update reconciles those results into the system model, architecture projections, migration position, and remaining waves.

The baseline does not claim to update itself from code alone. Product results, operational evidence, stakeholder decisions, and documentation remain distinct sources. Write-back projections must not become independent corroboration when surveyed again.

## Implementation requirements

- Add a durable definition-home layout and typed, unknown-field-rejecting DTOs for scope, coverage, system model, modernization dispositions, architecture revisions, migration plan, and wave handoff.
- Add `emery system survey` and `emery system plan` guest orchestrations plus read-only status, coverage, model, architecture, and migration projections. Keep product build and merge operations out of the `system` surface.
- Add `emery system review <wave> --handoff <digest>` as the sole writer of `system.wave.reviewed`; compare-and-set every handoff input before append, make same-handoff re-entry a read-only no-op, and expose the current reviewed handoff as a read-only projection.
- Extend the closed event taxonomy and transport projection with the definition-scoped `system.wave.reviewed` payload. Reuse RFC-86's writer/sequence union semantics while keeping definition and change event roots separate.
- Resolve included location sources once to RFC-87 CIDs and pin exact adapter identities. Preserve every candidate disposition, including failed recognition and access.
- Generalize plan-time leads from slice-sized units to source-local evidence scopes. Run `extract` into definition-home Evidence independently of slices and retain focused child-lead support for RFC-88.
- Add deterministic validation for provenance closure, stable element identity, relationship endpoints, coverage accounting, model/projection digests, disposition authority, transition continuity, wave dependencies, state-movement declarations, and RFC-88 handoff.
- Add deterministic architecture projection and diagram rendering from stable model ids. Generated prose and graphics remain non-authoritative views.
- Extend RFC-88 binding and plan coverage with the exact reviewed handoff digest and its selected definition, system-model, migration-plan, and wave identities.
- Extend RFC-103 outcome projection, when implemented, with definition coverage, architecture amendment rate, migration-wave assumptions, and post-wave model drift. Learning remains offline and cannot rewrite the current definition.
- Add integration fixtures spanning several repositories, a shared database, asynchronous messaging, a scheduled job, runtime captures, an inaccessible source, and a target architecture that deliberately changes state ownership.

## Acceptance criteria

1. A survey over a declared multi-repository boundary records every candidate as `included | excluded | inaccessible | unsupported | unresolved`; no failed candidate disappears from the durable coverage projection.
2. Included code, documentation, contracts, infrastructure descriptions, and runtime captures resolve to immutable values and produce system-level Evidence without creating a slice or target workspace.
3. The system model represents repositories, runtime components, interfaces, state stores, messaging, jobs, environments, ownership, and context-only dependencies with claim-level provenance, conflicts, inference, and unknowns kept distinct.
4. Re-running architecture projection over the same model digest produces byte-identical diagram source and equivalent rendered views. Editing a projection cannot change authority, and stale projections fail validation.
5. A stateful fixture records ownership, transaction or consistency constraints, temporal invariants, and migration risks. Captured request/response behaviour alone cannot satisfy those fields.
6. A reviewed target architecture changes a legacy state boundary without turning that boundary into a preserved requirement. The migration plan records the responsible decision, transition state, data movement, reconciliation, cutover, and rollback.
7. A migration plan can contain context-only systems and an evidence-collection wave that produce no product migration slice.
8. `system plan` produces a canonical handoff whose target, evidence-scope, delivery-mapping, affected, touched, context, dependency, precondition, disposition, state-movement, coexistence, cutover, rollback, operational-readiness, acceptance, verification, conservation, gap, assumption, and decision references all resolve against its covered definition digests.
9. `system review` compare-and-sets the current handoff inputs and appends one `system.wave.reviewed` fact. A stale handoff changes no state, same-handoff re-entry is a read-only no-op, and review grants no product mutation authority.
10. RFC-88 imports exactly one current reviewed handoff plus the matching review-event envelope under content digests, binds its handoff, system-model, migration-plan, and wave digests, and projects only that wave into delivery conflict domains. Unselected system elements do not become slices.
11. Changing scope, coverage, sources, model identity, a material disposition, target architecture, transition architecture, or selected wave invalidates the handoff and every uncommitted closed plan that consumed it; the old review fact remains historical only.
12. The definition loop can complete and archive a client-reviewable archaeology package without invoking `plan author`, preparing a product workspace, opening an execution epoch, or resolving a target adapter.
13. After an accepted RFC-95 publication, a new survey can reconcile the resulting CIDs into the living model without treating Emery-generated documentation as independent evidence.
14. `cargo make ci` passes with integration coverage for bounded completeness, source pinning, system Evidence, model correlation and validation, diagrams, state analysis, dispositions, transition planning, handoff canonicalization, review authority, stale-review refusal, and RFC-88 import.

## Rejected alternatives

- **Treat RFC-88 decomposition as system architecture** — conflict domains optimize one delivery wave for build and verification; they do not model context-only systems, state ownership, or transition architecture.
- **Call repository discovery complete estate discovery** — repositories are one evidence class. Runtime, state, infrastructure, interfaces, operations, ownership, and inaccessible evidence remain material.
- **Persist only successful matches** — prevents an operator or client from knowing what the survey omitted and makes completeness unauditable.
- **Generate diagrams directly from prose** — produces persuasive pictures with no stable element identity, provenance, or drift check. Diagrams project the system model.
- **Make diagrams authoritative** — drawing layout is presentation. Architecture authority remains structured and evidence-linked.
- **Use per-slice `design.md` as the target architecture** — target and transition architecture must constrain slice boundaries before those slices exist.
- **Rename `plan.yaml` a migration plan** — build order omits state movement, coexistence, cutover, rollback, context-only dependencies, and architecture transitions.
- **Preserve every observed behaviour or structure** — converts defects and accidental architecture into requirements and defeats modernization.
- **Let a model choose the target architecture without review** — architectural acceptability, risk, and authority are client and practitioner decisions.
- **Infer wave review from `system plan`, file presence, or later `plan author`** — projection is not an operator decision. Only an exact `system.wave.reviewed` fact makes a handoff eligible for RFC-88 import.
- **Copy free-form architecture prose into the handoff** — duplicates authority and creates drift. The handoff carries canonical identities and digest-bound references to the reviewed definition.
- **Require perfect knowledge before any wave** — large estates never reach it. Bounded coverage, explicit unknowns, and a responsible first wave are the product.
- **Introduce a permanent product workspace registry** — the definition home stores client architecture, not product checkouts, repository heads, or execution coordination.
