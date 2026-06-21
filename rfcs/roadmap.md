# Specify Roadmap

> Status: Draft
> Source: Review of Cloudflare's internal AI engineering stack, especially the platform, knowledge, and enforcement layers described in [https://blog.cloudflare.com/internal-ai-engineering-stack/](https://blog.cloudflare.com/internal-ai-engineering-stack/).

## Thesis

Specify should be the spec-driven workflow control plane for agentic software delivery. It should use developer portals, model gateways, CI, forges, and hosted runners without becoming any of them.

The local substrate is credible: slice/change vocabulary, registry-aware planning, workspace execution, branch preparation, push/finalize handoff, declared tools, and layered skills are all in place. The **enforcement** and **reconciliation** pillars are in place too — `lint`, `validate`, and framework checks share one `Diagnostic` substrate (`specify-diagnostics`) while keeping distinct gate authority, and core owns how sources reconcile into slices. Durable specs live in [engine/docs/standards/workflow.md](../engine/docs/standards/workflow.md), [engine/DECISIONS.md](../engine/DECISIONS.md), and [From sources to slices](../docs/explanation/reconciliation.md). The next phase makes the loop observable and portable across teams, forges, agents, and catalogs.

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
- **Core owns reconciliation.** If a rule decides how sources combine, how evidence becomes artifacts, or how one slice drives multiple outputs, it belongs in the CLI or a CLI-owned schema — not only in a skill body. This does not move *any grouping judgment* off the agent: the agent owns "are these two leads the same work?" and expresses it in `slices[]`; the CLI computes no groupings. What core owns is the typed schema those judgments are recorded in, the coverage guarantee over the result, and the audit trail around that judgment. The lead-side fields are agent-authored typed facts a deterministic layer *checks and surfaces*, never a deterministic replacement for the agent's grouping. See [From sources to slices](../docs/explanation/reconciliation.md) and [engine/DECISIONS.md](../engine/DECISIONS.md).

## Effect-oriented architecture stages (S0–S4)

The runtime architecture — Specify as a family of wasm guests on the generic Omnia runtime, with judgment behind the `eval` effect — is fixed in [architecture.md](architecture.md); this section sequences it. Each stage is independently mergeable, independently valuable, and forward-compatible on the same typed contract.

- **S0–S1 · Typed contract** ([RFC-51](rfc-51-adapter-wit.md)) — the versioned `augentic:specify` WIT package (records + per-axis interface/world signatures + host bindings) authored and published as the single source of truth. The consuming work it unlocks — schema-drift retirement (S2), callable `tool` dispatch (S3), and the component mandate (S4) — is sequenced into the stages that own it.
- **S2 · Name the effects** ([RFC-52](rfc-52-effect.md)) — `eval` (handed a `brief-path`), host-data, the `references` fallback, `kv` (host-held memoization), and the `journal` / `transition` lifecycle hooks become typed WIT imports, initially backed by today's handoff. Behaviour-neutral; the payoff is record/replay, and the agent handoff is typed against the RFC-51 records so the `*_JSON_SCHEMA` drift surface is finally retired.
- **S3 · Guests orchestrate** ([RFC-53](rfc-53-orchestration.md)) — the deterministic `tool` operations become callable through the RFC-51 bindings (retiring `wasi:cli/run` on that path), then adapters run their own multi-step operations and reach the model through `eval` rather than handing the whole operation back. The architecture first becomes visible here, and it is the last unconditional stage.
- **S4 · The runtime move** ([RFC-55](rfc-55-runtime-move.md)) — the keystone: the generic Omnia binary plus Specify backends retires the bespoke `specify` host, the deterministic effects get real host-service backends, execution becomes instance-per-call, and shipping a WASM component becomes mandatory on both axes (the prose-only adapter ends). The runtime move is **committed**, and it is a cross-repo pairing with `augentic/omnia` (the Omnia model host, [RFC-54](rfc-54-model-host.md)).
- **S4 · Workflow (and development) as guests** ([RFC-57](rfc-57-specify-guests.md)) — the workflow runs on the new runtime like every adapter; *how much* of each phase compiles into the guest versus stays agent-driven behind `eval` is the per-phase, evidence-gated call RFC-57 owns. The framework's own development tooling follows as the mechanical tail.
- **Parallel · The model fleet** ([RFC-56](rfc-56-eval-fleet.md)) — turns the S2 `eval` seam into real backends (frontier API, spawned agent, difficulty/cost router) and delivers the interactive and headless deployment modes. Independent of the runtime move — it needs only the seam — and it brings [RFC-18](future/rfc-18-slm.md) (the SLM) in as a fleet member.

[architecture.md](architecture.md) fixes the direction; the per-stage detail and open decisions live in the linked RFCs.

## Sequenced Roadmap

Items are identified as `RM-NN`. Earlier items unblock later ones unless noted otherwise. Command examples are target surfaces unless the item explicitly says the command is already implemented.

### Current priorities

The near-term focus is observability and portability (RM-14 / RM-15 / RM-18): a known build of the operator-facing surfaces over the existing `journal` substrate — the substrate exists; the surfaces do not.

#### Deferred until trigger conditions or prerequisites

- [RFC-46a](future/rfc-46a-web-asset-materialization.md) — web asset materialization (build-time `app-icon` gate extension for `web`, `sources.web`, favicon / manifest icons); deferred until a web shell scaffold exists. Extends the existing asset-materialization capability additively.
- RM-12 / RM-13 — catalog import and read-oriented MCP; integration surfaces that enrich an already-trustworthy core loop.

---

### Near Term

#### RM-14: Local structured workflow events

**Goal:** Measure workflow performance, failure modes, and model/tool usage without requiring hosted infrastructure.

**Remaining:** the operator-facing *measurement* surface on top of the existing `journal` substrate — a follow/tail view and an `export` to a JSONL file or configurable telemetry sink with run identity. `specify journal show` covers a one-shot filtered read (`--filter` / `--limit`) but not a follow stream or a sink.  
**Events should also surface:** command/version, project/adapter, validation result, invoked skill, review findings, recovery attempts, human intervention points, and model/tool metadata when available.

**Naming decision needed:** the substrate is already exposed under `specify journal`. Decide whether the consumer surface extends `journal` (`specify journal export`) or introduces a parallel `events` noun (`specify events tail|export`) — do not ship two commands over one log.

**Target surface (subject to the naming decision):**

```bash
specify journal show --follow   # or: specify events tail
specify journal export          # or: specify events export
```

#### RM-15: Structured change-lifecycle status for re-entry

**Goal:** Make the `/spec:plan` → `/spec:execute` → `/spec:finalize` lifecycle's re-entry and pause points machine-readable.
**Output:** JSON status with current step, last completed step, pending human action, owner, and next valid resume point.
**Consumes:** *Local structured workflow events*.
**Remaining:** `specify plan status` already projects `current-step` / `last-completed` / `resume` alongside `next-action`; the open fields are **pending human action** and **owner**, both of which depend on the human-intervention and owner signals RM-14's event surface is meant to carry.

---

### Mid Term

#### RM-11: Dependency-aware compatibility gates

**Goal:** Block producer slices from reaching `done` while breaking consumer follow-up is unaccounted for.
**Depends:** [From sources to slices](../docs/explanation/reconciliation.md) (typed slice model, per-slice fan-out via `depends-on`); the diagnostic substrate and consumer scanner (`IFACE-`* contract findings — [DECISIONS.md §Diagnostic substrate](../engine/DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate) and [§Standards layer split](../engine/DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema)).
**Answers:** whether consumer plan entries exist, whether producer completion is allowed, and what SemVer or release impact is implied.
**Consumes:** the neutral `Diagnostic` envelope. RM-11 owns the structured-evidence shape for `IFACE-`* contract findings via `schemas/review/finding/contracts-evidence.schema.json`; the `Diagnostic` evidence union reserves the `evidence.kind: structured` branch but leaves the inner `data` shape to this item, so contracts-specific decisions land with the gate that needs them.
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

**Goal:** Make adapters feel like a dependable ecosystem rather than bespoke first-party packages, building on the adapter semver identity, host-CLI compatibility floor, and OCI packaging/transport now in place.
**Remaining:** third-party namespacing beyond the `specify:` namespace, a per-adapter release index, a semver-*range* host-CLI floor policy, a cross-version compatibility matrix, migration guidance, and quality gates, examples, and ownership (rules, artifact templates, adapter extensions) beyond the first-party Omnia/Vectis/contracts set.

#### RM-22: Hosted observability dashboards

**Goal:** Build hosted dashboards on top of local structured workflow events without making local workflows depend on hosted infrastructure.

---

## Ideas (parked)

Each is one paragraph of intent. An idea graduates to active roadmap work only when it gains an owner and a trigger condition.

- **Type-safe skill expression.** As the skill count grows, graduate skill authoring from prose-with-frontmatter to structured YAML manifests or a Rust DSL that separates the typed skeleton from the prose body, building on the `CORE-*` framework checks (frontmatter schema enforcement, reference resolution, variable consistency, cross-skill directive validation).
- **Specialized SLM code generation.** Train a specialized Small Language Model to generate Omnia Rust crates from Specify artifacts (Vectis following once proven), making the model behind the Omnia `build/crate.md` brief cheaper, faster, and more reproducible — without replacing the workflow.
- **CLI observability.** First-class `tracing`-based ephemeral diagnostics for command execution, lifecycle transitions, plan orchestration, workspace operations, and tool runs, complementing the durable journal without changing the existing stdout contract.
- **Source catalogue and source-clone cache.** A durable platform-level catalogue of legacy source repositories (`sources.yaml`), a shared source-clone cache, and a `--source @<key>` selector so a platform repo declares dozens of legacy sources once and reuses them across changes.
- **Migration ledger and slice mapping.** Cumulative cross-change state answering "is this source migrated yet?" and "what's the source-to-target pattern of this slice?" for migrations spanning many changes.
- **Omnia plan composition.** Teach `plan.yaml` to express the composition shape Omnia migrations produce — services composed of crates composed of handlers — without a parallel artifact or breaking existing plans.
- **Standards baseline.** The cross-run lint lifecycle: acknowledging a body of legitimate findings as baseline debt, diffing scans against prior runs, and staging remediation across releases. Deferred — no consumers under fix-before-release on Specify-native codebases.
- **Orchestration trace replay for eval scenarios.** Deterministic structural grading lives in the [assertion taxonomy](../evals/shared/assertions.md) (per-assertion `Probe` vs `Judgment flag`), so structure is self-graded and only prose is human-judged. What remains deferred is recorded-transcript **orchestration replay** — capture a `cursor-agent` run via `@cursor/sdk` and replay it against the real CLI — parked in [`docs/contributing/evals.md` §"Synthesis byte-replay (deferred)"](../docs/contributing/evals.md). Activation needs *both* a stable `@cursor/sdk` capture surface *and* a reversal of the `transcript-replay-added` / `automated-runner-added` negative-expectations every scenario encodes — a deliberate operator-driven posture.

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

- Which rules should ship as deterministic scanners next, and which should stay model-assisted findings?
- What is the minimum Backstage registry projection needed for useful planning?
- What compatibility classifier is sufficient before producer changes can gate on consumer impact (RM-11)?
- What is the smallest forge adapter contract for push, PR/MR handoff, CI state, and finalize?
- How should orchestration ownership and handoff work across multiple operators or agents?
- What cross-version compatibility matrix and semver-range host-CLI floor policy should adapter authors provide across adapter versions (RM-21)?
- How much telemetry should emit by default, and what requires explicit opt-in?
- What approval model is required before hosted execution can push branches or open pull requests?

