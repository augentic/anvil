# Migration Intake and Source Selection

> Status: Draft
>
> Owns: durable source membership, source materialization, the repository profile schema and deterministic profiler, source-adapter selection policy, recommendation and approval, and the lowering of approved sources into change plans.
>
> Depends on: [Adapter Descriptors and Registry Trust](rfc-71-discovery.md) — Stage 1 (the static first-party index) suffices for the migration walking skeleton.
>
> Supersedes after reconciliation: [Source Catalogue and Source-Clone Cache](future/rfc-21-catalogue.md).

## Abstract

Let an operator provide a list of repositories and supporting inputs once, then reuse those inputs across many Specify changes.

Migration intake creates a CLI-owned `sources.yaml`, materializes immutable source snapshots in an out-of-tree cache, profiles each repository, recommends one or more source adapters, and records approved exact bindings. A later change selects source keys instead of repeating URLs, paths, and adapter names.

Source selection policy lives here — profiling, composition, auto-bind conditions, and approval — built over the descriptor, filtering, explanation, and report substrate defined by [Adapter Descriptors and Registry Trust](rfc-71-discovery.md).

This RFC keeps source inputs separate from target projects:

- `sources.yaml` says what legacy material is being understood;
- `registry.yaml` says which target projects slices may modify;
- `plan.yaml` binds specific source leads to one change and one target project.

## Motivation

The current workflow can bind several sources to one plan, but each change must supply those bindings. That becomes operationally expensive for migrations involving tens or hundreds of repositories:

- repository URLs and local paths are repeated;
- clone and profile work is repeated;
- adapter choice is repeated;
- there is no durable inventory of migration inputs;
- a source repository, a design document, screenshots, and runtime captures cannot be managed as one intake set;
- source snapshots and mutable target workspaces are easy to conflate.

The deferred source-catalogue RFC identified the catalogue and clone-cache need. The current architecture changes two details:

1. source adapter operations and plan orchestration are CLI-owned, so concurrency does not belong in a skill;
2. regenerable caches live out of tree, so source snapshots should not return under `.specify/cache/`.

## Goals

1. Import repository lists, local repositories, design documents, screenshots, captures, and operator intent into one source catalogue.
2. Materialize reproducible, immutable source snapshots.
3. Profile repositories once per revision and reuse the result.
4. Recommend and approve multiple source adapters per source when useful.
5. Lower catalogue keys into ordinary `plan.yaml.sources` bindings.
6. Keep source snapshots read-only and target workspace slots writable.
7. Support incremental intake and re-synchronization.
8. Keep every write behind a CLI operation.

## Non-goals

- Choosing or creating target projects.
- Materializing writable target workspace slots. [Managed Workspace Materialization](rfc-73-materialization.md) owns that.
- Scheduling changes across the whole migration.
- Recording cross-change completion. [Migration Programs](rfc-74-program.md) owns the ledger projection.
- Mutating imported repositories.
- Replacing source adapter `survey` or `extract`.
- Treating one repository as exactly one source binding.

## Decision

### Source catalogue

A workspace may carry `sources.yaml` at its root:

```yaml
version: 1
sources:
  - key: legacy-billing
    kind: repository
    location: git@github.com:org/legacy-billing.git
    revision: 2f3c...
    profile: sha256:...
    bindings:
      - key: code
        adapter: specify:typescript@1.4.0
        status: approved
      - key: docs
        adapter: specify:documentation@1.1.0
        subpath: docs
        status: approved
  - key: migration-architecture
    kind: documentation
    location: ./inputs/migration-architecture
    revision: sha256:...
    bindings:
      - key: design
        adapter: specify:documentation@1.1.0
        status: approved
```

The catalogue is CLI-owned and never hand-edited. It is committed at the workspace root beside `registry.yaml`: membership, pinned revisions, and approved bindings are reviewed workspace state, while snapshots and profiles remain regenerable out-of-tree cache.

Required source fields:

- `key` — stable workspace-local source identity;
- `kind` — repository, documentation, screenshots, captures, or value;
- `location` — operator-supplied origin;
- `revision` — immutable Git revision or content digest;
- `profile` — digest of the latest repository or media profile when applicable;
- `bindings[]` — zero or more named source-adapter bindings.

A binding records:

- binding key unique within its source;
- exact adapter selector;
- optional source subpath;
- recommendation or approval status;
- recommendation report id;
- approval actor and timestamp when approved.

The component digest remains in the recommendation report and adapter store metadata. `sources.yaml` does not duplicate the package store.

### One source may have several bindings

Repository identity and adapter identity are separate.

A TypeScript frontend repository may produce these plan bindings:

```yaml
sources:
  legacy-billing-code:
    adapter: specify:typescript@1.4.0
    snapshot:
      revision: 2f3c...
      content: sha256:...
      subpath: .
  legacy-billing-docs:
    adapter: specify:documentation@1.1.0
    snapshot:
      revision: 2f3c...
      content: sha256:...
      subpath: docs
```

Each lowered binding records the exact adapter selector, the pinned revision, the content digest, and the subpath. The snapshot location is re-derived from the content digest at execution time, so an evicted cache entry costs re-materialization, never plan invalidity.

The catalogue source key is durable across changes. The lowered plan keys are change-local and preserve the current `(source, lead)` identity model.

### Monorepos

A monorepo is one catalogue source: one origin, one pinned revision, one immutable snapshot. Independently useful subprojects are expressed as additional bindings with `subpath`, not as duplicate sources:

```yaml
- key: legacy-platform
  kind: repository
  location: git@github.com:org/platform.git
  revision: 9a1b...
  bindings:
    - key: billing-api
      adapter: specify:typescript@1.4.0
      subpath: services/billing
      status: approved
    - key: admin-ui
      adapter: specify:typescript@1.4.0
      subpath: apps/admin
      status: approved
```

Profiles are computed per binding subpath (see [Profiling](#profiling)), so recommendation evidence stays scoped to the subtree an adapter would actually read. Snapshot storage is never duplicated across subpaths.

### Intake inputs

Intake accepts:

- repeated repository URLs;
- a newline-delimited repository file;
- a local directory;
- a registry or developer-portal import projection;
- documentation and design directories;
- screenshots and capture trees;
- literal operator intent.

The initial CLI surface:

```bash
specify source import --repo git@github.com:org/legacy-a.git
specify source import --repo-file repos.txt
specify source import --documentation ./migration-design
specify source import --screenshots ./screens
specify source import --captures ./captures
specify source list
specify source show <key>
specify source remove <key>
```

Imports are idempotent by normalized origin plus revision. Aliases require an explicit `--key`.

A first intake session:

```console
$ specify source import --repo git@github.com:org/legacy-billing.git
imported legacy-billing (repository) at 2f3c… — snapshot materialized, profile sha256:4f15…
$ specify source import --documentation ./inputs/migration-architecture
imported migration-architecture (documentation) at sha256:91d0… — snapshot materialized
$ specify source import --repo git@github.com:org/legacy-billing.git
legacy-billing already present at 2f3c… — unchanged
$ specify source list
KEY                     KIND           REVISION      BINDINGS
legacy-billing          repository     2f3c…         (none)
migration-architecture  documentation  sha256:91d0…  (none)
```

The third command shows the idempotency contract: re-importing an unchanged origin is a no-op report, never a duplicate entry or a silent revision bump. Bindings stay empty until [recommendation and approval](#recommendation-and-approval) below.

### Source snapshot store

Remote and local source material is materialized into an out-of-tree source store. The first version is deliberately plain — one materialized tree per source and revision:

```text
<source-store>/<source-key>/<revision>/
```

For the catalogue above, with derived profiles alongside as cache tenants:

```text
~/.cache/specify/sources/
├── legacy-billing/
│   └── 2f3c…/                 # exact-commit tree, no .git, read-only
│       ├── package.json
│       ├── src/…
│       └── docs/…
├── migration-architecture/
│   └── sha256:91d0…/          # local docs keyed by content digest
│       └── …
└── profiles/
    ├── 4f15….yaml             # keyed by revision+subpath+profiler+policy
    └── 91d0….yaml
```

Deleting any of it costs re-materialization from the pinned origin and re-profiling — `sources.yaml` and its digests are untouched.

The default parent follows the platform cache convention and is relocatable for tests and hosted execution (out-of-tree, beside the project cache).

Content-addressed object storage with cross-source deduplication (an `objects/<content-id>/` pool behind per-workspace symlink trees) is deferred until a real migration's store size demands it.

The likely convergence point is the architecture's host-materialized working-tree capability ([RFC-55](future/rfc-55-working-tree.md)): the same content-addressed materializer that produces writable target trees can serve read-only source snapshots — locally from clones, hosted from whatever backend materializes trees on a cluster node. The store therefore stays behind one narrow seam (`content digest → readable tree`), so swapping the plain layout for the RFC-55 materializer or a hosted backend changes no catalogue, plan, or adapter contract. That seam is what keeps intake identical between the operator-local CLI and a hosted deployment.

Properties:

- snapshots are immutable after creation;
- a Git source resolves to an exact commit before profiling;
- local content resolves to a deterministic tree digest;
- duplicate revisions may share object storage;
- source adapter operations receive a read-only preopen;
- mutable Git metadata and credentials are not exposed to source adapters;
- eviction removes unreferenced cache objects, never `sources.yaml`.

`specify source sync` resolves moving origins to new revisions and creates new snapshots. It does not silently update the catalogue's active revision when an approved migration program pins the old revision; it reports drift and requires an explicit refresh:

```console
$ specify source sync
legacy-billing          drift: origin head 8e4a… ahead of pinned 2f3c… — pinned by program legacy-modernization; refresh explicitly
migration-architecture  ok: content unchanged
```

### Profiling

This RFC owns the repository profile schema and the deterministic profiler. After materialization, intake profiles each source; the profile is the typed input to the deterministic candidate filtering defined by [Adapter Descriptors and Registry Trust](rfc-71-discovery.md).

The profiler emits a normalized `RepositoryProfile`:

```yaml
revision: 2f3c...
media: repository
languages:
  - name: typescript
    share: 0.72
  - name: css
    share: 0.18
frameworks:
  - name: react
    evidence: package.json#dependencies.react
sentinels:
  - package.json
  - tsconfig.json
workload-signals:
  - kind: web-frontend
    evidence: package.json#dependencies.react-dom
artifacts:
  - kind: documentation
    path: docs/
```

Every fact carries a source anchor or deterministic detector id. The profile contains observations, not target recommendations; `workload-signals` use the closed workload-kind taxonomy owned by [Adapter Descriptors and Registry Trust](rfc-71-discovery.md).

Profiles live as derived cache entries keyed by:

```text
source revision + binding subpath + profiler version + detector policy digest
```

`sources.yaml` records only the profile digest. `specify source show` projects the profile and its evidence. A changed revision invalidates the cached profile.

For non-repository media:

- documentation profiles record format, structure, and recognized schemas;
- screenshots record dimensions, grouping, and optional platform hints;
- captures record capture schema and replay digest coverage;
- value sources record only kind and content digest.

Profiling never calls a model and never executes repository code.

### Source selection policy

Source adapters are observational and may compose. One repository can bind several source adapters, for example:

- `typescript` for code structure;
- `documentation` for design and runbooks;
- `screenshots` for UI evidence;
- `captures` for runtime behaviour.

The default policy may auto-bind a source candidate when:

- it passes trust policy;
- it is the only candidate surviving deterministic filtering for its media kind;
- no mutually exclusive adapter is already bound.

Anything else — several surviving candidates, an adjudicated selection, or a policy that disables auto-binding — is recorded as a recommendation requiring review. There is deliberately no score threshold or ambiguity margin to configure; see [Adapter Descriptors and Registry Trust §Explanation and adjudication](rfc-71-discovery.md#explanation-and-adjudication).

### Recommendation and approval

Intake filters and ranks candidates for each profile through the substrate in [Adapter Descriptors and Registry Trust](rfc-71-discovery.md) and stores the resulting recommendation report.

```bash
specify source recommend [<key>...]
specify source approve <key> <recommendation>
```

Continuing the intake session, against the recommendation report shown in [Adapter Descriptors and Registry Trust §Recommendation reports](rfc-71-discovery.md#recommendation-reports):

```console
$ specify source recommend legacy-billing
legacy-billing: report rec-2026-07-19-legacy-billing-01
  code  → specify:typescript@1.4.0     sole candidate for media `repository`
  docs  → specify:documentation@1.1.0  scoped to profile artifact `docs/`
$ specify source approve legacy-billing rec-2026-07-19-legacy-billing-01
approved 2 bindings on legacy-billing (actor: operator) — code, docs
```

The approval writes the `bindings[]` entries shown in the catalogue example above (`status: approved`, report id, actor, timestamp) and fires the matching journal event.

Approval may create several bindings for one source. Only approved bindings are eligible for automatic plan lowering.

A sole-candidate recommendation may be auto-approved when workspace policy permits it, under the conditions above. The journal records whether approval came from an operator or policy.

Within a migration program, Program Gate M1 ([Migration Programs](rfc-74-program.md#program-approval)) covers the source bindings the program consumes, so program items do not require a separate `source approve` pass. The standalone surface here serves catalogue use outside programs — reusable sources for ordinary changes.

### Lowering into a change

`plan author` accepts catalogue selectors:

```bash
specify plan author migrate-billing \
  --source @legacy-billing \
  --source @migration-architecture
```

`@legacy-billing` expands to every approved binding unless a binding suffix narrows it:

```bash
--source @legacy-billing/code
```

Before survey, the plan author orchestration:

1. loads the pinned snapshot;
2. confirms the profile and approved binding are current;
3. ensures each exact source adapter;
4. lowers each selected binding into `plan.yaml.sources`;
5. runs `survey` through the ordinary source capability.

The resulting plan is self-describing for later `extract`: each lowered binding carries the exact adapter selector and the pinned snapshot identity (the wire shape shown under [One source may have several bindings](#one-source-may-have-several-bindings)).

### Concurrency

Materialization, profiling, recommendation, and survey are independent per source. Their concurrency controls live on the owning CLI operations:

```bash
specify source sync --jobs 4
specify source profile --jobs 4
specify source recommend --jobs 4
specify plan author ... --survey-jobs 4
```

Results are sorted by stable source and binding keys before any authored artifact is written.

### Removal and retention

`specify source remove <key>` refuses when:

- an active plan references a lowered binding from the source;
- an approved migration program includes the source;
- a workspace policy marks the source as retained.

Removal deletes catalogue membership, not immutable audit references in archived plans or journal events.

Snapshot garbage collection is a separate operation: `specify source prune` follows the retention-policy pattern of `specify archive prune`, evicting snapshot objects and cached profiles unreferenced by the catalogue, an active plan, or an approved migration program. It never touches `sources.yaml` or audit state; everything it removes is reproducible from the pinned origin and revision.

## Validation

`specify source validate` checks:

- schema and unique keys;
- normalized origin and revision shape;
- profile digest availability;
- binding key uniqueness;
- exact adapter selectors for approved bindings;
- recommendation-report and approval coherence;
- source snapshot availability;
- active plan and migration-program cross-references.

Drift is reported separately from malformed state:

- origin head differs from pinned revision;
- profile inputs changed;
- recommendation descriptor or policy changed;
- approved adapter version is yanked or outside current policy;
- snapshot cache entry is absent but reproducible.

## First delivery

In-house intake is a reviewed catalogue of Git repos and local docs, pinned snapshots, profiles, explicit approve (or Program Gate M1), and `@key` plan lowering into ordinary survey. Serial CLI is fine; monorepo multi-binding and portal import wait for a real request.

**In first delivery**

- Stages 1–2 with **Git + local documentation** only; one approved binding per source is enough.
- Explicit `source approve` (programs may cover this at Gate M1 — see [RFC-74](rfc-74-program.md)).
- Stage 3 plan lowering: `@key` → exact adapter + snapshot identity → `survey` (serial).
- Inline `--source <key>=<adapter>:<binding>` remains available as the escape hatch.

**Deferred until needed**

| Capability | Pull in when |
| ---------- | ------------ |
| Screenshots, captures, other media kinds | A migration needs them as intake |
| Multiple bindings / monorepo subpaths | One origin needs two adapters or scoped trees |
| Policy-driven auto-approval | Manual approve (or M1) becomes rubber-stamping fatigue |
| Concurrent `--jobs` fan-out | Wall-clock pain on multi-repo sync/profile/survey |
| Profile-cache reuse as a hard gate | Nice-to-have; recompute is acceptable at first |
| `source prune` | Disk pressure from snapshot cache |
| External catalogue import (Stage 4 / RM-12) | Portal-backed membership is required |

Program sequencing: [RFC-74 §First delivery](rfc-74-program.md#first-delivery).

## Implementation stages

### Stage 1 — Catalogue and snapshots (first delivery)

1. Reconcile and replace the deferred source-catalogue schema.
2. Add CLI-owned import, list, show, remove, validate, and sync operations.
3. Add the out-of-tree immutable snapshot store.
4. Support Git repositories and local documentation first.

### Stage 2 — Profiles and recommendations (first delivery)

1. Add repository profiling and cache keys.
2. Store recommendation reports.
3. Add explicit approve; policy-driven auto-approval waits for demonstrated fatigue.
4. Multiple bindings per catalogue source wait for a monorepo need; the schema may allow them early if cheap.

### Stage 3 — Plan lowering (first delivery)

1. Add `@source` selectors (and `@source/binding` when multiple bindings exist).
2. Ensure exact adapters before survey.
3. Preserve snapshot identity in the plan binding.
4. Deterministic concurrent survey fan-out — **when serial intake is too slow**.

### Stage 4 — External catalogue imports (gated on RM-12)

1. Define a read-only import DTO.
2. Map Backstage or another developer portal into proposed source entries.
3. Show a diff before applying membership changes.
4. Keep external catalogues advisory; `sources.yaml` remains the reviewed workspace projection.

This stage is the parked roadmap item RM-12 applied to sources. It graduates with that item and must share its registry-import DTO rather than defining a second external-catalogue boundary.

## Acceptance criteria

**First delivery (Stages 1–3 serial)**

1. An operator can import a repository list without naming source adapters.
2. Every repository is materialized at an immutable revision before profiling.
3. A repository profile is deterministic for a fixed revision, subpath, profiler version, and detector policy.
4. Source snapshots are read-only to adapters and distinct from target workspace slots.
5. `plan author --source @key` lowers approved bindings into ordinary plan sources carrying exact adapter and snapshot identity.
6. Plan execution remains valid if the source catalogue later changes.
7. No skill body owns intake, synchronization, profiling, or fan-out.
8. Removing an active source is refused with the referencing plan or migration program.
9. Cache deletion costs recomputation only and cannot delete catalogue or audit state.
10. Existing inline `--source <key>=<adapter>:<binding>` remains available.

**Later**

11. Re-profiling an unchanged revision reuses the derived result.
12. One source can carry several approved source-adapter bindings, including subpath-scoped bindings within one monorepo snapshot.
13. External catalogue imports present a reviewable diff before changing membership.

## Testing

- Import, sync, snapshot immutability, catalogue validation, and plan lowering are exercised as crate-level integration tests over fixture repositories and local trees, per the integration-first posture.
- Profiler determinism is asserted by re-profiling fixed fixture revisions; representative repositories (single-language, monorepo, docs-only) cover the detector matrix.
- Recommendation and approval flows run over the static first-party descriptor index with scripted mock answers; no live registry or model in CI.
- End-to-end intake-to-`survey` coverage belongs to the eval rung (`cargo make eval`), not the per-push suite.

## Open questions

1. When [RFC-55](future/rfc-55-working-tree.md)'s working-tree materializer lands, should the source store become its read-only face immediately, or remain a separate plain store until the hosted deployment needs the shared backend?
2. Which source kinds belong in the first schema beyond repository and documentation?
3. Which language and framework detector should the profiler adopt rather than implement?
4. Should source adapter composition declare explicit mutual-exclusion groups?
5. What retention policy should `specify source prune` default to while protecting reproducibility?
6. Should source revisions be refreshed only by `source sync --update`, leaving plain `sync` as ensure-present?
7. Which profile fields are safe to export to a hosted catalogue when repository contents are private?

