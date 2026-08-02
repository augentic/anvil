# Next Stage: Platform-Scale Migration

> Status: Planning spine for the RFC-85…RFC-91 series — each RFC owns its own decisions; this document owns the sequence and the fit
>
> Audience: contributors starting work on the series; operators evaluating what Emery is becoming

## The vision

Point Emery at a legacy system — a mobile app, a realtime platform, any estate of repositories that together comprise one product — and have it migrate that system: discover the repositories, profile them, author a plan, and execute it with a **swarm of concurrent agents** working across **multiple repositories** on **multiple nodes**, converging on verified, published changes.

Concretely, the exemplar workload is a migration the size of AT's mobile app or AT's realtime platform: tens of repositories, hundreds of slices, weeks of wall-clock — infeasible as today's serial, single-repo, operator-tended loop, and exactly what the series below makes routine.

Everything Emery already is stays load-bearing: the slice loop (`refine → build → merge`), artifact authority, the journal as single write authority, adapter seams over WIT, operator-owned publication. The series scales those invariants out; it does not replace them.

## Where we are

Today one change runs in one repository (or a hand-tended workspace of them), serially: one judgment leg at a time, one working tree the operator prepared, verify as prompt text inside the agent loop, publication tracked in the operator's head. The measured walls (RFC-78's `wasm-omnia-r9k` runs): a ~30-minute serialized build with an unobservable nested review team, an 11–54 minute synthesis leg, and no way to run two of anything at once.

## The target architecture

Four moves, layered:

1. **Trees become values.** A working tree is materialized from a content-addressed `revision` (plus pending `changeset` layers), mutated under an exclusive lease, and its delta extracted as a `changeset`. Values are the only thing that crosses operations or nodes — no shared volumes, ever.
2. **Verification becomes host-owned.** Closed, sandboxed verify profiles replace cargo-commands-in-prompts, producing normalized findings any orchestrator can route.
3. **Judgment becomes a swarm.** Within a slice: focused workers with exclusive write manifests, converging through the verify gate. Across slices: independent plan entries build in parallel on separate leases, with a trial-integration gate measuring joint health continuously. Across nodes: three separated planes (coordination / convergence / publication) move events, values, and PRs respectively.
4. **Location becomes ephemeral.** A change opens in a bare directory, discovers and pins its member repositories from the forge (creating new ones when none match), materializes them as leased slots, and leaves nothing behind after finalize except merged baselines and forge history.

```text
            coordination plane (journal projection, leases, plan status)
                 │
  ┌──────────────┼──────────────────┐
  node A         node B             node C
  slice 1        slice 2            trial-integration gate
  worker swarm   worker swarm       base + α + β → verify → findings
  tree ← lease   tree ← lease
  └─ changeset α └─ changeset β     serial merge gate (unchanged authority)
                 │
            value plane (revision / changeset — iroh / NATS / S3, deployment-bound)
                 │
            publication plane (branches, PRs, forge — operator-owned, verified at finalize)
```

## The series

Numbering is implementation order. Each step ships operator value alone; nothing waits for the whole series.

| RFC | Title | Delivers | Depends on |
| --- | ----- | -------- | ---------- |
| [RFC-85](rfc-85-migration-program.md) | Migration Program | Adapter descriptors, durable source intake, the serial program coordinator — point Emery at a repository list today, on operator-prepared slots | — (RFC-71 landed) |
| [RFC-86](rfc-86-working-trees.md) | Working Trees | `materialize` / `changes()` over `revision` / `changeset`, the exclusive lease, managed slot policy, source/target tree separation | 85 (Part B snapshots) |
| [RFC-87](rfc-87-verify-profiles.md) | Verify Profiles | Closed, sandboxed, host-owned verification with normalized findings — the gate every concurrent design converges through | 86 |
| [RFC-88](rfc-88-concurrent-execution.md) | Concurrent Execution | The swarm within one slice: focused build workers, write-ownership manifests, the convergence gate, staged backend concurrency, refine/plan fan-outs, synthesis payload restructuring | 87 (gate), 86 (Stage C) |
| [RFC-89](rfc-89-node-sync.md) | Node Sync | The multi-node fabric: three planes, values-only transport, control-plane leases, concurrent plan entries, the trial-integration gate | 86, 87, 88 |
| [RFC-90](rfc-90-detached-changes.md) | Detached Changes | Location independence: the disposable change directory, forge discovery with pinned members, ephemeral slots, greenfield `create-repository` | 85 (discovery), 86 (slots) |
| [RFC-91](rfc-91-cross-repo-changesets.md) | Cross-Repo Changesets | Publication identity: one change's branches and PRs bound across repositories, ordered landing, verification at finalize | 85; simplified by 90 |

Sequencing notes:

- **RFC-85 is the walking skeleton** — it proves the end-to-end migration story serially before anything concurrent exists, and every later RFC hangs policy off its intake and coordinator.
- **RFC-86 + RFC-87 are the physics** — trees-as-values and host-owned verify are what every concurrency decision downstream assumes. Neither has UX of its own; both are consumed by everything after them.
- **RFC-88 then RFC-89 is concurrency inside-out** — first many workers within one slice on one node, then many slices across many nodes. The same two invariants make both safe: exclusive write ownership and values-only transport.
- **RFC-91's projection (Phase A) can start any time after RFC-85** — it is a read-only `plan.yaml` + forge projection with no dependency on the concurrency work; its full weight (verification with no committed registry) arrives with RFC-90.

## The migration, end to end

What the exemplar migration looks like once the series lands:

1. `emery change open` in a bare directory ([RFC-90](rfc-90-detached-changes.md)); `emery source discover` profiles the estate and proposes members — including repositories that don't exist yet ([RFC-85](rfc-85-migration-program.md) intake, RFC-90 greenfield).
2. `/emery:plan` authors over the pinned members; plan-level write manifests validate that parallel entries are disjoint ([RFC-89](rfc-89-node-sync.md) D10).
3. `emery plan execute` drains the plan: independent entries build concurrently on leased trees across nodes (RFC-89 D8), each entry's build a swarm of focused workers converging through verify profiles ([RFC-88](rfc-88-concurrent-execution.md), [RFC-87](rfc-87-verify-profiles.md)), changesets published at round boundaries over the value plane ([RFC-86](rfc-86-working-trees.md), RFC-89).
4. The trial-integration gate composes in-flight changesets continuously and journals joint-health findings (RFC-89 D9); the serial merge gate lands entries one at a time — lifecycle authority unmoved.
5. The operator publishes; finalize verifies every member PR exists, merged, in declared order ([RFC-91](rfc-91-cross-repo-changesets.md)); the change directory is deleted. Nothing of record is lost.

## Renumbering map

| Was | Now | Disposition |
| --- | --- | ----------- |
| RFC-70 Migration Walking Skeleton | [RFC-85](rfc-85-migration-program.md) | Renumbered |
| RFC-55 Working-Tree Materialization (future) | [RFC-86](rfc-86-working-trees.md) | Merged (mechanics) — original [archived](archive/rfc-55-working-tree.md) |
| RFC-72 Managed Workspace Materialization (future) | [RFC-86](rfc-86-working-trees.md) | Merged (policy + lease) — original [archived](archive/rfc-72-materialization.md) |
| RFC-60 Verify Profiles (future) | [RFC-87](rfc-87-verify-profiles.md) | Promoted and renumbered |
| RFC-79 Swarm Build | [RFC-88](rfc-88-concurrent-execution.md) | Merged (build-time) — original [archived](archive/rfc-79-swarm-build.md) |
| RFC-80 Synthesis Redesign | [RFC-88](rfc-88-concurrent-execution.md) | Merged (refine-time) — original [archived](archive/rfc-80-synthesis-redesign.md) |
| RFC-83 Near-Realtime Node Sync | [RFC-89](rfc-89-node-sync.md) | Renumbered (was an uncommitted draft) |
| RFC-84 Detached Changes | [RFC-90](rfc-90-detached-changes.md) | Renumbered (was an uncommitted draft) |
| RFC-82 Cross-Repo Changesets | [RFC-91](rfc-91-cross-repo-changesets.md) | Renumbered |

## Outside the series

Unchanged and orthogonal — not part of this arc, not blocked by it:

- **[RFC-71 Self-Assembling Wasm Deployment](rfc-71-deployment.md)** — largely landed; Stage 2 diagnostics remain draft.
- **[RFC-77 Release Process](rfc-77-release-process.md)** — operational policy for releasing Emery itself; its WIT-breaking shape becomes RFC-91's first in-house changeset.
- **[RFC-18 Specialized SLM Code Generation](future/rfc-18-slm.md)** (future) — an optional cost lever behind RFC-88's per-worker model-selection hook; a ratchet rung, not a stage.
- **[RFC-46a Web Asset Materialization](future/rfc-46a-web-asset.md)** (future) — content-triggered vectis work, independent of this series.

Known external reference: `augentic/remedium` RFC-81 cites "RFC-82" for the changeset record; update that citation to RFC-91 when next touching that repo.
