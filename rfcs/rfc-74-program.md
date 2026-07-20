# Migration Programs and Durable Progress

> Status: Draft
>
> Owns: a migration-sized umbrella above changes, repository-by-repository scheduling, target selection policy, approved adapter and topology decisions, durable progress, re-entry, and migration audit projections.
>
> Depends on: [Adapter Descriptors and Registry Trust](rfc-71-discovery.md) (Stage 1) and [Migration Intake and Source Selection](rfc-72-migration.md). [Managed Workspace Materialization](rfc-73-materialization.md) is optional for the walking skeleton — see [Sequencing](#sequencing).
>
> Supersedes after reconciliation: [Migration Ledger and Slice Mapping](future/rfc-22-ledger.md).

## Abstract

Add a **migration program** for work that spans many repositories and many Specify changes.

An operator supplies repository membership, desired-state intent, and policy. Specify profiles the sources, recommends source adapters, classifies workload kinds, proposes target projects and target adapters, and presents one reviewable program plan. After approval, it works through the migration in bounded batches. Each work item still uses the existing change and slice loop:

```text
migration program
  → change
    → slice refine
    → slice build
    → slice merge
  → publication handoff
  → next change
```

The migration program coordinates over `change.md`, `plan.yaml`, Gate 1, slice metadata, target build, and merge.

## Vision

The intended operator experience is:

```bash
specify migration create legacy-modernization \
  --repo-file repos.txt \
  --intent migration.md \
  --policy migration-policy.yaml

specify migration inspect legacy-modernization
specify migration approve legacy-modernization
specify migration execute legacy-modernization
specify migration status legacy-modernization
```

From those inputs Specify can:

- identify repository languages, frameworks, workload kinds, and supporting documents;
- choose compatible source adapters;
- propose whether each workload becomes an Omnia service, Vectis application, contracts project, or another allowed target;
- install exact approved adapter components;
- initialize target projects through normal CLI writers;
- materialize one target workspace at a time;
- author and execute ordinary changes;
- stop at required human gates;
- retain durable progress and resume after interruption.

“Intelligently chooses” means evidence-backed recommendation under policy.

## Motivation

A change is the right umbrella for one approved set of slices. An 80-repository modernization needs a longer-lived queue:

- repositories enter at different times;
- profiling and adapter selection happen before any one change;
- target topology may need approval before project initialization;
- work should often proceed repository by repository;
- publication can block one item while another is ready;
- migration progress must survive archived changes;
- operators need a cumulative answer to pending, active, blocked, migrated, and abandoned sources.

The deferred migration-ledger RFC proposed cross-change state but did not define the program that owns scheduling and approval. A ledger without a coordinator remains an audit index; a coordinator without journal-derived progress creates duplicate lifecycle authority. This RFC defines both together.

## Goals

1. Coordinate many repositories through many ordinary Specify changes.
2. Preserve change and slice lifecycle authority.
3. Separate observed repository profile from desired target architecture.
4. Approve source bindings, target routing, and target adapter choices before execution.
5. Support serial execution by default and bounded parallel execution later.
6. Stop cleanly at Gate 1, publication, ambiguity, or failure.
7. Project durable progress from journals, plan archives, and approved program decisions.
8. Resume deterministically after process or machine interruption.
9. Make every automatic decision and policy exception auditable.

## Non-goals

- Replacing the existing change or slice loop.
- Auto-approving Gate 1 by default.
- Publishing branches or merging pull requests in the first version.
- Becoming a general project-management or developer-portal system.
- Treating repository completion as proof that every business capability was migrated.
- Using a model as lifecycle authority.
- Running several writers against one target project.
- Inferring target architecture from legacy implementation alone.

## New workflow noun

A **migration program** is a workspace-scoped, domain-specific umbrella coordinating many changes over a stable source catalogue and target registry.

Use the terms distinctly:

- **migration program** — cross-change modernization queue and durable progress;
- **change** — operator-defined umbrella owning one `change.md` and `plan.yaml`;
- **slice** — unit flowing through `refine → build → merge`.

Program items track program progress; slices keep their ordinary lifecycle.

The noun is deliberately migration-specific. The roadmap's generic **initiative** (RM-20) may generalize it later; a concrete migration profile teaches what a generic initiative needs far better than designing the abstraction first. The renaming this may eventually cost — CLI namespace, artifact paths, journal ids — is accepted in exchange for not speculating now.

## Decision

### Program artifacts

Program state lives under:

```text
.specify/migrations/<name>/
├── brief.md
├── program.yaml
├── recommendations/
├── approvals/
└── progress.yaml
```

Ownership:

- `brief.md` — operator-authored desired-state intent and constraints;
- `program.yaml` — CLI-owned work-item queue and approved topology;
- `recommendations/` — immutable discovery reports;
- `approvals/` — immutable approval records;
- `progress.yaml` — regenerable projection over journals, active plans, and archives.

Only `brief.md` is hand-edited. Every other path has a CLI writer.

The brief carries desired state in prose, not adapter names — the judgment legs read it as intent, and policy constrains what that intent may bind:

```markdown
# Legacy modernization — desired state

Consolidate the billing stack onto our service platform. Services stay
in their existing repositories (in-place). The two legacy mobile apps
(iOS + Android) are replaced by one cross-platform application in a new
repository; feature parity is defined by the screenshots and captured
flows, not the legacy code.

Constraints: no new web frontends this program; contract files move to
the shared contracts project rather than staying with their services.
```

### Program shape

The initial `program.yaml` shape:

```yaml
version: 1
name: legacy-modernization
lifecycle: pending
policy: sha256:...
sources:
  - legacy-billing
  - legacy-mobile
intent: brief.md
strategy:
  order: dependency
  max-active-changes: 1
  approval: per-change
items:
  - key: legacy-billing
    source: legacy-billing
    source-bindings:
      - code
      - docs
    workload-kind: web-service
    target:
      mode: in-place
      project: billing
      adapter: specify:omnia@1.3.0
      platforms: [core]
    depends-on: []
    status: ready
  - key: legacy-mobile
    source: legacy-mobile
    source-bindings:
      - code
      - screenshots
    workload-kind: mobile-app
    target:
      mode: greenfield
      project: customer-app
      adapter: specify:vectis@1.2.0
      platforms: [core, ios, android]
    depends-on:
      - legacy-billing
    status: pending
```

Item status is coordination state:

- `pending` — recommendation or approval is incomplete;
- `ready` — approved and eligible for change authoring;
- `active` — owns an active change;
- `publication` — change drained locally and awaits operator publication;
- `blocked` — requires a named recovery action;
- `completed` — all required work and publication are recorded;
- `abandoned` — operator ended the item with a reason.

Item status is program coordination; slice lifecycle remains the per-slice authority.

### Program lifecycle

The statuses above are per-item coordination state. The program itself carries a small closed lifecycle:

- `pending` — created; inspection or Program Gate M1 approval is incomplete;
- `approved` — Gate M1 stamped; execution is permitted;
- `finalized` — every item completed or abandoned, publication confirmed, program archived;
- `abandoned` — the operator ended the program with a reason.

Execution state is projected from items and the journal, not duplicated as a program-level `active` value. Program lifecycle is written only by `specify migration approve|finalize|abandon`; there is no other writer.

### Program planning

`specify migration inspect` runs deterministic and judgment legs in order:

1. validate the source catalogue and target registry;
2. materialize and profile each source revision;
3. recommend source adapter bindings;
4. classify observed workload signals;
5. read the operator brief and target policy;
6. recommend target mode, project, adapter, and platforms;
7. detect dependencies and shared-source relationships;
8. produce recommendation reports;
9. write a pending program proposal.

Workload classification is observational. Target recommendation is desired-state reasoning constrained by policy.

Examples:

- a Node HTTP service may be classified `web-service`, while target policy recommends `omnia`;
- a React frontend may be classified `web-frontend`, while policy may require an existing web target, mark it unsupported, or route it to a future adapter;
- an iOS and Android pair may be classified as related mobile apps, while intent recommends one Vectis target;
- OpenAPI files may bind the documentation source adapter and route contract work to the contracts target without making the source repository itself a contracts project.

Unknown or unsupported target cases remain blocked with explicit missing capability.

### Target selection policy

Target selection is normative, not observational. It uses:

- migration intent;
- organization target policy;
- desired workload kind;
- required platforms;
- greenfield or in-place mode;
- target repository constraints;
- design documents and approved decisions;
- target adapter capabilities, from the descriptors in [Adapter Descriptors and Registry Trust](rfc-71-discovery.md).

Legacy implementation facts are supporting evidence, never the sole authority.

The default policy always requires approval before a target adapter binding is written. An organization may opt into automatic target binding only with a checked-in policy that maps desired-state facts to allowed exact package ranges:

```yaml
version: 1
targets:
  web-service:
    allow: [specify:omnia@^1]
    preferred: specify:omnia@1.3.0
  mobile-app:
    allow: [specify:vectis@^1]
    required-platforms: [core, ios, android]
approval:
  target-binding: required
```

Workload kinds are the closed taxonomy owned by [Adapter Descriptors and Registry Trust](rfc-71-discovery.md); the policy schema is owned here, where it is consumed.

### Program approval

Approval is a distinct gate before any target project is initialized or adapter is installed:

```bash
specify migration approve <name>
```

Approval covers:

- source membership and pinned revisions;
- approved source adapter bindings;
- workload classifications;
- source-to-target mapping;
- target project mode and location;
- exact target adapter and platform set;
- dependency order;
- execution and human-approval policy.

The approval record carries the input, profile, descriptor, policy, and recommendation digests. Any change invalidates approval for affected items — an immutable record under `approvals/`:

```yaml
# .specify/migrations/legacy-modernization/approvals/m1-2026-07-19.yaml
gate: m1
program: legacy-modernization
actor: operator
approved: 2026-07-19T20:45:03Z
inputs:
  brief: sha256:6a90…
  policy: sha256:77aa…
items:
  - key: legacy-billing
    source-revision: 2f3c…
    profile: sha256:4f15…
    recommendations: [rec-2026-07-19-legacy-billing-01]
    target: { project: billing, adapter: "specify:omnia@1.3.0", component-digest: "sha256:b3e7…" }
  - key: legacy-mobile
    source-revision: 77c1…
    profile: sha256:2e08…
    recommendations: [rec-2026-07-19-legacy-mobile-01]
    target: { project: customer-app, adapter: "specify:vectis@1.2.0", component-digest: "sha256:71c8…" }
```

If `legacy-mobile`'s profile digest later changes, only that item drops back to `pending`; `legacy-billing`'s approval stands.

Gate M1 is also the source-binding approval for program items: sources consumed through a program are covered by this review rather than a separate `specify source approve` ([Migration Intake and Source Selection](rfc-72-migration.md#recommendation-and-approval)). One review covers membership, bindings, and topology, keeping the operator at three meaningful stops — M1, per-change Gate 1 (individually or batched), and publication.

This is **Program Gate M1**. Ordinary Gate 1 (`plan.lifecycle: approved`) remains a separate stamp on each change.

### Applying approved topology

After Program Gate M1, Specify may:

- hydrate exact approved source and target adapters;
- add approved greenfield projects to the registry through its writer;
- materialize managed target slots;
- run `specify init` in uninitialized targets with approved adapter and platforms;
- validate existing project configuration against the approved recommendation.

For an uninitialized target, the approved item carries an initialization proposal — project name and description, exact target adapter selector, declared platforms, and mode — applied through the normal `specify init` writer inside the slot.

Existing target configuration wins. A mismatch returns the program item to `blocked` with an amendment workflow.

### Creating ordinary changes

For the next ready item, the program creates one ordinary change:

1. select approved catalogue source bindings;
2. materialize the target slot and acquire its lease;
3. invoke `plan author` with the program item's intent and sources;
4. record the generated change name on the program item;
5. stop at the plan's normal `pending` lifecycle.

The operator reviews and stamps Gate 1:

```bash
specify plan transition <change> approved
specify migration execute <program>
```

An organization policy may permit pre-approved classes of changes in the future, but that must be a separate operator-owned approval mechanism that calls the same transition and records the actor and policy.

### Gate 1 at scale

An 80-repository program means roughly 80 Gate 1 stamps. The default remains one review per change, but the program adds a batch surface so review cost scales with operator attention rather than keystrokes:

```bash
specify migration approve-changes <name> [--item <key>...]
```

`approve-changes` projects every pending Gate 1 plan in the program — change name, slices, bound sources, target project — requires one explicit operator confirmation for the projected batch, then invokes the same `specify plan transition <change> approved` writer per change, recording the operator actor and the batch context in the journal for every stamp. It introduces no new lifecycle writer and no auto-approval: an operator is always in the loop, and a change excluded from the projection (validation findings, an amended plan, an invalidated recommendation) is skipped and reported rather than stamped.

```console
$ specify migration approve-changes legacy-modernization
2 pending Gate 1 plans in legacy-modernization:

  migrate-billing      → billing       3 slices  sources: legacy-billing/code, legacy-billing/docs
  migrate-customer-app → customer-app  5 slices  sources: legacy-mobile/code, legacy-mobile/screenshots

skipped (not stamped):
  migrate-reporting    plan amended after projection — re-run to include

Approve the 2 projected plans? [y/N] y
stamped migrate-billing approved (actor: operator, batch: legacy-modernization/2026-07-19)
stamped migrate-customer-app approved (actor: operator, batch: legacy-modernization/2026-07-19)
```

### Workspace routing

The current guest-routed `plan execute` refuses workspace-root execution. The serial coordinator therefore begins with the supported hand-driven sequence:

```text
plan status / plan next at the workspace
  → slice refine in the selected project context
  → slice build in the selected project context
  → slice merge in the selected project context
  → repeat
```

The coordinator invokes those typed CLI operations and relays their structured outcomes.

Workspace-root `plan execute` may later absorb this routing and become the coordinator's implementation detail — an optimization after routing parity is proven.

### Execution strategy

Default strategy:

- dependency order;
- one active change;
- one target project lease;
- stop on ambiguity, failure, Gate 1, or publication handoff.

Dependency order is schema-ready but detection-deferred: the first version executes items in listed order with `depends-on` empty unless the operator authors dependencies through `migration amend`. Automatic dependency and shared-source detection (planning step 7) waits until a real migration exhibits cross-repository ordering constraints — the fields exist in the schema so Gate M1 covers them, but no detector ships speculatively.

Later bounded parallelism may run independent items when:

- their target projects differ;
- their dependency closures do not overlap;
- source snapshots are immutable;
- each target has an independent lease;
- policy permits the resulting model and tool concurrency.

The scheduler selects work. In a single-project target, `plan execute` drives the selected change. In a workspace, the coordinator drives the equivalent `plan next → refine → build → merge` sequence until workspace-root `plan execute` supports project routing.

### Stop conditions

`migration execute` returns a structured next action:

- `approve-program`;
- `approve-change <name>`;
- `resolve-recommendation <item>`;
- `materialize <project>`;
- `repair <change> <slice>`;
- `publish <project> <branch>`;
- `resume <change>`;
- `drained`.

The next action is the whole return value — structured for the coordinator loop and any wrapping automation, mirroring the `plan status` projection shape:

```console
$ specify migration execute legacy-modernization --format json
{
  "program": "legacy-modernization",
  "stopped": "publication-handoff",
  "next": {
    "action": "publish",
    "item": "legacy-billing",
    "project": "billing",
    "branch": "specify/migrate-billing/billing"
  },
  "resumable": true
}
```

Unchanged external state is a wait condition, not a failure. A program waiting for publication remains resumable: re-running `migration execute` after the operator pushes and merges the branch records the publication confirmation and moves on to `legacy-mobile`.

### Durable progress projection

`progress.yaml` is a materialized view, not a second event log. It projects:

- approved program items;
- source revisions and bindings;
- active `plan.yaml` entries;
- slice metadata;
- journal events;
- archived plans and slice outcomes;
- publication confirmations;
- abandonment records.

The projection records per item:

- current coordination status;
- active or archived change;
- completed slices;
- source leads and target project mappings;
- target adapter and platforms;
- merge commit or change-set identity;
- last event and next action;
- blocking diagnostic fingerprints;
- publication reference when supplied.

One item mid-program, as projected:

```yaml
# progress.yaml (excerpt) — regenerable; every line traces to a durable input
items:
  - key: legacy-billing
    status: publication
    change: migrate-billing            # archived: .specify/archive/migrate-billing
    slices:
      - { name: billing-api,   outcome: merged, commit: 9f60… }
      - { name: billing-jobs,  outcome: merged, commit: c2a4… }
      - { name: billing-admin, outcome: dropped, reason: out-of-scope }
    target: { project: billing, adapter: "specify:omnia@1.3.0" }
    scope:
      covered: [billing-api, billing-jobs]
      residual: [billing-admin]        # abandoned with reason, stays visible
    last-event: slice.merge.succeeded
    next: publish billing specify/migrate-billing/billing
    publication: null
```

`specify migration rebuild-progress <name>` must reproduce the projection byte-for-byte from durable inputs.

### Completion semantics

A source repository is not automatically “migrated” because one slice merged.

Program completion uses explicit item scope:

- the approved item declares which leads or capability groups are in scope;
- completed changes cover those groups;
- unresolved leads remain visible;
- the operator may amend scope or abandon a lead with a reason;
- publication must be confirmed for every required target.

The ledger can therefore distinguish:

- partially migrated;
- locally complete, awaiting publication;
- completed;
- abandoned with residual scope;
- superseded by another program item.

### Source-to-target mapping

Mapping is derived from approved items and completed changes rather than a free-standing audit label:

- one source to one target;
- many sources to one target;
- one source to many targets;
- greenfield target with no legacy source.

Each change still binds one slice to one project. Many-to-many migration shapes are represented by several items or slices, preserving the existing invariant.

### CLI surface

```bash
specify migration create <name> [inputs...]
specify migration inspect <name>
specify migration amend <name> ...
specify migration approve <name>
specify migration next <name>
specify migration execute <name>
specify migration status <name> [--format json]
specify migration rebuild-progress <name>
specify migration abandon <name> <item> --reason <text>
specify migration finalize <name>
```

`finalize` requires every item to be completed or explicitly abandoned and every completed target to have publication confirmation. It archives the program while preserving source catalogue and registry membership.

## Policy

Migration policy is a checked-in, reviewed input. It may constrain:

- trusted adapter registries and publishers;
- allowed target adapters and version ranges by workload kind;
- required platforms;
- in-place versus greenfield target modes;
- repository naming and branch conventions;
- sole-candidate source auto-approval;
- maximum active changes;
- required human gates;
- unsupported workload handling;
- publication requirements.

Policy can narrow operator choices but cannot weaken component digest verification or lifecycle validation.

## Observability

Add journal events for:

- program created, inspected, amended, approved, and finalized;
- recommendation accepted or rejected;
- item readied, activated, blocked, resumed, completed, or abandoned;
- project initialized;
- adapter hydrated;
- publication requested and confirmed.

Events carry program, item, change, project, source, and adapter identities where applicable. The migration status surface is a projection over these events and workflow artifacts.

The journal's `Event` / `EventKind` taxonomy is closed (`crates/project/src/journal.rs`). Each stage that introduces program events lands the matching engine variants — kebab-case wire ids such as `migration.program.created` and `migration.item.activated` — in the same change. Events are never emitted around the closed taxonomy through a side channel.

## Sequencing

The header dependency chain describes ownership, not build order. The first vertical slice is a walking skeleton that proves the coordinator semantics — program planning, Gate M1, stop conditions, re-entry, and progress projection — over the smallest substrate:

- [Adapter Descriptors and Registry Trust](rfc-71-discovery.md) Stage 1 only: the static first-party descriptor index, with no registry search or trust infrastructure;
- [Migration Intake and Source Selection](rfc-72-migration.md) Stages 1–2: catalogue, snapshots, profiling, and recommendation over that index;
- operator-prepared workspace slots: [Managed Workspace Materialization](rfc-73-materialization.md) is skipped entirely at first;
- serial execution through the existing plan and slice verbs.

Managed materialization (RFC-73) and registry discovery (RFC-71 Stage 3) backfill skeleton friction as real migrations demand them. Omnia lazy guest resolution ([Self-Assembling Wasm Deployment](rfc-70-deployment.md)) is **expected substrate** for any program that installs adapters mid-run — hydrate via `ensure_*`, dispatch via Omnia's generic miss-hook. The walking skeleton (Stages 1–2 below) can still run on a pre-hydrated first-party store.

A working rule for the skeleton: leave `program.yaml`, the item-status enum, and `progress.yaml` flexible until at least one real migration has been driven over the source catalogue by hand. The first real run informs those schemas more than any design review; the schemas in this RFC are proposals to be corrected by that run.

Throughout, the coordinator consumes only deployment-neutral capabilities — the materialization lease, the seam provider, `wasi-model`, the journal — so the hosted product (roadmap RM-18) is a backend swap under the same coordinator.

## Implementation stages

### Stage 1 — Program plan and approval

1. Define migration program artifacts and writers.
2. Integrate source profiles and adapter recommendation reports.
3. Add target topology recommendations under policy.
4. Add Program Gate M1 and approval invalidation.

### Stage 2 — Serial coordinator

1. Add `migration next|execute|status`.
2. Create one ordinary change for one ready item.
3. Stop at ordinary Gate 1.
4. Resume through single-project `plan execute` or the workspace breakout sequence.
5. Hand off publication and release the workspace lease.

### Stage 3 — Progress projection

1. Project item status from journals, active plans, and archives.
2. Add scope coverage and partial-migration reporting.
3. Add byte-stable progress rebuilding.
4. Reconcile and replace the deferred migration ledger.

### Stage 4 — Bounded parallelism

1. Build an item dependency graph.
2. Schedule only disjoint target leases.
3. Bound model, materialization, and build concurrency independently.
4. Preserve stable status and event ordering.

### Stage 5 — Hosted and forge integration

1. Use hosted working-tree leases.
2. Add operator-approved push and pull-request providers.
3. Wait durably for CI, review, and merge.
4. Confirm publication before item completion.

## Acceptance criteria

1. A migration program can be created from a repository list, intent brief, and policy without adapter names.
2. Program inspection produces evidence-backed source and target recommendations.
3. Program Gate M1 approves exact adapter identities and target topology.
4. Approval changes invalidate only affected items.
5. The program initializes projects only through existing CLI writers.
6. Every generated change uses ordinary `change.md`, `plan.yaml`, and slice artifacts.
7. Ordinary Gate 1 remains a separate stamp; Program Gate M1 does not write `plan.lifecycle: approved`.
8. Serial execution stops with a structured next action at every human or failure boundary.
9. Progress can be rebuilt from journals, active plans, archives, and approval records.
10. One merged slice does not automatically mark a source repository complete.
11. Source-to-target mapping remains compatible with one slice targeting one project.
12. Existing single-change workflows require no migration program.
13. Parallel execution cannot acquire two writers for one target project.
14. Unsupported target workload kinds remain explicit blockers.
15. Finalization requires publication confirmation or explicit abandonment for every item.
16. Workspace execution uses typed lifecycle operations and does not bypass the current workspace-root execute guard.
17. Batch change approval invokes the ordinary plan transition writer per change with a recorded operator actor; no path auto-approves Gate 1.

## Testing

- Program artifact writers, lifecycle transitions, approval invalidation, stop conditions, and the serial coordinator are crate-level integration tests over the mock adapter catalogue and scripted answers, driving the same plan and slice verbs the coordinator invokes.
- Byte-stable `progress.yaml` rebuilding is a golden-file assertion over fixture journals and archives, following the existing `REGENERATE_GOLDENS` pattern.
- The judgment legs (workload classification, target recommendation) pin their answer shapes through the answers-goldens family; prompt quality is eval-rung coverage (`cargo make eval`), including at least one multi-repository migration scenario over `sandbox/`.
- Re-entry is asserted by interrupting and resuming the coordinator at every stop condition in an integration harness.

## Open questions

1. Should program artifacts live under `.specify/migrations/` or at the workspace root while active, mirroring one active `plan.yaml`?
2. What is the smallest useful scope taxonomy for deciding whether a repository is fully migrated?
3. Can several program items share one active change, or should v1 enforce one item per change for simpler re-entry?
4. Should publication confirmation be an operator command, a forge query, or either under one provider contract?
5. How should a program handle one legacy repository that must remain active for a long strangler migration?
6. Which recommendation changes should invalidate the whole program rather than one item?
7. When target policy has exactly one allowed adapter, is Program Gate M1 sufficient target approval or should each project still require a separate stamp?

