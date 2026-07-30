# Swarm Build — Focused Convergent Build Requests

> Status: Draft — implementation not started
>
> Owns: the target-build execution model that replaces the fat sequential legs: decomposition of one build into focused judgment requests, the convergence gate that supersedes the in-prompt verify-repair channel, the backend concurrency substrate (agent pool, workspace policy), and the deployment expression that runs workers on remote nodes.
>
> Depends: [RFC-78](archive/rfc-78-prompt-budget.md) (the enabling layer — per-request byte budget, timeout semantics, session model), [RFC-60](future/rfc-60-verify-profiles.md) (**promoted from deferred** — host-owned verify is this RFC's convergence gate), [RFC-55](future/rfc-55-working-tree.md) (materialized working trees — per-worker isolation and the multi-node unlock).
>
> Related: [RFC-18](future/rfc-18-slm.md) (per-worker model selection is this RFC's hook for cheaper backends), [RFC-80](rfc-80-synthesis-redesign.md) (applies the same decomposition pattern at refine time and consumes this RFC's concurrency substrate).

## Intent

Replace the large single-conversation build with a **swarm**: many smaller, focused build requests that converge on a built omnia (or vectis) component. One fat generation leg holding crate / test / guest writers together exists today for exactly one reason — it is the only shared verify-repair channel. This RFC replaces that channel with an explicit convergence architecture, so the split stops being a regression and becomes the design.

Emery is in PoC, graduating to trial in ~4 weeks. The swarm is the architecture we graduate **to**, not an optimization of the current shape: focused requests bound each agent's blast radius, make every worker individually observable and timeout-able, open per-worker model selection (cheap models for focused tasks), and — because the whole framework is Omnia-backed — extend without redesign into a distributed, cloud-hosted service where workers are nodes connected only by content-addressed values ([RFC-55](future/rfc-55-working-tree.md)).

## Why now (evidence)

From [RFC-78](archive/rfc-78-prompt-budget.md)'s `wasm-omnia-r9k` runs:

- The five-leg omnia build serializes ~30 minutes of agent wall-clock; the review leg nests an invisible agent team (three specialists + antagonist + remediation) inside **one** completion, where the host cannot observe, bound, or time out any member individually — the run died there.
- The generation leg is one ~64 KB conversation doing four jobs (crate writer, test writer, guest writer, verify-repair loop), because splitting it without a convergence gate would strand repairs.
- Verify (cargo build / clippy / test) runs as prompt text inside the agent loop — unobservable, unbounded, and unshareable between workers.

The fat leg's ceiling is structural: it cannot parallelize, cannot mix models, cannot bound sub-work, and its cost grows with slice size rather than with task size.

## The model

### Topology

- **The WIT seam does not change.** `target.build` remains one dispatch per slice; the adapter core already owns leg sequencing (`targets/omnia/src/operations.rs` makes five `create` calls today). The swarm is the same pattern with more, smaller, possibly concurrent `create`s issued by the in-guest orchestrator.
- **The orchestrator is deterministic guest code**, not a lead agent. It partitions work, issues worker requests, runs the convergence loop, and folds typed worker outcomes into the `BuildReport` (completing [RFC-78](archive/rfc-78-prompt-budget.md) D6's report absorption). Judgment stays in the workers; sequencing and arbitration stay compiled-in.
- **A worker is one focused judgment request**: a thin role brief (system), only the inputs its task needs (path-first `input` records from RFC-78 D1), an explicit write-ownership manifest, and the shared `PHASE_ANSWER_SCHEMA`-style answer gate. Workers never receive the whole prompt corpus; references stay MCP-lazy.

### Partitioning

Work units derive from what the engine already produces:

- **Writer roles** are the first cut: crate writer, test writer, guest writer (create mode) — today's co-tenants of the fat leg become separate workers.
- **`tasks.md` is the finer partition source**: the refined slice's task list already decomposes the build into reviewable steps with implicit file ownership; the orchestrator maps tasks (or task groups) to workers and derives each worker's write-ownership manifest from them.
- **Review specialists become first-class workers** (Security / Correctness / Quality / antagonist), replacing the nested in-agent `Task` team — each individually budgeted, observable, and timeout-able, reporting typed findings the orchestrator routes.

Two workers never share write ownership of a path. Ownership conflicts are an orchestrator-time error, not a merge problem.

### Convergence

The shared verify-repair channel is replaced by a **host-owned convergence gate** over [RFC-60](future/rfc-60-verify-profiles.md) verify profiles:

1. Workers complete their focused writes and answer with typed outcomes (no cargo commands in worker prompts — the RFC-60 posture).
2. The orchestrator requests closed verify profiles (`build`, `clippy`, `test`, …) through the host; the host runs them sandboxed and returns normalized findings.
3. Findings route back **to the owning worker** (by write-ownership manifest) as focused repair requests — resume + findings delta per RFC-78 D5, never a fresh full prompt.
4. Rounds are bounded by an explicit convergence budget; exhaustion is a typed `failure` report with the residual findings attached.

This is strictly stronger than today's in-prompt loop: verify output is normalized once, repair context is per-owner instead of per-everything, and the budget is host policy instead of prompt prose.

### Concurrency substrate (backend)

Staged, because workspace policy is the real constraint:

- **Stage A — sequential swarm, single lent tree.** No backend change: workers run one at a time against the live tree, write-ownership enforced by the orchestrator, verify serialized between rounds. This already lands the observability, model-selection, and bounded-blast-radius wins, and it is the shape the trial can ship on.
- **Stage B — concurrent swarm, single tree, partitioned writes.** Requires backend support for concurrent completions: per-spawn MCP config isolation (today's `McpGuard` races on one `.cursor/mcp.json`), per-worker prompt spills (already pid/counter-disambiguated), and an agent-pool with a concurrency cap. Verify remains serialized (cargo lock); disjoint write ownership keeps concurrent workers from conflicting.
- **Stage C — distributed swarm, per-worker trees.** [RFC-55](future/rfc-55-working-tree.md) materializes a working tree per worker from a `revision`, workers produce `changeset`s, and the orchestrator composes them by dependency-layering before the convergence gate. Workers become location-independent: an Omnia node in the same process, a pool VM, or a Cursor cloud-runtime agent (viable exactly here, where the worker operates on a materialized checkout rather than the operator's live mount). Pathless workers receive `payload.body` inputs — the case RFC-78 D1's exclusive body arm was reserved for.

### Agent pool and per-worker policy

- The backend grows from "spawn one agent per completion" to a **pool**: bounded concurrent spawns, per-worker inactivity timeout (RFC-78 D4 semantics), per-worker session scoped to its repair chain (RFC-78 D5), and pool-level cancellation when the orchestrator aborts a round. Implementation options: a Rust process-pool over today's spawn path (default), or a Cursor SDK sidecar if lifecycle management outgrows it — decided by Stage B evidence, not up front.
- **Per-worker model selection**: focused tasks tolerate cheaper, faster models; the request's `model` field already crosses the backend (guest-supplied wins). The orchestrator may bind writer workers, review specialists, and repair rounds to different model tiers — the concrete hook [RFC-18](future/rfc-18-slm.md) needs.

## Decisions

### D1 — In-guest deterministic orchestrator (adapters + SDK)

Move leg sequencing from five hardcoded fat legs to a partition → dispatch → converge → fold loop in the adapter core, with the shared scaffolding (worker brief assembly, ownership manifests, outcome folding) in the adapter SDK so omnia, vectis, and contracts share one orchestrator shape. The WIT `target` interface is untouched.

### D2 — Convergence gate over RFC-60 verify profiles (engine + Omnia host)

Activate [RFC-60](future/rfc-60-verify-profiles.md): closed profile names, host-owned argv, sandboxed execution, normalized findings. The adapter requests profiles through the model tool loop's `verify` grant (stubbed today); cargo command text leaves worker prompts entirely. Findings carry artifact-relative locations the orchestrator maps to owning workers.

### D3 — Write-ownership partitioning (adapters)

Every worker request carries an explicit ownership manifest (paths it may create / modify); the orchestrator derives manifests from writer roles and `tasks.md`, rejects overlaps before dispatch, and treats out-of-manifest writes in a worker's answer as a blocking finding. This is the invariant that makes Stages B and C safe.

### D4 — Backend concurrency + workspace policy stages (backends)

Stage A needs nothing. Stage B: per-spawn MCP config isolation, an agent-pool with a concurrency cap, pool-level cancellation. Stage C: `local-path` per worker from RFC-55 materialization, `changeset` extraction on worker completion. The parallel survey / extract fan-outs ([RFC-80](rfc-80-synthesis-redesign.md)) consume Stage B as-is.

### D5 — Host-visible review swarm (adapters)

Dissolve the nested in-agent review team into first-class specialist workers with typed findings, individual budgets, and individual timeouts. The antagonist becomes a worker gated on the specialists' outcomes; remediation becomes routed repair requests through the same convergence gate as generation.

### D6 — Distributed deployment expression (engine + Omnia + backends)

Nothing in D1–D5 may assume process-locality beyond the stage it ships in. Values ([RFC-55](future/rfc-55-working-tree.md) `revision` / `changeset`) are the only connective tissue between orchestrator and Stage C workers; a worker's node is a scheduling decision. The cloud-hosted service is this architecture with a remote pool bound in — not a fork of it.

## Non-goals

- Changing the WIT `target` interface or the `BuildReport` contract — the swarm is adapter-internal orchestration over the existing seam.
- Synthesis decomposition — [RFC-80](rfc-80-synthesis-redesign.md) applies this pattern at refine time.
- Model backend replacement ([RFC-18](future/rfc-18-slm.md)) — this RFC only creates the per-worker selection hook.
- A lead-agent orchestrator. Arbitration, scheduling, and budget are deterministic guest code; adding a judgment leg to decide what other judgment legs to run re-creates the fat leg one level up.

## Ownership

| Decision | Repo |
| -------- | ---- |
| D1 orchestrator + SDK scaffolding | `augentic/emery` (`crates/adapter`) + `augentic/emery-adapters` |
| D2 verify profiles activation | `augentic/omnia` (`wasi-model` verify) + `augentic/emery` policy |
| D3 ownership manifests | `augentic/emery-adapters` (+ SDK types in `augentic/emery`) |
| D4 pool / workspace stages | `augentic/backends` (Stage C with `augentic/omnia` RFC-55 backend) |
| D5 review swarm | `augentic/emery-adapters` |
| D6 distribution invariants | all three, enforced in review |

## Acceptance criteria

1. An omnia build for a slice the size of `at-r9k-position-adapter` completes as a set of focused worker requests, each with a spilled prompt **≤ ~15 KB**, and no worker prompt contains cargo command text.
2. Verify runs only through closed RFC-60 profiles; findings route to the owning worker; the convergence budget is host/orchestrator policy and exhaustion produces a typed `failure` report with residual findings.
3. Review specialists are individually observable in logs/journal and individually timeout-able; no nested in-agent `Task` team remains in the omnia review path.
4. Two workers with overlapping ownership manifests are rejected before dispatch; an out-of-manifest write surfaces as a blocking finding.
5. Stage B: two workers run concurrently against one tree with isolated MCP config and no shared-file races; pool cancellation reaps all in-flight workers.
6. Stage C (gated on RFC-55): one worker builds against a materialized tree on a node with no shared mount, returns a `changeset`, and the orchestrator's composition passes the convergence gate.
7. The `BuildReport` seam contract and `emery slice build` behavior are unchanged from the CLI's perspective; `cargo make ci` green in touched repos; live eval (`omnia-r9k`) shows build quality parity.

## Risks and invariants

- **The convergence gate must exist before the split.** A swarm without host-owned verify is the fat leg's verify-repair loop deleted, not replaced — Stage A ships with D2, not before it.
- **Verify is a security boundary** (RFC-60's posture verbatim): closed profiles, sandboxed, no model-supplied argv. The swarm multiplies how often verify runs; it must not widen what verify accepts.
- **Ownership is exclusive and checked.** Concurrency safety comes from partition discipline, not merge cleverness; overlap is an error, never a three-way merge.
- **Deterministic orchestration.** The orchestrator never consults a model to decide dispatch; budget, ordering, and routing are compiled-in policy, auditable in the journal.
- **Values-only distribution.** Stage C workers share nothing but `revision` / `changeset` values; any design that needs a shared live handle across nodes is out of contract.
- **Per-worker budget assertions** (RFC-78 D7) extend to the swarm: worker brief sizes are locked in adapter tests so the swarm cannot silently re-grow fat prompts.

## Open questions

- Can the Omnia host service **concurrent in-flight `create` calls from one guest** (async WIT dispatch), or does Stage B interleave at the host while the guest awaits joins? This bounds real fan-out and should be confirmed first.
- Partition granularity: writer-role workers first, or go straight to `tasks.md`-derived units? (Recommendation: roles for Stage A, tasks once convergence is proven.)
- Convergence budget shape: global rounds, per-worker rounds, or finding-count-weighted?
- How does slice-level parallelism (multiple plan entries in flight) compose with worker-level parallelism against one backend pool?
- Stage C changeset composition ordering when repair rounds interleave with fresh worker output — is RFC-55's dependency-layering sufficient as-is?
- Which model tiers do writer / specialist / repair workers default to, and where does that policy live (adapter prose vs host config)?
