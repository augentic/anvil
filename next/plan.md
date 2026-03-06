# Multi-repo spec-driven development plan

*Exported on 07/03/2026 at 7:36:58 GMT+13 from Cursor (2.6.12)*

---

**User**

Starting with this repo but broadening your research to look at what others are doing, draft me a plan on how to evolve multi-repo spec-driven development into a powerful usable tool fit for augentic's clients.
Diagrams are useful

---

**Cursor**

Let me start by understanding the current codebase and then research the broader landscape.

Now I have a thorough picture of both the codebase and the landscape. Here's the plan.

---

# Evolution Plan: OPSX Lifecycle → Production-Grade Multi-Repo Spec-Driven Platform

## 1. Where You Stand Today

```
┌──────────────────────────────────────────────────────────────────────┐
│                        opsx v0.1 (current)                           │
│                                                                      │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐           │
│  │ propose  │──▶│ fan-out  │──▶│  apply   │──▶│ archive  │           │
│  └──────────┘   └──────────┘   └──────────┘   └──────────┘           │
│       │              │              │              │                 │
│  Agent drafts   Clone+branch   Agent per      Move to                │
│  specs/design   distribute     target in      archive/               │
│  pipeline.toml  draft PRs      topo order                            │
│                                                                      │
│  Strengths:                    Gaps:                                 │
│  ✓ Pluggable SpecEngine       ✗ CLI-only, no dashboard               │
│  ✓ Topo-sorted dependencies   ✗ Sequential apply (no parallelism)    │
│  ✓ Idempotent, resumable      ✗ Single agent backend (claude)        │
│  ✓ Per-target state machine   ✗ No verification/drift detection      │
│  ✓ Multi-crate repo grouping  ✗ No observability/metrics             │
│  ✓ Schema pinning             ✗ No external tool sync (JIRA, etc.)   │
│  ✓ Registry-driven topology   ✗ No contract testing                  │
└──────────────────────────────────────────────────────────────────────┘
```

The core design is sound: the orchestrator/spec-engine separation, the registry-driven topology, the toml-based state machine. These are genuine differentiators. But the tool is an internal CLI — turning it into a client-facing product requires filling the gaps above.

---

## 2. Competitive Landscape

```
                        Multi-Repo Orchestration
                                  ▲
                                  │
                         opsx ★   │
                                  │   SpecWeave
                    Gordion       │      ★
                       ★          │
                                  │         ByteBell
                                  │            ★
         ─────────────────────────┼──────────────────────▶
         Single-Agent             │         Multi-Agent
                                  │
                    Spec Kit      │   Compozy
                       ★          │      ★
                                  │
                    Traycer       │   Spec Forge
                       ★          │      ★
                                  │
                        Single-Repo Focus
```


| Tool                  | Stars/Traction | Strength                                                        | Weakness (vs opsx)                 |
| --------------------- | -------------- | --------------------------------------------------------------- | ---------------------------------- |
| **Spec Kit** (GitHub) | 73K stars      | Spec-as-executable-intent, 20+ agent backends, slash commands   | Single-repo only, no orchestration |
| **SpecWeave**         | Enterprise     | 23 plugins, JIRA/Azure DevOps sync, multi-agent teams           | Heavier process, less composable   |
| **Traycer**           | Growing        | Drift verification, phase-based execution                       | Single-repo, no DAG dependencies   |
| **Compozy**           | Growing        | Parallel worktree isolation, full audit trail, 11+ AI providers | No cross-repo dependency ordering  |
| **Spec Forge**        | Early          | 20+ agent types, sandboxed execution                            | No multi-repo story                |
| **Gordion**           | Niche          | Diamond dependency resolution, commit pinning                   | No spec/agent integration          |


**Key insight**: Nobody has combined multi-repo DAG-ordered orchestration with spec-driven AI agents and verification. That's the gap opsx can own.

---

## 3. Target Architecture (v1.0)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          opsx Platform (v1.0)                               │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                        Control Plane                                │    │
│  │                                                                     │    │
│  │   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────────┐    │    │
│  │   │ Registry │  │ Pipeline │  │  Status  │  │ Event Log/Audit  │    │    │
│  │   │  Store   │  │  Engine  │  │  Machine │  │     Trail        │    │    │
│  │   └────┬─────┘  └────┬─────┘  └────┬─────┘  └────────┬─────────┘    │    │
│  │        │             │             │                 │              │    │
│  │        └─────────────┴───────┬─────┴─────────────────┘              │    │
│  │                              │                                      │    │
│  │                     ┌────────▼────────┐                             │    │
│  │                     │   Scheduler /   │                             │    │
│  │                     │   DAG Executor  │                             │    │
│  │                     └────────┬────────┘                             │    │
│  └──────────────────────────────┼──────────────────────────────────────┘    │
│                                 │                                           │
│  ┌──────────────────────────────┼──────────────────────────────────────┐    │
│  │                     Agent Execution Layer                           │    │
│  │                              │                                      │    │
│  │    ┌─────────┐    ┌──────────▼────────┐    ┌─────────────────┐      │    │
│  │    │ Agent   │    │  Worktree Pool    │    │   Verification  │      │    │
│  │    │ Router  │◄──▶│  (isolated git    │ ──▶│   Engine        │      │    │
│  │    │         │    │   worktrees)      │    │   (drift check) │      │    │
│  │    └────┬────┘    └───────────────────┘    └────────┬────────┘      │    │
│  │         │                                           │               │    │
│  │    ┌────▼───────────────────────────────────────────▼────┐          │    │
│  │    │            Agent Backend Adapters                   │          │    │
│  │    │  ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐ ┌──────┐   │          │    │
│  │    │  │Claude │ │Cursor │ │Codex  │ │Gemini │ │ MCP  │   │          │    │
│  │    │  └───────┘ └───────┘ └───────┘ └───────┘ └──────┘   │          │    │
│  │    └─────────────────────────────────────────────────────┘          │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │                     Integration Layer                           │        │
│  │                                                                 │        │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌─────────────────┐     │        │
│  │  │ GitHub   │ │  JIRA /  │ │  Slack / │ │   Webhook /     │     │        │
│  │  │ (PR,     │ │  Linear  │ │  Teams   │ │   Event Source  │     │        │
│  │  │  checks) │ │          │ │          │ │                 │     │        │
│  │  └──────────┘ └──────────┘ └──────────┘ └─────────────────┘     │        │
│  └─────────────────────────────────────────────────────────────────┘        │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────┐        │
│  │                     Presentation Layer                          │        │
│  │                                                                 │        │
│  │  ┌──────────┐ ┌──────────────────┐ ┌────────────────────────┐   │        │
│  │  │  CLI     │ │  Web Dashboard   │ │  GitHub App / Bot      │   │        │
│  │  │  (opsx)  │ │  (live pipeline  │ │  (PR comments, check   │   │        │
│  │  │          │ │   DAG view)      │ │   runs, status badges) │   │        │
│  │  └──────────┘ └──────────────────┘ └────────────────────────┘   │        │
│  └─────────────────────────────────────────────────────────────────┘        │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Phased Roadmap

### Phase 1: Harden the Core (Weeks 1–4)

Make the existing tool reliable enough for a paying client to depend on.

```
                    Phase 1 Focus
    ┌─────────────────────────────────────┐
    │                                     │
    │  ┌───────────┐     ┌─────────────┐  │
    │  │ Error     │     │ Structured  │  │
    │  │ Recovery  │     │ Logging +   │  │
    │  │ + Retry   │     │ Tracing     │  │
    │  └─────┬─────┘     └───────┬─────┘  │
    │        │                   │        │
    │        └─────────┬─────────┘        │
    │                  │                  │
    │          ┌───────▼───────┐          │
    │          │  Idempotent   │          │
    │          │  Resume with  │          │
    │          │  Checkpoints  │          │
    │          └───────┬───────┘          │
    │                  │                  │
    │          ┌───────▼───────┐          │
    │          │  Integration  │          │
    │          │  Tests (real  │          │
    │          │  repos, real  │          │
    │          │  PRs)         │          │
    │          └───────────────┘          │
    └─────────────────────────────────────┘
```

**Deliverables:**

1. **Structured tracing with OpenTelemetry spans** — every `fan-out`, `apply`, `sync` emits spans with target ID, repo, duration, outcome. Replace `println!` with `tracing::info!` throughout.
2. **Retry with exponential backoff** — `git clone`, `gh pr create`, and agent invocation all retry on transient failure (network, rate limit). The current `std::process::Command` calls have no retry logic.
3. **Checkpoint-based resume** — write `status.toml` after *each* target operation, not just at end. Currently if the process crashes mid-fan-out, the partially-distributed targets are orphaned.
4. **Config validation on startup** — validate `registry.toml` references in `pipeline.toml` upfront, before any cloning. Catch typos early.
5. **Integration test harness** — test against real GitHub repos (or a local `git daemon` in CI). The current `tests/e2e_example.rs` only exercises `status` parsing.
6. `**opsx doctor`** — check prerequisites: `git`, `gh` auth, agent backend availability, SSH key access to registry repos.

---

### Phase 2: Parallel Execution + Multi-Agent (Weeks 5–10)

This is where opsx pulls away from every competitor.

```
    Current: Sequential                Target: DAG-Parallel
    
    A ──▶ B ──▶ C ──▶ D               A ──┬──▶ B ──┐
                                           │        ├──▶ D
                                           └──▶ C ──┘
    
    Total: T(A)+T(B)+T(C)+T(D)        Total: T(A)+max(T(B),T(C))+T(D)
    
    ┌───────────────────────────────────────────────────────────┐
    │                DAG Executor                               │
    │                                                           │
    │  ┌──────┐   Ready    ┌──────────────┐   Done   ┌───────┐  │
    │  │ Topo │───────────▶│  Worktree    │─────────▶│ State │  │
    │  │ Sort │  targets   │  Pool        │  result  │ Write │  │
    │  │      │◀───────────│  (N workers) │          │       │  │
    │  └──────┘  unblocked └──────────────┘          └───────┘  │
    │                                                           │
    │  Concurrency: pipeline.toml `concurrency = N`             │
    │  Isolation: git worktree per target (not full clone)      │
    └───────────────────────────────────────────────────────────┘
```

**Deliverables:**

1. **Async runtime (tokio)** — move from `std::process::Command` to `tokio::process::Command`. The DAG executor spawns up to `concurrency` targets simultaneously, respecting dependency edges.
2. **Git worktree isolation** — instead of full clones per target, use `git worktree add` from a single clone per repo. Saves disk and network. Compozy validated this pattern at scale.
3. **Agent backend trait** — abstract the agent invocation behind a trait:
  ```rust
    #[async_trait]
    pub trait AgentBackend: Send + Sync {
        async fn invoke(&self, working_dir: &Path, command: &str, context: &AgentContext) -> Result<AgentResult>;
        fn name(&self) -> &str;
    }
  ```
    Implement for: Claude CLI, Cursor CLI, Codex CLI, dry-run. Let `pipeline.toml` or `registry.toml` specify the backend per target.
4. **Agent context enrichment** — pass the agent not just the slash command but structured context: upstream dependency outputs, contract files, test expectations. This reduces drift.
5. **Live progress streaming** — `opsx apply` shows a live terminal dashboard (using `ratatui` or similar):
  ```
    r9k-http apply                            3/5 targets
    ┌────────────────┬────────┬───────────────────────┐
    │ Target         │ State  │ Progress              │
    ├────────────────┼────────┼───────────────────────┤
    │ r9k-connector  │ ✓ done │ ████████████████ 100% │
    │ r9k-adapter    │ ✓ done │ ████████████████ 100% │
    │ r9k-router     │ ◐ run  │ ████████░░░░░░░░  50% │
    │ r9k-gateway    │ ◐ run  │ ██░░░░░░░░░░░░░░  12% │
    │ r9k-dashboard  │ ○ wait │ ░░░░░░░░░░░░░░░░   0% │
    └────────────────┴────────┴───────────────────────┘
  ```

---

### Phase 3: Verification + Contract Testing (Weeks 11–16)

Traycer proved that spec-driven verification catches agent drift. opsx should do this across repos, which nobody else does.

```
    ┌───────────────────────────────────────────────────────────────┐
    │                  Verification Pipeline                        │
    │                                                               │
    │                  ┌─────────────┐                              │
    │                  │   Agent     │                              │
    │                  │   Output    │                              │
    │                  └──────┬──────┘                              │
    │                         │                                     │
    │           ┌─────────────▼──────────────┐                      │
    │           │                            │                      │
    │     ┌─────▼─────┐            ┌────────▼─────────┐             │
    │     │ Spec      │            │  Contract        │             │
    │     │ Compliance│            │  Compatibility   │             │
    │     │ Check     │            │  Check           │             │
    │     │           │            │                  │             │
    │     │ "Did the  │            │ "Does the output │             │
    │     │  agent    │            │  match what      │             │
    │     │  fulfill  │            │  downstream      │             │
    │     │  the      │            │  targets         │             │
    │     │  spec?"   │            │  expect?"        │             │
    │     └─────┬─────┘            └────────┬─────────┘             │
    │           │                           │                       │
    │           └──────────┬────────────────┘                       │
    │                      │                                        │
    │               ┌──────▼──────┐                                 │
    │               │  Verdict:   │                                 │
    │               │  pass/warn/ │───▶ PR comment + status check   │
    │               │  fail       │                                 │
    │               └─────────────┘                                 │
    └───────────────────────────────────────────────────────────────┘
```

**Deliverables:**

1. **Post-apply verification agent** — after each `apply`, run a second agent pass that compares the diff against the spec. Emit a structured verdict (pass/warn/fail with line references). This is the "adversarial review" pattern from Spec2Ship.
2. **Cross-repo contract testing** — the `[[dependencies]]` in `pipeline.toml` already declare contracts (`contract = "domains/train/shared-types.md#R9kEvent"`). After upstream applies, extract the actual contract artifact and verify downstream specs are compatible. This is unique to opsx.
3. **Verification report in PR** — post verification results as a PR comment and a GitHub check run. Clients see red/green directly in their PR workflow.
4. **Spec coverage metric** — for each spec requirement (`+` additions, `-` removals), track whether the implementation addressed it. Surface as a percentage in `opsx status`.

---

### Phase 4: Integration + Dashboard (Weeks 17–24)

Make opsx accessible to non-CLI users and integrate with enterprise tooling.

```
    ┌─────────────────────────────────────────────────────────────────┐
    │                                                                 │
    │                    Web Dashboard                                │
    │                                                                 │
    │  ┌──────────────────────────────────────────────────────────┐   │
    │  │  Initiative: r9k-http                          ● Live    │   │
    │  │                                                          │   │
    │  │     ┌──────────┐          ┌──────────┐                   │   │
    │  │     │connector │─────────▶│ adapter  │                   │   │
    │  │     │  ✓ done  │          │  ✓ done  │──┐                │   │
    │  │     └──────────┘          └──────────┘  │                │   │
    │  │                                         │  ┌──────────┐  │   │
    │  │     ┌──────────┐                        ├─▶│ gateway  │  │   │
    │  │     │  router  │────────────────────────┘  │ ◐ apply  │  │   │
    │  │     │  ✓ done  │                           └──────────┘  │   │
    │  │     └──────────┘                                         │   │
    │  │                                                          │   │
    │  │  Progress: 3/4 ████████████████████░░░░ 75%              │   │
    │  │  PRs: 2 merged, 1 reviewing, 1 applying                  │   │
    │  │  Est. completion: ~25 min                                │   │
    │  └──────────────────────────────────────────────────────────┘   │
    │                                                                 │
    │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐     │
    │  │  Specs   │  │  Agent   │  │  Audit   │  │  Settings    │     │
    │  │  Browser │  │  Logs    │  │  Trail   │  │  & Registry  │     │
    │  └──────────┘  └──────────┘  └──────────┘  └──────────────┘     │
    └─────────────────────────────────────────────────────────────────┘
```

**Deliverables:**

1. **GitHub App** — a GitHub App that:
  - Adds status badges to PRs managed by opsx
  - Posts verification results as check runs
  - Listens for PR merge events to auto-advance the state machine (replacing `opsx sync` polling)
  - Provides a `/opsx status` slash command in PR comments
2. **Web dashboard** — read-only view of the DAG, target states, agent logs, and verification results. Powered by the event log. Could be a simple static site reading from `status.toml` + event log, or a lightweight server.
3. **External tracker sync** — bidirectional sync with JIRA/Linear:
  - Initiative → Epic
  - Target → Story/Task
  - State transitions → Status updates
  - This is table-stakes for enterprise clients (SpecWeave has it, opsx doesn't)
4. **Notification hooks** — Slack/Teams notifications on state transitions: "r9k-connector applied successfully, r9k-adapter unblocked", "All PRs merged for r9k-http, ready to archive".

---

### Phase 5: Self-Service + Ecosystem (Weeks 25–36)

Turn opsx from a tool into a platform.

```
    ┌────────────────────────────────────────────────────────────────┐
    │                    opsx Platform Ecosystem                     │
    │                                                                │
    │  ┌──────────────────────────────────────────────────────┐      │
    │  │                 Spec Engine Marketplace              │      │
    │  │                                                      │      │
    │  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌─────────┐  │      │
    │  │  │ OpenSpec │ │ SpecKit  │ │  Custom  │ │  MADR   │  │      │
    │  │  │ (built-  │ │ (GitHub  │ │  (client │ │ (arch   │  │      │
    │  │  │  in)     │ │  compat) │ │  schema) │ │  docs)  │  │      │
    │  │  └──────────┘ └──────────┘ └──────────┘ └─────────┘  │      │
    │  └──────────────────────────────────────────────────────┘      │
    │                                                                │
    │  ┌──────────────────────────────────────────────────────┐      │
    │  │                 Registry Federation                  │      │
    │  │                                                      │      │
    │  │  Client A          Client B          Client C        │      │
    │  │  registry.toml     registry.toml     registry.toml   │      │
    │  │  (80 repos)        (200 repos)       (30 repos)      │      │
    │  │                                                      │      │
    │  │  Each client owns their registry, schemas, and       │      │
    │  │  pipelines. opsx provides the execution engine.      │      │
    │  └──────────────────────────────────────────────────────┘      │
    │                                                                │
    │  ┌──────────────────────────────────────────────────────┐      │
    │  │                 Template Library                     │      │
    │  │                                                      │      │
    │  │  "Migrate API version"    "Add new domain event"     │      │
    │  │  "Rotate credentials"     "Update shared dependency" │      │
    │  │  "Add observability"      "Security patch rollout"   │      │
    │  └──────────────────────────────────────────────────────┘      │
    └────────────────────────────────────────────────────────────────┘
```

**Deliverables:**

1. `**opsx init`** — bootstrap a new client workspace: generate `registry.toml` from GitHub org scan, install default schema, create first example change. Time-to-first-fan-out under 10 minutes.
2. **Spec engine compatibility with Spec Kit** — given Spec Kit's 73K stars, implement a `SpecKitEngine` adapter so clients already using Spec Kit can adopt opsx for multi-repo orchestration without changing their spec format.
3. **Change templates** — pre-built pipeline templates for common cross-cutting concerns (dependency bumps, security patches, API version migrations). `opsx propose` can start from a template instead of a blank slate.
4. **Registry federation** — support multi-org, multi-registry setups. A parent registry can reference child registries for large enterprises with multiple GitHub orgs.
5. **Metrics and billing** — track agent invocations, tokens consumed, time saved. Surface in dashboard for ROI reporting.

---

## 5. Differentiator Summary

```
    What opsx uniquely offers (no competitor has all of these):

    ┌───────────────────────────────────────────────────────────┐
    │                                                           │
    │   1. DAG-ordered multi-repo execution                     │
    │      (Spec Kit, Traycer, Compozy: single-repo only)       │
    │                                                           │
    │   2. Cross-repo contract verification                     │
    │      (Nobody does this — upstream/downstream checks)      │
    │                                                           │
    │   3. Registry-driven topology                             │
    │      (Service catalog → automatic target resolution)      │
    │                                                           │
    │   4. Pluggable spec engine                                │
    │      (Bring your own spec format, even Spec Kit)          │
    │                                                           │
    │   5. Deterministic fan-out with schema pinning            │
    │      (All targets get same snapshot — reproducible)       │
    │                                                           │
    │   6. Built for 80+ repos                                  │
    │      (SpecWeave is closest but less composable)           │
    │                                                           │
    └───────────────────────────────────────────────────────────┘
```

---

## 6. Technical Priorities (What to Build Next)

Given the phases above, here's a prioritized ordering based on client impact:


| Priority | Item                                 | Why Now                                                    |
| -------- | ------------------------------------ | ---------------------------------------------------------- |
| **P0**   | `opsx doctor` + config validation    | Clients will hit setup issues immediately                  |
| **P0**   | Structured tracing + OTel            | Can't debug failures at 80 repos without observability     |
| **P0**   | Checkpoint-based resume              | Partial failures are guaranteed; recovery must be seamless |
| **P1**   | Async DAG executor + worktree pool   | Sequential apply across 80 repos is untenable              |
| **P1**   | Agent backend trait + multi-backend  | Clients use different agents; can't lock to Claude only    |
| **P1**   | Post-apply verification              | Trust requires proof the agent did what the spec asked     |
| **P2**   | GitHub App (status checks, webhooks) | Replaces manual `opsx sync` polling                        |
| **P2**   | Web dashboard (read-only DAG view)   | Non-CLI stakeholders need visibility                       |
| **P2**   | JIRA/Linear sync                     | Enterprise clients require issue tracker integration       |
| **P3**   | Spec Kit engine adapter              | Piggyback on 73K-star ecosystem                            |
| **P3**   | Change templates                     | Reduce time-to-first-value                                 |
| **P3**   | `opsx init` (org scanner)            | Self-service onboarding                                    |


---

## 7. Key Architectural Decisions to Lock In Early

1. **Event sourcing for status** — instead of overwriting `status.toml`, append events to an event log. The status file becomes a materialized view. This unlocks audit trail, dashboard, and time-travel debugging for free.
2. **Worktrees over clones** — adopt `git worktree` as the isolation primitive from day one. Saves ~80% of disk and network for multi-target repos.
3. **Agent backend as a trait, not an env var** — the current `OPSX_AGENT_BACKEND` env var approach won't scale to per-target agent selection. Make it a first-class trait with per-target override in `pipeline.toml`.
4. **Verification as a pipeline stage, not an afterthought** — add `verified` as a state between `implemented` and `reviewing`. No PR gets marked ready-for-review without passing verification.
5. **Keep the CLI as the source of truth** — the dashboard and GitHub App are read-only views. All mutations go through `opsx` commands. This keeps the system simple and auditable.

---

This plan positions opsx as the only tool that combines multi-repo DAG orchestration, pluggable spec engines, and cross-repo contract verification — the three things enterprise clients with 50+ repos need and can't get anywhere else today.