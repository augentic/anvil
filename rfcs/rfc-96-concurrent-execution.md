# RFC-96: Concurrent Execution

> Status: Active scheduled work in the [Services Delivery Programme](platform.md). Phase A (work-item scheduler and read-heavy pool) before Phase B (concurrent build and convergence); cap one remains the reference mode.
>
> Owns: single-node concurrent execution — phase work items and local claims, target-proposed task graphs, private-workspace composition, protected verification inputs, domain convergence and multi-member waves, the shared local pool, and the synthesis payload redesign.
>
> Builds on implemented [RFC-91](rfc-91-refinement-stage.md) and [RFC-88](rfc-88-detached-changes.md). Amends RFC-90 D1, D2, D5, and D6; extends RFC-88 D8. Absorbs the former swarm-build and synthesis-redesign scope. [RFC-100](rfc-100-distributed-execution.md) may place the same tasks remotely; [RFC-18](future/rfc-18-slm.md) may later use the per-task model-selection hook.

## Intent

The slice remains Emery's smallest buildable, verifiable, repairable, and mergeable lifecycle unit. This RFC replaces large, multi-purpose model calls with focused agent tasks that converge on one slice result.

Plan authoring already decomposes surveyed leads recursively. RFC-88 refinement can send a leaf back through focused survey when its Evidence reveals separately acceptable child boundaries. A successful refinement still produces one coherent slice — Evidence, `spec.md`, `design.md`, and `tasks.md` — and that slice remains the lifecycle and acceptance unit.

Some implementation complexity appears only after refinement. One behavioural slice may require coordinated crate, test, and guest work that is too large for one model request but incoherent as separately accepted slices. This RFC divides that implementation into tasks without changing the slice boundary.

Today, one Omnia generation conversation combines all of this work with an opaque verify-repair loop. Observed builds serialized about 30 minutes of agent time and could then fail inside a hidden review team. Synthesis has taken 11–54 minutes while repeatedly carrying about 50 KB of playbook and artifact bodies. Survey and extract also remain serial.

Under this RFC, the engine invokes `target.decompose` at most once per slice-build attempt and receives a complete `graph | escalate` answer. Omnia uses a model-assisted prompt; an adapter that needs no decomposition may return a deterministic singleton graph. The engine validates and executes the graph in private workspaces, then runs RFC-90's slice-wide verify-repair-review loop. Escalation routes an incoherent slice back to plan authoring. A later build attempt reuses the persisted graph unless the previous attempt exposed a graph-attributable failure.

One shared pool also runs independent plan leaves, decomposition calls, review specialists, survey, and extract, all under the same scheduling and cancellation contract. Synthesis remains one cross-domain model call: it fetches its playbook lazily and writes artifacts through staging.

## Flow and terms

```mermaid
sequenceDiagram
    participant E as Engine
    participant T as Target adapter
    participant W as Engine-private workspace provider

    E->>T: decompose(slice)
    T-->>E: complete graph or escalate
    E->>E: validate complete graph
    loop ready task layer
        E->>W: prepare(candidate)
        E->>T: build(task, workspace)
        T-->>E: phase report
        E->>W: capture(workspace)
        E->>W: compose(layer base, patches)
        W-->>E: next candidate
    end
    E->>E: run RFC-90 phase machine on candidate
    Note over E,T: verify and review inspect the whole candidate
    Note over E,W: task-scoped repairs use fresh workspaces, then capture and compose
```

A **slice build** is one `emery slice build` attempt: one terminal report and one lifecycle outcome.

A **task** is an ephemeral, agent-sized leaf in a target-proposed **task graph**. It carries:

- a thin brief
- path-first inputs
- exact product and artifact write grants
- MCP-lazy references

A **worker** is one engine-dispatched `target.build` or `target.repair` call, scoped to a validated task and returning a typed phase report.

Tasks need not be independently buildable or mergeable. The composed slice candidate is the first result eligible for verification and acceptance. A task graph orders tasks into same-base **task layers**; a later **fan-in task** exclusively owns any shared path.

A **protected verification input** is an exact in-tree file or tree in the admission-covered leaf envelope that no build or repair worker may change. A **protected oracle** is external read-only material identified by a covered logical id and content digest. Either may hold baseline tests, contract fixtures, or behaviour replays. Candidate-authored tests remain ordinary writable product paths and provide self-consistency evidence instead.

A completed writing worker yields an RFC-87 **code patch**:

```text
{ base snapshot, result snapshot, touched paths }
```

A singleton graph preserves RFC-90's single-writer shape: one `target.build` before the slice-wide verify-repair-review loop. At plan level, a **domain round** records convergence over child slice results. A **target wave** is RFC-88's frozen same-target leaf set, accepted atomically. Task graphs and plan domains gain records; they do not gain lifecycle status or claims.

## Worked examples

One Omnia slice asks for a Rust library, its integration tests, and a WASI guest. An initial candidate decomposition might be:

```text
crate writer  owns tree crates/payments/src
test writer   owns tree crates/payments/tests
guest writer  owns tree guests/payments
```

The candidate proposes three tasks as agent-sized implementation scopes, not independently acceptable slices. One of them also owns build-level reporting; that designation creates no extra task.

When graph validation finds no shared-path interaction, all three tasks start from `sha256:base`. The engine captures their disjoint patches and composes them into `sha256:composed`. Only this composed candidate receives slice-wide verification. A finding at `crates/payments/src/client.rs` routes to the crate task, which receives the complete candidate but may change only its grant.

If several writers need to change `crates/payments/src/lib.rs`, the initial candidate is invalid: that file cannot remain under the crate writer's `tree crates/payments/src` grant. A `tree` grant already includes the file, and the closed `file | tree` grammar has no exclusion form. Before preparing any workspace, bounded answer repair returns a complete replacement graph — it does not subtract a path from the tree grant. The replacement uses exact file grants for the parallel layer and assigns `lib.rs` exclusively to a later fan-in task:

```text
layer 1
  crate writer  owns file crates/payments/src/client.rs
  test writer   owns tree crates/payments/tests
  guest writer  owns tree guests/payments
layer 2
  integration task owns file crates/payments/src/lib.rs and build reporting
```

Layer 2 starts from Layer 1's composed candidate. Its integration patch therefore names the intermediate snapshot as its base, not `sha256:base`.

Unexpected overlap rejects the entire layer and fails the attempt before composition. The engine records an ownership finding for every contributor. Those findings become input to the next complete graph proposal, which can narrow the writers' grants so they no longer cover the shared path and assign that path to the fan-in task. No subset of a failed layer becomes authoritative, and the engine does not use textual auto-merge.

## Decisions

### D1 — The engine owns one slice build and its task-graph phase

RFC-90's single build attempt gains one engine-owned phase before verification:

```text
decompose → validate → execute
```

The engine executes the graph one ready layer at a time. Each layer follows RFC-87:

```text
prepare → target.build → capture → compose
```

Every task in a layer starts from the same current candidate. The composed result becomes the base for the next layer. These are orchestration steps, not new lifecycle states. `prepare` and `capture` come from RFC-87. For a singleton graph, composition is the identity.

The engine owns:

- graph validation and ordering
- workspaces and retries
- budgets
- report aggregation
- verification
- the terminal report
- the slice transition

The target adapter owns one-pass target behavior:

- the decomposition prompt
- task-specific build and repair
- candidate-wide verify and review

Before preparing a writable workspace, the engine validates the graph and binds its digest to the slice revision, target identity, model-capability-profile digest, resolved inputs, and base snapshot.

Each task-scoped `build` or `repair` returns an RFC-90 phase report. The engine persists it under the slice-build attempt with the graph digest and task id; the report's ordinal also emits RFC-90's `slice.build.phase-completed` event. Tasks have no independent terminal report — the engine aggregates their reports into the attempt's one terminal result.

On terminal success, that aggregated result feeds one RFC-86 `BuildRecord` at `builds/<digest>.yaml` for the slice attempt — base/result/`touched` of the composed candidate, its authorization-anchor digest, and the terminal report. Closed execution uses the open wave anchor; RFC-99 progressive execution later adds the candidate-batch variant. Tasks never mint their own `BuildRecord`, wave, candidate batch, or merge fact.

The validated graph record is independent of any attempt; its key is the decomposition operation key. Every re-entry creates a new RFC-90 attempt id, new workspaces, and new continuations. The next attempt may reuse the completed graph when its profile digest is unchanged after:

- an abandoned attempt
- an infrastructure or dispatch failure
- an invalid phase report
- an exhausted repair budget where existing task grants cover the findings

The following failures are **graph-attributable**:

- an out-of-grant write
- overlap within a layer
- an unresolved finding that no task grant covers

After a graph-attributable failure, the next attempt calls `decompose` again, supplying the prior graph and findings as planning evidence. The new operation key covers both digests. A deterministic adapter may return the same singleton graph under the new key.

The replacement graph's base depends on the failure point. Overlap and out-of-grant writes happen before composition, so the next attempt restarts from the recorded base. An unowned residual finding happens after the engine has composed and verified a candidate; the replacement graph may then use that immutable candidate as its base snapshot. This carries completed work forward — the follow-up graph can often contain one small fan-in task that owns the previously unowned path.

A task has no slice lifecycle, plan status, claim, independent terminal report, or merge authority.

### D2 — Target decomposition proposes one complete graph

`target.decompose` receives:

- the refined slice, including `tasks.md` and `spec.md`
- target guidance
- path-first target context
- the pinned model-capability profile and task-complexity threshold

After a graph-attributable failure, it also receives the prior graph and relevant findings. If the failure happened after composition, it receives the prior composed candidate as the proposed base. Decomposition never edits the candidate. `target.repair` remains the separate, attempt-scoped writer and may write only within one validated task grant.

Decomposition returns one typed `graph | escalate` answer. A graph contains the complete task set and all dependency edges. The adapter owns its prompt and its interpretation of target architecture. The engine accepts a graph only if:

- every `tasks.md` entry is covered
- every refined `spec.md` requirement id is covered
- the task count is within the fixed budget
- every task carries the closed complexity assessment and fits the profile's task threshold
- dependencies are acyclic
- every grant is inside the slice's authorized ownership envelope
- no grant intersects a protected verification input
- predicted interaction has no ambiguous path ownership

The adapter proposes task complexity dimensions and rationale. The engine computes each score from the pinned profile. Neither the adapter nor the model selects thresholds or broadens the profile. Exceeding one task's threshold is a reason to add or split tasks inside the graph; it is not by itself a reason to create more slices.

An `escalate` answer states that the refined slice is not coherent and includes a typed `boundary | envelope` rationale: `boundary` when the Evidence supports independently acceptable children; `envelope` when no bounded task graph can fit the target's complete build and verification envelope.

For a `boundary` escalation, the engine uses RFC-88's refinement feedback path. Focused source surveys author child leads, affected-domain decomposition authors candidate nodes, and one inert proposal carries those candidate planning revisions. The target answer cannot author leads or slices.

For an `envelope` escalation with no coherent split, the inert proposal records the blocking profile and target envelope and carries no invented child leads. The operator may change the model-capability profile, target, or change scope through a legal amendment.

An envelope escalation may specifically identify an architectural obstruction outside the reviewed slice ownership envelope — a path, API, or design decision that must change before any valid task graph can implement the slice. The answer names the obstruction and nearest affected conflict domain as evidence only. It cannot widen a grant or license an out-of-scope patch; RFC-88's inert proposal and operator amendment boundary decide whether to introduce or reorder prerequisite work.

The engine then fails the attempt with a typed `decomposition-escalated` terminal report, before preparing any writable workspace. Escalation cannot edit the plan, execute product work, or alter a frozen wave; the operator applies or discards the proposal through `emery plan amend --proposal`. Tasks never become slices — a boundary mistake routes up to plan authoring, not down into the task graph.

Re-decomposition has a fixed budget: at most two graph-attributable re-decompositions per slice revision. No adapter or model can reset that engine constant. A third graph-attributable failure parks the slice for the operator, who can amend the plan or use the standing escalation path. Repeated decomposition failure is evidence of a boundary problem; the engine does not retry it indefinitely.

Each validated task record contains:

- a stable task id
- a role and brief
- covered scope
- the closed complexity assessment and engine-computed score
- path-first inputs
- exact product and artifact grants
- dependencies
- whether the task owns build-level reporting

Exactly one task owns build-level reporting. That designation alone creates no extra layer. The model may propose task values only within the slice's authorized ownership envelope. It cannot dispatch workers, select budgets, create lifecycle state, or broaden the envelope. Invalid answers enter the ordinary bounded answer-repair path before any product write.

SDK helpers provide a deterministic singleton graph whose task owns build-level reporting, so adapters need not decompose work merely because the operation exists. An adapter may also assemble a candidate graph from deterministic target metadata — for Omnia, the Cargo workspace member graph — and let a model call validate and adjust the candidate instead of inventing one.

### D3 — Every task has exclusive, enforced write ownership

The engine lowers RFC-88's slice ownership envelope into RFC-90's exact `file | tree` grammar. Task grants contain no globs and must remain within that envelope.

The leaf may carry `protected-verification-inputs[]` in the same exact `file | tree` grammar and `protected-oracles[] { id, digest }` for external material. Target metadata may nominate target- and platform-specific defaults during plan authoring, but has no authority by itself: protection becomes effective only through the exact RFC-88 decomposition revision covered by the current build admission. RFC-91 closed execution supplies operator authorization; RFC-99 may supply policy admission for an unattended candidate build.

`target.decompose` receives those closed sets and cannot add, remove, or widen them. The engine rejects overlap between any build or repair grant and an in-tree protected input. A target cannot freeze a path or introduce an oracle after admission. Protected inputs are materialized read-only into every task and candidate-wide verification workspace; captured writes remain authoritative, and any operation that changes a protected path fails the attempt with a typed ownership finding. Digest-verified oracle material is mounted read-only outside the candidate tree under its covered logical id.

Predicted interaction between disjoint grants becomes a dependency. If several writers need the same path, the graph must not leave that path under any parallel writer's grant. A `tree` grant that would cover the shared path is replaced with narrower `file` or `tree` grants that do not cover it; the shared path is assigned exclusively to a fan-in task. A dependency alone does not authorize shared-path writes. Ambiguous ownership fails graph validation.

Only the graph's report owner may write the artifact stage during `build` or a later task-scoped `repair`. Only that task's `build` report may return outputs or a UI-surface declaration; every other `build` report, and every `repair` report, must leave those fields empty, as required by RFC-90. Non-owner reports may still list product-workspace writes, but not artifact writes. When integration work is needed, the report owner may be the terminal fan-in task; otherwise an existing writer can own reporting without adding a task.

The engine aggregates findings from every task report and takes outputs and UI-surface only from the report owner's `build` report. RFC-90's output-existence gate checks the fully composed candidate, not the report owner's private workspace. Captured touched paths are authoritative: an out-of-grant write or layer overlap fails the attempt before composition and invalidates the graph for the next attempt.

A model may propose the task graph, but the admission-covered decomposition already fixes protected inputs. Only engine validation authorizes the graph. Textual merge never resolves ownership.

### D4 — Concurrency changes dispatch, not semantics

The engine uses one scheduler and one bounded local pool. The concurrency cap changes dispatch only; it does not select a different orchestration path. A cap of one provides the serial reference mode. Higher caps run ready same-base writers concurrently. After verification passes, specialist model calls inside one `target.review` may also run concurrently. The shipped default cap is four. Cap-one/four equivalence is an acceptance gate, not a separate delivery stage: the same task outcomes must produce the same ordered composition and slice result.

The first implementation uses the project model for every operation. Retained telemetry records the effective route and model identity when the backend exposes them, but the scheduler does not choose a model from price or task labels. The per-task model-selection hook remains inert until comparative evaluation shows a stable benefit; RFC-18 or another follow-on may activate it without changing task, ownership, or lifecycle semantics. Capability profiles continue to size work for a model class and are not overloaded with provider routing policy.

Workers never share a writable tree, live handle, MCP state, or prompt state. RFC-100 owns remote placement.

This replaces RFC-90 D5's one physical workspace for the complete loop. A build attempt instead has one logical candidate:

- a code snapshot
- a staged-artifact tree

The engine materializes that candidate into a fresh private workspace for every build, repair, verify, or review operation. During initial graph execution, only the report owner receives a writable artifact stage; every other task sees staged artifacts as read-only. The engine carries captured code and the report owner's validated artifact diff forward, so later operations observe the same logical candidate.

No two operations share a writable directory or artifact-stage handle. Intermediate snapshots and artifact diffs remain inert execution values. The engine records the final code result and promotes staged artifacts through one success gate.

A `build` or `repair` continuation may resume only for the same task, attempt, and graph, against the current logical candidate. The candidate-wide `review` continuation is scoped to its attempt and graph. No continuation may encode or depend on a workspace path or live handle; every resumed operation receives a newly materialized workspace.

### D5 — RFC-90 verification and review converge the complete slice

Writer and repair workers receive findings, not Cargo commands.

After the complete graph produces one composed candidate, the engine dispatches RFC-90 `verify`.

For one blocking verify report, the engine first applies RFC-90 D4 once to the complete slice report, producing at most 16 globally ordered repair findings. It then groups the located findings in that brief by their unique task owner. It may run pairwise-disjoint repairs concurrently:

```text
target.repair(origin: verification, task, findings, continuation)
```

All repairs caused by one report consume one RFC-90 verification-repair round, regardless of worker count. The cap is per source report, not per worker. Findings omitted from the brief remain in the complete persisted report and may reappear after the next candidate-wide verification; fan-out never multiplies RFC-90's repair-input bound. The engine captures and composes the repair patches, then verifies the next complete candidate.

A task continuation cannot cross task, attempt, or graph identity. Verification is model-assisted evidence, not deterministic proof or a security boundary. A check over candidate-writable tests is self-consistency evidence. A check over protected inputs is stronger only to the extent that the report identifies those inputs and the engine enforced their write exclusion; RFC-97 defines host-attested profile assurance.

Some blocking findings cannot be routed:

- unlocated findings
- located findings on paths that no task grant covers

These remain typed residual failures. The engine does not broaden an arbitrary task's authority. Residual failures invalidate the graph and become planning evidence for the next attempt's decomposition call. Under D1, the follow-up graph may use the composed candidate as its base. Findings already covered by a task grant do not invalidate the graph. The engine never widens a task grant or mutates the graph during an attempt. Repair grants produced by one report must remain pairwise disjoint; captured overlap fails the attempt.

After verification passes, the engine dispatches one `target.review`.

Inside that dispatch, Omnia may run Security, Correctness, and Quality as separate specialist model calls. Each call is observable and has its own timeout. Compiled adapter code joins their results into one review phase report.

Those specialists provide concern-based lenses in the first cut. Live evaluation may compare decorrelated inputs or model classes—for example candidate-only against candidate-plus-slice-artifacts—but the adapter still returns one bounded review report under RFC-90's single review budget. Another lens graduates only when it finds additional blocking defects at acceptable marginal cost; the runtime does not multiply reviewers or remediation rounds from a model response.

Blocking review findings use the same global RFC-90 repair-brief projection and task-owner routing:

```text
target.repair(origin: review, task, findings, continuation)
```

All repairs caused by one review report consume RFC-90's one review-remediation round. The engine then returns to `verify → review`.

The terminal report contains:

- aggregated task-build findings
- the latest verification findings
- the latest review findings

RFC-90's existing projection rules still apply. Task-repair findings and superseded verify or review rounds remain in attempt-scoped phase records. No task passes independently.

### D6 — Code-patch composition is one reusable deterministic kernel

The engine-private RFC-87 capability gains `compose(base, patches)`.

The operation requires one base and disjoint touched paths. It copies exact result-tree values in fixed order, captures the candidate, and discards the temporary workspace. A base mismatch or overlap fails before verification.

The same kernel combines code patches for task layers, single-target frontier rounds, and final target-wave commit. Domain verdicts and RFC-88's spec fold remain separate operations. `augentic/backends` owns pooling and cancellation; it does not own composition.

### D7 — The synthesis playbook moves to an engine references shelf

An engine shelf such as `/mcp/engine/synthesis` serves embedded guidance through `list_docs` and `read_doc`. The prompt keeps `synthesize.md`, its contract, its answer schema, and a measured inline minimum, and fetches the remaining roughly 50 KB only when needed. Emery owns the shelf and its grants.

### D8 — Synthesis artifacts use a lent staging tree and an outcome-only answer

The host lends synthesis an execution-local staging tree. The answer carries only an outcome. The deterministic tail validates the whole tree: on success it promotes the tree atomically; on failure it returns findings so the same agent can repair the tree in place. D8 follows D7's live-eval gate. It changes neither synthesis authority nor provenance semantics.

### D9 — Survey and extract fan out through the shared pool

After RFC-88 pins topology, `plan author` runs independent initial and focused source surveys concurrently. The shared scheduler extracts each selected refinement work item's bound Evidence concurrently. Results merge in canonical order — binding order or `(source, parent lead, child lead)` — never completion order.

RFC-88's refinement boundary assessment runs only after all bound Evidence has joined successfully. A failed extract fails refinement through the ordinary path. No partial Evidence set can author a boundary proposal.

A validated boundary escalation promotes no synthesis artifacts and performs no `refined` transition. The engine fans out the requested focused surveys through the same pool, merges child leads canonically, and evaluates independent affected domains concurrently. All affected leads and candidate domain revisions join into one RFC-88 amendment proposal; completion order cannot change its content. Cancellation reaps every focused survey and decomposition call if proposal assembly fails.

RFC-104's system-discovery reads and RFC-88's exact delivery-binding reads retain their separate budgets.

### D10 — Recursive plan decomposition is bounded engine orchestration

After the initial inventory, one compiled orchestration evaluates independent RFC-88 conflict domains concurrently. Each bounded model call receives one domain and returns a typed `split | leaf` answer. The engine owns queueing, budgets, scope reduction, coverage, identity, and ordering.

Each call's operation key covers the relevant model-capability profile digests. A profile change cannot reuse a decomposition result from the previous planning revision. `decomposition.yaml` and `plan.yaml` publish together only after the complete tree passes. Partial publication and model-spawned recursion remain deferred.

### D11 — The local scheduler folds ready leaves through domain gates

The scheduler projects deterministic **phase work items** keyed by `(slice, phase, input-digest)` for `refine | build | merge`. The input digest fences dispatch against changed planning, refinement, wave, or dependency inputs.

Readiness remains phase-relative:

- refine requires fresh predecessor refinement manifests;
- build requires an exact admission-covered refinement manifest, a passing gap policy, and accepted predecessors;
- merge requires a successful build record, passing gates, and a current accepted frontier.

RFC-99 progressive candidate mode amends only build readiness: a direct predecessor may be an exact successful `BuildRecord` under the same parent run, and the work-item digest then covers that result plus the current target candidate-frontier digest. Merge readiness is unchanged.

Unlike RFC-91's serial refinement drain and the landed execute cursor, this scheduler may hold multiple entries and phases in progress. Selection is canonical by target, topological layer, plan order, slice, and phase before the pool cap truncates the ready set.

A local operation claim names the parent work-item identity plus the concrete operation, attempt, and task identity where applicable. It prevents duplicate execution of that operation, not sibling operations, later phases, or changed inputs for the slice. The claim is released on success, terminal failure, cancellation, or retraction. Task workers remain subordinate to one slice-build work item and gain no slice lifecycle or merge authority.

RFC-100 transports the same work-item identity through durable offers, lease-backed claims, ownership generations, and stale-result rejection. It does not redefine local readiness or selection.

```mermaid
flowchart LR
    A[Ready leaves] --> B[Frozen same-target wave]
    B --> C[Build members<br/>slice verify and review included]
    C --> D[Frontier domain gates]
    D --> E[Atomic wave commit]
    E --> F[Accepted target CID]
    F --> G{Domain complete?}
    G -- no --> A
    G -- yes --> H[Complete domain round]
    H --> I[Parent domain or target drain]
```

`plan execute` opens at most one wave per target. It chooses from a bounded antichain where:

- dependencies are accepted
- ownership envelopes share the accepted base

The first implementation scans canonical target and leaf order up to the pool cap. It adds no optimizer or fairness policy.

The immutable wave manifest exists before claims and builds. RFC-86's landed cut enforces `members.len() == 1` (`target-wave-member-count`). This RFC retires that one-member-only gate for the concurrent executor: the same manifest schema, open fact, commit fact, and per-member `BuildRecord` revalidation accept a frozen multi-member set. Merge facts and accepted-CID semantics do not change. A slice-build failure creates a new slice attempt under D1 without changing wave membership. Each successful member still writes its own `BuildRecord`; wave commit consumes the complete member set. Membership stays frozen until atomic commit or operator amendment; an amendment retracts the whole uncommitted wave and does not shrink it.

After member builds, the engine groups results by their nearest frontier domain.

A single-target domain derives one canonical protected-input closure before verification. In-tree protection starts as the exact `file | tree` intersection of every contributing descendant's covered protected set. The engine removes an entire protected entry when any contributing patch touches that file or any descendant of that tree; it never invents an exclusion grammar or expands a tree into ambient filesystem entries. External protection is the intersection of identical `(id, digest)` oracle entries. The engine persists the closed lists, hashes them, and includes that digest in the domain operation key. An empty intersection is valid and carries no protected-oracle assurance.

A single-target `frontier` round composes only that domain's same-base child patches, then dispatches one `target.verify` over the composed candidate. This domain convergence gate checks interaction above the completed slice-wide verify/review loops; it is not another slice build. One wave may therefore require several frontier rounds. Multi-target rounds do not compose trees or dispatch cross-target verification — they aggregate ordered target results and dependency health.

For a single-target domain, a `complete` round runs the same domain-level `target.verify` over the current accepted tree and committed frontier chain, only after every child and dependency is complete, and never recomposes cross-base patches. A multi-target `complete` round only aggregates ordered child verdicts and dependency health.

A frontier failure blocks the current frozen wave. A complete-round failure preserves accepted waves and blocks dependants and drain until an operator-authorized repair or fan-in leaf advances a new epoch. RFC-102 policy-gated autonomy treats a blocking post-acceptance complete round as a hard stop; it cannot apply standing recovery after accepted-state mutation.

### D12 — Domain rounds are durable and target waves accept atomically

Before emitting `domain.convergence.recorded`, the engine atomically writes one closed `frontier | complete` record.

The record contains:

- revisions
- child slice-attempt or domain-record digests
- authorization anchors
- bases
- the patch chain or committed-wave chain
- result CIDs
- protected-input closure digest
- the domain-level `target.verify` report digest, when verification ran
- the verdict

The digest of the validated inputs and accepted frontier is the operation key. On re-entry, the engine reuses a completed record and does not rerun composition or the domain gate. The operation key does not imply deterministic model output; live records root candidate snapshots.

After all frontier gates pass, RFC-86's target-wave merge revalidates every member and its exact commit authorization. The engine composes the frozen set and publishes one `target.merge.wave-committed` fact, which advances the accepted CID and projects every member as merged. No prefix of the wave is authoritative.

A target drains only when:

- all leaves have merged
- postflight failures have been acknowledged
- every root domain has a passing `complete` round for the current revision and CID

### D13 — Harness evaluation measures accepted outcomes and coordination cost

The cap-one reference path and cap-four path run comparative live fixtures with the same source set, model configuration, time budget, and blind acceptance set. Blind inputs remain unavailable to planning, decomposition, build, repair, verify, and review; they grade the completed attempt outside workflow authority.

The retained facts, build phases, graphs, domain records, and available model-backend observations project:

- merged requirements and accepted CIDs per wall-clock hour and per reported model cost
- time to first accepted result
- planner/decomposition calls and the worker tokens and cost they induce
- graph reuse, graph-attributable re-decomposition, residual findings, and amendment proposals
- touched-path heat, fan-in depth, ownership overlap, and waves per target
- generated code and module growth as coordination signals, not quality verdicts

Missing provider usage remains unknown. Raw model calls, worker count, commits, and generated lines are activity, not success. These projections neither gate lifecycle nor select a model. They establish the evidence needed to tune fixed budgets, activate model routing, add review lenses, or revisit RFC-88's complete-plan publication in later work.

## Delivery slices

This RFC has two independently useful implementation slices over one scheduler contract:

### Phase A — Work-item scheduler and read-heavy pool

Land `(slice, phase, input-digest)` identity, deterministic ready-frontier projection, cancellation, local operation claims, and the bounded cap-one/cap-four pool. Move initial survey, focused survey, extract, refinement, and independent decomposition judgments onto that pool with canonical result ordering.

Phase A keeps complete-plan publication and one-member target waves. It adds no target operation and does not require task graphs, code-patch composition, domain rounds, or multi-member commit.

RFC-99 progressive refinement may depend on this phase rather than completed RFC-96. Its branch publications add ready refinement items to the same pool; they do not create a second scheduler.

### Phase B — Concurrent build and convergence

Add `target.decompose`, task graphs, protected inputs and oracles, isolated writers, code-patch composition, task-scoped repair, domain rounds, and multi-member waves. This phase completes the RFC and unlocks RFC-99 progressive build.

Cap one remains the reference path in both phases. Phase B cannot replace Phase A's identities, selection order, cancellation, or claim semantics.

## Implementation requirements

- **Task execution implements D1–D3 and D5–D6.** Add `target.decompose` and task context to RFC-90 `build` and `repair`. Add profile-scored task complexity, digest-bound graph and phase records, engine-private `compose`, admission-covered protected verification inputs and oracle digests, Omnia's model-assisted graph, and the SDK singleton. Implement typed residual failure, escalation, and graph-attributable re-decomposition. Enforce its two-round budget and candidate-based follow-up graphs.
- **One pool implements D4 and D9–D12.** Add one isolated host pool that supports both the cap-one reference mode and the default cap of four. Replace the serial entry cursor with deterministic `(slice, phase, input-digest)` work items and operation-scoped local claims; project phase frontiers and release every terminal, cancelled, or retracted claim. Use the same scheduler path for both caps. Add initial survey, focused resurvey, extract, affected-domain, review, task, and repair fan-outs. Add canonical bounded-antichain scheduling, domain records, and multi-member target waves (retire `Wave::enforce_one_member` for the concurrent executor only; keep the same manifest and `target.merge.wave-committed` shape). Aggregate each slice attempt into one `BuildRecord`. Cancellation must reap every call.
- **Synthesis implements D7–D8.** Land the engine shelf and pass `omnia-r9k`. Then land staged synthesis and pass `orders-contracts`. Neither final grade may regress.
- Derive closed graph and domain schemas from Rust DTOs. Reject unknown fields.
- Persist validated-content digests, operation keys, task-scoped phase events, canonical protected-input closures, and domain records as specified above. Add no extension map or second domain-state artifact.
- Enforce D4's continuation scopes, logical-candidate scopes, worker inactivity timeouts, and capture-then-discard lifecycle.
- Use the project model by default. Journal compiled budgets, decomposition decisions, and ordering; correlate them with backend-provided effective routing/model identity when available. Do not activate per-task model choice in this RFC.
- Extend the live fixtures with D13's same-input cap-one/cap-four comparison and blind acceptance grading. Project metrics from existing facts and phase/backend telemetry rather than writing a score into workflow artifacts.

## Acceptance criteria

1. **Omnia decomposition.** A build the size of `at-r9k-position-adapter` produces one complete graph through one model-assisted decomposition operation. Every task fits the pinned profile's task-complexity threshold. Spilled build prompts remain at or below 15 KiB. Exactly one task owns build-level reporting. No task independently passes. RFC-90 retains verification, repair budgets, and observable review specialists.
2. **Invalid graphs and re-entry.** Invalid graphs produce no writable workspace. Escalation creates only its inert proposal and terminal report. Re-entry always creates a fresh attempt. It reuses the graph unless an out-of-grant write, layer overlap, or unowned finding requires new decomposition. That call receives the prior graph and findings under a new operation key. After a post-composition failure, the follow-up graph builds from the composed candidate without rerunning completed tasks. A third graph-attributable failure parks the slice without another decomposition call.
3. **Ownership.** Predicted interaction creates dependencies. Shared paths have one fan-in owner. No task grant overlaps an admission-covered protected verification input, a write to one fails closed, and external oracle material must match its covered digest. Candidate-writable and protected-test results remain distinguishable. Captured overlap rejects the whole layer and attempt, then informs the next graph proposal. No textual merge occurs.
4. **Residual findings.** A located blocking finding on an unowned path fails the attempt as a typed residual failure. The next attempt's complete graph proposal receives it as typed input. No task grant or graph mutates in place.
5. **Private composition.** The engine composes only same-base, disjoint patches. Every target operation receives a fresh private materialization of the candidate. Targets never receive workspace lifecycle operations. Failure exposes no authoritative workspace or staged-artifact change.
6. **Concurrent target tasks.** With the pool cap set to four, two target tasks run concurrently in isolated workspaces. Cancellation reaps both. Caps of one and four produce the same ordered composition and one slice-wide result.
7. **Synthesis staging.** Synthesis loads nonessential playbook prose from the engine shelf. Its answer returns no artifact bodies. The engine promotes its staged tree only after validation.
8. **Concurrent survey and refinement feedback.** Concurrent RFC-104 system survey/extract and RFC-88 focused delivery survey preserve canonical order within their separate artifacts and budgets. Two independent refinement work items may run concurrently and produce the same canonically ordered manifests and status projection as cap one. A refinement boundary escalation runs focused surveys and affected-domain decomposition concurrently, then produces one byte-stable inert proposal without promoting slice artifacts. Three-level plan decomposition evaluates independent nodes concurrently and publishes no partial plan.
9. **Domain restart.** Independent leaves pass both same-target domain gates. Each operation key and record bind the canonical protected-input closure. On restart, the engine reuses each digest-bound record and candidate without repeating composition or verification.
10. **Atomic waves.** Two same-base leaves merge under one wave commit only after both complete. Retry preserves membership. Amendment retracts the uncommitted wave. Replay is idempotent. Dependencies use accepted bases. Complete-round failure blocks drain and RFC-95 sealing without rolling back accepted waves.
11. **Quality gates.** `cargo make ci` passes in every touched repository. D8 goldens regenerate. The `omnia-r9k` and `orders-contracts` live grades do not regress.
12. **Harness economics.** Cap-one and cap-four live fixtures use the same source set, model configuration, time budget, and blind acceptance set. The acceptance set is unavailable to every workflow model call and does not affect lifecycle. Results report D13's accepted-outcome, induced-worker-cost, graph-stability, amendment, and contention projections when their raw observations exist; missing provider usage stays unknown.
13. **Phase scheduling and claims.** Refine, build, and merge work may be in progress on different slices simultaneously. Status and selection do not depend on a singular active entry. Duplicate local claims on one fenced operation fail; a changed parent input creates a distinct work-item identity; every terminal, cancelled, or retracted operation releases its claim.
14. **Phase-A independence.** Concurrent survey, extract, refinement, cancellation, and cap-one/cap-four equivalence pass while target decomposition, composition, domain rounds, and multi-member waves remain disabled. RFC-99 can publish a closed branch into that scheduler without activating build authority.

## Rejected alternatives

- **Require model-free decomposition.** Fixed role templates are useful as singleton graphs or fast paths. Arbitrary refined tasks and target architecture still require bounded model reasoning. The engine validates and bounds the call instead of treating its output as deterministic.
- **Promote agent tasks to slices.** Crate, test, guest, and integration tasks may share one behavioural contract. They become valid only after composition. Giving them lifecycle or merge authority would duplicate the workflow below its smallest acceptance unit.
- **Keep large generation and review calls, or let an adapter recursively spawn workers.** This preserves opaque nested work. One adapter `decompose` operation returns a complete `graph | escalate` answer. Compiled engine policy owns validation, budgets, dispatch, and termination.
- **Force a mis-scoped slice through task decomposition.** When refinement reveals independently acceptable boundaries, tasks would hide a planning mistake. `escalate` routes the decision to an operator-applied RFC-88 amendment. Complexity above one task's threshold does not qualify by itself because a coherent slice may use several tasks.
- **Always decompose after terminal failure, mutate the graph in place, or retry decomposition without a bound.** Technical failures and covered findings do not invalidate a graph. Repeated graph failure signals a boundary problem. A graph-attributable failure triggers a fresh, budgeted proposal for the next attempt. The engine neither widens tasks nor accepts an overlap-free subset.
- **Let adapters own workspaces or loops, share writable trees, or use textual merge.** These choices cross the workflow boundary, hide retries, and make safety depend on timing.
- **Add task phase operations, writer commands, or full repair prompts.** RFC-90 already owns verification and repair. Another loop would duplicate vocabulary and payload.
- **Partition synthesis.** Cross-domain reconciliation is the reason for the model call. D7–D8 reduce payload without changing that call.
- **Require task graphs from every target.** Only Omnia has evidence for non-singleton decomposition.
- **Add remote workers now.** RFC-100 owns placement after the single-node contract settles.
