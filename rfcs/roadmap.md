# Specify Roadmap

> Status: Draft
> Source: Review of Cloudflare's internal AI engineering stack, especially the platform, knowledge, and enforcement layers described in [https://blog.cloudflare.com/internal-ai-engineering-stack/](https://blog.cloudflare.com/internal-ai-engineering-stack/).

## Thesis

Specify should be the spec-driven workflow control plane for agentic software delivery. It should use developer portals, model gateways, CI, forges, and hosted runners without becoming any of them.

The local substrate is now credible: slice/change vocabulary, registry-aware planning, workspace execution, branch preparation, push/finalize handoff, declared tools, and layered skills have landed across RFCs 10, 13, 15, and 16. The **enforcement** pillar is landing too: [RFC-28](done/rfc-28-standards-contract.md) (contract + export), [RFC-32](done/rfc-32-standards-enforcement.md) (`specrun lint`), [RFC-33a](rfc-33a-ignore-directives.md) (ignore directives + `lint-completed` telemetry), and [RFC-34](rfc-34-rules-convergence.md) (`CORE-*` framework rules + `specdev lint`). Enforcement has since converged on a single neutral currency: `lint`, `validate`, and the framework checks all emit the shared `Diagnostic` / `DiagnosticReport` substrate (the `specify-diagnostics` leaf), so the data type, fingerprint, and renderers are uniform while lint and validate keep distinct gate authority. The next phase should make the **reconciliation** loop provable end-to-end — not only enforceable — and then observable and portable across teams, forges, agents, and catalogs.

At scale, Specify spans three connected layers:

1. **Platform:** models, tools, sandboxes, logs, and long-running execution.
2. **Knowledge:** repositories, owners, dependencies, standards, adapters, and plans.
3. **Enforcement:** review, compatibility checks, standards checks, and stale-context detection.

Specify owns the workflow semantics across those layers: intent becomes artifacts; artifacts become plans; plans route work to repositories; repositories change through controlled phases; outcomes are reviewed and recorded for audit and recovery.

## Principles

- **Keep the CLI authoritative.** Skills, MCP servers, CI, and cloud runners may orchestrate `specify`; they must not reimplement lifecycle transitions, plan validation, registry validation, workspace sync, or merge behavior.
- **Treat `registry.yaml` as a projection.** Rich catalog metadata can live in Backstage or another catalog; Specify should consume reviewable registry projections for routing, workspace sync, and execution.
- **Separate workflow, standards, and artifacts.** Workflow skills orchestrate phases; rules carry durable engineering policy; artifacts capture slice-local and baseline product intent.
- **Optimize for local first, cloud later.** `/spec:execute` remains the proving ground, but plan locks, journals, phase outcomes, workspace state, review results, and recovery records should be durable enough for hosted execution.
- **Prove the whole loop.** Acceptance coverage should exercise realistic multi-repo flows, not just isolated command behavior.
- **Abstract external systems at the boundary.** Forges, catalogs, agents, and hosted runners should integrate through narrow adapters.
- **Keep enforcement surfaces distinct.** Reserve separate enforcement surfaces for framework-repo **authoring standards** (`specdev lint`) and consumer-project **engineering standards** (`specrun lint`). Both share rule ids and the neutral `Diagnostic` finding shape via [RFC-28](done/rfc-28-standards-contract.md); [RFC-32](done/rfc-32-standards-enforcement.md) adds the consumer scanner substrate; [RFC-34](rfc-34-rules-convergence.md) adds declarative `CORE-*` convergence on the framework side. Surfaces converge on the data type, fingerprint, validator, renderer, and blocking predicate — never on gate authority: `validate` gates lifecycle transitions and is non-silenceable, while `lint` is lifecycle-neutral and silenceable. See [docs/explanation/standards-layer.md](../docs/explanation/standards-layer.md).
- **Core owns reconciliation.** If a rule decides how sources combine, how evidence becomes artifacts, or how one slice drives multiple outputs, it belongs in the CLI or a CLI-owned schema — not only in a skill body. See [From sources to slices](../docs/explanation/reconciliation.md) and [`specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md).

## Sequenced Roadmap

Items are identified as `RM-NN`. **Near Term** order reflects deliberate prioritisation after RFC-33a and RFC-34 — not every `RM-NN` id is strictly sequential. Earlier items unblock later ones unless noted otherwise. Command examples are target surfaces unless the item explicitly says the command is already implemented.

### Current priorities

After the standards layer lands, two tracks run in parallel:

1. **Reconciliation contract (RM-06)** — **shipped.** Fan-in/fan-out is CLI-owned end-to-end: `specrun source survey` / `extract`, `specrun plan propose`, `specrun slice synthesize`, the typed `model.yaml`, `specrun slice build`, and the fan-in/fan-out acceptance fixture. Operator narrative: [From sources to slices](../docs/explanation/reconciliation.md). Implementation contract: [`specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) and [`docs/standards/workflow.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md). Agent authoring: [`plugins/spec/references/synthesis/`](../plugins/spec/references/synthesis/).
2. **Acceptance proof (RM-05)** — validation debt. The 2.0.0 cross-repo queue is the release gate; scenario #1 is a blocker. Run it against the live 2.0 binary now that RM-06's deterministic seams are in place.

**Deferred until trigger conditions or prerequisites:**

- [RFC-33b](future/rfc-33b-standards-baseline.md) — cross-run baseline/diff; no consumers under fix-before-release on Specify-native codebases.
- RM-14 / RM-15 — workflow telemetry and re-entry status; most valuable once RM-06 serialises phase outcomes.
- RM-12 / RM-13 — catalog import and read-oriented MCP; integration surfaces that assume a trustworthy core loop.
- RM-18 — hosted execute; requires RM-06's deterministic phase contracts.

---

### Near Term

#### RM-06: Fan-in/fan-out workflow contract

**Goal:** Turn Specify's fan-in/fan-out promise into a CLI-owned end-to-end contract so reconciliation is a framework invariant, not agent discipline.
**Status:** Shipped — source operations, plan-time lead reconciliation, slice synthesis with inline-provenance `model.yaml`, per-slice target build envelopes, project facets via `topology.lock`, and the `tests/fan_in_fan_out.rs` acceptance fixture.
**Depends:** [RFC-25](done/rfc-25-workflow.md), [RFC-27](done/rfc-27-synthesis.md), [RFC-28](done/rfc-28-standards-contract.md), [RFC-35](done/rfc-35-synthesis-determinism.md).
**Source of truth:** [From sources to slices](../docs/explanation/reconciliation.md) (operator narrative); [`specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) and [`docs/standards/workflow.md`](https://github.com/augentic/specify-cli/blob/main/docs/standards/workflow.md) (implementation contract); [`plugins/spec/references/synthesis/`](../plugins/spec/references/synthesis/) (agent authoring).
**Surface:**

```bash
specrun source survey <source> [--format json]
specrun source extract <source> <lead> --slice <name> [--format json]
specrun plan propose --dry-run | --from <response.json>
specrun slice synthesize <slice> --dry-run | --from <response.json>
specrun slice build <slice> [--phase prepare|finalize]
```

**Unblocks:** RM-05 durable proof path, RM-11 compatibility gates, RM-14 meaningful workflow telemetry, RM-18 hosted execute.

#### RM-05: Multi-repo acceptance suite

**Goal:** Prove the `/spec:plan` → Gate 1 → `/spec:execute` → `/spec:finalize` loop end-to-end on realistic multi-repo flows — not only isolated command behaviour.
**Status:** Partial — `tests/cross-repo/runs/2.0.0/` defines 20 scenarios including extract failure (`05f`), invalid evidence (`05g`), source sandbox denial (`05j`), execute build failure (`09`), step-through breakout (`08`), workspace breakout (`11`), and dual-driving refusal (`12`). **All run-summaries are still pending**; scenario #1 (pure intent, N=1) is the release blocker per the queue README.
**Immediate task:** Run scenario #1 against the live 2.0 binary and fill the run-summary. Halt on failure; triage before continuing.
**Remaining fixture gap:** A dedicated stale-workspace recovery scenario (not yet stubbed).
**Relationship to RM-06:** RM-06's CLI-owned seams make synthesis and build proof automatable; the cross-repo queue still exercises LLM-emitted prose manually. See [docs/contributing/acceptance.md](../docs/contributing/acceptance.md).

---

### Mid Term

#### RM-10: CI-native standards enforcement

**Goal:** Continuously enforce engineering standards on consumer projects (not a workflow phase — findings may block CI but never transition plan or slice lifecycle).
**Status:** Core implemented — [RFC-28](done/rfc-28-standards-contract.md), [RFC-32](done/rfc-32-standards-enforcement.md), [RFC-33a](rfc-33a-ignore-directives.md), and [RFC-34](rfc-34-rules-convergence.md) cover the contract, consumer scanner, per-line tolerance, and framework convergence (`CORE-*` rules under `adapters/shared/rules/core/`, gated by `--include-core`) respectively. The finding currency has since converged: `lint`, `validate`, and the `specdev` framework checks all emit the neutral `Diagnostic` / `DiagnosticReport` substrate (`specify-diagnostics`), sharing the data type, fingerprint, validator, and renderers without sharing gate authority. Shared codex distribution has landed, so consumer projects resolve shared `UNI-*` rules without `--rules-root` (`specrun init` / `specrun rules sync` populate `.specify/.cache/codex/`). Optional deferred follow-on: [RFC-33b](future/rfc-33b-standards-baseline.md) (cross-run baseline/diff — lands only when trigger conditions in that RFC are met).
**Source of truth:** [RFC-28](done/rfc-28-standards-contract.md) is canonical for the resolved rule export wire shape (`schemas/rules/resolved.schema.json`, `specrun rules export`), the structured finding schema (`schemas/diagnostics/diagnostic.schema.json` + `diagnostic-report.schema.json`, the neutral `Diagnostic` / `DiagnosticReport` substrate that superseded the `LintFinding` envelope), the fingerprint algorithm, the closed severity enum (`critical` / `important` / `suggestion` / `optional`) and its orthogonal `source` (`deterministic` / `model-assisted` / `hybrid` / `human` / `tool`) and `kind` (`violation` / `review`) axes, and the evidence union; [RFC-32](done/rfc-32-standards-enforcement.md) owns `specrun lint`, hint execution, and the WorkspaceModel that consumes those shapes — RM-10 should not redefine any of them.
**Consumes:** RFC-28's resolved codex export and structured finding schema; RFC-32's deterministic standards scanner.
**Target surface:**

```bash
specrun lint run
specrun lint run --slice <name>
specrun lint run --output-format json
specdev lint --format json            # framework repo; RFC-28 Phase 3 + RFC-34
```

**Inspects:** artifact completeness, responsibility boundaries, schema validation, plan/registry consistency, compatibility classification, stale `AGENTS.md`, codex compliance, source changes missing spec coverage, and specs missing implementation evidence.
**Output:** structured findings via the settled `Diagnostic` / `DiagnosticReport` schema; `lint-completed` journal summary per RFC-33a.

#### RM-11: Dependency-aware compatibility gates

**Goal:** Block producer slices from reaching `done` while breaking consumer follow-up is unaccounted for.
**Depends:** RM-06 (the typed slice model and per-slice fan-out joined by `depends-on` make producer/consumer impact machine-readable); RM-10 (standards findings for `IFACE-*` contract rules).
**Answers:** whether consumer plan entries exist, whether producer completion is allowed, and what SemVer or release impact is implied.
**Consumes:** RFC-28's neutral `Diagnostic` envelope. RM-11 owns the structured-evidence shape for `IFACE-*` contract findings (producer project, consumer project, operation id, schema pointer, channel, message, classification, `change-kind`) via `schemas/review/finding/contracts-evidence.schema.json`; the `Diagnostic` evidence union defines the `evidence.kind: structured` branch but deliberately leaves the inner `data` shape to the consumer roadmap item so contracts-specific decisions land alongside the gate that needs them.
**Target surface:**

```bash
specrun plan impact --change <name>
```

#### RM-12: Catalog import: Backstage adapter

**Goal:** Enrich Specify planning from external catalogs without making Specify a developer portal.
**Depends:** RM-06 (plan-time reconciliation and registry routing should consume stable candidate/slice shapes before catalog enrichment adds another input).
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
**Depends:** RM-06 for mutating-tool parity later; read-only tools can start once plan/slice validation surfaces are stable.
**Initial tools:** direct readers for `plan.yaml`, `registry.yaml`, workspace slots, slice metadata, plus wrappers around `specrun plan next` and `specrun slice validate`.
**Boundary:** mutating tools may come later only as wrappers around existing CLI verbs.

#### RM-14: Local structured workflow events

**Goal:** Measure workflow performance, failure modes, and model/tool usage without requiring hosted infrastructure.
**Depends:** RM-06 (CLI-owned source/synthesis/build steps produce serialisable phase boundaries; agent-only steps do not). RFC-33a's `lint-completed` journal event is the standards-side precedent.
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
**Requires:** RM-06 (resumable phase contracts and the typed slice model); sandboxed workspace clones, durable lock ownership, resumable agent sessions, serialized phase outcomes and journals, human approval gates, controlled push/PR creation, deterministic recovery, and parity with `/spec:execute`.
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
**Depends:** [RFC-30](next/rfc-30-init.md) (bootstrap/upgrade/migrate lifecycle) for adoption at scale; RM-06 for executable adapter operations as the contract authors target.
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

- Which rules should ship as deterministic scanners next, and which should stay model-assisted findings?
- What is the minimum Backstage registry projection needed for useful planning?
- What compatibility classifier is sufficient before producer changes can gate on consumer impact (RM-11)?
- Which acceptance fixtures best represent the product proof path now that scenario #1 is the release blocker?
- What is the smallest forge adapter contract for push, PR/MR handoff, CI state, and finalize?
- How should orchestration ownership and handoff work across multiple operators or agents?
- What compatibility guarantees should adapter authors provide across adapter and declared-tool versions?
- How much telemetry should emit by default, and what requires explicit opt-in?
- What approval model is required before hosted execution can push branches or open pull requests?

