# Managed Workspace Materialization

> **Status: Superseded by [RFC-87 Private Workspaces](../rfc-87-working-trees.md)**, which replaces slots and leases with disposable workspaces over immutable snapshots. Retained for the rejected long-lived-slot policy detail.
>
> Owns: cloning and refreshing registry projects, creating writable workspace slots, branch preparation, cleanliness checks, and the lease boundary required by automated multi-repository execution.
>
> Source-list authoring scope and topology now belong to [RFC-88](../rfc-88-detached-changes.md); source snapshots belong to [RFC-87](../rfc-87-working-trees.md).

## Abstract

Add an optional managed materialization layer for workspace slots so Emery can work through a list of repositories without requiring the operator to clone and prepare every target checkout by hand.

The registry remains the declaration of target project membership and location. A materializer turns a registry entry into a writable `workspace/<project>/` slot at an exact base revision, applies branch and cleanliness policy, and grants the workflow a lease for one change at a time.

Publication remains operator-owned in the first version. Managed push and pull-request operations are a later forge capability.

## Motivation

Current workspace mode intentionally leaves slot materialization to the operator:

- the operator clones or links every registry project;
- the operator prepares branches and clean working trees;
- `plan execute` assumes required slots already exist;
- the operator publishes and merges changes before finalize.

That boundary is appropriate for an explicit multi-repo change. It blocks the desired migration experience where an operator supplies a repository list and Emery processes repositories serially or in bounded parallel batches.

A source snapshot is immutable input for `survey` and `extract`; a workspace slot is a mutable target tree for `build` and `merge`. Even when both originate from the same repository and revision, they are different capabilities.

## Goals

1. Materialize registry projects into writable workspace slots on demand.
2. Pin every slot to an exact base revision before execution.
3. Enforce clean-tree, branch, remote, and lease policy.
4. Preserve local-path and operator-prepared slot workflows.
5. Keep source snapshots separate from writable target trees.
6. Support local execution now and a hosted working-tree backend later.
7. Make materialization idempotent, observable, and recoverable.
8. Avoid giving source adapters write access to target slots.

## Non-goals

- Selecting target projects or target adapters.
- Publishing branches, opening pull requests, or waiting for CI in the first version.
- Replacing Git with a Emery-specific version-control model.
- Allowing concurrent writers in one slot.
- Moving build or merge lifecycle authority out of the workflow CLI.
- Making registry URLs writable configuration inside adapters.
- Solving distributed working-tree transport; the deferred working-tree RFC remains the cloud follow-up.

## Decision

### Registry stays declarative

`registry.yaml` continues to own project membership and location. Add optional materialization policy:

```yaml
version: 1
projects:
  - name: billing
    url: git@github.com:org/billing.git
    materialization:
      mode: managed
      base: refs/heads/main
      branch-prefix: emery/
  - name: mobile
    url: ../mobile
    materialization:
      mode: operator
```

Modes:

- `operator` — current behavior; Emery validates an existing slot but never creates or resets it;
- `managed` — Emery may create and refresh the slot under the declared policy;
- `external` — reserved: a deployment backend supplies a working-tree lease without a local top-level slot. This mode ships with the hosted backend (Stage 4), and registry validation refuses it until then.

`operator` remains the default for existing registries.

### Materialization capability

Workflow code receives a deployment-neutral capability:

```text
ensure(project, requested-base, purpose) → working-tree lease
inspect(project) → materialization status
release(lease, outcome)
```

As a provider capability it sits beside `adapter::Resolver` on the seam — a trait the workflow consumes, with the local Git implementation and a hosted backend as interchangeable providers:

```rust
/// Deployment-neutral workspace-slot materialization.
trait Materializer: Send + Sync {
    /// Create, refresh, or reuse the slot for `project` at the policy
    /// base, then grant the exclusive working-tree lease.
    fn ensure(&self, project: &str, base: BaseRequest, purpose: Purpose)
        -> impl Future<Output = Result<WorkingTreeLease, Error>> + Send;

    /// Read-only status projection; never mutates the slot.
    fn inspect(&self, project: &str) -> Result<MaterializationStatus, Error>;

    /// Validate the tree, record the outcome, drop the lease.
    fn release(&self, lease: WorkingTreeLease, outcome: Outcome)
        -> impl Future<Output = Result<(), Error>> + Send;
}

struct WorkingTreeLease {
    project: String,
    base: String,              // exact commit, resolved before mutation
    tree: TreeRef,             // local path or opaque hosted reference
    branch: Option<String>,    // "emery/migrate-payments/billing"
    lease: LeaseId,
    expiry: Timestamp,         // recorded locally, acted on by hosted backends
    disposition: Disposition,  // created | refreshed | reused
}
```

The capability returns:

- project identity;
- exact base revision;
- writable tree location or opaque working-tree reference;
- branch name when applicable;
- lease id and expiry;
- remote and publication hints;
- whether the tree was created, refreshed, or reused.

The local provider implements the capability with Git and filesystem operations. A hosted provider may implement it with ephemeral clones or worktrees.

This capability is intended to converge on the architecture's host-materialized working tree ([RFC-55](rfc-55-working-tree.md)): a tree materialized from a content-addressed base revision, mutated in place, with the change-set extracted by the host. This RFC adds the policy layer — registry modes, branch and cleanliness rules, and the lease. When RFC-55's git-aware filesystem backend lands, the local provider becomes a thin policy layer over it, and the hosted provider is the same policy over the backend a cluster node already uses. The dual deployment posture rides entirely on the provider: the workflow consumes one contract in both.

Slice and change orchestrations consume the returned working-tree value.

### Local slot layout

Managed local projects use the existing path:

```text
<workspace>/workspace/<project>/
```

Derived materialization state lives out of tree:

```text
<project-cache>/workspace/
├── leases/<project>.json
└── status/<project>.json
```

A live lease record, matching the model below:

```json
{
  "workspace": "modernization",
  "project": "billing",
  "change": "migrate-payments",
  "entry": "billing-api",
  "owner": { "kind": "process", "pid": 48213, "host": "op-laptop" },
  "base": "5d92…",
  "branch": "emery/migrate-payments/billing",
  "acquired": "2026-07-19T21:04:11Z",
  "expiry": "2026-07-20T21:04:11Z",
  "last-event": "slice.build.started"
}
```

No materialization state is committed in the target project. The workspace journal records durable start, success, failure, and release events.

### Topology projection

Workspace planning also requires `.emery/topology.lock`, while the current engine only reads and staleness-checks that projection. Managed materialization therefore adds one CLI-owned writer:

```bash
emery workspace topology refresh
```

The refresh operation:

1. validates every registry entry required by the requested scope;
2. reads each materialized slot's `project.yaml`, baseline specs, decisions, and journal tail;
3. writes the same deterministic target, description, `surface[]`, `decisions[]`, and `recent[]` projection consumed by plan authoring;
4. records the registry digest, slot revision, and projection input digest;
5. refuses missing or dirty operator-owned slots rather than projecting partial state silently.

`workspace sync` refreshes topology after a successful managed sync unless `--no-topology` is passed for clone-only preparation. The topology lock remains committed, machine-written state; managed materialization changes its writer from unspecified surrounding tooling to an explicit CLI operation.

### Exact base before mutation

Before `plan execute` starts work for a project, managed materialization:

1. resolves the declared base ref through the configured remote;
2. records the exact commit;
3. creates or refreshes the slot;
4. verifies the slot has no unaccounted changes;
5. creates the change branch;
6. acquires an exclusive lease;
7. returns the exact base to the workflow.

The branch name is deterministic:

```text
<branch-prefix><change>/<project>
```

Collisions fail with a recovery hint unless policy explicitly permits re-entry onto the matching branch and recorded base.

Emery never performs an implicit destructive reset of an operator-owned slot.

Branch preparation belongs to materialization; commit ownership stays with `slice merge`. The materializer creates the change branch and records the exact base, but it never commits workflow content — the merge orchestration remains the only commit writer. A unified Git provider capability may later subsume both behind one seam once forge operations (push, pull requests) land; the first version keeps the two writers distinct rather than designing that provider speculatively.

### Lease model

One writable lease exists per project slot.

A lease records:

- workspace and project identity;
- change and plan entry;
- process or hosted run owner;
- base revision;
- branch;
- acquisition and expiry time;
- last journal event.

The lease **contract** is deployment-neutral; its first **implementation** is deliberately small. Locally, a lease is an advisory lock file plus the cleanliness classification below — there is no expiry reaper, and recovery is always the explicit `lease recover` path. The expiry field exists for the hosted backend, where an abandoned remote run must eventually release its slot with no operator at the machine; local execution records it but does not act on it. Serial local migration therefore pays for a lock file, not a distributed lease system, while hosted execution (roadmap RM-18) slots into the same contract unchanged.

Lease files under the project cache are the authoritative ownership record; the journal carries the durable audit trail. Recovery validates both, but a journal event never grants ownership by itself — `lease recover` rewrites the cache record only after validating that tree, branch, base, and journal agree.

Lease acquisition fails when another live lease exists. Stale lease recovery is explicit:

```bash
emery workspace lease inspect <project>
emery workspace lease recover <project>
```

Recovery validates the working tree and journal before changing ownership. It never discards uncommitted work automatically.

### Cleanliness and re-entry

Managed slots distinguish:

- clean at declared base;
- clean on the expected Emery branch;
- dirty with changes explained by the active slice;
- dirty with unaccounted changes;
- base drifted because the remote advanced;
- branch diverged from the recorded base.

Only the first three states may proceed. Unaccounted or diverged states stop execution with a structured diagnostic:

```console
$ emery workspace inspect billing --format json
{
  "project": "billing",
  "mode": "managed",
  "state": "dirty-unaccounted",
  "base": "5d92…",
  "branch": "emery/migrate-payments/billing",
  "unaccounted": ["src/lib/patch.ts"],
  "lease": null,
  "next": "inspect the tree; commit or discard src/lib/patch.ts, then `emery workspace prepare billing --change migrate-payments`"
}
```

Re-entry uses the journal and slice metadata to determine whether existing changes belong to the interrupted run. A matching lease and branch may resume; a merely similar branch is not assumed safe.

### Source and target separation

When a repository is both migration source and target:

- source adapters read an immutable snapshot from the source store;
- target adapters write the managed workspace slot;
- both are initially pinned to the same commit unless the migration program declares otherwise;
- source extraction remains reproducible if target generation mutates the slot;
- no source adapter receives the target slot preopen.

This preserves evidence integrity during in-place migration.

### Project initialization and target binding

A newly materialized target may lack `.emery/project.yaml`. Proposing and applying project initialization — name, exact target adapter, platforms, and mode — is owned by [RFC-88 Detached Changes](../rfc-88-detached-changes.md). An existing configuration remains authoritative.

One slot carries one target adapter. A repository holding two independent workloads is split into two registry projects — and therefore two slots — before the program schedules it; the materializer never sees a multi-target project.

### CLI surface

Add explicit workspace operations:

```bash
emery workspace sync [<project>...] [--jobs <n>]
emery workspace inspect [<project>] [--format json]
emery workspace topology refresh
emery workspace prepare <project> --change <name>
emery workspace release <project> --change <name>
emery workspace clean <project>
```

`sync` creates or fetches managed slots. `prepare` pins the base, creates the branch, and acquires the lease. `release` drops the lease after validating the tree and recording the outcome.

One managed project through the full cycle:

```console
$ emery workspace sync billing
billing  cloned git@github.com:org/billing.git at refs/heads/main → workspace/billing (topology refreshed)
$ emery workspace prepare billing --change migrate-payments
billing  base 5d92… branch emery/migrate-payments/billing lease acquired (expires 2026-07-20T21:04:11Z)
$ emery workspace prepare billing --change other-change
error: workspace-lease-held: billing is leased by change `migrate-payments` (acquired 2026-07-19T21:04:11Z); `emery workspace lease inspect billing`
$ emery workspace release billing --change migrate-payments
billing  lease released — tree clean on emery/migrate-payments/billing, outcome recorded
```

`clean` removes only a clean, unleased managed slot. It refuses dirty, leased, or operator-mode paths.

`plan execute` may call `prepare` and `release` through the capability automatically when the registry entry is managed.

### Execution routing boundary

The current guest-routed `plan execute` refuses workspace-root execution. The workspace-root execute guard still applies under managed materialization.

The first migration coordinator uses the existing root plan-status/advance surface and invokes the project-bound refine, build, and merge verbs against the selected slot. Those CLI verbs remain the only lifecycle writers. A later change may teach `plan execute` to perform the same workspace routing internally; until then, this RFC's automatic `prepare` / `release` integration applies to single-project execution and to the migration coordinator.

### Publication boundary

The first version stops after local execution:

1. Emery materializes and modifies the slot.
2. Target merge commits the local change according to existing semantics.
3. The operator inspects, pushes, opens a pull request, and merges it.
4. Finalize archives only after operator-owned publication.

A later forge provider may add:

- push;
- pull-request create or update;
- CI and mergeability status;
- merged-state verification.

Those operations must wrap existing workflow state rather than becoming new lifecycle authorities.

## Security and safety

- Credentials stay in the host Git backend and are never mounted into guests.
- Remote URLs are validated against workspace policy before cloning.
- Local paths are canonicalized and cannot escape allowed roots through symlinks.
- Managed cleanup refuses paths not stamped by the materializer.
- Destructive Git operations require an exact lease and expected base.
- Source snapshots and target slots use separate roots and permissions.
- Hosted leases must provide equivalent isolation and ownership semantics.

## First delivery

An in-house team can ship the first migration programs on **operator-prepared slots** (today's default): clone or link targets by hand, keep trees clean, publish yourself. That is usable for a small expert team; it is not the long-term UX.

**In first delivery**

- No managed materialization required — the workflow can run against prepared slots.
- Document the operator checklist (clone, branch hygiene, publish) beside the migration program how-to.

**Pull in next for the same team (still local)**

| Stage | Pull in when |
| ----- | ------------ |
| Stage 1 — `workspace sync\|inspect` | Cloning / linking many targets is the dominant friction |
| Stage 2 — prepare + exclusive lease | Two changes contend for one slot, or the coordinator should own branches |
| Stage 3 — program-integrated materialize-next | Sync+lease exist and idle slots need lifecycle |
| Stage 4 — hosted backend | Roadmap RM-18 |

Program sequencing: [platform-migration series](../platform.md).

## Implementation stages

Operator-prepared slots satisfy first delivery. These stages backfill cloning and preparation friction once the coordination loop is in daily use.

### Stage 1 — Inspect and sync

1. Add materialization mode to registry entries.
2. Implement local Git clone/fetch and local-path linking.
3. Add `workspace sync|inspect`.
4. Add the deterministic `workspace topology refresh` writer.
5. Preserve operator mode as the default.

### Stage 2 — Prepare and lease

1. Add exact-base resolution and branch policy.
2. Add exclusive lease state and journal events.
3. Add cleanliness classification and explicit recovery.
4. Integrate managed prepare/release with single-project `plan execute` and the migration coordinator.

### Stage 3 — Program integration

1. Materialize only projects required by the next migration change.
2. Bound concurrent materialization separately from build concurrency.
3. Release idle slots while retaining reproducible source snapshots.
4. Surface next operator publication actions in migration status.

### Stage 4 — Hosted backend (gated on RM-18)

1. Implement opaque working-tree leases over hosted clones.
2. Preserve exact base and change-set semantics.
3. Keep the same workflow capability contract.
4. Add durable lease ownership and recovery.

## Acceptance criteria

1. Existing registries default to operator materialization with no behavior change.
2. A managed registry entry can be cloned and inspected through Emery.
3. Managed sync can regenerate a byte-stable topology lock from the materialized slots.
4. A missing or dirty required slot cannot silently disappear from the topology projection.
5. Every execution records an exact target base revision.
6. An unaccounted dirty tree blocks execution without being reset.
7. One project slot cannot have two live writers.
8. Interrupted work can resume only when lease, branch, base, and journal agree.
9. Source adapters cannot write or read through the target slot capability.
10. A repository used as both source and target has separate immutable and writable trees.
11. Project initialization runs through `emery init`, not materializer file writes.
12. Publication remains operator-owned and finalize preserves its current gate.
13. Managed cleanup cannot remove an operator-owned or dirty checkout.
14. The local and hosted implementations expose the same lease semantics.
15. Workspace coordination uses existing lifecycle verbs until workspace-root `plan execute` explicitly gains routing support.

## Testing

- Clone, sync, cleanliness classification, lease acquisition and recovery, and topology refresh are crate-level integration tests over local fixture Git repositories and temp directories; no live forge access in CI.
- Cleanliness and re-entry states form a dense deterministic matrix (clean, expected branch, explained-dirty, unaccounted-dirty, drifted, diverged) asserted at the CLI boundary.
- Byte-stable topology-lock regeneration is a golden-file assertion following the existing `REGENERATE_GOLDENS` pattern.
- Coordinator integration (prepare/release around refine, build, merge) is covered with [RFC-88](../rfc-88-detached-changes.md); the hosted backend contract is exercised only by its own provider's suite.

## Open questions

1. Should local managed slots use full clones, shared-object clones, or Git worktrees from a workspace bare mirror?
2. Should `plan execute` acquire one project lease at a time or all leases required by the approved plan before starting?
3. How should long-running migrations respond when a target base branch advances before publication?
4. Which Git hosts and URL schemes are allowed by default?
5. What is the minimum forge follow-up needed before unattended hosted execution is safe?

