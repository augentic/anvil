# Specify Roadmap

> Status: Draft
> Source: Review of Cloudflare's internal AI engineering stack, especially the platform, knowledge, and enforcement layers described in <https://blog.cloudflare.com/internal-ai-engineering-stack/>.

## Purpose

Specify is moving toward a highly opinionated, spec-driven workflow framework for agentic software delivery. The existing direction is sound: deterministic CLI operations, durable artifacts, explicit lifecycle state, registry-aware planning, workspace execution, and specialist skills are the right foundations.

This roadmap captures the next strategic corrections and extensions. The goal is not to turn Specify into a general developer portal, AI gateway, or CI system. The goal is to make Specify the workflow control plane that can use those systems while preserving local, reviewable, deterministic execution.

## Product Thesis

AI engineering at scale needs three connected layers:

1. **Platform layer.** Authenticated access to models, tools, sandboxes, logs, and long-running execution.
2. **Knowledge layer.** Explicit context about repositories, owners, dependencies, standards, capabilities, and current plans.
3. **Enforcement layer.** Continuous review, compatibility checks, standards checks, and stale-context detection.

Specify should own the spec-driven workflow semantics across those layers:

- intent becomes artifacts;
- artifacts become executable plans;
- plans route work to repositories;
- repositories are changed through controlled phases;
- changes are reviewed against capabilities, contracts, and standards;
- outcomes are recorded for recovery and audit.

## Directional Principles

### Keep The CLI Authoritative

The `specify` CLI should remain the source of deterministic behavior. Skills, MCP servers, CI integrations, and cloud runners may orchestrate the CLI, but they should not reimplement lifecycle transitions, plan validation, registry validation, workspace sync, or merge behavior.

This keeps every integration honest: if a behavior matters, it belongs in one deterministic command surface.

### Treat The Registry As A Projection

`registry.yaml` should remain a compact execution snapshot, not grow into a full developer catalog. Catalog systems such as Backstage are better suited to long-lived organizational knowledge: owners, teams, systems, domains, APIs, databases, dependencies, and documentation.

Specify should consume that knowledge through importers and projections:

```text
Backstage or another catalog
  -> Specify registry projection
  -> plan routing, workspace sync, execute loop
```

The registry should stay local, reviewable, and reproducible. Rich catalog metadata can remain upstream.

### Separate Workflow, Standards, And Artifacts

Specify should make a clear distinction between:

- **Workflow skills**: phase orchestration and specialist generation behavior.
- **Standards**: durable engineering rules with stable identifiers.
- **Artifacts**: slice-local and baseline material produced by the workflow.

This avoids overloading `SKILL.md` with general policy, and gives reviewers and generators a shared rule vocabulary.

### Optimize For Background Execution Later

The local `/change:execute --loop` path should remain first-class, but the primitives should be portable to cloud execution: plan locks, journals, phase outcomes, workspace state, review results, and recovery records should all be serializable and durable.

The long-term shape is:

```text
local operator-driven execute loop
cloud background execute loop
```

The same CLI and artifacts should support both.

## Roadmap

### 1. Foundation: Skill And Context Hygiene

**Goal:** Make agent behavior easier to select, cheaper to load, and less dependent on inference.

Current RFC-10 work should remain the immediate priority:

- finish skill frontmatter cleanup;
- keep skill names globally discoverable;
- keep skill bodies under the progressive-disclosure ceiling;
- factor duplicated phase outcome, journal, and plan-mutation instructions into shared references;
- preserve stable Specify artifact identifiers while improving skill discoverability.

Next, add a first-class repository context output:

- generate concise `AGENTS.md` files from Specify project metadata, capability references, repo inspection, and registry data;
- include runtime, test command, lint command, navigation hints, conventions, boundaries, and dependencies;
- keep the file short enough to sit directly in agent context;
- add checks that warn when repo structure changes imply `AGENTS.md` should be refreshed.

Candidate surfaces:

```bash
specify context generate
specify context check
```

Open question: whether this belongs under `specify context`, `specify project`, or a new plugin skill such as `/spec:context`.

### 2. Catalog Integration Without Catalog Ownership

**Goal:** Let external catalogs enrich Specify planning without making Specify a developer portal.

Add registry import and validation adapters:

```bash
specify registry import backstage
specify registry import <source>
specify registry diff <source>
```

The first supported adapter should map Backstage catalog entities into `registry.yaml`:

- Backstage `System` -> platform or product boundary;
- Backstage `Component` -> Specify registry project;
- Backstage `API` -> interface contract inventory;
- ownership and domain data -> project descriptions and routing signals;
- dependency relations -> future plan and review signals.

The output should be an explicit file diff, not an implicit remote dependency. Operators should be able to review the projected registry before planning or execution.

Non-goal: replacing `registry.yaml`, `.specify/project.yaml`, `plan.yaml`, or workspace state with live Backstage lookups.

### 3. Standards As A First-Class Codex

**Goal:** Give generators and reviewers stable, citeable engineering rules.

Introduce a markdown-first Specify codex format:

```text
codex/
  rust/errors.md
  interfaces/compatibility.md
  security/secrets.md
```

Each rule should have:

- a stable rule id;
- a concise trigger;
- normative guidance;
- examples or references where useful;
- applicability metadata for capabilities, plugins, or languages.

Skills should be able to cite codex rules while generating artifacts. Reviewers should cite the same rule ids when reporting violations.

This should complement, not replace, artifact schemas. Artifact schemas define structure. Codex rules define durable engineering policy.

### 4. CI-Native Specify Review

**Goal:** Move from workflow correctness to continuous enforcement.

Add a review mode that can run locally or in CI:

```bash
specify review
specify review --slice <name>
specify review --format json
```

The reviewer should inspect:

- artifact completeness and responsibility boundaries;
- schema validation results;
- plan and registry consistency;
- cross-project contract compatibility;
- stale `AGENTS.md` or stale project context;
- codex rule compliance;
- source changes whose behavior is not reflected in specs;
- specs whose expected implementation appears absent.

Review output should be structured by severity:

- critical;
- important;
- suggestion;
- optional.

Findings should include file references, rule ids where applicable, and clear remediation guidance. The same output shape should support terminal display, CI annotations, and pull request comments.

### 5. Specify MCP Surface

**Goal:** Make Specify available to agents through tools without duplicating business logic.

Expose a thin MCP server over CLI-backed operations:

- `specify_status`;
- `specify_registry_show`;
- `specify_workspace_status`;
- `specify_change_plan_status`;
- `specify_change_plan_next`;
- `specify_change_plan_doctor`;
- `specify_slice_validate`;
- `specify_slice_outcome_show`.

The MCP server should be mostly read-oriented at first. Mutating tools can come later, but only as wrappers around existing CLI verbs with the same validation and failure semantics.

Non-goal: placing independent plan, registry, or lifecycle logic in the MCP server.

### 6. Observability For Agentic Work

**Goal:** Make workflow performance, failure modes, and model/tool usage measurable.

Add structured event emission for major workflow operations:

- command name and version;
- project and capability;
- slice or plan entry;
- phase start and finish;
- validation result;
- skill invoked;
- review findings;
- recovery attempts;
- human intervention points;
- model and tool metadata when available.

This should begin as local JSONL output or a configurable telemetry sink. The design should avoid requiring a hosted service, but should make hosted dashboards possible later.

Candidate surfaces:

```bash
specify status --format json
specify events tail
specify events export
```

### 7. Cloud-Hosted Execution

**Goal:** Allow durable background execution of Specify plans while preserving the local workflow contract.

The current primitives already point in this direction: plan locks, workspace clones, phase outcomes, journals, and explicit workspace push. Cloud execution should reuse those primitives rather than introduce a parallel workflow.

Requirements:

- sandboxed workspace clones;
- durable plan lock ownership;
- resumable agent sessions;
- serialized phase outcomes and journals;
- explicit human approval gates;
- controlled push and PR/MR creation;
- deterministic recovery after interruption;
- parity with local `/change:execute --loop`.

Candidate surface:

```bash
specify execute submit
specify execute status <run-id>
specify execute resume <run-id>
```

This should remain a long-term track. Local execution is the proving ground.

## Phasing

### Near Term

- Complete RFC-10.
- Add concise `AGENTS.md` generation and checking.
- Define the codex rule format.
- Keep the Backstage/catalog decision to adapter design, not core registry replacement.
- Add initial structured review output for Specify artifacts.

### Mid Term

- Add `specify registry import` with a Backstage adapter.
- Add CI-native `specify review`.
- Add a read-oriented Specify MCP server.
- Add local structured workflow events.
- Expand cross-repo contract and dependency checks using registry/catalog projections.

### Long Term

- Add cloud-hosted `/change:execute --loop` equivalents.
- Support durable background agents with sandboxed workspace clones.
- Add first-class PR/MR creation and review loops.
- Support catalog-backed initiatives across many repositories.
- Build toward a full spec-driven engineering control plane: define, plan, execute, review, enforce, observe.

## Non-Goals

- Do not make Specify a general developer portal.
- Do not replace catalog systems such as Backstage.
- Do not put lifecycle authority in skills, MCP servers, or hosted services.
- Do not require hosted infrastructure for the core workflow.
- Do not make `AGENTS.md` a dumping ground for long-form documentation.
- Do not blur stable artifact schemas with mutable engineering standards.

## Open Questions

- Should repo context generation live under `specify context`, `specify project`, or a plugin skill?
- Should codex rules live inside `.specify/`, at the repository root, or in a shared catalog?
- Which parts of `specify review` should be deterministic CLI checks versus model-assisted analysis?
- What is the minimum registry projection needed from Backstage for useful multi-repo planning?
- How much telemetry should be emitted by default, and what should require explicit opt-in?
- What approval model is required before cloud-hosted execution can push or open pull requests?
