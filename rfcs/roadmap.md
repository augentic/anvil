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
- **Prove the whole loop.** Eval coverage should exercise realistic multi-repo flows, not just isolated command behavior.
- **Abstract external systems at the boundary.** Forges, catalogs, agents, and hosted runners should integrate through narrow adapters.
- **Keep enforcement surfaces distinct.** Reserve separate enforcement surfaces for framework-repo **authoring standards** (`specdev lint`) and consumer-project **engineering standards** (`specrun lint`). Both share rule ids and the neutral `Diagnostic` finding shape via the RFC-28 substrate ([`DECISIONS.md` §Diagnostic substrate](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate)); RFC-32 adds the consumer scanner substrate ([`DECISIONS.md` §Standards layer split](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#standards-layer-split-into-specify-standards-and-specify-schema)); RFC-34 adds declarative `CORE-*` convergence on the framework side ([standards layer](../docs/explanation/standards-layer.md)). Surfaces converge on the data type, fingerprint, validator, renderer, and blocking predicate — never on gate authority: `validate` gates lifecycle transitions and is non-silenceable, while `lint` is lifecycle-neutral and silenceable. See [docs/explanation/standards-layer.md](../docs/explanation/standards-layer.md).
- **Core owns reconciliation.** If a rule decides how sources combine, how evidence becomes artifacts, or how one slice drives multiple outputs, it belongs in the CLI or a CLI-owned schema — not only in a skill body. See [From sources to slices](../docs/explanation/reconciliation.md) and [`specify-cli` `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md).

## Sequenced Roadmap

Items are identified as `RM-NN`. Earlier items unblock later ones unless noted otherwise. Command examples are target surfaces unless the item explicitly says the command is already implemented.

### Current priorities

Three tracks run in parallel:

1. **Eval proof (RM-05)** — the release gate. The 2.0.0 cross-repo queue is the blocker; scenario #1 (pure intent, N=1) must pass before the rest of the queue drains. The deterministic CLI proof for fan-in/fan-out runs under `cargo make test` in `specify-cli` ([`tests/plan/end_to_end.rs`](https://github.com/augentic/specify-cli/blob/main/tests/plan/end_to_end.rs)); the remaining debt is the operator-driven eval sweep and generated-output-correctness gates per target.
2. **Reconciliation polish (RFC-38)** — additive deterministic hints on the lead side (`topics[]`, advisory `clusters[]`, binding `affinity`, decision-conflict warnings), wiring baseline context into synthesis (`advisory-context`), and a greenfield identity seed.
3. **Observability and portability (RM-14 / RM-15 / RM-18)** — most valuable once RM-05 proves the loop on realistic flows.

**Deferred until trigger conditions or prerequisites:**

- [Standards baseline (was RFC-33b)](#ideas-parked-future-rfcs) — cross-run baseline/diff; no consumers under fix-before-release on Specify-native codebases.
- RM-12 / RM-13 — catalog import and read-oriented MCP; integration surfaces that enrich an already-trustworthy core loop.

---

### Near Term

#### RM-05: Multi-repo eval suite

**Goal:** Prove the `/spec:plan` → Gate 1 → `/spec:execute` → `/spec:finalize` loop end-to-end on realistic multi-repo flows — not only isolated command behaviour.
**Status:** Partial — the unified [`evals/scenarios/`](../evals/scenarios/README.md) pack defines 13 operator-driven scenarios including execute build failure (`execute-build-failure`), step-through breakout (`stepthrough-breakout`), workspace breakout (`workspace-breakout`), and stale-workspace recovery (`stale-workspace-recovery`); fully deterministic behaviors (extract failure, invalid evidence, source sandbox denial, dual-driving refusal via the plan-lock probe, …) are named tests in `specify-cli`, not catalog entries. 9 of 13 scenarios are `passed` per the catalog — all three `release-blocker` rows (`pure-intent`, the N=1 hard halt; `execute-build-failure`, the build-failure park/resume; `workspace-execute-two-projects`, the cross-project workspace execute), the core synthesis, planning, and routing set, and `target-shape-injection` — each backed by a committed record under [`evals/runs/`](../evals/runs/README.md); the remaining 4 `full`-tier rows are `pending`.
**Immediate task:** Drain the remaining `full`-tier catalog (`cross-repo-contract-flow`, `stepthrough-breakout`, `workspace-breakout`, `stale-workspace-recovery`). `pure-intent` hard halt unchanged on any re-run.
**Remaining fixture gap:** None outstanding in the catalog; the stale-workspace recovery scenario is now authored as `stale-workspace-recovery`.
**Proof surfaces:** The fan-in/fan-out contract and its deterministic CLI proof are shipped ([`tests/plan/end_to_end.rs`](https://github.com/augentic/specify-cli/blob/main/tests/plan/end_to_end.rs)). RM-05 owns the remaining debt: the operator-driven eval sweep and per-target generated-output correctness (see [docs/contributing/evals.md](../docs/contributing/evals.md)).

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
**First consumer landed (RFC-44 R3):** `specify plan status` carries `current-step` / `last-completed` / `resume` alongside `next-action` — current step, last completed step, and the literal resume command, projected from `plan.yaml`, slice metadata, and the journal. Pending human action and owner remain open.

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
**Depends:** [RFC-30](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#bootstrap-upgrade-and-migration-lifecycle) (bootstrap/upgrade/migrate lifecycle) for adoption at scale.
**Includes:** publishing and discovery conventions, version compatibility tests, declared-tool compatibility, migration guidance, quality gates, examples beyond Omnia/Vectis/contracts, and ownership for rules, artifact templates, and tool manifests.

#### RM-22: Hosted observability dashboards

**Goal:** Build hosted dashboards on top of local structured workflow events without making local workflows depend on hosted infrastructure.

---

## Ideas (parked future RFCs)

The former `rfcs/future/` drafts are consolidated here as parked ideas. Each is one paragraph of intent; the full drafts live in git history (removed 2026-06-12). An idea graduates back to an RFC only when it gains an owner and a trigger condition.

- **Type-safe skill expression (was RFC-4).** Extend framework tooling validation to skill authoring — frontmatter schema enforcement, reference resolution, variable consistency, cross-skill directive validation — and, as the skill count grows, graduate to structured YAML manifests or a Rust DSL separating the typed skeleton from the prose body. Much of the validation half has since landed as `CORE-*` framework checks.
- **Specialized SLM code generation (was RFC-18).** Train a specialized Small Language Model to generate Omnia Rust crates from Specify artifacts (Vectis following once proven), making the model behind the Omnia `build/crate.md` brief cheaper, faster, and more reproducible — without replacing the workflow.
- **CLI observability (was RFC-19).** First-class `tracing`-based structured diagnostics for command execution, lifecycle transitions, plan orchestration, workspace operations, and tool runs, without changing the existing stdout contract. Partially superseded by the journal (RM-14 lineage).
- **Source catalogue and tier-1 cache (was RFC-21).** A durable platform-level catalogue of legacy source repositories (`sources.yaml`), a shared tier-1 clone cache, and a `--source @<key>` selector so a platform repo declares dozens of legacy sources once and reuses them across changes.
- **Migration ledger and slice mapping (was RFC-22).** Cumulative cross-change state answering "is this source migrated yet?" and "what's the source-to-target pattern of this slice?" for migrations spanning many changes.
- **Omnia plan composition (was RFC-24).** Teach `plan.yaml` to express the composition shape Omnia migrations produce — services composed of crates composed of handlers — without a parallel artifact or breaking existing plans.
- **Standards baseline (was RFC-33b).** The cross-run lint lifecycle: acknowledging a body of legitimate findings as baseline debt, diffing scans against prior runs, and staging remediation across releases. Deferred — no consumers under fix-before-release on Specify-native codebases.
- **Acceptance shape assertions and orchestration traces (was RFC-39).** A deterministic middle tier for eval scenarios — shape assertions over synthesized artifacts and orchestration traces over journal events — between manual runs and byte-replay fixtures. The `backend` frontmatter carrier it assumed has since been removed, so activation needs a new carrier.

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
- Which eval fixtures best represent the product proof path now that scenario #1 is the release blocker?
- What is the smallest forge adapter contract for push, PR/MR handoff, CI state, and finalize?
- How should orchestration ownership and handoff work across multiple operators or agents?
- What compatibility guarantees should adapter authors provide across adapter and declared-tool versions?
- How much telemetry should emit by default, and what requires explicit opt-in?
- What approval model is required before hosted execution can push branches or open pull requests?
