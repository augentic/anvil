# RFC-90: Detached Changes

> Status: Draft — step 6 of the platform-migration series ([next-stage.md](next-stage.md)); bones only — materialization mechanics and forge verbs deferred to the RFCs it composes
>
> Owns: location-independent operation — the change as the unit of location: the self-contained, disposable change directory; the no-state-outlives-the-change invariant; forge discovery of member repositories by criteria with pinned resolution at approval; ephemeral slot population; and greenfield repository provisioning when no member matches.
>
> Depends: [RFC-85](rfc-85-migration-program.md) Part B (the intake/profiler discovery runs over; A7's read-only discovery posture), [RFC-86](rfc-86-working-trees.md) (slot materialization, leases, and the value↔tree boundary).
>
> Related: [RFC-89](rfc-89-node-sync.md) (a detached change spanning nodes binds its journal to that RFC's control plane), [RFC-91](rfc-91-cross-repo-changesets.md) (with no committed registry, the forge markers become the only out-of-band record — exactly what its reconstruction path was designed for).

## Intent

Start from a bare directory and run a whole change: discover the repositories that comprise the platform, pin them, materialize them, execute the plan, publish, finalize — then delete the directory. Emery today anchors multi-repo work in a permanent platform repo (`workspace: true`, committed `registry.yaml`, tended `workspace/<project>/` slots). Detached mode replaces that anchor with the change itself: the change directory is the one self-contained home for coordination state, valid exactly as long as the change is live.

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | **The change is the unit of location.** A detached change opens as one self-contained directory — plan, journal, slice artifacts, and materialized slots — created at open, valid for the change's duration, deletable after finalize. There is no platform repo and nothing to commit at the coordination level. | "Where does this change live?" has a one-directory answer on any machine. Ephemeral means *disposable after finalize*, not scratch: the directory is the durable-for-the-change home, and deleting it mid-change forfeits orchestration state (approvals, journal, lifecycle position) even though pushed branches survive. |
| D2 | **No Emery state outlives the change.** Durable outcomes land where they already land: baselines in member repos, code and PRs on the forge, identity reconstructible per [RFC-91](rfc-91-cross-repo-changesets.md) D3. Post-finalize retention of the change directory (or its archive) is an operator convenience under the existing prune posture, never a requirement. | The lifecycle authority story is unchanged — the CLI remains the single writer over `plan.yaml` and the journal for the change's duration; it just stops pretending those files are the product. Nothing new to back up, replicate, or migrate. |
| D3 | **Members are discovered, then pinned.** `emery source discover` queries the forge by criteria (language, topics, org, manifest sentinels — the [RFC-85](rfc-85-migration-program.md) Part B profiler runs over shallow fetches to keep matching deterministic) and emits an immutable candidate report, following RFC-85 A7 verbatim: discovery never installs, binds, or writes config. Approval pins each member — repository plus base revision — into the plan's resolved bindings. | Dynamic population is not re-querying: re-materializing a half-done change resolves the recorded pins, deterministically. The membership ledger does not vanish; it shrinks to per-change resolved bindings inside `plan.yaml`, honoring one-authored-home-per-fact. |
| D4 | **The registry is demoted from ledger to derivation.** Detached mode authors no `registry.yaml`; membership and location derive from the plan's resolved bindings, and any registry-shaped view is a projection. The committed platform-repo workspace posture is superseded. | [RFC-91](rfc-91-cross-repo-changesets.md)'s member derivation simplifies to `plan.yaml` alone, and its forge reconstruction gains weight — with no committed registry, the forge markers are the only out-of-band record, which is exactly what D3 of that RFC designed for. `emery init --workspace` is reinterpreted or retired in this RFC's implementation cut. |
| D5 | **Slots are ephemeral materializations.** Each member populates on demand under a lease ([RFC-86](rfc-86-working-trees.md) mechanics and value↔tree boundary), scoped to the change directory, torn down with it. Baseline reads, builds, and merges route into the slot exactly as workspace routing does today. | The slot is a cache with a lease, not a checkout you tend. Two concurrent changes touching the same repository get two slots under two leases — the isolation [RFC-89](rfc-89-node-sync.md) D3 already requires. |
| D6 | **Greenfield is a first-class discovery outcome.** When no repository matches, the candidate report proposes creation. On operator approval the forge adapter performs its first write verb — `create-repository` — journaled, followed by ordinary `emery init` inside the new slot. Provisioning an empty repository is not publication: branch push and PR merge remain operator-owned everywhere. | The path from bare directory to brand-new member is gated but unbroken. The forge adapter's write surface stays minimal and auditable — one provisioning verb, no publication verbs, preserving [RFC-91](rfc-91-cross-repo-changesets.md) D5 and the cli-contract's operator ownership. |
| D7 | **The journal externalizes only when the change does.** Single-node: the change directory holds the one journal, and the existing file-backed contract suffices. Multi-node ([RFC-89](rfc-89-node-sync.md)): the same journal contract binds to the shared control plane for the change's duration — same object, different backend, still change-scoped. | No always-on service for desktop use; no fork of the journal taxonomy for hosted use. RM-18's "resumability comes from the journal and `.emery/` state" is satisfied by whichever binding the deployment chose. |
| D8 | **Single-repo mode is untouched.** A repository already carrying `.emery/` never routes through discovery or detached materialization; detached mode is a different anchor for change-scoped state, not a second workflow or lifecycle. | One workflow, two anchors. The slice loop, plan verbs, gates, and validation are byte-identical in both; only "where the change directory is" and "who populates the slots" differ. |

## Lifecycle sketch

```text
emery change open <dir>            # bare directory becomes the change home
emery source discover --criteria … # forge query → immutable candidate report
                                   # (report may propose create-repository)
/emery:plan                        # author over discovered members; approval pins
                                   # repo + base revision into resolved bindings
emery plan execute                 # slots materialize on demand under leases;
                                   # refine → build → merge per entry
                                   # operator publishes (push, PRs, merge)
/emery:finalize                    # verify publication (RFC-91), archive
rm -rf <dir>                       # nothing of record is lost
```

## Rejected alternatives

- **Permanent platform repo** (current workspace mode) — a durable coordination anchor for inherently change-scoped state; forces registry tending and slot hygiene between changes.
- **Durable out-of-tree change store** (`~/.emery/changes/…`) — recreates the platform repo one directory over; still state to back up and migrate.
- **Committing coordination state into a member repo** — pollutes members with cross-repo state and makes membership circular.
- **Re-resolving discovery at materialization time** — non-deterministic membership; pins exist so a half-done change re-materializes exactly.
- **Full publication autonomy** (push / PR merge verbs) — collapses the operator-owned publication boundary that every other RFC preserves.

## Phased delivery

- **Phase A — Detached change home.** `emery change open`, change-scoped `plan.yaml` / journal / slice artifacts in the change directory; manual member binding (operator supplies repo URLs); ephemeral slots over [RFC-86](rfc-86-working-trees.md).
- **Phase B — Forge discovery.** `emery source discover` criteria → immutable candidate report → pinned resolved bindings at approval.
- **Phase C — Greenfield provisioning.** The `create-repository` forge verb, journaled, followed by in-slot `emery init`.
- **Phase D — Multi-node binding.** The change's journal and leases bind to [RFC-89](rfc-89-node-sync.md)'s control plane; slots materialize on whichever node holds the lease.

## Open questions

- Mid-change loss posture: is "deleting the directory forfeits orchestration state" acceptable, or does Phase D's control-plane binding become the default durability story earlier?
- Discovery criteria language: structured filters only, or an operator-authored prose brief the profiler interprets?
- Verb naming: `emery change open` vs overloading `emery init --detached`.
- The fate of `emery init --workspace` and the registry reference docs once D4 lands.
- Two changes sharing a machine: slot cache roots per change directory, or a shared content-addressed cache with per-change leases?
- Greenfield defaults: org, visibility, license, branch protection — operator flags vs a provisioning policy file.
