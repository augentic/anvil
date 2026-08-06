# Emery Roadmap

> Status: Draft
> Source: Review of Cloudflare's internal AI engineering stack, especially the platform, knowledge, and enforcement layers described in [https://blog.cloudflare.com/internal-ai-engineering-stack/](https://blog.cloudflare.com/internal-ai-engineering-stack/).

## Thesis

Emery should be the spec-driven workflow control plane for agentic software delivery. It should use developer portals, model gateways, CI, forges, and hosted runners without becoming any of them.

The local substrate is credible: slice/change vocabulary, registry-aware planning, workspace execution, branch preparation, push/finalize handoff, and layered skills are all in place. The **enforcement** and **reconciliation** pillars are in place too — `validate` uses the `diagnostics` crate while keeping distinct gate authority from consumer-project engineering standards, and core owns how sources reconcile into slices. Durable specs live in [docs/standards/workflow.md](../docs/standards/workflow.md), [Workflow, standards, and artifacts](../docs/explanation/standards-layer.md), and [From sources to slices](../docs/explanation/reconciliation.md). The next phase makes the loop observable and portable across teams, forges, agents, and catalogs.

At scale, Emery spans three connected layers:

1. **Platform:** models, tools, sandboxes, logs, and long-running execution.
2. **Knowledge:** repositories, owners, dependencies, standards, adapters, and plans.
3. **Enforcement:** review, compatibility checks, standards checks, and stale-context detection.

Emery owns the workflow semantics across those layers: intent becomes artifacts; artifacts become plans; plans route work to repositories; repositories change through controlled phases; outcomes are reviewed and recorded for audit and recovery.

## Principles

- **Keep the CLI authoritative for workflow state.** Skills, MCP servers, CI, and cloud runners may orchestrate `emery`; they must not reimplement lifecycle transitions, plan validation, registry validation, or merge behavior. Repository checkout and publication remain operator-owned.
- **One authored home per fact; derive the rest.** Each repository's durable intent (`adapter`, `description`, `product`) lives in `.emery/project.yaml`; routing identity (`surface[]`, `decisions[]`, `recent[]`) is a deterministic baseline projection committed as `.emery/topology.lock`. A detached change records its pinned repositories, exact revisions, and resolved target topology once under `plan.yaml.projects`; RFC-88 removes committed `registry.yaml` and tended workspace slots from multi-repository coordination. Rich catalog metadata can still live in Backstage or another catalog; Emery consumes reviewable projections at the boundary.
- **Separate workflow, standards, and artifacts.** Workflow skills orchestrate phases; rules carry durable engineering policy; artifacts capture slice-local and baseline product intent.
- **Optimize for local first, cloud later.** `emery plan execute` remains the proving ground, but guest locks, journals, phase outcomes, workspace state, review results, and recovery records should be durable enough for hosted execution.
- **Prove the whole loop.** Eval coverage should exercise realistic multi-repo flows, not just isolated command behavior.
- **Abstract external systems at the boundary.** Forges, catalogs, agents, and hosted runners should integrate through narrow adapters.
- **Keep enforcement surfaces distinct.** Framework-repo consistency (the mdBook links gate) and consumer-project **engineering standards** (embedded in each target adapter and applied by its build review prompts) never share gate authority: `validate` gates lifecycle transitions and is non-silenceable, while standards review is lifecycle-neutral and silenceable. See [docs/explanation/standards-layer.md](../docs/explanation/standards-layer.md).
- **Core owns reconciliation.** If a rule decides how sources combine, how evidence becomes artifacts, or how one slice drives multiple outputs, it belongs in the CLI or a CLI-owned schema — not only in a skill body. This does not move *any grouping judgment* off the agent: the agent owns "are these two leads the same work?" and expresses it in `slices[]`; the CLI computes no groupings. What core owns is the typed schema those judgments are recorded in, the coverage guarantee over the result, and the audit trail around that judgment. The lead-side fields are agent-authored typed facts a deterministic layer *checks and surfaces*, never a deterministic replacement for the agent's grouping. See [From sources to slices](../docs/explanation/reconciliation.md).

## Effect-oriented architecture

The runtime architecture — Emery as a family of Wasm guests on the Omnia runtime, with **judgment as the `wasi-model` host effect** behind a swappable model backend — is fixed in [architecture.md](architecture.md). Formerly deferred, now sequenced in the platform-migration series ([platform.md](platform.md)): critical path [RFC-86](rfc-86-change-facts.md) → [RFC-87](rfc-87-working-trees.md) → [RFC-88](rfc-88-detached-changes.md) → [RFC-89](rfc-89-publication-sets.md); scale track [RFC-90](rfc-90-build-verification.md) → [RFC-91](rfc-91-concurrent-execution.md) → [RFC-92](rfc-92-node-sync.md).

### Cross-repo coordination

Realising the architecture spans four repositories, coordinated only through versioned WIT seams — never a shared build or a lockstep release.

- **`augentic/emery`** (this repo) — owns the typed contract (the `emery:adapter` package), the Emery runtime binary (the `runtime!` deployment that binds the model backend and serves the MCP routes), the engine guest, and the operator CLI surface.
- **`augentic/omnia`** — owns the generic runtime library (the Wasmtime interpreter, the pluggable host-service framework, multi-guest deployments, host-mediated linking) and the general-purpose host interfaces, including `wasi-model` (`omnia:model/completion.create`). It carries zero Emery domain knowledge and zero model knowledge.
- **`augentic/backends`** — owns the model backends behind `wasi-model`: `omnia-cursor` (spawns `cursor-agent` against the private workspace with MCP grants) and `omnia-genai` (frontier / hosted APIs); Omnia's in-tree `ModelDefault` covers deterministic replay.
- **`augentic/emery-adapters`** — consumes the `emery:adapter` package as a pinned dependency and ships a WASM component per adapter: its axis world plus the `wasi:http` MCP export serving its compiled-in references.

One Emery-owned seam is versioned across the boundary: `emery:adapter` (this repo → adapters). Land a published `emery:adapter` pin before the adapter components that consume it, and treat the seam as a contract so neither repo blocks the other. The Omnia runtime — including the `wasi-model` host interface — is consumed as an ordinary upstream dependency.

That decoupling governs steady state. The transition moment when a seam itself must move — [RFC-77](rfc-77-release-process.md)'s WIT-breaking shape — is an ordered multi-repo landing, and that landing is a publication set ([RFC-89](rfc-89-publication-sets.md)): derived from the plan, marked on the forge, tracked and verified by Emery, published by the operator.

## Sequenced Roadmap

Items are identified as `RM-NN`. Earlier items unblock later ones unless noted otherwise. Command examples are target surfaces unless the item explicitly says the command is already implemented.

### Current priorities

The near-term focus is observability and portability (RM-14 / RM-15 / RM-18): a known build of the operator-facing surfaces over the existing `journal` substrate — the substrate exists; the surfaces do not.

In parallel, the migration critical path is [RFC-86 Change Facts](rfc-86-change-facts.md) → [RFC-87 Private Workspaces](rfc-87-working-trees.md) → [RFC-88 Detached Changes](rfc-88-detached-changes.md) → [RFC-89 Publication Sets](rfc-89-publication-sets.md) (see [platform.md](platform.md)). Installation already works ([RFC-71](rfc-71-deployment.md)); Stage 2 diagnostics (`resolution.json` / `deployment show|doctor`) stay separate. Omnia stays free of Emery vocabulary throughout.

Target experience: thin prior context (forge auth/org + source material) → bare change directory → discover (migrate or ongoing-change mode) → record members (create repos when needed) → plan → prepare private workspaces on execute → publish → finalize. RFC-88 also removes operator-authored source keys and lets ordinary single-project planning reuse its deterministic local source selector. Binding constraint is **adapter inventory** — today one code source (`typescript`) and no web target.

Delivery is strictly sequential: finish every decision and acceptance criterion in one RFC before implementing the next. Later RFCs consume settled capabilities; no RFC retains a phase gated on a successor.

#### Deferred until trigger conditions or prerequisites

- [RFC-46a](future/rfc-46a-web-asset.md) — web asset materialization; deferred until a web shell scaffold exists.
- Scale-track concurrency ([RFC-90](rfc-90-build-verification.md) / [RFC-91](rfc-91-concurrent-execution.md) / [RFC-92](rfc-92-node-sync.md)) — after the migrate/change location story works on the critical path.
- RM-12 / RM-13 — catalog import and read-oriented MCP.

---

### Near Term

#### RM-14: Local structured workflow events

**Goal:** Measure workflow performance, failure modes, and model/tool usage without requiring hosted infrastructure.

**Remaining:** the operator-facing *measurement* surface on top of the existing `journal` substrate — a follow/tail view and an `export` to a JSONL file or configurable telemetry sink with run identity. `emery journal show` covers a one-shot filtered read (`--filter` / `--limit`) but not a follow stream or a sink.  
**Events should also surface:** command/version, project/adapter, validation result, invoked skill, review findings, recovery attempts, human intervention points, and model/tool metadata when available.

**Naming decision needed:** the substrate is already exposed under `emery journal`. Decide whether the consumer surface extends `journal` (`emery journal export`) or introduces a parallel `events` noun (`emery events tail|export`) — do not ship two commands over one log.

**Target surface (subject to the naming decision):**

```bash
emery journal show --follow   # or: emery events tail
emery journal export          # or: emery events export
```

#### RM-15: Structured change-lifecycle status for re-entry

**Goal:** Make the `/emery:plan` → `emery plan execute` → `/emery:finalize` lifecycle's re-entry and pause points machine-readable.
**Output:** JSON status with current step, last completed step, pending human action, owner, and next valid resume point.
**Consumes:** *Local structured workflow events*.
**Remaining:** `emery plan status` already projects `current-step` / `last-completed` / `resume` alongside `next-action`; the open fields are **pending human action** and **owner**, both of which depend on the human-intervention and owner signals RM-14's event surface is meant to carry.

---

### Mid Term

#### RM-11: Dependency-aware compatibility gates

**Goal:** Block producer slices from reaching `done` while breaking consumer follow-up is unaccounted for.
**Depends:** [From sources to slices](../docs/explanation/reconciliation.md) (typed slice model, per-slice fan-out via `depends-on`); the [`diagnostics`](../crates/diagnostics/) substrate for `IFACE-`* contract findings.
**Answers:** whether consumer plan entries exist, whether producer completion is allowed, and what SemVer or release impact is implied.
**Consumes:** the neutral `Diagnostic` envelope. RM-11 owns the structured-evidence shape for `IFACE-`* contract findings via `schemas/review/finding/contracts-evidence.schema.json`; the `Diagnostic` evidence union reserves the `evidence.kind: structured` branch but leaves the inner `data` shape to this item, so contracts-specific decisions land with the gate that needs them.
**Target surface:**

```bash
emery plan impact --change <name>
```

#### RM-12: Catalog import: Backstage adapter

**Goal:** Enrich Emery planning from external catalogs without making Emery a developer portal.
**Target surface:**

```bash
emery registry import backstage
emery registry import <source>
emery registry diff <source>
```

**Mapping:** Backstage `System` to platform/product boundary; `Component` to registry project; `API` to interface inventory; ownership/domain/dependencies to routing and review signals.
**Output:** explicit registry diff for operator review before planning or execution.

#### RM-13: Read-oriented Emery MCP server

**Goal:** Make Emery state available to agents through MCP without duplicating business logic.
**Substrate:** every adapter guest can export `wasi:http/incoming-handler` over `omnia_guest::mcp`. Ordinary-path derived MCP route projection (`/mcp/<axis>/<name>[@<version>]` via `launcher::mcp_route`) is already landed ([RFC-71](rfc-71-deployment.md)). This item becomes another route on that deployment (plausibly an export of the engine guest), not a standalone server.
**Initial tools:** direct readers for `plan.yaml` (including detached `projects`), materialized slot status, and slice metadata, plus wrappers around `emery plan status` and `emery slice validate`.
**Boundary:** mutating tools may come later only as wrappers around existing CLI verbs.

#### RM-17: Operator-owned forge integration

**Goal:** Support branch transport, PR/MR creation, and finalize beyond GitHub CLI.
**Provider extension covers:** branch transport, push permissions, PR/MR create-or-update, CI/mergeability status, and provider links over RFC-88's forge capability. RFC-88 already owns GitHub discovery/create and RFC-89 already owns the PR reads needed for verification; neither waits for this item.
**Target surface:**

```bash
emery forge doctor
gh pr create
emery plan finalize --forge github
```

---

### Long Term

#### RM-18: Cloud-hosted execute loop

**Goal:** Run Emery plans durably in the background while preserving local workflow semantics.
**Shape:** hosted execution means hosting the Omnia deployment durably. Model calls are session-less by design (fresh spawn per `create`); resumability comes from coordination facts and immutable snapshots, not from retaining an agent session or workspace.
**Requires:** completed [RFC-92](rfc-92-node-sync.md) for remote workspaces, fact and snapshot transport, fenced claims, and remote worker pools; digest coverage for privileged work comes from [RFC-86](rfc-86-change-facts.md)'s `plan.execute.started`; plus controlled push/PR creation, deterministic recovery, and parity with `emery plan execute`.
**Target surface:**

```bash
emery execute submit
emery execute status <run-id>
emery execute resume <run-id>
```

#### RM-19: Multi-forge adapter coverage

**Goal:** Add GitLab, Bitbucket, and self-hosted bindings for RFC-88's forge provider; GitHub is already the completed first binding.

#### RM-20: Catalog-backed initiatives across many repositories

**Goal:** Drive multi-repo initiatives from live catalog-backed registry projections.
**First profile:** the migrate/change loop ([RFC-87](rfc-87-working-trees.md) through [RFC-88](rfc-88-detached-changes.md)) is the first concrete initiative shape; RM-20 generalises the noun only after that profile proves the coordination semantics.
**Coordination semantics:** one change spanning many repositories is a publication set ([RFC-89](rfc-89-publication-sets.md)) — plan-derived members, `Emery-Change` markers, publication order from `depends-on`, verification at finalize. RM-20 layers initiative scoping over that record; it does not define a second coordination model.

#### RM-21: Adapter ecosystem operating model

**Goal:** Make adapters feel like a dependable ecosystem rather than bespoke first-party packages.
**Frame:** an adapter is a wasm component implementing one axis of the versioned `emery:adapter` WIT contract, so compatibility is WIT-package versioning. Adapters publish as single components (`wkg publish`) and install into the global single-file store; in-runtime OCI guest sources remain a runtime capability to unlock.
**First-party seam:** [RFC-76 Adapter Publish and Install](archive/rfc-76-adapter-install.md) (archived — Phases A–D landed; Actions GHCR publish landed; CI no-repush + attestations remain under RFC-77 Phase B) closed the build → publish → pull-on-miss → resolve loop. Release lines and host↔adapter coordination are owned by [RFC-77 Release Process](rfc-77-release-process.md) (Phase A landed; B/C deferred). Live-seam cost: [RFC-78 Judgment Leg Budget](archive/rfc-78-prompt-budget.md) (archived — D1–D8 landed: inactivity-based cursor timeouts, session-resume repairs, deterministic replay-skip and report absorption, SDK-level path-first inputs, dropped guidance refreshers, thinned build preambles; the D4.3 mismatch fail-fast and engine-kernel session resume remain follow-ons) shrank lent-workspace target-build cost after `wasm-omnia-r9k` measured ~64 KB generation spills and a 600s cursor timeout in standards review. It remains the enabling layer for [RFC-91 Concurrent Execution](rfc-91-concurrent-execution.md) (focused convergent build requests over RFC-90's engine-owned model-assisted verification and repair loop ([RFC-90](rfc-90-build-verification.md)), with local per-worker trees over [RFC-87](rfc-87-working-trees.md), plus the engine references shelf, staged write-to-tree artifacts, and parallel fan-outs). RFC-92 alone distributes the completed worker model.
**Remaining:** third-party namespacing beyond the `emery:` namespace, a per-adapter release index, a WIT-contract compatibility matrix and semver-range floor policy, OCI (or equivalent) component distribution, migration guidance, and quality gates, examples, and ownership (rules, prompt briefs, references) beyond the first-party Omnia/Vectis/contracts set.
**Coverage:** automatic source selection ([RFC-88](rfc-88-detached-changes.md#source-adapter-selection)) can only choose identities in the engine's bounded first-party selector profiles. Today that is one code source adapter (`typescript`, TS/JS only). Language source adapters (Java, Python, Go, C#, Ruby) and a web target are concrete inventory this item owes the migration track; in the first-party cut, adding an automatically selectable source identity requires adapter publication plus an Emery release.
**Selection substrate:** [RFC-88](rfc-88-detached-changes.md#source-adapter-selection) keeps first-party selector profiles in the engine and resolves only the selected adapter. Dynamic registry inventory, third-party metadata, publisher trust, and optional organizational allowlists land together under this item rather than through a hand-maintained local roster or a premature WIT contract.

#### RM-22: Hosted observability dashboards

**Goal:** Build hosted dashboards on top of local structured workflow events without making local workflows depend on hosted infrastructure.

---

## Ideas (parked)

Each is one paragraph of intent. An idea graduates to active roadmap work only when it gains an owner and a trigger condition.

- **Specialized SLM code generation.** Train a specialized Small Language Model to generate Omnia Rust crates from Emery artifacts (Vectis following once proven), making the model behind the Omnia `build/crate.md` prompt cheaper, faster, and more reproducible — without replacing the workflow. This slots cleanly behind the swappable `wasi-model` backend.
- **CLI observability.** The runtime binary binds `WasiOtel`, so `tracing`-based diagnostics for guest execution already exist. What remains parked is the residue wasi:otel does not cover — host-side deployment diagnostics and any stdout-contract-preserving ephemeral views over them.
- **Source adapter selection.** Deterministic first-party selection is part of [RFC-88](rfc-88-detached-changes.md); dynamic catalog and trust belong to RM-21.
- **Omnia plan composition.** Teach `plan.yaml` to express the composition shape Omnia migrations produce — services composed of crates composed of handlers — without a parallel artifact or breaking existing plans.
- **Standards baseline.** The cross-run lint lifecycle: acknowledging a body of legitimate findings as baseline debt, diffing scans against prior runs, and staging remediation across releases. Deferred — no consumers under fix-before-release on Emery-native codebases.
- **Orchestration replay coverage.** Canonical scenarios separate hard assertions from semantic rubrics. `ModelDefault` provides deterministic request-key replay at the `wasi-model` boundary; native and composed profiles reuse that contract without capturing editor transcripts. Live profiles remain outside ordinary CI.

## Non-Goals

- Do not make Emery a general developer portal, AI gateway, CI system, or forge policy engine.
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
- What cross-version compatibility matrix and semver-range host-CLI floor policy should adapter authors provide across adapter versions (RM-21)? Draft process answer: [RFC-77](rfc-77-release-process.md) (exact pins + `emery-floor` + short compatibility rows; ranges still open).
- How much telemetry should emit by default, and what requires explicit opt-in?
- What authorization is required before hosted execution can push branches or open pull requests?

