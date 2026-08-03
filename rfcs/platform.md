# Next Stage: Platform-Scale Migration

> Status: Planning spine for the RFC-86…RFC-91 series — each RFC owns its own decisions; this document owns the sequence and the fit
>
> Audience: contributors starting work on the series; operators evaluating what Emery is becoming

## The vision

Point Emery at a legacy system — a mobile app, a realtime platform, any estate of repositories that together comprise one product — and have it migrate that system: discover the repositories, profile them, author a plan, and execute it with a **swarm of concurrent agents** working across **multiple repositories** on **multiple nodes**, converging on verified, published changes.

Then keep changing that platform the same way: open a disposable change directory, survey the organisation for repositories whose `.emery/project.yaml` declares `product:` (membership ids; the build set lives under `platforms:`), record the affected set (creating repositories when the change needs greenfield members), execute, publish, finalize.

Concretely, the exemplar workload is a migration the size of AT's mobile app or AT's realtime platform: tens of repositories, hundreds of slices, weeks of wall-clock — infeasible as today's serial, single-repo, operator-tended loop, and exactly what the series below makes routine.

Prior context for either job is intentionally thin: GitHub authentication and organisation, plus source material (documents and/or code). Emery creates target repositories when they do not exist; it does not assume a tended platform repo or pre-checked-out slots.

Everything Emery already is stays load-bearing: the slice loop (`refine → build → merge`), artifact authority, the journal as single write authority, adapter seams over WIT, operator-owned publication. The series scales those invariants out; it does not replace them.

## Where we are

Today one change runs in one repository (or a hand-tended workspace of them), serially: one judgment leg at a time, one working tree the operator prepared, verify as prompt text inside the agent loop, publication tracked in the operator's head. The measured walls (RFC-78's `wasm-omnia-r9k` runs): a ~30-minute serialized build with an unobservable nested review team, an 11–54 minute synthesis leg, and no way to run two of anything at once.

## The target architecture

Four moves, layered:

1. **Trees become values.** RFC-86 materializes one content-addressed `revision` under an exclusive local lease and extracts a `changeset`; RFC-90 later composes ordered same-base changesets; RFC-91 transports the settled values across nodes. No shared volume crosses an operation.
2. **Location becomes ephemeral.** A change opens in a bare directory, discovers and records its member repositories from the forge (creating new ones when none match), materializes them as leased slots, and leaves nothing behind after finalize except merged baselines and forge history.
3. **Verification becomes host-owned.** Closed, sandboxed verify profiles replace cargo-commands-in-prompts, producing normalized findings any orchestrator can route.
4. **Judgment becomes a swarm.** Within a slice: focused workers with exclusive write manifests, converging through the verify gate. Across slices: independent plan entries build in parallel on separate leases, with a trial-integration gate measuring joint health continuously. Across nodes: three separated planes (coordination / convergence / publication) move events, values, and PRs respectively.

```text
            coordination plane (hosted journal, leases, plan status)
                 │
  ┌──────────────┼──────────────────┐
  node A         node B             node C
  slice 1        slice 2            per-project trial gate
  worker swarm   worker swarm       repo A + α; repo B + β → findings
  tree ← lease   tree ← lease
  └─ tree delta α └─ tree delta β   serial merge gate (unchanged authority)
                 │
            value plane (revision / changeset — iroh / NATS / S3, deployment-bound)
                 │
            publication plane (branches, PRs, forge — operator-owned, verified at finalize)
```

## The series

Numbering is **implementation order** along the operator-story critical path first, then the scale track. Work completes one RFC before implementation starts on the next. Every RFC depends only on completed earlier steps, owns one deployable path, and has no acceptance criterion or phase gated on a later RFC.

### Product critical path — migrate and change a platform

| RFC | Title | Delivers | Depends on |
| --- | ----- | -------- | ---------- |
| [RFC-86](rfc-86-working-trees.md) | Local Working Trees | Complete local value↔tree loop: source grants and snapshots, bare-mirror Git materialization, `revision → tree → changeset`, exact-base policy, local leases, and immutable source/target separation | — |
| [RFC-87](rfc-87-detached-changes.md) | Detached Changes | Complete single-node migrate/change loop: generated source identities, deterministic selection, disposable change directory, GitHub discovery, recorded members, target-topology proposals, local ephemeral slots, and greenfield creation | completed 86 |
| [RFC-88](rfc-88-publication-sets.md) | Publication Sets | Publication identity: one change's branches and PRs bound across repositories, ordered landing, verification at finalize | 87 (member derivation) |

### Scale track — concurrency after the location story works

| RFC | Title | Delivers | Depends on |
| --- | ----- | -------- | ---------- |
| [RFC-89](rfc-89-verify-profiles.md) | Verify Profiles | Complete Omnia/Rust path for closed, sandboxed, host-owned verification with normalized findings and typed unavailability elsewhere | completed 86 |
| [RFC-90](rfc-90-concurrent-execution.md) | Concurrent Execution | Complete single-node Omnia swarm: focused workers, write ownership, local pool, per-worker trees, deterministic changeset composition, convergence, refine/plan fan-outs, and synthesis payload restructuring | completed 86, 89 |
| [RFC-91](rfc-91-node-sync.md) | Node Sync | Complete hosted/multi-node Omnia path: JetStream journal and values, fenced leases, hosted trees, remote pools, concurrent plan entries, and per-project trial integration | completed 86, 87, 89, 90 |

Sequencing notes:

- **Complete RFC-86 first.** It ships the whole local tree capability through patch round-trip and immutable source/target separation. Layering and hosted backends are explicitly absent.
- **Then complete RFC-87.** Bare directory, forge auth/org, source material → select adapters → discover → record members (create repos when needed) → plan → local materialization on execute → finalize. It is single-node and removes the old workspace/registry coordinator.
- **Then complete RFC-88.** Publication projection and finalize verification consume only RFC-87's settled plan bindings and forge provider; there is no registry or `gh` bridge.
- **Then scale inside-out through RFC-89 → RFC-90 → RFC-91.** Host-owned verify completes first; RFC-90 completes the local swarm and changeset composer; RFC-91 alone adds hosted trees, remote pools, durable control-plane state, and cross-node execution.

## Two operator jobs, one loop

What both jobs look like once the critical path lands:

### 1. Migrate a legacy platform

1. `emery change open` in a bare directory ([RFC-87](rfc-87-detached-changes.md)).
2. `emery source discover` with **migrate** criteria fingerprints shallow source trees through [RFC-87](rfc-87-detached-changes.md)'s exact-one source selector and proposes members and target topology — including repositories that do not exist yet (RFC-87 greenfield).
3. The operator runs `emery change approve`; it atomically records `plan.yaml.projects`, exact revisions, resolved target topology, and generated source bindings, and journals initialized `create-repository` for greenfield targets.
4. `/emery:plan` authors slices over the recorded target-capable projects.
5. `emery plan execute` materializes leased slots on demand ([RFC-86](rfc-86-working-trees.md)), runs refine → build → merge per entry.
6. Operator publishes; finalize verifies the publication set ([RFC-88](rfc-88-publication-sets.md)); the change directory is deleted.

### 2. Ongoing change to the migrated platform

1. Same `emery change open` in a bare directory.
2. `emery source discover` with **change** criteria surveys the organisation for repositories whose `.emery/project.yaml` declares `product:` (membership ids); optional criteria intersect with the change's requested product ids. The build set is `platforms:` (`core` / `ios` / `android` / …) — not the membership key.
3. Approval records the affected members; greenfield proposals cover members the change needs that do not exist yet.
4. Plan → execute → publish → finalize as above.

Once the scale track lands, step 5 gains concurrent workers, concurrent plan entries, and multi-node execution — same loop, higher throughput.

```text
emery change open <dir>
emery source discover --mode migrate|change --criteria …
         │  immutable candidate report (may propose create-repository)
         ▼
emery change approve
                   →  record projects, revisions, topology, sources;
                      create initialized repositories where needed
/emery:plan        →  author slices over recorded projects
emery plan execute →  materialize slots on demand; refine → build → merge
operator publishes
/emery:finalize    →  verify publication set (RFC-88); archive
rm -rf <dir>
```

## Outside the series

Unchanged and orthogonal — not part of this arc, not blocked by it:

- **[RFC-71 Self-Assembling Wasm Deployment](rfc-71-deployment.md)** — largely landed; Stage 2 diagnostics remain draft.
- **[RFC-77 Release Process](rfc-77-release-process.md)** — operational policy for releasing Emery itself; its WIT-breaking shape becomes RFC-88's first in-house publication set.
- **[RFC-18 Specialized SLM Code Generation](future/rfc-18-slm.md)** (future) — an optional cost lever behind RFC-90's per-worker model-selection hook; a ratchet rung, not a stage.
- **[RFC-46a Web Asset Materialization](future/rfc-46a-web-asset.md)** (future) — content-triggered vectis work, independent of this series.

Known external reference: `augentic/remedium` RFC-81 cites "RFC-82" for what is now RFC-88's publication-set record; update that citation when next touching that repo.
