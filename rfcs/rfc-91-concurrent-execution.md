# RFC-91: Concurrent Execution

> Status: Draft — step 6 of the platform-migration series (scale track) ([platform.md](platform.md))
>
> Owns: one complete single-node concurrent path: the Omnia target build decomposed into focused convergent workers, the host-owned convergence gate, write-ownership partitioning, the local agent pool, per-worker local trees, deterministic changeset composition, plus engine-wide parallel survey/extract fan-outs and synthesis payload restructuring.
>
> Absorbs: [RFC-79 Swarm Build](archive/rfc-79-swarm-build.md) and [RFC-80 Synthesis Redesign](archive/rfc-80-synthesis-redesign.md) — one concurrency model, applied at build time and refine time.
>
> Depends on completed [RFC-86](rfc-86-change-facts.md) (merge-finalized requirement identity is what makes same-base concurrent synthesis safe, and worker/round outcomes land as per-actor facts), [RFC-90](rfc-90-verify-profiles.md) (host-owned verify is the convergence gate), and [RFC-87](rfc-87-working-trees.md) (local per-worker materialized trees and changesets). [RFC-78](archive/rfc-78-prompt-budget.md) already supplies per-request budgets, timeout semantics, and the session model.
>
> Consumed after completion by: [RFC-92](rfc-92-node-sync.md), which places this RFC's workers on remote nodes without changing their request, ownership, tree, or changeset semantics. Related: [RFC-18](future/rfc-18-slm.md) (per-worker model selection is this RFC's hook for cheaper backends).

## Intent

Replace the large single-conversation judgment legs with a **swarm**: many smaller, focused requests that converge on a verified result. One fat generation leg holding crate / test / guest writers together exists today for exactly one reason — it is the only shared verify-repair channel. This RFC replaces that channel with an explicit convergence architecture at build time, then applies the same decomposition posture to the other serial walls: plan-time survey, refine-time extract, and the synthesis leg's payload shape.

Focused requests bound each agent's blast radius, make every worker individually observable and timeout-able, open per-worker model selection, and complete a concurrent implementation on one machine. RFC-92 may distribute that settled worker contract later.

## Why now (evidence)

From [RFC-78](archive/rfc-78-prompt-budget.md)'s `wasm-omnia-r9k` runs:

- The five-leg omnia build serializes ~30 minutes of agent wall-clock; the review leg nests an invisible agent team inside **one** completion, where the host cannot observe, bound, or time out any member — the run died there.
- The generation leg is one ~64 KB conversation doing four jobs (crate writer, test writer, guest writer, verify-repair loop); verify (cargo build / clippy / test) runs as prompt text inside the agent loop — unobservable, unbounded, unshareable.
- Synthesis is the worst-observed wall-clock phase (11 and 54 minutes across the two runs): a ~50 KB inlined playbook per attempt, artifact bodies riding the schema-gated JSON answer (regenerated wholesale on every repair), and survey / extract fan-outs serialized only because the backend cannot isolate concurrent completions.

The fat leg's ceiling is structural: it cannot parallelize, cannot mix models, cannot bound sub-work, and its cost grows with slice size rather than task size.

## The model

### Topology

- **The WIT seam does not change.** `target.build` remains one dispatch per slice; the swarm is the adapter core's existing leg sequencing with more, smaller, possibly concurrent `create`s.
- **The orchestrator is deterministic guest code**, never a lead agent. It partitions work, issues worker requests, runs the convergence loop, and folds typed outcomes into the `BuildReport`. Judgment stays in the workers; sequencing and arbitration stay compiled-in.
- **A worker is one focused judgment request**: a thin role brief, only the inputs its task needs (path-first records per RFC-78 D1), an explicit write-ownership manifest, and a typed answer gate. Workers never receive the whole prompt corpus; references stay MCP-lazy.

### Partitioning

- **Writer roles first**: crate writer, test writer, guest writer — today's co-tenants of the fat leg become separate workers.
- **`tasks.md` is the finer partition source**: the refined slice's task list already decomposes the build into steps with implicit file ownership; the orchestrator maps task groups to workers and derives each manifest from them.
- **Review specialists become first-class workers** (Security / Correctness / Quality / antagonist), replacing the nested in-agent team — each individually budgeted, observable, and timeout-able.

Two workers never share write ownership of a path. Ownership conflicts are an orchestrator-time error, not a merge problem.

### Convergence

The in-prompt verify-repair channel is replaced by a **host-owned convergence gate** over [RFC-90](rfc-90-verify-profiles.md):

1. Workers complete focused writes and answer with typed outcomes — no cargo commands in worker prompts.
2. The orchestrator requests closed verify profiles through the host; the host runs them sandboxed and returns normalized findings.
3. Findings route back **to the owning worker** (by manifest) as focused repair requests — session resume + findings delta per RFC-78 D5, never a fresh full prompt.
4. Rounds are bounded by an explicit convergence budget; exhaustion is a typed `failure` report with residual findings attached.

### Concurrency substrate (backend, staged)

- **Stage A — sequential swarm, single lent tree.** No backend change: workers run one at a time, ownership enforced by the orchestrator, verify serialized between rounds. Lands observability, model selection, and bounded blast radius.
- **Stage B — concurrent swarm, single tree, partitioned writes.** Requires per-spawn MCP config isolation, per-worker prompt spills, and an agent pool with a concurrency cap. Verify remains serialized; disjoint ownership keeps workers from conflicting. The refine/plan fan-outs below consume Stage B as-is.
- **Stage C — concurrent swarm, per-worker local trees.** [RFC-87](rfc-87-working-trees.md) materializes one local tree per worker from the same `revision`; workers produce `changeset`s; this RFC applies them in dependency order to a fresh integration tree before the convergence gate. A base mismatch or patch conflict is a typed composition failure.

Every `changeset` in this RFC is RFC-87's tree-delta value; RFC-89's publication set is a separate forge-side record.

### Agent pool and per-worker policy

- The backend grows from "spawn one agent per completion" to a **pool**: bounded concurrent spawns, per-worker inactivity timeout, per-worker session scoped to its repair chain, pool-level cancellation. Implementation (Rust process-pool vs SDK sidecar) decided by Stage B evidence.
- **Per-worker model selection**: the request's `model` field already crosses the backend; the orchestrator may bind writers, specialists, and repair rounds to different tiers — the concrete hook [RFC-18](future/rfc-18-slm.md) needs.

## Decisions

| # | Decision | Consequence |
| - | -------- | ----------- |
| D1 | **In-guest deterministic orchestrator.** Partition → dispatch → converge → fold in the Omnia adapter core; reusable brief, manifest, and outcome helpers live in the adapter SDK, but no other target must adopt them in this RFC. WIT stays unchanged. | One target path completes end to end; Vectis and Contracts keep their current serial build behavior rather than becoming hidden follow-up phases. |
| D2 | **Convergence gate over RFC-90 verify profiles.** Closed profile names, host-owned argv, sandboxed execution, normalized findings mapped to owning workers. | Cargo command text leaves worker prompts entirely; verify output normalizes once; the budget is host policy, not prompt prose. Stage A ships with D2, not before it. |
| D3 | **Write-ownership partitioning.** Every worker carries an explicit manifest; overlaps are rejected before dispatch; out-of-manifest writes are blocking findings. | The invariant that makes Stages B and C — and [RFC-92](rfc-92-node-sync.md) D10's plan-level manifests — safe. |
| D4 | **Local backend concurrency is staged A → B → C.** Stage A is sequential; Stage B isolates concurrent completions in one workspace; Stage C gives every worker its own local tree and composes changesets into a fresh integration tree. | RFC-91 completes all three local stages. Remote placement is not a hidden fourth stage. |
| D5 | **Host-visible review swarm.** Specialists as first-class workers with typed findings, individual budgets and timeouts; the antagonist gated on specialist outcomes; remediation as routed repairs through the same gate. | No nested invisible agent team; the failure mode that killed the r9k run is structurally removed. |
| D6 | **Changeset composition is a reusable deterministic kernel.** Given one base revision and an ordered list of RFC-87 changesets that all name that base, the kernel applies them to a fresh integration tree and refuses base mismatch or patch conflict before verify. Stage C orders worker outputs by worker dependencies; RFC-92 later projects plan dependencies into per-project lists for the same kernel. | RFC-87 remains a one-base materializer, RFC-91 completely owns composition mechanics, and later scheduling cannot redefine apply semantics. |
| D7 | **Engine references shelf: the synthesis playbook goes lazy.** Extend the launcher route table with an engine shelf (e.g. `/mcp/engine/synthesis`) serving the embedded playbook corpus through the existing `list_docs` / `read_doc` contract; engine judgment legs gain the MCP grant. The synthesis system prompt shrinks to `synthesize.md` plus the measured always-inline subset. | Most of ~50 KB saved per synthesis attempt, ×1–3 under repair — the same lazy posture adapter legs already have. |
| D8 | **Synthesis artifacts move to the lent tree; the answer becomes an outcome record.** The agent writes artifacts into a staging directory; the answer shrinks to a typed outcome record; the deterministic tail gates the staged tree (validate-before-visible), promotes on a clean gate, and issues repairs with findings only — the agent edits staged files in place. | A synthesis repair becomes an edit round, not a full regeneration; the answer schema becomes a gate the host can actually enforce. Largest single item; lands behind the live eval gate, after D7. |
| D9 | **Parallel survey / extract fan-outs** (over Stage B): `survey_all` dispatches all bound sources concurrently and merges into `discovery.md` in binding order; refine's extract fan-out likewise, with per-source evidence files as the natural disjoint write set. | Concurrency is a dispatch property, never an output property: merged outputs stay byte-identical to the serial order. Plan-time surveys are the first consumer. |

## Non-goals

- Changing the WIT `target` interface, the `BuildReport` contract, or the synthesis authority model (`[conflict]` / `[divergence]` / `[unknown]`, provenance) — this RFC moves bytes, channels, and dispatch, not semantics.
- Cross-slice scheduling, remote worker placement, value transport, and hosted execution — [RFC-92](rfc-92-node-sync.md).
- Model backend replacement ([RFC-18](future/rfc-18-slm.md)) — only the per-worker selection hook is created here.
- A lead-agent orchestrator. A judgment leg deciding what other judgment legs to run re-creates the fat leg one level up.
- Decomposing synthesis by domain. Cross-domain reconciliation is the judgment being purchased; RFC-91 reduces its payload and repair cost without partitioning it.
- Swarm adoption by Vectis or Contracts. The SDK helpers are available, but each target requires its own evidence and RFC or bounded change.

## Ownership

| Decision | Repo |
| -------- | ---- |
| D1 orchestrator + SDK helpers | `augentic/emery` (`crates/adapter`) + `augentic/emery-adapters` (`targets/omnia`) |
| D2 verify activation | `augentic/omnia` (`wasi-model` verify) + `augentic/emery` policy |
| D3 ownership manifests | `augentic/emery-adapters` (`targets/omnia`) + SDK types in `augentic/emery` |
| D4 pool / local workspace stages | `augentic/backends` (Stage C with the RFC-87 backend) |
| D5 review swarm | `augentic/emery-adapters` (`targets/omnia`) |
| D6 changeset composition | `augentic/emery` + `augentic/backends` |
| D7 engine shelf + grants | `augentic/emery` (`crates/guest`, `crates/launcher`, `crates/project`) |
| D8 staged artifacts + outcome record | `augentic/emery` (`crates/slice` persist / answers / prompts) |
| D9 parallel fan-outs | `augentic/emery` (`crates/change`, `crates/slice`), gated on Stage B |

## Phased delivery

- **Phase A — Observable sequential swarm.** Split Omnia build writer/review roles, enforce manifests, route all verification through completed RFC-90 profiles, and bound repair rounds while workers still run serially.
- **Phase B — One-tree concurrency.** Add concurrent in-flight model calls, the shared local pool, isolated MCP/prompt state, host-visible review specialists, and parallel survey/extract fan-outs with deterministic output order.
- **Phase C — Per-worker trees and composition.** Materialize one RFC-87 tree per worker, extract changesets, compose ordered same-base lists through D6, and run convergence in a fresh integration tree.
- **Phase D — Synthesis payload completion.** Ship the engine reference shelf, lease-local staged artifacts, outcome-only answers, atomic promotion, and focused repair deltas. RFC-91 is complete when Phase D and the live eval gates pass.

## Acceptance criteria

1. An omnia build for a slice the size of `at-r9k-position-adapter` completes as focused worker requests, each with a spilled prompt ≤ ~15 KB, and no worker prompt contains cargo command text.
2. Verify runs only through closed RFC-90 profiles; findings route to the owning worker; convergence-budget exhaustion produces a typed `failure` report with residual findings.
3. Review specialists are individually observable and timeout-able; no nested in-agent team remains in the omnia review path.
4. Overlapping ownership manifests are rejected before dispatch; an out-of-manifest write surfaces as a blocking finding.
5. Stage B: two workers run concurrently against one tree with isolated MCP config and no shared-file races; pool cancellation reaps all in-flight workers.
6. Stage C: concurrent workers build against separate RFC-87 trees on one machine, return changesets against the same base, and deterministic composition in a fresh integration tree passes the convergence gate; base mismatch and patch conflict fail before verify.
7. The synthesis system prompt carries `synthesize.md` plus the measured always-inline subset, with the rest fetched lazily from the engine shelf; the synthesis answer is an outcome record and artifact bodies never cross the answer channel; staged artifacts are never visible slice state before the gate passes.
8. With Stage B available, `plan author` over N sources dispatches surveys concurrently and `discovery.md` is byte-identical to the serial run's.
9. `cargo make ci` green in touched repos; goldens regenerated with D8; live eval (`omnia-r9k` / `orders-contracts`) shows quality parity after each of D7 and D8, evaluated separately.

## Risks and invariants

- **The convergence gate must exist before the split.** A swarm without host-owned verify is the fat leg's verify-repair loop deleted, not replaced.
- **Verify is a security boundary** (RFC-90 verbatim): the swarm multiplies how often verify runs; it must not widen what verify accepts.
- **Ownership is exclusive and checked.** Concurrency safety comes from partition discipline, not merge cleverness.
- **Deterministic orchestration.** Budget, ordering, and routing are compiled-in policy, auditable in the journal.
- **Tree isolation.** Stage C workers share no writable tree or live handle; their only code-bearing outputs are `changeset` values.
- **Quality is the product.** Synthesis changes (D7–D8) ship behind the live eval gate; domain decomposition is explicitly out of scope.
- **Per-worker budget assertions** (RFC-78 D7) extend to the swarm: brief sizes are locked in adapter tests.

## Fixed implementation cut

- Stage B requires concurrent in-flight `create` calls from one guest; that Omnia capability lands as part of this RFC's backend work, not as a later prerequisite.
- Initial partitions are the existing writer roles. `tasks.md` subdivides a role only when task paths prove disjoint under D3; there is no model-chosen partition.
- Each worker receives at most two repair rounds, and one build receives at most three convergence rounds. Exhaustion returns the residual findings.
- One host-level pool cap covers build, review, survey, and extract workers. The default is four; deployment configuration may lower it. RFC-92 schedules whole remote pools without adding another local limit.
- The synthesis system contract and answer schema stay inline; all playbook guidance moves to the engine shelf.
- Staged synthesis artifacts live in a lease-local host tempdir lent to the worker and are promoted atomically only after validation. They never appear under the authoritative slice directory before promotion.
- Every worker uses the configured project model by default. Per-role model overrides remain the RFC-18 extension point and are not required to complete RFC-91.
- The live eval is an intentional terminal gate, not a later dependency: capture the pre-change case grades, run `cargo make eval omnia-r9k --restart` after D7 and `cargo make eval orders-contracts --restart` after D8 in `augentic/emery-adapters`, and require every typed case gate to pass with no lower final grade than its baseline. If credentials or the model backend are unavailable, RFC-91 is not complete and RFC-92 does not start; there is no CI-only substitute.
