# Specify Roadmap

> Status: Draft
> Source: Review of Cloudflare's internal AI engineering stack, especially the platform, knowledge, and enforcement layers described in <https://blog.cloudflare.com/internal-ai-engineering-stack/>.

## Purpose

Specify is moving toward a highly opinionated, spec-driven workflow framework for agentic software delivery. The existing direction is sound: deterministic CLI operations, durable artifacts, explicit lifecycle state, registry-aware planning, workspace execution, and specialist skills are the right foundations.

This roadmap captures the next strategic corrections and extensions. The goal is not to turn Specify into a general developer portal, AI gateway, or CI system. The goal is to make Specify the workflow control plane that can use those systems while preserving local, reviewable, deterministic execution.

Recent multi-repo review confirms that the core local substrate is now credible: slice and change vocabulary, registry-aware planning, workspace execution, branch preparation, push/finalize handoff, declared tools, and layered skills are in place. The next phase should make that substrate enforceable, observable, provable end-to-end, and portable across teams, forges, agents, and catalogs.

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

The local `/change:execute loop` path should remain first-class, but the primitives should be portable to cloud execution: plan locks, journals, phase outcomes, workspace state, review results, and recovery records should all be serializable and durable.

The long-term shape is:

```text
local operator-driven execute loop
cloud background execute loop
```

The same CLI and artifacts should support both.

### Prove The Whole Multi-Repo Loop

The framework should be judged by realistic end-to-end runs, not just individual command correctness. Acceptance coverage should exercise plan authoring, per-project execution, branch preparation, residue and baseline commits, workspace push, PR/MR handoff, finalize, recovery, and failure paths across more than one repository.

### Abstract External Systems At The Boundary

Specify should integrate with forges, catalogs, agents, and hosted runners through narrow adapters. GitHub, Backstage, Cursor, and local execution are good first adapters, but the durable product contract should be forge-neutral, catalog-neutral, and agent-neutral.

### Two Enforcement Surfaces, Distinct By Construction

Specify carries two scanners with shared vocabulary but separate inputs and lifecycles:

- **`specify check`** — framework-repo integrity. Runs in CI on this repo: skill frontmatter, marketplace alignment, capability briefs, declared-tool manifests, docs inventory.
- **`specify review`** — consumer-project review. Runs against a downstream project's slices, plans, contracts, and codex compliance.

They share rule-id vocabulary and finding shape; never the same scanner. Settling the names up front prevents the consumer reviewer from colliding with the framework linter port.

## Deliverables

A single ordered backlog. Items appear top-to-bottom in intended execution order. Each carries a phase label (`landed` / `near term` / `mid term` / `long term`), and within a phase the order reflects sequencing intent — earlier items unblock later ones unless flagged otherwise. The phase label is the source of truth for "what is shipped, ship next, ship later"; document position alone is not.

### RFC-10: skill body ceiling and plugin namespace `[landed]`

**What:** Plugin namespaces normalised; skill bodies capped at the progressive-disclosure ceiling.

### RFC-13: capability rename and slice/change vocabulary `[landed]`

**What:** "Schema" renamed to "capability"; per-loop *slices* split from umbrella *changes*; `/spec:plan` and `/spec:execute` moved to the `change` plugin; registry and change orchestration reframed as platform components rather than capabilities.
**Note:** Migration shims (`specify migrate slice-layout`, `specify migrate change-noun`, the `/spec:plan` and `/spec:execute` deprecation skills) are still in place — see *RFC-13 rename-tail cleanup* below.

### RFC-15: declared WASI capability tools `[landed]`

**What:** `specify tool` runner with `tools.yaml` sidecars; deterministic helpers run with explicit permissions and SHA-256 pins instead of as bundled native code; the contract validator is the first declared tool.

### RFC-16: Vectis WASI tools `[landed]`

**What:** `vectis-validate` and `vectis-scaffold` declared WASI components; `specify-vectis` host binary retired. Operators install one binary (`specify`).

### Multi-repo acceptance fixture `[near term]`

**Goal:** Prove the framework across a realistic multi-slice, multi-repo flow.
**Rationale:** The framework should be judged on whole-loop correctness, not individual command correctness. Without an end-to-end fixture, every other reviewer / codex / compatibility deliverable has no proof path. Highest-leverage near-term deliverable.
**Scope:** plan generation from a change brief and source material; registry routing across multiple projects; execution through several dependent slices; branch preparation and workspace sync; residue and baseline commit behavior; push and PR/MR handoff; finalize after external merge.
**Output:** an automated or semi-automated suite that runs against local fixture repositories with fake or recorded forge behavior, and that becomes the product proof path. Recovery and failure paths land later in *Multi-repo acceptance suite expansion*.

### `AGENTS.md` generation under `specify context` `[near term]`

**Goal:** Give every Specify project a first-class repository context output that is concise, deterministic, and refreshable.
**Rationale:** Smallest-scope end-to-end deliverable after the acceptance fixture, with the most direct user value, and it unblocks the stale-context check in *CI-native `specify review`*.
**Surface:**

```bash
specify context generate
specify context check
```

`specify context` is the durable home — every other artifact noun in the post-RFC-13 CLI lives at `specify <noun> <action>` (`registry`, `workspace`, `slice`, `change`, `capability`), and `AGENTS.md` is a first-party Specify artifact derived from those nouns. A plugin skill (`/spec:context`) can wrap the CLI later if useful, but the deterministic generator belongs in the CLI.
**Inputs:** Specify project metadata, capability references, repo inspection, and registry data.
**Output:** concise `AGENTS.md` covering runtime, test command, lint command, navigation hints, conventions, boundaries, and dependencies — short enough to sit directly in agent context. `specify context check` warns when repo structure changes imply the file should be refreshed.
**Non-goal:** Do not make `AGENTS.md` a dumping ground for long-form documentation.

### Codex rule format `[near term]`

**Goal:** Give generators and reviewers stable, citeable engineering rules.
**Rationale:** Must precede any reviewer code — without stable rule ids, review output cannot be cited or suppressed safely. `plugins/references/review-checks.md` is already the de facto codex (the `UNI-*` rule catalogue every reviewer skill cites today, with severity, "what to look for" prose, and spec-change indicators); formalising the format gives skills and reviewers a shared rule vocabulary.
**Layout:**

```text
codex/
  rust/errors.md
  interfaces/compatibility.md
  security/secrets.md
```

**Each rule carries:** stable rule id, concise trigger, normative guidance, examples or references where useful, applicability metadata (capability, plugin, language).
**Scope of the first deliverable:**

- formalise the rule-id namespace (the existing `UNI-*` ids are the seed) and reserve prefixes for new tracks (e.g. `RUST-*`, `IFACE-*`, `SEC-*`);
- add applicability metadata so skills and reviewers can filter rules;
- decide the storage location — `.specify/codex/`, repo-root `codex/`, or a shared catalog (open question);
- migrate `plugins/references/review-checks.md` into the chosen location without losing rule-id stability.

**Boundary:** Codex rules are durable engineering policy; artifact schemas define structure. Codex complements, not replaces, schemas.

### `specify review` finding schema `[near term]`

**Goal:** Define the structured finding shape `specify review` will emit before any reviewer code lands.
**Rationale:** Depends on *Codex rule format*. Settling the schema early lets the same finding shape be reused by `specify check` and any future hosted dashboards.
**Schema includes:** severity (`critical` / `important` / `suggestion` / `optional`); stable rule id; file and line references; verbatim evidence; remediation guidance; machine-readable output suitable for terminal display, CI annotations, and pull request comments.

### Cross-project compatibility classification `[near term]`

**Goal:** Move cross-project contract warnings from non-fatal discovery into a classified compatibility report.
**Rationale:** The vocabulary already exists in `plugins/contract/references/cross-project-compatibility.md` — the `change-kind` enumeration (`removed-field`, `required-field-added`, `type-narrowed`, `enum-value-removed`, `additional-properties-tightened`, `removed-endpoint`, `status-code-removed`, …) is the seed dictionary. This deliverable layers a deterministic classification on top: each `change-kind` maps to one of `additive` / `breaking` / `ambiguous` / `unverifiable`. Existing warning emitters keep working unchanged; the classifier is additive. Plan-level enforcement (gating producer slices on consumer follow-up entries) lands later as *Dependency-aware compatibility gates*.
**Surface:**

```bash
specify compatibility check
specify compatibility report --change <name>
```

**Initial scope:** contract-first, but the model is dependency-aware so it can extend beyond contracts later.
**Non-goal:** Do not treat cross-repo compatibility warnings as sufficient enforcement for breaking changes (that gate lands in *Dependency-aware compatibility gates*).

### RFC-13 rename-tail cleanup `[near term]`

**Goal:** Delete the RFC-13 transition shims before they become load-bearing.
**Rationale:** `specify migrate slice-layout`, `specify migrate change-noun`, and the `/spec:plan` / `/spec:execute` deprecation shims are still in the surface area. Pick a release in which they are removed.
**Output:** a release in which the migration commands and deprecation shims are deleted from `specify-cli` and from `plugins/spec/skills/`.

### RFC-5: `specify check` framework linter port `[near term]`

**Goal:** Port `scripts/checks.ts` (~1500 lines, Deno) into a Rust `specify-check` crate exposed via `specify check`, retire the Deno linter from `make checks`.
**Rationale:** Removes the Deno toolchain from CI; lets the linter share `specify-schema`'s parsers; lifts `crates/validate/src/rfc5.rs` from rule-id reservations (`tool.write-permission-too-broad`, `tool.lifecycle-state-write-denied`, `skill.invokes-host-binary-with-declared-tool-equivalent`) to a working scanner.
**Boundary:** `specify check` is the framework-repo linter (see *Two Enforcement Surfaces, Distinct By Construction*). It is not the same scanner as `specify review`.
**Unblocks:** *RFC-4 Option 1*, *Migrate remaining first-party host helpers to declared WASI tools*.

### RFC-4 Option 1: typed skill expression `[near term]`

**Goal:** Add deterministic structural validation for skill authoring inside the framework linter.
**Rationale:** Frontmatter schema enforcement, reference resolution, variable consistency, and cross-skill directive validation are all mechanical checks that today produce no feedback until runtime.
**Depends on:** *RFC-5 framework linter port*. Lands as additional rules inside the new `specify check` scanner.
**Defers:** Options 2 and 3 (typed YAML manifests, Rust DSL) until skill count makes the lift worthwhile.

### Skill-hygiene refactors `[near term]`

**Goal:** Compress always-loaded surface area and remove duplicated skill prose.
**Scope:**

- factor duplicated phase-outcome, journal, and plan-mutation instructions into shared references (`plugins/spec/references/` and `plugins/change/skills/execute/` are the right home; today the same prose recurs across multiple skill bodies);
- preserve stable Specify artifact identifiers while improving skill discoverability.

### Migrate remaining first-party host helpers to declared WASI tools `[near term]`

**Goal:** Move any remaining first-party host helpers behind `specify tool run` where the cost/benefit is favourable.
**Depends on:** *RFC-5 framework linter port* — the `skill.invokes-host-binary-with-declared-tool-equivalent` lint reserved by RFC-15 enforces this once the linter has enough context.

### CI-native `specify review` `[mid term]`

**Goal:** Move from workflow correctness to continuous enforcement against a consumer project.
**Boundary:** See *Two Enforcement Surfaces, Distinct By Construction* — `specify review` is the consumer-project scanner, separate from `specify check`.
**Surface:**

```bash
specify review
specify review --slice <name>
specify review --format json
```

**Inspects:** artifact completeness and responsibility boundaries; schema validation results; plan and registry consistency; cross-project contract compatibility (consumes *Cross-project compatibility classification*); stale `AGENTS.md` or stale project context (consumes *`AGENTS.md` generation under `specify context`*); codex rule compliance (consumes *Codex rule format*); source changes whose behavior is not reflected in specs; specs whose expected implementation appears absent.
**Output:** structured findings via the schema settled in *`specify review` finding schema*, suitable for terminal display, CI annotations, and PR comments.

### Dependency-aware compatibility gates `[mid term]`

**Goal:** Wire *Cross-project compatibility classification* into change-level enforcement so producer slices cannot be marked `done` while breaking consumer follow-up is unaccounted for.
**Outputs answer:**

- whether consumer update plan entries already exist;
- whether a producer slice can be marked done without follow-up work;
- what SemVer or release impact is implied where versioned artifacts exist.

**Surface:**

```bash
specify change plan impact --change <name>
```

### Catalog import: Backstage adapter `[mid term]`

**Goal:** Let external catalogs enrich Specify planning without making Specify a developer portal.
**Surface:**

```bash
specify registry import backstage
specify registry import <source>
specify registry diff <source>
```

**First-supported mapping (Backstage → registry):**

- `System` → platform or product boundary;
- `Component` → Specify registry project;
- `API` → interface contract inventory;
- ownership and domain data → project descriptions and routing signals;
- dependency relations → future plan and review signals.

**Output shape:** explicit file diff, not implicit remote dependency. Operators review the projected registry before planning or execution.
**Non-goal:** Do not replace `registry.yaml`, `.specify/project.yaml`, `plan.yaml`, or workspace state with live Backstage lookups (see *Treat The Registry As A Projection*).

### Multi-repo acceptance suite expansion `[mid term]`

**Goal:** Extend the *Multi-repo acceptance fixture* to cover blocked, failed, interrupted, and stale-workspace recovery paths.
**Rationale:** Acceptance becomes the product proof path only when failure modes are exercised, not just the happy path.

### Read-oriented Specify MCP server `[mid term]`

**Goal:** Make Specify available to agents through MCP without duplicating business logic.
**Initial tool surface (all read):**

- `specify_status`;
- `specify_registry_show`;
- `specify_workspace_status`;
- `specify_change_plan_status`;
- `specify_change_plan_next`;
- `specify_change_plan_doctor`;
- `specify_slice_validate`;
- `specify_slice_outcome_show`.

**Boundary:** Mutating tools later, only as wrappers around existing CLI verbs with the same validation and failure semantics.
**Non-goal:** Do not place independent plan, registry, or lifecycle logic in the MCP server.

### Local structured workflow events `[mid term]`

**Goal:** Make workflow performance, failure modes, and model/tool usage measurable without requiring a hosted service.
**Events emit:** command name and version; project and capability; slice or plan entry; phase start and finish; validation result; skill invoked; review findings; recovery attempts; human intervention points; model and tool metadata when available.
**Surface:**

```bash
specify status --format json
specify events tail
specify events export
```

**Output:** local JSONL or a configurable telemetry sink. Events carry a run identity so local, CI, and (later) hosted execution can be compared.

### Forge abstraction behind workspace push and change finalize `[mid term]`

**Goal:** Make branch transport, PR/MR creation, and finalize work beyond GitHub CLI.
**Adapter contract covers:** remote repository discovery and authentication checks; branch existence and push permissions; PR/MR create-or-update; CI and mergeability status; merged-state verification during finalize; provider-specific links and annotations.
**Surface:**

```bash
specify forge doctor
specify workspace push --forge github
specify change finalize --forge github
```

**Non-goal:** Specify does not merge PRs or replace forge policy. It prepares, publishes, observes, and verifies the handoff.

### Structured orchestration status for re-entry `[mid term]`

**Goal:** Make `/change:plan <name> orchestrate` re-entry and pause points machine-readable.
**Output:** a JSON status payload covering the current step, last completed step, pending human action, owning operator or agent, and the next valid resume point.
**Consumes:** *Local structured workflow events*.

### Cloud-hosted execute loop `[long term]`

**Goal:** Allow durable background execution of Specify plans while preserving the local workflow contract.
**Rationale:** Existing primitives (plan locks, workspace clones, phase outcomes, journals, explicit workspace push) already point in this direction; cloud execution should reuse them rather than introduce a parallel workflow.
**Requirements:** sandboxed workspace clones; durable plan lock ownership; resumable agent sessions; serialized phase outcomes and journals; explicit human approval gates; controlled push and PR/MR creation; deterministic recovery after interruption; parity with local `/change:execute loop`.
**Surface:**

```bash
specify execute submit
specify execute status <run-id>
specify execute resume <run-id>
```

**Stay-the-line:** Local execution remains the proving ground.

### Multi-forge adapter coverage `[long term]`

**Goal:** Extend the *Forge abstraction* to GitHub, GitLab, Bitbucket, and self-hosted forges through adapters.

### Catalog-backed initiatives across many repositories `[long term]`

**Goal:** Drive multi-repo initiatives where the registry projection is sourced from a live catalog (extends *Catalog import: Backstage adapter*).

### Capability ecosystem operating model `[long term]`

**Goal:** Make capabilities feel like a dependable ecosystem rather than bespoke first-party packages.
**Includes:** capability publishing and discovery conventions; compatibility testing for capability versions and declared tools; migration guidance when capability briefs or artifacts evolve; quality gates for first-party and third-party capabilities; examples beyond Omnia, Vectis, and contracts; clear ownership of codex rules, artifact templates, and tool manifests.
**Posture:** Avoid a heavy marketplace requirement. The near-term need is a reviewable way to know whether a capability is installable, compatible, and safe to use in a multi-repo plan.

### Hosted observability dashboards `[long term]`

**Goal:** Build hosted dashboards on top of *Local structured workflow events* without making any local workflow depend on hosted infrastructure.

## North Star

When the items above are in place, Specify is a full spec-driven engineering control plane: define, plan, execute, review, enforce, observe.

## Non-Goals

- Do not make Specify a general developer portal.
- Do not replace catalog systems such as Backstage.
- Do not put lifecycle authority in skills, MCP servers, or hosted services.
- Do not require hosted infrastructure for the core workflow.
- Do not make `AGENTS.md` a dumping ground for long-form documentation.
- Do not blur stable artifact schemas with mutable engineering standards.
- Do not hard-code the long-term landing model to one forge.
- Do not treat cross-repo compatibility warnings as sufficient enforcement for breaking changes.

## Open Questions

- Should codex rules live inside `.specify/codex/`, at the repository root (`codex/`), or in a shared catalog accessible to multiple repos?
- Which parts of `specify review` should be deterministic CLI checks versus model-assisted analysis, and where does the boundary sit relative to `specify check` (which stays deterministic by construction)?
- What is the minimum registry projection needed from Backstage for useful multi-repo planning?
- What is the minimum compatibility classifier needed before producer changes can gate on consumer impact, given the existing `change-kind` enumeration as the seed dictionary?
- Which multi-repo acceptance fixtures best represent the product proof path?
- What is the smallest forge adapter contract that supports push, PR/MR handoff, CI state, and finalize?
- How should orchestration ownership and handoff work when more than one operator or agent can touch the same change?
- What compatibility guarantees should capability authors provide across capability and declared-tool versions?
- How much telemetry should be emitted by default, and what should require explicit opt-in?
- What approval model is required before cloud-hosted execution can push or open pull requests?

Resolved:

- *Where does repo context generation live?* — `specify context generate` / `specify context check` (see *`AGENTS.md` generation under `specify context`*). The plugin skill (`/spec:context`) can wrap the CLI later if useful, but the deterministic generator belongs in the CLI.
- *What are the names for framework versus consumer enforcement?* — `specify check` is the framework-repo linter (RFC-5); `specify review` is the consumer-project reviewer. They share rule-id vocabulary and finding shape, never the same scanner. See *Two Enforcement Surfaces, Distinct By Construction* under Directional Principles.
