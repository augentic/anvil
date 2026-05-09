# Specify Roadmap

> Status: Draft
> Source: Review of Cloudflare's internal AI engineering stack, especially the platform, knowledge, and enforcement layers described in [https://blog.cloudflare.com/internal-ai-engineering-stack/](https://blog.cloudflare.com/internal-ai-engineering-stack/).

## Thesis

Specify should be the spec-driven workflow control plane for agentic software delivery. It should use developer portals, model gateways, CI, forges, and hosted runners without becoming any of them.

The local substrate is now credible: slice/change vocabulary, registry-aware planning, workspace execution, branch preparation, push/finalize handoff, declared tools, and layered skills have landed across RFCs 10, 13, 15, and 16. The next phase should make that substrate provable end-to-end, enforceable, observable, and portable across teams, forges, agents, and catalogs.

At scale, Specify spans three connected layers:

1. **Platform:** models, tools, sandboxes, logs, and long-running execution.
2. **Knowledge:** repositories, owners, dependencies, standards, capabilities, and plans.
3. **Enforcement:** review, compatibility checks, standards checks, and stale-context detection.

Specify owns the workflow semantics across those layers: intent becomes artifacts; artifacts become plans; plans route work to repositories; repositories change through controlled phases; outcomes are reviewed and recorded for audit and recovery.

## Principles

- **Keep the CLI authoritative.** Skills, MCP servers, CI, and cloud runners may orchestrate `specify`; they must not reimplement lifecycle transitions, plan validation, registry validation, workspace sync, or merge behavior.
- **Treat `registry.yaml` as a projection.** Rich catalog metadata can live in Backstage or another catalog; Specify should consume reviewable registry projections for routing, workspace sync, and execution.
- **Separate workflow, standards, and artifacts.** Workflow skills orchestrate phases; codex rules carry durable engineering policy; artifacts capture slice-local and baseline product intent.
- **Optimize for local first, cloud later.** `/change:execute loop` remains the proving ground, but plan locks, journals, phase outcomes, workspace state, review results, and recovery records should be durable enough for hosted execution.
- **Prove the whole loop.** Acceptance coverage should exercise realistic multi-repo flows, not just isolated command behavior.
- **Abstract external systems at the boundary.** Forges, catalogs, agents, and hosted runners should integrate through narrow adapters.
- **Keep enforcement surfaces distinct.** Reserve separate enforcement surfaces for framework-repo linting and consumer-project review. The planned `specify check` linter and `specify review` reviewer may share rule ids and finding shape, but not scanner lifecycle or inputs.

## Sequenced Roadmap

Items are ordered by intended sequencing and identified as `RM-NN`. Earlier items unblock later ones unless noted otherwise. Command examples in this roadmap are target surfaces unless the item explicitly says the command is already implemented.

---

### Near Term

#### RM-04: `specify review` finding schema

**Goal:** Define the structured finding shape before reviewer code lands.
**Depends on:** RM-03 codex rule format.
**Consumes:** `specify codex export --format json` as the resolved rule-catalog input. RM-04 owns finding-specific fields and should not redefine codex rule storage.
**Schema includes:** severity (`critical` / `important` / `suggestion` / `optional`), rule id, file/line references, verbatim evidence, remediation, and machine-readable output for terminals, CI annotations, PR comments, and future dashboards.

#### RM-05: Cross-project compatibility classification

**Goal:** Reconcile the retired consumer-compatibility vocabulary with the current contract validation gate, then expose a classified compatibility report.
**Seed:** `plugins/contract/references/cross-project-compatibility.md` and its `change-kind` vocabulary.
**Classification:** `additive`, `breaking`, `ambiguous`, or `unverifiable`.
**Target surface:**

```bash
specify compatibility check
specify compatibility report --change <name>
```

**Scope:** Contract-first and dependency-aware. This does not replace the current `specify tool run contract` baseline validation gate; it adds a consumer-impact classifier after the stale cross-project reference docs are reconciled. Change-level gates land later.

#### RM-06: RFC-13 rename-tail cleanup

**Goal:** Remove transition shims before they become load-bearing.
**Output:** a release that deletes the `specify migrate slice-layout` and `specify migrate change-noun` CLI shims from `specify-cli`, and deletes the `/spec:plan` / `/spec:execute` deprecation shims from `plugins/spec/skills/`.

#### RM-07: RFC-5: `specify check` framework linter port

**Goal:** Port `scripts/checks.ts` from Deno into a Rust `specify-check` crate exposed as `specify check`.
**Why now:** Removes Deno from CI, reuses CLI schema parsers, and turns reserved framework-lint rule ids into a working scanner.
**Codex follow-up:** May surface first-party codex shape validation by delegating to the codex parser/resolver, while leaving project-resolved review behavior under `specify codex` and future `specify review`.
**Unblocks:** *RFC-4 Option 1* and *Migrate remaining first-party host helpers to declared WASI tools*.

#### RM-08: RFC-4 Option 1: typed skill expression

**Goal:** Add deterministic structural validation for skill authoring inside `specify check`.
**Checks:** frontmatter schema, reference resolution, variable consistency, and cross-skill directive validation.
**Defers:** typed YAML manifests and a Rust DSL until skill count justifies them.

#### RM-09: Skill-hygiene refactors

**Goal:** Compress always-loaded surface area and remove duplicated skill prose.
**Scope:** factor repeated phase-outcome, journal, and plan-mutation instructions into shared references while preserving stable Specify artifact identifiers.

#### RM-10: Migrate remaining first-party host helpers to declared WASI tools

**Goal:** Move remaining first-party host helpers behind `specify tool run` where the cost/benefit is favorable.
**Depends on:** the `specify check` port, which can enforce `skill.invokes-host-binary-with-declared-tool-equivalent`.

---

### Mid Term

#### RM-11: CI-native `specify review`

**Goal:** Continuously review consumer projects.
**Consumes:** RM-03 resolved codex export plus the RM-04 finding schema.
**Target surface:**

```bash
specify review
specify review --slice <name>
specify review --format json
```

**Inspects:** artifact completeness, responsibility boundaries, schema validation, plan/registry consistency, compatibility classification, stale `AGENTS.md`, codex compliance, source changes missing spec coverage, and specs missing implementation evidence.
**Output:** structured findings via the settled review schema.

#### RM-12: Dependency-aware compatibility gates

**Goal:** Block producer slices from reaching `done` while breaking consumer follow-up is unaccounted for.
**Answers:** whether consumer plan entries exist, whether producer completion is allowed, and what SemVer or release impact is implied.
**Target surface:**

```bash
specify change plan impact --change <name>
```

#### RM-13: Catalog import: Backstage adapter

**Goal:** Enrich Specify planning from external catalogs without making Specify a developer portal.
**Target surface:**

```bash
specify registry import backstage
specify registry import <source>
specify registry diff <source>
```

**Mapping:** Backstage `System` to platform/product boundary; `Component` to registry project; `API` to interface inventory; ownership/domain/dependencies to routing and review signals.
**Output:** explicit registry diff for operator review before planning or execution.

#### RM-14: Multi-repo acceptance suite expansion

**Goal:** Extend the acceptance fixture to blocked, failed, interrupted, and stale-workspace recovery paths.

#### RM-15: Read-oriented Specify MCP server

**Goal:** Make Specify state available to agents through MCP without duplicating business logic.
**Initial tools:** `specify_status`, `specify_registry_show`, `specify_workspace_status`, `specify_change_plan_status`, `specify_change_plan_next`, `specify_change_plan_doctor`, `specify_slice_validate`, `specify_slice_outcome_show`.
**Boundary:** mutating tools may come later only as wrappers around existing CLI verbs.

#### RM-16: Local structured workflow events

**Goal:** Measure workflow performance, failure modes, and model/tool usage without requiring hosted infrastructure.
**Events include:** command/version, project/capability, slice or plan entry, phase start/finish, validation result, invoked skill, review findings, recovery attempts, human intervention points, and model/tool metadata when available.
**Target surface:**

```bash
specify status --format json
specify events tail
specify events export
```

**Output:** local JSONL or configurable telemetry sink with run identity.

#### RM-17: Forge abstraction behind workspace push and change finalize

**Goal:** Support branch transport, PR/MR creation, and finalize beyond GitHub CLI.
**Adapter covers:** remote discovery, auth checks, branch existence, push permissions, PR/MR create-or-update, CI/mergeability status, merged-state verification, and provider links.
**Target surface:**

```bash
specify forge doctor
specify workspace push --forge github
specify change finalize --forge github
```

#### RM-18: Structured orchestration status for re-entry

**Goal:** Make `/change:plan <name> orchestrate` re-entry and pause points machine-readable.
**Output:** JSON status with current step, last completed step, pending human action, owner, and next valid resume point.
**Consumes:** *Local structured workflow events*.

---

### Long Term

#### RM-19: Cloud-hosted execute loop

**Goal:** Run Specify plans durably in the background while preserving local workflow semantics.
**Requires:** sandboxed workspace clones, durable lock ownership, resumable agent sessions, serialized phase outcomes and journals, human approval gates, controlled push/PR creation, deterministic recovery, and parity with `/change:execute loop`.
**Target surface:**

```bash
specify execute submit
specify execute status <run-id>
specify execute resume <run-id>
```

#### RM-20: Multi-forge adapter coverage

**Goal:** Extend the forge abstraction to GitHub, GitLab, Bitbucket, and self-hosted forges.

#### RM-21: Catalog-backed initiatives across many repositories

**Goal:** Drive multi-repo initiatives from live catalog-backed registry projections.

#### RM-22: Capability ecosystem operating model

**Goal:** Make capabilities feel like a dependable ecosystem rather than bespoke first-party packages.
**Includes:** publishing and discovery conventions, version compatibility tests, declared-tool compatibility, migration guidance, quality gates, examples beyond Omnia/Vectis/contracts, and ownership for codex rules, artifact templates, and tool manifests.

#### RM-23: Hosted observability dashboards

---

### Completed

**RM-01:** Multi-repo acceptance fixture

**Goal:** Prove a realistic multi-slice, multi-repo flow.
**Covers:** plan generation, registry routing, dependent slice execution, branch preparation, workspace sync, residue and baseline commit behavior, push and PR/MR handoff, and finalize after external merge.
**Output:** an automated or semi-automated suite against local fixture repositories with fake or recorded forge behavior. Recovery paths land in *Multi-repo acceptance suite expansion*.
**Executable proof:** `specify-cli` repo's `tests/cross_repo.rs`.

#### RM-02: `AGENTS.md` generation under proposed `specify context`

**Goal:** Generate concise, deterministic, refreshable repository context.
**Target surface:**

```bash
specify context generate
specify context check
```

**Inputs:** Specify project metadata, capability references, repo inspection, and registry data.  
**Output:** short `AGENTS.md` guidance covering runtime, tests, linting, navigation, conventions, boundaries, and dependencies. The proposed `specify context check` warns when repo changes imply a refresh.  
**Why now:** High direct user value, and it unblocks stale-context checks in `specify review`.

#### RM-03: Codex rule format

**Goal:** Give generators and reviewers stable, citable engineering rules.
**Seed:** `plugins/references/review-checks.md` and its existing `UNI-`* catalogue.
**Each rule carries:** stable id, concise trigger, normative guidance, examples or references where useful, and applicability metadata.
**Implemented:** Markdown/YAML-frontmatter V1 schema; `specify codex {list, show, validate, export --format json}`; deterministic project resolution; default `UNI-001` through `UNI-021` migration; first-cut Omnia, Contracts, and Vectis codex packs; first-party codex validation in `make checks`; reviewer citation guidance for stable `rule_id` values.
**Storage answer:** First-party rules live at `capabilities/<name>/codex/`. The foundational `default` capability carries universal rules and is cached alongside selected first-party capabilities during `specify init`. Repo-root `codex/` is the local overlay. `.specify/codex/` remains reserved for generated cache or lock state. Shared catalog resolution is a reserved V1 hook with no config surface yet.
**Follow-ups:** RM-04 defines the review finding schema that consumes codex rule IDs and provenance. RM-11 implements CI-native review against the resolved codex. RM-07 may port first-party codex shape validation into `specify check`.

---

**Goal:** Build hosted dashboards on top of local structured workflow events without making local workflows depend on hosted infrastructure.

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

- Which codex rules should RM-11 implement as deterministic scanners first, and which should stay model-assisted findings?
- What is the minimum Backstage registry projection needed for useful planning?
- What compatibility classifier is sufficient before producer changes can gate on consumer impact?
- Which acceptance fixtures best represent the product proof path?
- What is the smallest forge adapter contract for push, PR/MR handoff, CI state, and finalize?
- How should orchestration ownership and handoff work across multiple operators or agents?
- What compatibility guarantees should capability authors provide across capability and declared-tool versions?
- How much telemetry should emit by default, and what requires explicit opt-in?
- What approval model is required before hosted execution can push branches or open pull requests?

