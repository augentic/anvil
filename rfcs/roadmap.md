# Specify Roadmap

> Status: Draft
> Source: Review of Cloudflare's internal AI engineering stack, especially the platform, knowledge, and enforcement layers described in [https://blog.cloudflare.com/internal-ai-engineering-stack/](https://blog.cloudflare.com/internal-ai-engineering-stack/).

## Thesis

Specify should be the spec-driven workflow control plane for agentic software delivery. It should use developer portals, model gateways, CI, forges, and hosted runners without becoming any of them.

The local substrate is credible: slice/change vocabulary, registry-aware planning, workspace execution, branch preparation, push/finalize handoff, declared tools, and layered skills are all in place. The **enforcement** and **reconciliation** pillars are in place too — `lint`, `validate`, and framework checks share one `Diagnostic` substrate (`diagnostics`) while keeping distinct gate authority, and core owns how sources reconcile into slices. Durable specs live in [docs/standards/workflow.md](../docs/standards/workflow.md), [DECISIONS.md](../DECISIONS.md), and [From sources to slices](../docs/explanation/reconciliation.md). The next phase makes the loop observable and portable across teams, forges, agents, and catalogs.

At scale, Specify spans three connected layers:

1. **Platform:** models, tools, sandboxes, logs, and long-running execution.
2. **Knowledge:** repositories, owners, dependencies, standards, adapters, and plans.
3. **Enforcement:** review, compatibility checks, standards checks, and stale-context detection.

Specify owns the workflow semantics across those layers: intent becomes artifacts; artifacts become plans; plans route work to repositories; repositories change through controlled phases; outcomes are reviewed and recorded for audit and recovery.

## Principles

- **Keep the CLI authoritative.** Skills, MCP servers, CI, and cloud runners may orchestrate `specify`; they must not reimplement lifecycle transitions, plan validation, registry validation, workspace sync, or merge behavior.
- **One authored home per fact; derive the rest.** Each project's intent (`adapter`, `description`) lives in `.specify/project.yaml`; routing identity (`surface[]`, `decisions[]`, `recent[]`) is a deterministic baseline projection committed as `.specify/topology.lock`. `registry.yaml` carries membership and location only (plus optional greenfield adapter seed and cross-project `contracts` wiring) — not adapter/description for plan-time topology. Rich catalog metadata can still live in Backstage or another catalog; Specify consumes reviewable projections at the boundary.
- **Separate workflow, standards, and artifacts.** Workflow skills orchestrate phases; rules carry durable engineering policy; artifacts capture slice-local and baseline product intent.
- **Optimize for local first, cloud later.** `specify plan execute` remains the proving ground, but guest locks, journals, phase outcomes, workspace state, review results, and recovery records should be durable enough for hosted execution.
- **Prove the whole loop.** Eval coverage should exercise realistic multi-repo flows, not just isolated command behavior.
- **Abstract external systems at the boundary.** Forges, catalogs, agents, and hosted runners should integrate through narrow adapters.
- **Keep enforcement surfaces distinct.** Framework-repo **authoring standards** (the framework-quality cargo tests at `tests/framework/`) and consumer-project **engineering standards** (resolved by `specify rules export`; the consumer scanner `specify lint project` retired from the operational surface) never share gate authority: `validate` gates lifecycle transitions and is non-silenceable, while standards review is lifecycle-neutral and silenceable. See [docs/explanation/standards-layer.md](../docs/explanation/standards-layer.md).
- **Core owns reconciliation.** If a rule decides how sources combine, how evidence becomes artifacts, or how one slice drives multiple outputs, it belongs in the CLI or a CLI-owned schema — not only in a skill body. This does not move *any grouping judgment* off the agent: the agent owns "are these two leads the same work?" and expresses it in `slices[]`; the CLI computes no groupings. What core owns is the typed schema those judgments are recorded in, the coverage guarantee over the result, and the audit trail around that judgment. The lead-side fields are agent-authored typed facts a deterministic layer *checks and surfaces*, never a deterministic replacement for the agent's grouping. See [From sources to slices](../docs/explanation/reconciliation.md) and [DECISIONS.md](../DECISIONS.md).

## Effect-oriented architecture: the in-place migration

The runtime architecture — Specify as a family of Wasm guests on the Omnia runtime, with **judgment as the `wasi-model` host effect** behind a swappable model backend — is fixed in [architecture.md](architecture.md). The original S1–S4 staging (RFC-51–60) predates the Omnia refactoring and has been removed from the tree (recoverable from git history); RFC-61, the in-place migration of the framework and its adapters onto the implemented runtime, is complete (all five steps and the Step 4 parity audit) and likewise removed, as are RFC-62 (the adapter author's dev loop — the prose overlay, operation selector, model override, and watch task, implemented in `specify-adapters`), RFC-65 (standalone deployment — the provisioning / runtime surface split, hydration kernel, central store, generated manifest, and binary-versioned core guest; decisions recorded in DECISIONS.md), and RFC-66 (publishing and distribution — the one-transport wasm-pkg/OCI registry backing behind `augentic.io`, the idempotent tag-driven publish legs for the adapters, the core guest, and the WIT contract, and the codex ownership flip; tap automation and the overlay manifest-cache extraction ride later work). Two relatives are deferred, not archived: [RFC-55](future/rfc-55-working-tree.md) (distributed working trees — not needed while every guest shares the deployment's `[[mount]]` preopens) and [RFC-60](future/rfc-60-verify-profiles.md) (verify profiles — the `verify` grant is accepted but stubbed).

### Cross-repo coordination

Realising the architecture spans four repositories, coordinated only through versioned WIT seams — never a shared build or a lockstep release.

- **`augentic/specify`** (this repo) — owns the typed contract (the `specify:adapter` package), the Specify runtime binary (the `runtime!` deployment that binds the model backend and serves the MCP routes), the workflow guest, and the operator CLI surface.
- **`augentic/omnia`** — owns the generic runtime library (the Wasmtime interpreter, the pluggable host-service framework, multi-guest deployments, host-mediated linking) and the general-purpose host interfaces, including `wasi-model` (`omnia:model/completion.create`). It carries zero Specify domain knowledge and zero model knowledge.
- **`augentic/backends`** — owns the model backends behind `wasi-model`: `omnia-cursor` (spawns `cursor-agent` against the mounted working tree with MCP grants) and `omnia-genai` (frontier / hosted APIs); Omnia's in-tree `ModelDefault` covers deterministic replay.
- **`augentic/specify-adapters`** — consumes the `specify:adapter` package as a pinned dependency and ships a WASM component per adapter: its axis world plus the `wasi:http` MCP export serving its compiled-in references.

One Specify-owned seam is versioned across the boundary: `specify:adapter` (this repo → adapters). Land a published `specify:adapter` pin before the adapter components that consume it, and treat the seam as a contract so neither repo blocks the other. The Omnia runtime — including the `wasi-model` host interface — is consumed as an ordinary upstream dependency.

## Sequenced Roadmap

Items are identified as `RM-NN`. Earlier items unblock later ones unless noted otherwise. Command examples are target surfaces unless the item explicitly says the command is already implemented.

### Current priorities

The near-term focus is observability and portability (RM-14 / RM-15 / RM-18): a known build of the operator-facing surfaces over the existing `journal` substrate — the substrate exists; the surfaces do not.

#### Recently implemented

- RFC-46: Asset materialization and mandatory app icon — canonical SVG inputs with auto-convert or operator-pinned `exports/<platform>/` hand-built assets, deterministic in-guest asset materialization (the vectis build prelude in `specify-adapters`), render-by-`kind` shell writers, one logical `app-icon` with per-platform delivery, and bootstrap-only `plan-bootstrap-app-icon-missing` (shell-resident launcher icons satisfy incremental plans). Scoped to iOS/Android; web asset materialization is split out to [RFC-46a](future/rfc-46a-web-asset-materialization.md) (deferred).

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

**Goal:** Make the `/spec:plan` → `specify plan execute` → `/spec:finalize` lifecycle's re-entry and pause points machine-readable.
**Output:** JSON status with current step, last completed step, pending human action, owner, and next valid resume point.
**Consumes:** *Local structured workflow events*.
**Remaining:** `specify plan status` already projects `current-step` / `last-completed` / `resume` alongside `next-action`; the open fields are **pending human action** and **owner**, both of which depend on the human-intervention and owner signals RM-14's event surface is meant to carry.

---

### Mid Term

#### RM-11: Dependency-aware compatibility gates

**Goal:** Block producer slices from reaching `done` while breaking consumer follow-up is unaccounted for.
**Depends:** [From sources to slices](../docs/explanation/reconciliation.md) (typed slice model, per-slice fan-out via `depends-on`); the diagnostic substrate and consumer scanner (`IFACE-`* contract findings — [DECISIONS.md §Diagnostic substrate](../DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate) and [§Standards layer split](../DECISIONS.md#standards-layer-split-into-standards-and-schema)).
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
**Substrate (post-RFC-61):** the deployment already serves MCP — every adapter guest exports `wasi:http/incoming-handler` over `omnia_guest::mcp`, and the runtime binary routes MCP prefixes. This item becomes another route on the existing deployment (plausibly an export of the workflow guest), not a standalone server.
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
**Shape (post-RFC-61):** hosted execution means hosting the Omnia deployment durably. Model calls are session-less by design (fresh spawn per `create`, state carried in the working tree and `.specify/`), so resumability comes from the journal and `.specify/` state — there are no agent sessions to resume.
**Requires:** sandboxed workspace clones, durable lock ownership, serialized phase outcomes and journals, human approval gates, controlled push/PR creation, deterministic recovery, and parity with `specify plan execute`.
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
**Reframed by RFC-61:** an adapter is a wasm component implementing one axis of the versioned `specify:adapter` WIT contract, so compatibility becomes WIT-package versioning rather than manifest brief-path machinery (deleted at RFC-64). Adapters publish as single components (`wkg publish`) and install into the global single-file store; in-runtime OCI guest sources remain a runtime capability to unlock.
**Remaining:** third-party namespacing beyond the `specify:` namespace, a per-adapter release index, a WIT-contract compatibility matrix and semver-range floor policy, OCI (or equivalent) component distribution, migration guidance, and quality gates, examples, and ownership (rules, prompt briefs, references) beyond the first-party Omnia/Vectis/contracts set.

#### RM-22: Hosted observability dashboards

**Goal:** Build hosted dashboards on top of local structured workflow events without making local workflows depend on hosted infrastructure.

---

## Ideas (parked)

Each is one paragraph of intent. An idea graduates to active roadmap work only when it gains an owner and a trigger condition.

- **Specialized SLM code generation.** Train a specialized Small Language Model to generate Omnia Rust crates from Specify artifacts (Vectis following once proven), making the model behind the Omnia `build/crate.md` brief cheaper, faster, and more reproducible — without replacing the workflow. Post-RFC-61 this slots cleanly behind the swappable `wasi-model` backend.
- **CLI observability.** RFC-61's runtime binary binds `WasiOtel`, so `tracing`-based diagnostics for guest execution largely arrive with the migration. What remains parked is the residue wasi:otel does not cover — host-side deployment diagnostics and any stdout-contract-preserving ephemeral views over them.
- **Source catalogue and source-clone cache.** A durable platform-level catalogue of legacy source repositories (`sources.yaml`), a shared source-clone cache, and a `--source @<key>` selector so a platform repo declares dozens of legacy sources once and reuses them across changes.
- **Migration ledger and slice mapping.** Cumulative cross-change state answering "is this source migrated yet?" and "what's the source-to-target pattern of this slice?" for migrations spanning many changes.
- **Omnia plan composition.** Teach `plan.yaml` to express the composition shape Omnia migrations produce — services composed of crates composed of handlers — without a parallel artifact or breaking existing plans.
- **Standards baseline.** The cross-run lint lifecycle: acknowledging a body of legitimate findings as baseline debt, diffing scans against prior runs, and staging remediation across releases. Deferred — no consumers under fix-before-release on Specify-native codebases.
- **Orchestration trace replay for eval scenarios.** Deterministic structural grading lives in the [assertion taxonomy](../evals/shared/assertions.md) (per-assertion `Probe` vs `Judgment flag`), so structure is self-graded and only prose is human-judged. RFC-61 supplies a cleaner seam for the same goal: `ModelDefault` replays recorded *answers* at the `wasi-model` boundary, giving deterministic replay without capturing editor transcripts at all. The recorded-transcript approach — capture a `cursor-agent` run via `@cursor/sdk` and replay it against the real CLI, parked in [`docs/contributing/evals.md` §"Synthesis byte-replay (deferred)"](../docs/contributing/evals.md) — is largely absorbed by the model-seam replay; anything left of it stays parked behind the `transcript-replay-added` / `automated-runner-added` negative-expectations every scenario encodes.

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

