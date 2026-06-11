# Specify Roadmap

> Status: Draft
> Source: Review of Cloudflare's internal AI engineering stack, especially the platform, knowledge, and enforcement layers described in [https://blog.cloudflare.com/internal-ai-engineering-stack/](https://blog.cloudflare.com/internal-ai-engineering-stack/).

## Thesis

Specify should be the spec-driven workflow control plane for agentic software delivery. It should use developer portals, model gateways, CI, forges, and hosted runners without becoming any of them.

The local substrate is credible: slice/change vocabulary, registry-aware planning, workspace execution, branch preparation, push/finalize handoff, declared tools, and layered skills are in place (durable behaviour in [`docs/standards/workflow.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md)). The **enforcement** pillar is in place (durable spec in [`specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) — [Diagnostic substrate](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate), [standards layer split](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema), [lint finding lifecycle](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#lint-finding-status-disposition-and-exit), and [declarative `CORE-*` rules](../docs/explanation/standards-layer.md)) — `lint`, `validate`, and framework checks share the `Diagnostic` / `DiagnosticReport` substrate (`specify-diagnostics`) while keeping distinct gate authority. The **reconciliation** pillar is in place — see [From sources to slices](../docs/explanation/reconciliation.md); durable spec in [`specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) and [`docs/standards/workflow.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md). The next phase should **prove** that loop end-to-end on realistic multi-repo flows (RM-05), sharpen the remaining reconciliation seams (RFC-38), and then make it observable and portable across teams, forges, agents, and catalogs.

At scale, Specify spans three connected layers:

1. **Platform:** models, tools, sandboxes, logs, and long-running execution.
2. **Knowledge:** repositories, owners, dependencies, standards, adapters, and plans.
3. **Enforcement:** review, compatibility checks, standards checks, and stale-context detection.

Specify owns the workflow semantics across those layers: intent becomes artifacts; artifacts become plans; plans route work to repositories; repositories change through controlled phases; outcomes are reviewed and recorded for audit and recovery.

## Principles

- **Keep the CLI authoritative.** Skills, MCP servers, CI, and cloud runners may orchestrate `specify`; they must not reimplement lifecycle transitions, plan validation, registry validation, workspace sync, or merge behavior.
- **One authored home per fact; derive the rest.** Each project's intent (`adapter`, `description`) lives in `.specify/project.yaml`; routing identity (`surface[]`, `decisions[]`, `recent[]`) is a deterministic baseline projection committed as `.specify/topology.lock` (RFC-36). `registry.yaml` carries membership and location only (plus optional greenfield adapter seed and cross-project `contracts` wiring) — not adapter/description for plan-time topology. Rich catalog metadata can still live in Backstage or another catalog; Specify consumes reviewable projections at the boundary.
- **Separate workflow, standards, and artifacts.** Workflow skills orchestrate phases; rules carry durable engineering policy; artifacts capture slice-local and baseline product intent.
- **Optimize for local first, cloud later.** `/spec:execute` remains the proving ground, but plan locks, journals, phase outcomes, workspace state, review results, and recovery records should be durable enough for hosted execution.
- **Prove the whole loop.** Acceptance coverage should exercise realistic multi-repo flows, not just isolated command behavior.
- **Abstract external systems at the boundary.** Forges, catalogs, agents, and hosted runners should integrate through narrow adapters.
- **Keep enforcement surfaces distinct.** Reserve separate enforcement surfaces for framework-repo **authoring standards** (`specdev lint`) and consumer-project **engineering standards** (`specrun lint`). Both share rule ids and the neutral `Diagnostic` finding shape via the RFC-28 substrate ([`DECISIONS.md` §Diagnostic substrate](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate)); RFC-32 adds the consumer scanner substrate ([`DECISIONS.md` §Standards layer split](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema)); RFC-34 adds declarative `CORE-*` convergence on the framework side ([standards layer](../docs/explanation/standards-layer.md)). Surfaces converge on the data type, fingerprint, validator, renderer, and blocking predicate — never on gate authority: `validate` gates lifecycle transitions and is non-silenceable, while `lint` is lifecycle-neutral and silenceable. See [docs/explanation/standards-layer.md](../docs/explanation/standards-layer.md).
- **Core owns reconciliation.** If a rule decides how sources combine, how evidence becomes artifacts, or how one slice drives multiple outputs, it belongs in the CLI or a CLI-owned schema — not only in a skill body. See [From sources to slices](../docs/explanation/reconciliation.md) and [`specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md).

## Sequenced Roadmap

Items are identified as `RM-NN`. Earlier items unblock later ones unless noted otherwise. Command examples are target surfaces unless the item explicitly says the command is already implemented.

### Current priorities

Three tracks run in parallel:

1. **Acceptance proof (RM-05)** — the release gate. The 2.0.0 cross-repo queue is the blocker; scenario #1 (pure intent, N=1) must pass before the rest of the queue drains. The deterministic CLI proof for fan-in/fan-out runs under `cargo make test` in `specify-cli` ([`tests/plan/end_to_end.rs`](https://github.com/augentic/specify-cli/blob/main/tests/plan/end_to_end.rs)); the remaining debt is manual LLM-driven scenario sweeps and generated-output-correctness gates per target.
2. **Reconciliation polish (RFC-38)** — additive deterministic hints on the lead side (`topics[]`, advisory `clusters[]`, binding `affinity`, decision-conflict warnings), wiring baseline context into synthesis (`advisory-context`), and a greenfield identity seed.
3. **Observability and portability (RM-14 / RM-15 / RM-18)** — most valuable once RM-05 proves the loop on realistic flows.

**In flight (RFC refinement — not yet implementation):**

- [RFC-45: Asset materialization and mandatory app icon](rfc-45-asset-materialization.md) — canonical SVG inputs with auto-convert or operator-pinned `exports/<platform>/` hand-built assets, deterministic `vectis materialize assets`, render-by-`kind` shell writers, one logical `app-icon` with per-platform delivery, and bootstrap-only `plan-bootstrap-app-icon-missing` (shell-resident launcher icons satisfy incremental plans). Scoped to iOS/Android so it ships as a single initiative; web asset materialization is split out to [RFC-45a](future/rfc-45a-web-asset-materialization.md) (deferred). Refinement branch: `rfc-45` in `augentic/specify` and `augentic/specify-cli`.

**Deferred until trigger conditions or prerequisites:**

- [RFC-45a](future/rfc-45a-web-asset-materialization.md) — web asset materialization (`sources.web`, favicon / manifest icons, `bootstrap-web`); deferred until a web shell scaffold exists. Extends RFC-45 additively.
- [RFC-33b](future/rfc-33b-standards-baseline.md) — cross-run baseline/diff; no consumers under fix-before-release on Specify-native codebases.
- RM-12 / RM-13 — catalog import and read-oriented MCP; integration surfaces that enrich an already-trustworthy core loop.

---

### Near Term

#### RM-05: Multi-repo acceptance suite

**Goal:** Prove the `/spec:plan` → Gate 1 → `/spec:execute` → `/spec:finalize` loop end-to-end on realistic multi-repo flows — not only isolated command behaviour.
**Status:** Partial — the unified [`acceptance/scenarios/`](../acceptance/scenarios/README.md) pack defines 23 scenarios including extract failure (`extract-failure`), invalid evidence (`invalid-evidence`), source sandbox denial (`source-sandbox-denied`), execute build failure (`execute-build-failure`), step-through breakout (`stepthrough-breakout`), workspace breakout (`workspace-breakout`), dual-driving refusal (`dual-driving-refused`), and stale-workspace recovery (`stale-workspace-recovery`). `pure-intent` (N=1) — the N=1 release blocker — has `passed`; the rest of the catalog's run-summaries remain pending per the catalog.
**Immediate task:** Run scenario #1 against the live `specify` binary and fill the run-summary. Halt on failure; triage before continuing.
**Remaining fixture gap:** None outstanding in the catalog; the stale-workspace recovery scenario is now authored as `stale-workspace-recovery`.
**Acceptance surfaces:** The fan-in/fan-out contract and its deterministic CLI proof are shipped ([`tests/plan/end_to_end.rs`](https://github.com/augentic/specify-cli/blob/main/tests/plan/end_to_end.rs)). RM-05 owns the remaining debt: manual LLM-driven scenario sweeps and per-target generated-output correctness (see [docs/contributing/acceptance.md](../docs/contributing/acceptance.md)).

---

### Mid Term

#### RM-11: Dependency-aware compatibility gates

**Goal:** Block producer slices from reaching `done` while breaking consumer follow-up is unaccounted for.
**Depends:** [From sources to slices](../docs/explanation/reconciliation.md) (typed slice model, per-slice fan-out via `depends-on`); RFC-28 / RFC-32 (`IFACE-*` contract findings — [`DECISIONS.md` §Diagnostic substrate](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate) and [§Standards layer split](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema)).
**Answers:** whether consumer plan entries exist, whether producer completion is allowed, and what SemVer or release impact is implied.
**Consumes:** RFC-28's neutral `Diagnostic` envelope. RM-11 owns the structured-evidence shape for `IFACE-*` contract findings (producer project, consumer project, operation id, schema pointer, channel, message, classification, `change-kind`) via `schemas/review/finding/contracts-evidence.schema.json`; the `Diagnostic` evidence union defines the `evidence.kind: structured` branch but deliberately leaves the inner `data` shape to the consumer roadmap item so contracts-specific decisions land alongside the gate that needs them.
**Target surface:**

```bash
specrun plan impact --change <name>
```

#### RM-12: Catalog import: Backstage adapter

**Goal:** Enrich Specify planning from external catalogs without making Specify a developer portal.
**Target surface:**

```bash
specrun registry import backstage
specrun registry import <source>
specrun registry diff <source>
```

**Mapping:** Backstage `System` to platform/product boundary; `Component` to registry project; `API` to interface inventory; ownership/domain/dependencies to routing and review signals.
**Output:** explicit registry diff for operator review before planning or execution.

#### RM-13: Read-oriented Specify MCP server

**Goal:** Make Specify state available to agents through MCP without duplicating business logic.
**Initial tools:** direct readers for `plan.yaml`, `registry.yaml`, workspace slots, slice metadata, plus wrappers around `specrun plan next` and `specrun slice validate`.
**Boundary:** mutating tools may come later only as wrappers around existing CLI verbs.

#### RM-14: Local structured workflow events

**Goal:** Measure workflow performance, failure modes, and model/tool usage without requiring hosted infrastructure.
**Precedent:** RFC-33a's `lint-completed` journal event (standards side); RFC-29's `slice.build.*`, `slice.synthesize.*`, and `plan.reconcile.completed` events (workflow side).
**Events include:** command/version, project/adapter, slice or plan entry, phase start/finish, validation result, invoked skill, review findings, recovery attempts, human intervention points, and model/tool metadata when available.
**Target surface:**

```bash
specrun events tail
specrun events export
```

**Output:** local JSONL or configurable telemetry sink with run identity.

#### RM-15: Structured change-lifecycle status for re-entry

**Goal:** Make the `/spec:plan` → `/spec:execute` → `/spec:finalize` lifecycle's re-entry and pause points machine-readable.
**Output:** JSON status with current step, last completed step, pending human action, owner, and next valid resume point.
**Consumes:** *Local structured workflow events*.

#### RM-17: Forge abstraction behind workspace push and change finalize

**Goal:** Support branch transport, PR/MR creation, and finalize beyond GitHub CLI.
**Adapter covers:** remote discovery, auth checks, branch existence, push permissions, PR/MR create-or-update, CI/mergeability status, merged-state verification, and provider links.
**Target surface:**

```bash
specrun forge doctor
specrun workspace push --forge github
specrun plan finalize --forge github
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
**Depends:** [RFC-30](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#bootstrap-upgrade-and-migration-lifecycle-rfc-30) (bootstrap/upgrade/migrate lifecycle) for adoption at scale.
**Includes:** publishing and discovery conventions, version compatibility tests, declared-tool compatibility, migration guidance, quality gates, examples beyond Omnia/Vectis/contracts, and ownership for rules, artifact templates, and tool manifests.

#### RM-22: Hosted observability dashboards

**Goal:** Build hosted dashboards on top of local structured workflow events without making local workflows depend on hosted infrastructure.

---

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

- Which RFC-38 surfaces land first — lead-side `topics[]` / `clusters[]`, binding `affinity`, or synthesis `advisory-context`?
- Which rules should ship as deterministic scanners next, and which should stay model-assisted findings?
- What is the minimum Backstage registry projection needed for useful planning?
- What compatibility classifier is sufficient before producer changes can gate on consumer impact (RM-11)?
- Which acceptance fixtures best represent the product proof path now that scenario #1 is the release blocker?
- What is the smallest forge adapter contract for push, PR/MR handoff, CI state, and finalize?
- How should orchestration ownership and handoff work across multiple operators or agents?
- What compatibility guarantees should adapter authors provide across adapter and declared-tool versions?
- How much telemetry should emit by default, and what requires explicit opt-in?
- What approval model is required before hosted execution can push branches or open pull requests?
