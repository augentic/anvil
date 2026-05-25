# Specify Roadmap

> Status: Draft
> Source: Review of Cloudflare's internal AI engineering stack, especially the platform, knowledge, and enforcement layers described in [https://blog.cloudflare.com/internal-ai-engineering-stack/](https://blog.cloudflare.com/internal-ai-engineering-stack/).

## Thesis

Specify should be the spec-driven workflow control plane for agentic software delivery. It should use developer portals, model gateways, CI, forges, and hosted runners without becoming any of them.

The local substrate is now credible: slice/change vocabulary, registry-aware planning, workspace execution, branch preparation, push/finalize handoff, declared tools, and layered skills have landed across RFCs 10, 13, 15, and 16. The next phase should make that substrate provable end-to-end, enforceable, observable, and portable across teams, forges, agents, and catalogs.

At scale, Specify spans three connected layers:

1. **Platform:** models, tools, sandboxes, logs, and long-running execution.
2. **Knowledge:** repositories, owners, dependencies, standards, adapters, and plans.
3. **Enforcement:** review, compatibility checks, standards checks, and stale-context detection.

Specify owns the workflow semantics across those layers: intent becomes artifacts; artifacts become plans; plans route work to repositories; repositories change through controlled phases; outcomes are reviewed and recorded for audit and recovery.

## Principles

- **Keep the CLI authoritative.** Skills, MCP servers, CI, and cloud runners may orchestrate `specify`; they must not reimplement lifecycle transitions, plan validation, registry validation, workspace sync, or merge behavior.
- **Treat `registry.yaml` as a projection.** Rich catalog metadata can live in Backstage or another catalog; Specify should consume reviewable registry projections for routing, workspace sync, and execution.
- **Separate workflow, standards, and artifacts.** Workflow skills orchestrate phases; codex rules carry durable engineering policy; artifacts capture slice-local and baseline product intent.
- **Optimize for local first, cloud later.** `/spec:execute` remains the proving ground, but plan locks, journals, phase outcomes, workspace state, review results, and recovery records should be durable enough for hosted execution.
- **Prove the whole loop.** Acceptance coverage should exercise realistic multi-repo flows, not just isolated command behavior.
- **Abstract external systems at the boundary.** Forges, catalogs, agents, and hosted runners should integrate through narrow adapters.
- **Keep enforcement surfaces distinct.** Reserve separate enforcement surfaces for framework-repo tooling and consumer-project review. The planned `tooling check` validator and `specify review` reviewer share rule ids and finding shape via [RFC-28](rfc-28-codex-rules.md); [RFC-31](rfc-31-workspace-model.md) adds the shared execution substrate for consumer review, with optional framework convergence in Phase 3.

## Sequenced Roadmap

Items are ordered by intended sequencing and identified as `RM-NN`. Earlier items unblock later ones unless noted otherwise. Command examples in this roadmap are target surfaces unless the item explicitly says the command is already implemented.

---

### Near Term

#### RM-05: Multi-repo acceptance suite expansion

**Goal:** Extend the acceptance fixture to blocked, failed, interrupted, and stale-workspace recovery paths.

#### RM-07: RFC-4 Option 1: typed skill expression

**Goal:** Add deterministic structural validation for skill authoring inside `tooling check`.  
**Checks:** frontmatter schema, reference resolution, variable consistency, and cross-skill directive validation.  
**Defers:** typed YAML manifests and a Rust DSL until skill count justifies them.

---

### Mid Term

#### RM-10: CI-native `specify review`

**Goal:** Continuously review consumer projects.
**Depends:** [RFC-28](rfc-28-codex-rules.md) (contract + export) then [RFC-31](rfc-31-workspace-model.md) Phase 2 (WorkspaceModel + hint interpreter + `specify review`).
**Consumes:** RFC-28's resolved codex export and structured finding schema; RFC-31's deterministic scanner.
**Target surface:**

```bash
specify review
specify review --slice <name>
specify review --format json
```

**First task:** Define the structured finding shape before reviewer code lands. The schema consumes a project-resolved rule catalogue owned by the future review surface, owns finding-specific fields, and should not redefine codex rule storage.
**Schema includes:** severity (`critical` / `important` / `suggestion` / `optional`), rule id, file/line references, verbatim evidence, remediation, and machine-readable output for terminals, CI annotations, PR comments, and future dashboards.
**Inspects:** artifact completeness, responsibility boundaries, schema validation, plan/registry consistency, compatibility classification, stale `AGENTS.md`, codex compliance, source changes missing spec coverage, and specs missing implementation evidence.
**Output:** structured findings via the settled review schema.

**Optional follow-on:** [RFC-31](rfc-31-workspace-model.md) Phase 3 — framework `tooling check` may emit the same finding shape or migrate select predicates to declarative `FRAME-*` rules; not required for RM-10.

#### RM-11: Dependency-aware compatibility gates

**Goal:** Block producer slices from reaching `done` while breaking consumer follow-up is unaccounted for.
**Answers:** whether consumer plan entries exist, whether producer completion is allowed, and what SemVer or release impact is implied.
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

#### RM-14: Local structured workflow events

**Goal:** Measure workflow performance, failure modes, and model/tool usage without requiring hosted infrastructure.
**Events include:** command/version, project/adapter, slice or plan entry, phase start/finish, validation result, invoked skill, review findings, recovery attempts, human intervention points, and model/tool metadata when available.
**Target surface:**

```bash
specify events tail
specify events export
```

**Output:** local JSONL or configurable telemetry sink with run identity.

#### RM-15: Structured change-lifecycle status for re-entry

**Goal:** Make the `/spec:plan` → `/spec:execute` → `/spec:finalize` lifecycle's re-entry and pause points machine-readable.
**Output:** JSON status with current step, last completed step, pending human action, owner, and next valid resume point.
**Consumes:** *Local structured workflow events*.

#### RM-16: RFC-5: framework developer tooling workspace

**Goal:** Land the framework dev-tooling workspace at `augentic/specify/tooling/` per [RFC-5](done/rfc-5-tooling.md) — schema-first authoring feedback in Cursor, a single `tooling` binary with `check` and `docgen` subcommands for CI and local use, integration tests that replace `tests/cross_repo.ts`, and full Deno retirement.
**Why now:** Removes Deno from CI and contributor prerequisites, kills the parser duplication between `tests/lib/spec_provenance.ts` and `specify-domain`, shifts most violations into the editor via JSON Schema, and turns reserved framework-lint rule ids into a working scanner — without bloating the operator `specify` binary with dev-only surfaces.
**Codex follow-up:** First-party codex shape validation lives in `check::codex` and preserves RFC-28's namespace-ownership contract; project-resolved review behavior stays under future `specify review`.
**Unblocks:** *RFC-4 Option 1* and *Migrate remaining first-party host helpers to declared WASI tools*.

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
**Includes:** publishing and discovery conventions, version compatibility tests, declared-tool compatibility, migration guidance, quality gates, examples beyond Omnia/Vectis/contracts, and ownership for codex rules, artifact templates, and tool manifests.

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

- Which codex rules should RM-10 implement as deterministic scanners first, and which should stay model-assisted findings?
- What is the minimum Backstage registry projection needed for useful planning?
- What compatibility classifier is sufficient before producer changes can gate on consumer impact?
- Which acceptance fixtures best represent the product proof path?
- What is the smallest forge adapter contract for push, PR/MR handoff, CI state, and finalize?
- How should orchestration ownership and handoff work across multiple operators or agents?
- What compatibility guarantees should adapter authors provide across adapter and declared-tool versions?
- How much telemetry should emit by default, and what requires explicit opt-in?
- What approval model is required before hosted execution can push branches or open pull requests?

