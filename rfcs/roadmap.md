# Specify Roadmap

> Status: Draft
> Source: Review of Cloudflare's internal AI engineering stack, especially the platform, knowledge, and enforcement layers described in [https://blog.cloudflare.com/internal-ai-engineering-stack/](https://blog.cloudflare.com/internal-ai-engineering-stack/).

## Thesis

Specify should be the spec-driven workflow control plane for agentic software delivery. It should use developer portals, model gateways, CI, forges, and hosted runners without becoming any of them.

The local substrate is credible: slice/change vocabulary, registry-aware planning, workspace execution, branch preparation, push/finalize handoff, declared tools, and layered skills are all in place. The **enforcement** and **reconciliation** pillars have landed too — `lint`, `validate`, and framework checks now share one `Diagnostic` substrate (`specify-diagnostics`) while keeping distinct gate authority, and core owns how sources reconcile into slices. Durable specs live in [`docs/standards/workflow.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md), [`specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md), and [From sources to slices](../docs/explanation/reconciliation.md). With that loop now proven end-to-end on realistic multi-repo flows, the next phase should sharpen the remaining reconciliation seams, then make it observable and portable across teams, forges, agents, and catalogs.

At scale, Specify spans three connected layers:

1. **Platform:** models, tools, sandboxes, logs, and long-running execution.
2. **Knowledge:** repositories, owners, dependencies, standards, adapters, and plans.
3. **Enforcement:** review, compatibility checks, standards checks, and stale-context detection.

Specify owns the workflow semantics across those layers: intent becomes artifacts; artifacts become plans; plans route work to repositories; repositories change through controlled phases; outcomes are reviewed and recorded for audit and recovery.

## Principles

- **Keep the CLI authoritative.** Skills, MCP servers, CI, and cloud runners may orchestrate `specify`; they must not reimplement lifecycle transitions, plan validation, registry validation, workspace sync, or merge behavior.
- **One authored home per fact; derive the rest.** Each project's intent (`adapter`, `description`) lives in `.specify/project.yaml`; routing identity (`surface[]`, `decisions[]`, `recent[]`) is a deterministic baseline projection committed as `.specify/topology.lock`. `registry.yaml` carries membership and location only (plus optional greenfield adapter seed and cross-project `contracts` wiring) — not adapter/description for plan-time topology. Rich catalog metadata can still live in Backstage or another catalog; Specify consumes reviewable projections at the boundary.
- **Separate workflow, standards, and artifacts.** Workflow skills orchestrate phases; rules carry durable engineering policy; artifacts capture slice-local and baseline product intent.
- **Optimize for local first, cloud later.** `/spec:execute` remains the proving ground, but plan locks, journals, phase outcomes, workspace state, review results, and recovery records should be durable enough for hosted execution.
- **Prove the whole loop.** Eval coverage should exercise realistic multi-repo flows, not just isolated command behavior.
- **Abstract external systems at the boundary.** Forges, catalogs, agents, and hosted runners should integrate through narrow adapters.
- **Keep enforcement surfaces distinct.** Framework-repo **authoring standards** (`specify lint framework`) and consumer-project **engineering standards** (`specify lint project`) share rule ids and the neutral `Diagnostic` shape, but never gate authority: `validate` gates lifecycle transitions and is non-silenceable, while `lint` is lifecycle-neutral and silenceable. See [docs/explanation/standards-layer.md](../docs/explanation/standards-layer.md).
- **Core owns reconciliation.** If a rule decides how sources combine, how evidence becomes artifacts, or how one slice drives multiple outputs, it belongs in the CLI or a CLI-owned schema — not only in a skill body. This does not move the *matching judgment* off the agent: the agent owns "are these two leads the same work?"; the CLI owns the candidate-set construction, the coverage and conflict guarantees, and the audit trail around that judgment. The lead-side hints are typed facts a deterministic layer consumes, not a deterministic replacement for the agent. See [From sources to slices](../docs/explanation/reconciliation.md) and [`specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md).

## Sequenced Roadmap

Items are identified as `RM-NN`. Earlier items unblock later ones unless noted otherwise. Command examples are target surfaces unless the item explicitly says the command is already implemented.

### Current priorities

Two tracks were framed as parallel, but they are lopsided in practice: the observability track is nearly landed, while the reconciliation track has not started. They need different things — one needs implementation cycles, the other needs a design decision — so treat them as distinct, not co-equal.

1. **Reconciliation polish** — additive typed fields on the lead side (`topics[]`, advisory `clusters[]`, binding `affinity`, decision-conflict warnings), wiring baseline context into synthesis (`advisory-context`), and a greenfield identity seed. The determinism is in the *consumers* (clustering, coverage, conflict warnings), not in producing the fields — survey is agent-driven, so the agent populates `topics[]`; the win is moving the agent's output from un-checkable `synopsis` prose into typed facts a deterministic layer can join, check, and reproduce, **without moving the matching decision off the agent**. **Status: not started** — none of these surfaces exist yet (the lead schema is still `lead` / `source` / `synopsis`, with no `topics[]`), and Open Question #1 — which surface lands first — is still open. This track is blocked on a sequencing decision, not on effort, and it is the project's core differentiator: per *Core owns reconciliation*, the candidate-set, coverage, and conflict guarantees built on these fields decide artifact quality, which is what the whole loop exists to produce. **Needs the most attention next.** Resolve Open Question #1 and ship `topics[]` first — it is the substrate the rest of the track is blocked on, because advisory `clusters[]`, coverage, and decision-conflict warnings are all set operations *over* `topics[]`.
2. **Observability and portability (RM-14 / RM-15 / RM-18)** — now that the loop is proven on realistic multi-repo flows, make it measurable and portable across teams, forges, agents, and catalogs. **Status: mostly landed for the near-term items.** RM-15 is essentially complete; RM-14's event *substrate* ships (closed taxonomy, `journal show` / `journal emit`, bounded backward-tail reader) and only the operator-facing measurement/export surface remains. This is a known build, not a design risk — a good candidate to execute in parallel with the reconciliation decision.

**Deferred until trigger conditions or prerequisites:**

- [Standards baseline](#ideas-parked) — cross-run baseline/diff; no consumers under fix-before-release on Specify-native codebases.
- RM-12 / RM-13 — catalog import and read-oriented MCP; integration surfaces that enrich an already-trustworthy core loop.

---

### Near Term

#### RM-14: Local structured workflow events

**Goal:** Measure workflow performance, failure modes, and model/tool usage without requiring hosted infrastructure.
**Landed (substrate):** the journal already ships a closed ~31-id event taxonomy (`EventKind`), append-only JSONL at `.specify/journal.jsonl`, a bounded backward-tail reader, and the `specify journal show` (`--filter` / `--limit`) and `specify journal emit` verbs. Emitted kinds include `lint-completed`, `slice.build.*`, `slice.synthesize.*`, `slice.merge.*`, `plan.entry.advanced`, `plan.reconcile.completed`, and the workspace/bootstrap events.
**Remaining:** the operator-facing *measurement* surface — a follow/tail view and an `export` to a JSONL file or configurable telemetry sink with run identity. `journal show` covers a one-shot filtered read but not a follow stream or a sink.
**Events should also surface:** command/version, project/adapter, validation result, invoked skill, review findings, recovery attempts, human intervention points, and model/tool metadata when available.
**Naming decision needed:** the substrate already ships under `specify journal`. Decide whether the consumer surface extends `journal` (`specify journal export`) or introduces a parallel `events` noun (`specify events tail|export`) — do not ship two commands over one log.
**Target surface (subject to the naming decision):**

```bash
specify journal show --follow   # or: specify events tail
specify journal export          # or: specify events export
```

**Output:** local JSONL or configurable telemetry sink with run identity.

#### RM-15: Structured change-lifecycle status for re-entry

**Goal:** Make the `/spec:plan` → `/spec:execute` → `/spec:finalize` lifecycle's re-entry and pause points machine-readable.
**Output:** JSON status with current step, last completed step, pending human action, owner, and next valid resume point.
**Consumes:** *Local structured workflow events*.
**Status: essentially complete.** `specify plan status` already carries `current-step` / `last-completed` / `resume` alongside `next-action`, projected from `plan.yaml`, slice metadata, and the journal (`StatusBody` in `crates/workflow/src/change/plan/core/status.rs`). Only **pending human action** and **owner** remain open; both depend on the human-intervention and owner signals RM-14's event surface is meant to carry.

---

### Mid Term

#### RM-11: Dependency-aware compatibility gates

**Goal:** Block producer slices from reaching `done` while breaking consumer follow-up is unaccounted for.
**Depends:** [From sources to slices](../docs/explanation/reconciliation.md) (typed slice model, per-slice fan-out via `depends-on`); the diagnostic substrate and consumer scanner (`IFACE-*` contract findings — [`DECISIONS.md` §Diagnostic substrate](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate) and [§Standards layer split](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema)).
**Answers:** whether consumer plan entries exist, whether producer completion is allowed, and what SemVer or release impact is implied.
**Consumes:** the neutral `Diagnostic` envelope. RM-11 owns the structured-evidence shape for `IFACE-*` contract findings via `schemas/review/finding/contracts-evidence.schema.json`; the `Diagnostic` evidence union reserves the `evidence.kind: structured` branch but leaves the inner `data` shape to this item, so contracts-specific decisions land with the gate that needs them.
**Target surface:**

```bash
specify plan impact --change <name>
```

#### RM-12: Catalog import: Backstage adapter

**Goal:** Enrich Specify planning from external catalogs without making Specify a developer portal.
**Target surface:**

```bash
specify registry import backstage
specify registry import <source>
specify registry diff <source>
```

**Mapping:** Backstage `System` to platform/product boundary; `Component` to registry project; `API` to interface inventory; ownership/domain/dependencies to routing and review signals.
**Output:** explicit registry diff for operator review before planning or execution.

#### RM-13: Read-oriented Specify MCP server

**Goal:** Make Specify state available to agents through MCP without duplicating business logic.
**Initial tools:** direct readers for `plan.yaml`, `registry.yaml`, workspace slots, slice metadata, plus wrappers around `specify plan next` and `specify slice validate`.
**Boundary:** mutating tools may come later only as wrappers around existing CLI verbs.

#### RM-17: Forge abstraction behind workspace push and change finalize

**Goal:** Support branch transport, PR/MR creation, and finalize beyond GitHub CLI.
**Adapter covers:** remote discovery, auth checks, branch existence, push permissions, PR/MR create-or-update, CI/mergeability status, merged-state verification, and provider links.
**Target surface:**

```bash
specify forge doctor
specify workspace push --forge github
specify plan finalize --forge github
```

---

### Long Term

#### RM-18: Cloud-hosted execute loop

**Goal:** Run Specify plans durably in the background while preserving local workflow semantics.
**Requires:** sandboxed workspace clones, durable lock ownership, resumable agent sessions, serialized phase outcomes and journals, human approval gates, controlled push/PR creation, deterministic recovery, and parity with `/spec:execute`.
**Target surface:**

```bash
specify execute submit
specify execute status <run-id>
specify execute resume <run-id>
```

#### RM-19: Multi-forge adapter coverage

**Goal:** Extend the forge abstraction to GitHub, GitLab, Bitbucket, and self-hosted forges.

#### RM-20: Catalog-backed initiatives across many repositories

**Goal:** Drive multi-repo initiatives from live catalog-backed registry projections.

#### RM-21: Adapter ecosystem operating model

**Goal:** Make adapters feel like a dependable ecosystem rather than bespoke first-party packages.
**Depends:** the [bootstrap/upgrade lifecycle](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#bootstrap-and-upgrade-lifecycle) for adoption at scale.
**Includes:** publishing and discovery conventions, version compatibility tests, declared-tool compatibility, migration guidance, quality gates, examples beyond Omnia/Vectis/contracts, and ownership for rules, artifact templates, and tool manifests.

#### RM-22: Hosted observability dashboards

**Goal:** Build hosted dashboards on top of local structured workflow events without making local workflows depend on hosted infrastructure.

---

## Ideas (parked)

Each is one paragraph of intent. An idea graduates to active roadmap work only when it gains an owner and a trigger condition.

- **Type-safe skill expression.** Extend framework tooling validation to skill authoring — frontmatter schema enforcement, reference resolution, variable consistency, cross-skill directive validation — and, as the skill count grows, graduate to structured YAML manifests or a Rust DSL separating the typed skeleton from the prose body. Much of the validation half has since landed as `CORE-*` framework checks.
- **Specialized SLM code generation.** Train a specialized Small Language Model to generate Omnia Rust crates from Specify artifacts (Vectis following once proven), making the model behind the Omnia `build/crate.md` brief cheaper, faster, and more reproducible — without replacing the workflow.
- **CLI observability.** First-class `tracing`-based structured diagnostics for command execution, lifecycle transitions, plan orchestration, workspace operations, and tool runs, without changing the existing stdout contract. Partially superseded by the journal (RM-14 lineage).
- **Source catalogue and tier-1 cache.** A durable platform-level catalogue of legacy source repositories (`sources.yaml`), a shared tier-1 clone cache, and a `--source @<key>` selector so a platform repo declares dozens of legacy sources once and reuses them across changes.
- **Migration ledger and slice mapping.** Cumulative cross-change state answering "is this source migrated yet?" and "what's the source-to-target pattern of this slice?" for migrations spanning many changes.
- **Omnia plan composition.** Teach `plan.yaml` to express the composition shape Omnia migrations produce — services composed of crates composed of handlers — without a parallel artifact or breaking existing plans.
- **Standards baseline.** The cross-run lint lifecycle: acknowledging a body of legitimate findings as baseline debt, diffing scans against prior runs, and staging remediation across releases. Deferred — no consumers under fix-before-release on Specify-native codebases.
- **Acceptance shape assertions and orchestration traces.** A deterministic middle tier for eval scenarios — shape assertions over synthesized artifacts and orchestration traces over journal events — between manual runs and byte-replay fixtures. The `backend` frontmatter carrier it assumed has since been removed, so activation needs a new carrier.

## Non-Goals

- Do not make Specify a general developer portal, AI gateway, CI system, or forge policy engine.
- Do not replace Backstage or other catalog systems.
- Do not put lifecycle authority in skills, MCP servers, hosted services, or adapters.
- Do not require hosted infrastructure for the core workflow.
- Do not make `AGENTS.md` long-form documentation.
- Do not blur artifact schemas with mutable engineering standards.
- Do not hard-code the long-term model to one forge, catalog, or agent host.
- Do not treat compatibility warnings as sufficient enforcement for breaking changes.

## Open Questions

- Which reconciliation-polish surfaces land first — lead-side `topics[]` / `clusters[]`, binding `affinity`, or synthesis `advisory-context`?
- Which rules should ship as deterministic scanners next, and which should stay model-assisted findings?
- What is the minimum Backstage registry projection needed for useful planning?
- What compatibility classifier is sufficient before producer changes can gate on consumer impact (RM-11)?
- What is the smallest forge adapter contract for push, PR/MR handoff, CI state, and finalize?
- How should orchestration ownership and handoff work across multiple operators or agents?
- What compatibility guarantees should adapter authors provide across adapter and declared-tool versions?
- How much telemetry should emit by default, and what requires explicit opt-in?
- What approval model is required before hosted execution can push branches or open pull requests?
