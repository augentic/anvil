# RFC-91: Concurrent Execution

> Status: Draft — step 6 of the platform-migration series, on the scale track ([platform.md](platform.md))
>
> Owns: the complete single-node recursive execution path: focused Omnia build workers, model-assisted convergence through RFC-90's engine-owned phase machine, exclusive write ownership, per-worker private workspaces, concurrent ready-leaf scheduling, deterministic code-patch composition, durable domain-round records, bottom-up domain convergence, multi-member use of RFC-88's atomic target waves, the local agent pool, parallel survey and extract fan-outs, bounded concurrent evaluation of RFC-88 conflict-domain decomposition, and the synthesis payload redesign.
>
> Builds on completed [RFC-86](rfc-86-change-facts.md), [RFC-87](rfc-87-working-trees.md), [RFC-88](rfc-88-detached-changes.md), and [RFC-90](rfc-90-build-verification.md). [RFC-78](archive/rfc-78-prompt-budget.md) supplies request budgets, timeout semantics, and sessions. This RFC absorbs [RFC-79](archive/rfc-79-swarm-build.md) and [RFC-80](archive/rfc-80-synthesis-redesign.md).
>
> [RFC-92](rfc-92-node-sync.md) may place these workers on remote nodes without changing their requests, ownership, workspaces, or code-patch semantics. [RFC-18](future/rfc-18-slm.md) may later use the per-worker model-selection hook.

## Intent

Replace large, multi-purpose judgment legs with focused workers that converge on one verified result.

Today one Omnia generation conversation acts as crate writer, test writer, guest writer, and verify-repair loop. Its prompt is about 64 KB, and its cargo commands run inside an opaque agent loop. The review leg similarly hides an agent team inside one completion. In observed `wasm-omnia-r9k` runs, the five build legs serialized about 30 minutes of agent time and the hidden review team caused the run to die.

Synthesis has the same structural problem. Observed attempts took 11 and 54 minutes while repeatedly carrying about 50 KB of playbook prose and complete artifact bodies through the answer channel. Survey and extract also remain serial because the backend cannot yet isolate concurrent completions.

This RFC gives each worker a narrow brief, explicit write ownership, an individual timeout and budget, and a private workspace. A deterministic guest orchestrator composes their immutable results and dispatches RFC-90's model-assisted `verify` against the composed candidate. The same local scheduler claims independent plan leaves, folds their results through the persisted RFC-88 domain hierarchy, records every completed gate, and atomically accepts passing same-target waves. The pool also serves build, review, survey, extract, and independent decomposition nodes. Synthesis remains one cross-domain judgment leg, but its playbook becomes lazy and its artifacts move out of the answer.

## Flow and terms

1. The Omnia adapter core partitions a build into writer tasks with exclusive path manifests.
2. Each writing worker starts from the same immutable base CID in its own RFC-87 private workspace and returns a typed outcome plus a **code patch**.
3. The composer checks every completed patch, copies disjoint touched paths into a fresh integration workspace, and captures the next CID.
4. The engine dispatches RFC-90's target `verify` against the composed candidate; the target's verification agent runs one declared check pass and returns typed findings.
5. The orchestrator routes each finding to the worker that owns the affected path. That worker resumes its session with only the findings delta.
6. Bounded repair and convergence rounds continue until verification passes or the build returns a typed failure with residual findings.
7. After verification passes, the engine dispatches `review`; the target runs its specialist workers and antagonist, then returns one aggregated phase report.
8. Blocking review findings follow RFC-90's engine-owned `repair(origin: review) → verify → review` route.

A **worker** is one focused judgment request: a thin role brief, only the path-first task inputs it needs under RFC-78 D1, an explicit write-ownership manifest, MCP-lazy references, and a typed answer gate. A **wave** is an ordered set of code patches that all name the same base CID and have pairwise-disjoint touched paths. A **fan-in task** is a later worker with exclusive ownership of a path needed by several earlier tasks.

A **code patch** is RFC-87's `{ base snapshot, result snapshot, touched paths }` relation. RFC-89's publication set is a separate forge-side record. Merge-finalized requirement identity from RFC-86 makes same-base concurrent synthesis safe, while worker and round outcomes are recorded as per-actor facts.

RFC-90 has already split the target build surface into `build`, `repair`, `verify`, and `review`. Omnia's writing swarm is internal to `build` and `repair`, while its read-only specialist swarm is internal to `review`. Each operation still returns one phase report, and RFC-91 adds no further target operation.

At plan level, a **domain round** is one immutable record binding a domain id and decomposition revision to an ordered child-result set, target bases, composed candidate CIDs, verification phase reports, completeness (`frontier | complete`), and verdict. A **target wave** is RFC-88's immutable ready same-target leaf set accepted together after its required domain gates pass. Internal domains gain records and result facts, not lifecycle status or claims.

## Worked examples

### Disjoint worker composition

Suppose one Omnia slice asks for a Rust library, its integration tests, and a WASI guest:

```text
crate writer  owns crates/payments/src/**
test writer   owns crates/payments/tests/**
guest writer  owns guests/payments/**
```

All three workers start from CID `sha256:base`. They finish with three immutable result CIDs whose captured touched paths match those disjoint prefixes. The composer prepares a fresh workspace from `sha256:base`, copies each touched path's exact result-tree value in fixed dependency order, and captures `sha256:composed`. The order is fixed for byte stability even though these disjoint copies commute.

Only then does the engine dispatch the target's model-assisted `verify` against the composed candidate workspace. The adapter receives only that workspace; any domain identity and completeness remain in the engine's round record. If a test failure points to `crates/payments/src/client.rs`, the finding returns to the crate writer. The worker resumes its existing session, edits its private workspace, and returns a replacement patch. Cargo command text is confined to the target's one-pass verification prompt; writer and repair prompts receive findings, not command instructions.

### Shared-path fan-in

Now suppose those workers all need `crates/payments/src/lib.rs` to export their additions. That file cannot appear in three parallel manifests. The partitioner removes it from the first wave and creates a later integration task:

```text
wave 1
  crate writer  owns crates/payments/src/client.rs
  test writer   owns crates/payments/tests/client.rs
  guest writer  owns guests/payments/src/lib.rs

wave 2
  integration worker owns crates/payments/src/lib.rs
```

Wave 1 composes first. The integration worker then starts from that composed CID and updates `crates/payments/src/lib.rs` once. If the partitioner cannot identify a safe exclusive fan-in, it serializes the dependent tasks. It never asks a text merge to arbitrate shared ownership.

If all three workers unexpectedly touch `crates/payments/src/lib.rs`, their manifests were wrong but their captured touched paths reveal the conflict. The gate retains all three immutable results, rejects the complete wave before applying any patch, and sends an ownership finding to every contributor. Each worker recaptures a replacement that omits the shared path. The repaired disjoint wave composes, and one later fan-in task integrates `lib.rs`. No conflict-free subset becomes visible early.

## Decisions

### D1 — The engine and Omnia core own different orchestration levels

RFC-90's engine phase machine owns `build → verify ⇄ repair → review` ordering, repair budgets, terminal report assembly, and the slice transition. Inside `build`, compiled Omnia guest code performs partition → dispatch → compose; inside `repair`, it routes findings to existing owners and composes their replacement patches; inside `review`, it dispatches specialists followed by the antagonist and aggregates their findings without remediation. Each call returns one phase report. Omnia code decides worker sequencing and ownership arbitration; workers supply judgment. There is no lead agent deciding which other agents to run.

Reusable brief, manifest, and outcome helpers live in `augentic/emery`'s adapter SDK. The Omnia implementation lives in `augentic/emery-adapters/targets/omnia`. This RFC uses RFC-90's WIT seam without extending it and does not require Vectis or Contracts to adopt worker partitioning.

### D2 — RFC-90's model-assisted `verify` is the convergence gate

Writer and repair workers never receive Cargo command text. The engine calls RFC-90's `verify` for the composed candidate, and the target adapter runs one model-assisted check pass. RFC-90 owns operation order and the outer repair budget; the target adapter owns the verification prompt, commands, and finding report.

Findings map to the owning worker and resume that worker's session with a findings delta, never a fresh full prompt. Stage A includes this gate from its first release. RFC-90 must land first because it replaces the hidden repair channel with an engine-visible one before worker partitioning.

This gate is model-assisted evidence, not a security boundary or deterministic proof. A later `wasi:exec`-based RFC may replace the phase implementation without changing worker routing, phase order, or domain records.

### D3 — Every worker has exclusive, enforced write ownership

Each build worker carries an explicit manifest. Predicted overlap becomes a dependency or generated fan-in task before dispatch; ambiguous ownership fails planning. Initial build partitions follow the existing writer roles, and `tasks.md` subdivides a role only when task paths prove disjoint. A model never chooses the target-build partition.

Manifests are predictions, while RFC-87 captured touched paths are authoritative. Before applying any result, the gate compares the complete captured sets. Out-of-manifest writes are blocking findings. Undeclared overlap rejects the whole wave atomically and enters the same repair shape as other findings.

This partition discipline, rather than textual merge luck, is the safety invariant needed by Stage B and RFC-92's later plan-level manifests.

### D4 — Local concurrency lands as Stage A → Stage B

**Stage A — observable sequential swarm.** Omnia's focused writers run one at a time. Every writing worker still receives a fresh RFC-87 private workspace, and immutable results compose into a fresh integration workspace before serialized model-assisted verification. Only after verification passes does the engine dispatch `review`, whose specialists and antagonist also run one at a time against the verified candidate. Stage A therefore lands observability, model selection, ownership enforcement, bounded blast radius, and the convergence gate without a shared writable tree.

**Stage B — concurrent private workspaces.** A bounded local pool, concurrent in-flight `create` calls from one guest, per-spawn MCP isolation, and isolated prompt spills allow writing workers from the same base to run together and, after verification passes, allow review specialists to run together. Writing results still compose in dependency order into a fresh integration workspace before verification, and the antagonist still waits for every specialist outcome. The survey and extract fan-outs in D9 use this same pool.

Tree isolation exists in both stages; only dispatch changes. Workers never share a writable tree or live handle, and their only code-bearing outputs are RFC-87 base/result snapshot relations. Remote placement is RFC-92, not a hidden third stage.

### D5 — Review specialists are host-visible workers

Within one engine-dispatched `review` operation, Security, Correctness, and Quality specialists become separate model workers with typed findings, individual budgets, and individual inactivity timeouts. The antagonist runs only after their outcomes are available, and compiled Omnia code aggregates one review phase report for the engine. Blocking findings return to the engine, which follows RFC-90's `repair(origin: review) → verify → review` route; the repair implementation routes them to the existing owners and convergence gate.

This removes the nested in-agent review team. The host can observe, bound, cancel, and time out each specialist separately even though the engine sees one target `review` dispatch and one phase report.

### D6 — Code-patch composition is one reusable deterministic kernel

Given one base snapshot id and an ordered list of RFC-87 code patches naming that base, the kernel requires pairwise-disjoint touched paths. It copies each path's exact value from the corresponding result tree into a fresh integration workspace and captures the next immutable snapshot. Base mismatch or overlap fails before verify; no implicit text merge occurs.

Dependencies and fan-in tasks create later waves. This RFC projects RFC-88's domain and leaf dependencies into bottom-up convergence waves, and every single-target domain uses this same kernel. RFC-92 may place the work remotely but cannot redefine scheduling, composition, domain readiness, or acceptance semantics. RFC-87 continues to own prepare and capture; `augentic/emery` and `augentic/backends` own the composition implementation introduced here.

### D7 — The synthesis playbook moves to an engine references shelf

The launcher route table gains an engine shelf such as `/mcp/engine/synthesis`. It serves the embedded synthesis playbook through the existing `list_docs` / `read_doc` contract, and engine judgment legs receive the MCP grant.

The synthesis system prompt keeps `synthesize.md`, the system contract, the answer schema, and a measured always-inline subset. All remaining playbook guidance is fetched lazily. This removes most of the roughly 50 KB repeated on each initial or repaired synthesis attempt.

The shelf and grants are owned by `augentic/emery` in `crates/guest`, `crates/launcher`, and `crates/project`.

### D8 — Synthesis artifacts use a lent staging tree and an outcome-only answer

The host lends synthesis an execution-local temporary directory. The agent writes and repairs artifact files there; artifact bodies never cross the answer channel. The typed answer contains only the outcome record.

The deterministic tail validates the complete staged tree before it becomes visible, promotes it atomically on a clean gate, and otherwise returns findings only so the same agent can edit files in place. Staged files never appear under the authoritative slice directory before promotion. The implementation belongs in `augentic/emery/crates/slice` persistence, answers, and prompts, and its answer-schema change regenerates the goldens.

D8 lands only after D7 passes its own live-eval gate. It changes payload and repair mechanics, not synthesis authority, provenance, or `[conflict]` / `[divergence]` / `[unknown]` semantics. Cross-domain reconciliation remains one judgment leg.

### D9 — Survey and extract fan out through the Stage B pool

After RFC-88 discovery pins topology, Author slices runs the initial `survey_all` for all bound sources concurrently. It merges `leads.md` in binding order, so output remains byte-identical to serial execution. Focused survey calls requested by recursive decomposition use the same pool and merge by `(source, parent lead, child lead)`, never completion order.

Refine dispatches extract operations concurrently in the same way. Per-source Evidence files form their natural disjoint write set. Plan-time survey is the first consumer.

This decision covers Author-slices survey and refine-time extract in `augentic/emery/crates/change` and `crates/slice`. It does not cover RFC-88's Discover-topology host reads, which remain under RFC-88 D9's separate concurrency budget. Concurrency changes dispatch, never output order.

### D10 — Recursive plan decomposition is bounded engine orchestration

After the complete initial lead inventory exists, the engine evaluates RFC-88's open conflict domains. Independent nodes may be judged concurrently, but one compiled orchestration owns the queue, depth and node budgets, strict-scope-reduction check, coverage validation, stable node identities, and final byte ordering.

Each judgment receives one domain, its inherited topology and ownership envelope, and only the lead material relevant to that domain. It returns typed `split` or `leaf`. A split proposes children, ownership, dependencies, rationale, and any focused surveys needed for missing source-local detail. The engine validates the response before adding children to the queue. It does not let one model call spawn another or decide its own budget.

`decomposition.yaml` and `plan.yaml` become visible only after every branch terminates and RFC-88's complete-tree and leaf-projection gates pass. Failure retains `discovery.yaml` and `leads.md` for retry but exposes no partial plan. Concurrency affects latency only: the same responses produce the same canonical hierarchy and leaf plan regardless of completion order.

Publication is atomic only at that current-view boundary. Every referenced lead and decomposition revision is retained by digest, so this choice does not bake final-plan closure into build or merge facts. A future streaming authorization epoch may publish ready branch revisions while other branches continue; the local scheduler below already consumes explicit revisions rather than ambient current files.

### D11 — The local scheduler folds ready leaves through domain gates

On one desktop, `plan execute` opens at most one wave per target from a deterministic bounded antichain of leaves whose projected dependencies are already accepted and whose ownership envelopes can share the current accepted target base. The bound is the target share of the host pool cap. The immutable RFC-88 wave manifest is written before claims or builds; every claim and result binds it. A dependency edge can never occur between members, so a producer wave commits first and its dependant is selected into a later wave against the new accepted CID.

The scheduler groups completed wave members by their nearest affected domain. A single-target `frontier` round composes only the current wave's same-base child patches with D6, dispatches RFC-90 `verify` against the composed workspace, writes a domain-round record (domain id stays engine-side), and publishes `domain.convergence.recorded`. A multi-target domain stores one ordered target→CID/report set and dependency verdict; it never composes trees.

A `frontier` round covers a causally closed ready subset: every included leaf dependency is accepted or in the same wave, but unfinished dependant children may remain. It may gate that target wave but cannot satisfy its domain as a complete child of the parent. Later waves bind the accepted CID produced by earlier frontiers; they never need to build against an unaccepted producer candidate.

A `complete` single-target round does not recompose historical child patches across bases. It validates the ordered chain of committed frontier waves from the domain's recorded initial base to the current accepted CID, requires every child and domain dependency for that decomposition revision, dispatches a complete-domain verification phase against that current tree, and emits that CID as the final domain result. Only complete rounds fold upward as complete child results. Failure blocks that domain and its dependants only. This is the complete local semantics; RFC-92 transports it without adding another scheduler.

### D12 — Domain rounds are durable and target waves accept atomically

Before publishing `domain.convergence.recorded`, the engine atomically writes `domains/<domain>/rounds/<digest>.yaml`. The closed record contains the domain id, lead/decomposition revision, child leaf or round digests, build and commit authorization anchors as applicable, target base CIDs, ordered current-wave patches or historical committed-wave chain, candidate/final CIDs, verification-report digests, completeness, and verdict. Its deterministic operation key is the digest of that complete input set and current accepted frontier. Duplicate claimless evaluation converges on the same record; different output for one operation key is a blocking violation. The fact references the record digest. Snapshot GC roots every candidate reachable from a live change record, so restart, detach, and remote resume never recompute a completed gate.

When every required frontier gate passes, the engine performs RFC-86's target-wave merge over the already frozen manifest. It revalidates every member's build authorization and input fence, requires a closed-plan commit authorization covering the exact reviewed member/spec set, composes all member patches, and follows RFC-88's stable per-member commit-then-postflight sequence. One `target.merge.wave-committed` fact names the commit authorization, advances the accepted CID, and projects every member leaf `merged`; no prefix is authoritative. The succeeded or aggregate postflight-failed fact then records the non-rollback gate outcome. `emery slice merge <member>` resolves and commits this whole wave or refuses if another member is incomplete; it cannot downgrade membership to N=1.

## Implementation requirements

- Deliver **Phase A** with split Omnia writer and review roles, one private workspace per worker, D6 composition, RFC-90's completed engine-owned phase machine, and bounded repair while dispatch remains serial.
- Deliver **Phase B** with concurrent in-flight model calls from one guest, the shared local pool, isolated MCP and prompt state, host-visible specialists, deterministic parallel survey and extract, and bounded evaluation of independent decomposition nodes. Whether the backend uses a Rust process pool or an SDK sidecar is decided from Stage B evidence.
- In Phase B, add deterministic bounded-antichain wave selection, single-node ready-leaf scheduling, domain ancestry projection, frontier and complete domain-round records and facts, bottom-up single-target composition and multi-target aggregation, and multi-member RFC-88 target-wave merge. Hosted placement is not required to exercise any of these semantics.
- Deliver **Phase C** with D7's engine shelf followed by D8's execution-local staging tree, outcome-only answers, atomic promotion, and findings-only repair deltas. RFC-91 is complete only after Phase C and both live-eval gates pass.
- Give each worker at most two repair rounds and each build at most three convergence rounds. Exhaustion returns a typed `failure` report with residual findings.
- Apply one host-level pool cap to build, review, survey, extract, and decomposition workers. The default is four, and deployment configuration may lower it. RFC-92 schedules whole remote pools without adding another local limit.
- Scope each worker session to its repair chain. Enforce per-worker inactivity timeout and pool-level cancellation that reaps every in-flight worker.
- Use the configured project model for every worker by default. The request's existing `model` field permits later per-role model overrides for RFC-18, but those overrides are not required here.
- Keep budget, ordering, and finding routing as compiled-in policy visible in the journal. Extend RFC-78's per-worker budget assertions so adapter tests lock brief sizes.
- Keep RFC-90's `build` / `repair` / `verify` / `review` operations, final `BuildReport`, and the synthesis authority model unchanged. Do not add remote placement, value transport, hosted execution, a replacement model backend, domain-partitioned synthesis, or mandatory swarm adoption by Vectis or Contracts.
- Preserve ownership boundaries: `augentic/emery` owns the outer build phase machine, SDK helpers, orchestration policy, plan scheduling, domain records, target-wave acceptance, composition, the engine shelf, staged synthesis, and survey/extract fan-outs; `augentic/emery-adapters` owns Omnia target-build partitioning, verification and repair phase behavior, manifests, review workers, and its existing merge gates; `augentic/backends` owns the RFC-87-backed local pool and participates in composition.
- Treat live evaluation as a terminal implementation gate. Capture pre-change grades, run `cargo make eval omnia-r9k --restart` in `augentic/emery-adapters` after D7, then run `cargo make eval orders-contracts --restart` there after D8. Every typed case gate must pass, and neither final grade may be lower than its baseline. If credentials or the model backend are unavailable, RFC-91 remains incomplete and RFC-92 does not start; CI is not a substitute.

## Acceptance criteria

1. An Omnia build for a slice the size of `at-r9k-position-adapter` completes as focused worker requests, each with a spilled prompt no larger than about 15 KB; Cargo command text appears only in the target's model-assisted verification phase.
2. Verification runs only through RFC-90's engine-dispatched `verify`. Findings route to the owning worker, and convergence-budget exhaustion produces a typed `failure` report with residual findings.
3. Review specialists are individually observable and timeout-able. No nested in-agent team remains in the Omnia review path.
4. Predicted manifest overlap becomes an explicit dependency or fan-in task before dispatch. Captured touched-path overlap rejects the complete wave without partial composition and routes ownership findings to every contributor.
5. In Stage B, two workers run concurrently in separate RFC-87 workspaces with isolated MCP configuration and no shared writable files. Pool cancellation reaps every in-flight worker.
6. Concurrent workers return result snapshots against the same base. Deterministic composition in a fresh integration workspace passes the convergence gate; base mismatch and composition conflict fail before verify.
7. The synthesis system prompt carries `synthesize.md` plus the measured always-inline subset, while the engine shelf serves the rest lazily. The synthesis answer is an outcome record, artifact bodies never cross the answer channel, and staged artifacts never become visible slice state before validation passes.
8. With Stage B available, `plan author`'s Author-slices phase dispatches survey over N sources concurrently after RFC-88 discovery pins topology. Its `leads.md` output is byte-identical to serial output.
9. A migration-scale lead set decomposes through at least three domain levels with independent nodes evaluated concurrently. Canonical `decomposition.yaml` and `plan.yaml` are byte-identical to serial evaluation; partial failure exposes neither current-view artifact, and depth, node, scope-reduction, coverage, ownership, and leaf-readiness violations fail closed.
10. Independent plan leaves execute concurrently on one node and fold through at least two same-target domain gates. Every completed gate has one digest-bound record and fact; killing and resuming after each boundary reuses the record and byte-identical candidate without re-running composition or verification.
11. Two same-base independent leaves enter one frozen target wave. `slice merge` on either member refuses until both are complete, then one committed fact advances the accepted CID and projects both leaves merged; failure or process loss before that fact projects neither, and replay after it creates no duplicate acceptance or identity map.
12. A producer and dependant on one target enter different waves. A passing frontier round lets the producer wave commit without waiting for its unfinished dependant; the dependant then builds from that accepted CID. The eventual complete round validates the ordered committed-wave chain and current tree without feeding cross-base patches to D6, then folds the whole domain into its parent.
13. Three disjoint workers that need one shared module converge as one parallel wave followed by one exclusive integration task. No first-wave worker owns the shared path, and no textual auto-merge occurs.
14. When three workers unexpectedly modify the same shared module, the first wave is retained but not composed. Each worker recaptures a result omitting that path, the repaired wave composes, and one fan-in task integrates the module.
15. `cargo make ci` passes in every touched repository, D8 regenerates its goldens, and the separately evaluated `omnia-r9k` and `orders-contracts` live cases preserve quality after D7 and D8 respectively.

## Rejected alternatives

- **Keep the fat generation and review legs** — preserves the opaque verify-repair channel, unbounded nested work, large prompts, and serial wall-clock that this RFC exists to remove.
- **A lead-agent orchestrator** — moves the multi-purpose judgment leg up one level. Build partitioning and plan-decomposition queueing, budgets, ordering, termination, and arbitration must remain deterministic guest policy; individual judgments may only propose one bounded split or leaf.
- **Shared writable workspaces** — make safety depend on timing and filesystem races. Every stage uses private workspaces and immutable code patches.
- **Textual auto-merge for shared paths** — confuses conflicting ownership with merge mechanics. Shared paths require one later owner or serialized tasks.
- **Compose the conflict-free subset of an overlapping wave** — exposes partial state from a wave whose complete ownership check failed. The gate rejects and repairs the wave atomically.
- **Run Cargo commands inside writer or repair prompts** — restores the hidden loop. Command execution remains confined to one observable target verification phase until deterministic native execution can replace it.
- **Fresh full prompts for repair** — repeats the payload and discards useful session context. Repairs resume the owning worker with findings deltas.
- **Partition synthesis by domain** — cross-domain reconciliation is the judgment being purchased. D7 and D8 reduce bytes and repair cost without changing that semantic unit.
- **Extend the WIT target interface again** — RFC-90's `verify` and finding exchange already carry what domain convergence needs; domain identity stays in engine round records.
- **Require Vectis and Contracts to adopt the swarm** — hides two unevidenced target migrations inside Omnia's concurrency cut. SDK helpers remain available for later bounded changes.
- **Remote workers in this RFC** — single-node requests, ownership, workspaces, and composition must settle first. RFC-92 owns placement and transport.
