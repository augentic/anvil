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

## Roadmap

### 1. Foundation: Skill And Context Hygiene

**Goal:** Make agent behavior easier to select, cheaper to load, and less dependent on inference.

The skill-hygiene foundation has now landed across RFCs 10, 13, 15, and 16:

- RFC-10 normalised plugin namespaces and capped skill bodies at the progressive-disclosure ceiling;
- RFC-13 renamed "schema" to "capability", split per-loop *slices* from umbrella *changes*, moved `/spec:plan` and `/spec:execute` to the `change` plugin, and reframed the registry and change orchestration as platform components rather than capabilities;
- RFC-15 introduced declared WASI capability tools (`specify tool`, `tools.yaml` sidecars) so deterministic helpers run with explicit permissions and SHA-256 pins instead of as bundled native code;
- RFC-16 retired the `specify-vectis` host binary in favour of the declared `vectis-validate` and `vectis-scaffold` WASI components, leaving operators with one installed binary.

Two live RFCs sit alongside this strand and should be tracked here rather than drifting on their own:

- **RFC-4 (typed skill expression).** Frontmatter schema enforcement, reference resolution, variable consistency, and cross-skill directive validation. The Option 1 surface lands inside the framework linter once the `checks.ts` port is in place; Options 2 and 3 (typed manifests / Rust DSL) remain deferred until skill count makes the lift worthwhile.
- **RFC-5 (framework linter port).** `scripts/checks.ts` (~1500 lines, Deno) is still the framework-level linter; `make checks` invokes it. The port to a Rust `specify-check` crate exposed via `specify check` is one-for-one, message-preserving, and unblocks RFC-4 Option 1. `crates/validate/src/rfc5.rs` already reserves the rule-id namespace (`tool.write-permission-too-broad`, `tool.lifecycle-state-write-denied`, `skill.invokes-host-binary-with-declared-tool-equivalent`) but the scanner is a TODO.

Naming convention for the two enforcement surfaces:

- `specify check` — **framework-repo integrity** (RFC-5). Runs in CI on this repo: skill frontmatter, marketplace alignment, capability briefs, declared-tool manifests, docs inventory.
- `specify review` — **consumer-project review** (roadmap §4). Runs against a downstream project's slices, plans, contracts, and codex compliance.

The two surfaces share rule-id vocabulary but never the same scanner; keeping the names distinct prevents the §4 design from colliding with the RFC-5 port.

Open hygiene items still owned by this strand:

- factor duplicated phase outcome, journal, and plan-mutation instructions into shared references (the `plugins/spec/references/` and `plugins/change/skills/execute/` references are the right home; today the same prose recurs across multiple skill bodies);
- preserve stable Specify artifact identifiers while improving skill discoverability;
- continue compressing always-loaded surface area as more first-party helpers move to declared tools (`skill.invokes-host-binary-with-declared-tool-equivalent` enforces the migration once the linter lands);
- finish the RFC-13 rename tail before it becomes load-bearing: pick a release in which `specify migrate slice-layout`, `specify migrate change-noun`, and the `/spec:plan` / `/spec:execute` deprecation shims are deleted.

Next, add a first-class repository context output:

- generate concise `AGENTS.md` files from Specify project metadata, capability references, repo inspection, and registry data;
- include runtime, test command, lint command, navigation hints, conventions, boundaries, and dependencies;
- keep the file short enough to sit directly in agent context;
- add checks that warn when repo structure changes imply `AGENTS.md` should be refreshed.

Candidate surface (preferred):

```bash
specify context generate
specify context check
```

`specify context` is the durable home: every other artifact noun in the post-RFC-13 CLI lives at `specify <noun> <action>` (`registry`, `workspace`, `slice`, `change`, `capability`), and `AGENTS.md` is a first-party Specify artifact derived from those nouns. A plugin skill (`/spec:context`) can wrap it later if useful, but the deterministic generator belongs in the CLI.

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

`plugins/references/review-checks.md` is already the de facto codex: it carries the `UNI-*` rule catalogue that every reviewer skill cites today, with severity, "what to look for" prose, and spec-change indicators. The first codex deliverable is a one-RFC unit of work that:

- formalises the rule-id namespace (the existing `UNI-*` ids are the seed) and reserves prefixes for new tracks (e.g. `RUST-*`, `IFACE-*`, `SEC-*`);
- adds applicability metadata so skills and reviewers can filter rules by capability, plugin, or language;
- decides the storage location — `.specify/codex/` (per-project, reviewable, projection-friendly) versus repo-root `codex/` (framework-owned) versus a shared catalog (multi-repo);
- migrates `plugins/references/review-checks.md` into the chosen location without losing rule-id stability.

Defining the format must precede any reviewer code in §4 — without stable rule ids the review output cannot be cited or suppressed safely.

### 4. CI-Native Specify Review

**Goal:** Move from workflow correctness to continuous enforcement.

This surface is distinct from `specify check` (RFC-5, §1): `check` validates *this framework repo* at framework CI time; `review` validates a *consumer project* at consumer CI time. They share rule-id vocabulary and finding shape, but they are separate scanners with separate inputs. Settling the names now prevents the §4 design from colliding with the RFC-5 port.

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

### 5. Cross-Repo Compatibility Gates

**Goal:** Move from cross-project warnings to change-level coherence.

The current contract-warning loop is useful discovery, but multi-repo execution needs a stronger compatibility model before a change can be called complete. Producer changes should be classified by impact, connected to affected consumers, and reflected in the plan before finalization.

The vocabulary already exists in `plugins/contract/references/cross-project-compatibility.md`: the `change-kind` enumeration (`removed-field`, `required-field-added`, `type-narrowed`, `enum-value-removed`, `additional-properties-tightened`, `removed-endpoint`, `status-code-removed`, …) is the seed dictionary. The work here is layering a deterministic *classification* over that enumeration — each `change-kind` maps to one of `additive` / `breaking` / `ambiguous` / `unverifiable` — and then wiring the classification into the change-level plan so producer slices can require consumer follow-up entries before they are eligible for `done`. This staged model means the existing warning emitters keep working unchanged; the gate is layered on top.

Add deterministic compatibility outputs that can answer:

- which contracts, schemas, events, APIs, or shared capabilities changed;
- whether each change is additive, breaking, ambiguous, or unverifiable;
- which registered consumers are affected;
- whether consumer update plan entries already exist;
- whether a producer slice can be marked done without follow-up work;
- what SemVer or release impact is implied where versioned artifacts exist.

Candidate surfaces:

```bash
specify compatibility check
specify compatibility report --change <name>
specify change plan impact --change <name>
```

The initial scope can remain contract-first, but the model should be dependency-aware rather than tied permanently to one artifact format.

### 6. End-To-End Acceptance Suite

**Goal:** Prove the framework across realistic multi-slice, multi-repo flows.

Add an automated or semi-automated suite that exercises the full control plane with local fixture repositories and fake or recorded forge behavior:

- plan generation from a change brief and source material;
- registry routing across multiple projects;
- execution through several dependent slices;
- branch preparation and workspace sync;
- residue and baseline commit behavior;
- push and PR/MR handoff;
- finalize after external merge;
- recovery after interruption, blocked entries, stale clones, and failed validation.

This suite should become the product proof path for the framework. Unit and integration tests can validate individual verbs, but the acceptance suite should validate that the whole multi-repo story still works.

### 7. Specify MCP Surface

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

### 8. Observability For Agentic Work

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

Structured events should include a run identity so local, CI, and hosted execution can be compared. They should also make orchestration progress visible: the current step, last completed step, pending human action, owning operator or agent, and the next valid resume point.

### 9. Forge And Landing Abstraction

**Goal:** Make branch transport, PR/MR creation, and finalize work beyond GitHub CLI.

The first implementation can continue to use GitHub and `gh`, but the framework should expose a forge boundary before enterprise adoption depends on it. Specify needs a small adapter contract for:

- remote repository discovery and authentication checks;
- branch existence and push permissions;
- PR/MR create-or-update;
- CI and mergeability status;
- merged-state verification during finalize;
- provider-specific links and annotations.

Candidate surfaces:

```bash
specify forge doctor
specify workspace push --forge github
specify change finalize --forge github
```

Non-goal: Specify should not merge PRs or replace forge policy. It should prepare, publish, observe, and verify the handoff.

### 10. Cloud-Hosted Execution

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
- parity with local `/change:execute loop`.

Candidate surface:

```bash
specify execute submit
specify execute status <run-id>
specify execute resume <run-id>
```

This should remain a long-term track. Local execution is the proving ground.

### 11. Capability Ecosystem Operating Model

**Goal:** Make capabilities feel like a dependable ecosystem rather than bespoke first-party packages.

The capability and declared-tool protocol is a strong base. The next layer is the operating model around it:

- capability publishing and discovery conventions;
- compatibility testing for capability versions and declared tools;
- migration guidance when capability briefs or artifacts evolve;
- quality gates for first-party and third-party capabilities;
- examples beyond Omnia, Vectis, and contracts;
- clear ownership of codex rules, artifact templates, and tool manifests.

This should avoid a heavy marketplace requirement. The near-term need is a reviewable way to know whether a capability is installable, compatible, and safe to use in a multi-repo plan.

## Phasing

### Landed

- RFC-10: plugin namespace renormalisation and skill-body ceiling.
- RFC-13: capability rename, slice/change vocabulary, platform-component split, `change` plugin.
- RFC-15: declared WASI capability tools and the `specify tool` runner; contract validator as the first declared tool.
- RFC-16: Vectis WASI tools (`vectis-validate`, `vectis-scaffold`) and `specify-vectis` retirement.

### Near Term

Ordered by leverage. The multi-repo acceptance fixture comes first because §6 says the framework should be judged on whole-loop correctness, and nothing in §3–§5 is meaningful without that proof path.

- Create the first multi-repo acceptance fixture that runs through plan, execute, push handoff, and finalize without live forge dependencies (§6 / §1 proof path).
- Add concise `AGENTS.md` generation and checking under `specify context generate` / `specify context check` (§1) — second-best end-to-end deliverable after the acceptance fixture; smallest scope, most direct user value, unblocks staleness detection in §4.
- Define the codex rule format and migrate `plugins/references/review-checks.md` into the chosen layout without losing rule-id stability (§3). Must precede any reviewer code.
- Define the first structured `specify review` finding schema (severity, rule id, evidence, remediation, machine-readable output) — depends on the codex rule format.
- Promote cross-project contract warnings into a classified compatibility report by mapping the existing `change-kind` enumeration onto `additive` / `breaking` / `ambiguous` / `unverifiable` (§5).
- Finish the RFC-13 rename tail: pick a release in which `specify migrate slice-layout`, `specify migrate change-noun`, and the `/spec:plan` / `/spec:execute` deprecation shims are deleted.
- Land RFC-5: port `scripts/checks.ts` into the `specify-check` Rust crate exposed via `specify check`, retire the Deno linter from `make checks`, and lift `crates/validate/src/rfc5.rs` from rule-id reservations to a working scanner. Unblocks RFC-4 Option 1.
- Land RFC-4 Option 1 inside the framework linter (frontmatter schema, reference resolution, variable consistency, cross-skill directive validation) — depends on RFC-5. Options 2 and 3 stay deferred.
- Keep the Backstage/catalog decision to adapter design, not core registry replacement.
- Migrate any remaining first-party host helpers to declared WASI tools where the cost/benefit is favourable (the `skill.invokes-host-binary-with-declared-tool-equivalent` lint reserved by RFC-15 enforces this once the linter has enough context).

### Mid Term

- Add `specify registry import` with a Backstage adapter.
- Add CI-native `specify review`.
- Add dependency-aware compatibility gates that can require consumer follow-up plan entries for breaking producer changes.
- Expand the multi-repo acceptance suite to cover blocked, failed, interrupted, and stale-workspace recovery paths.
- Add a read-oriented Specify MCP server.
- Add local structured workflow events.
- Add a first forge abstraction behind workspace push and change finalize.
- Add structured orchestration status for `/change:plan <name> orchestrate` re-entry and pause points.

### Long Term

- Add cloud-hosted `/change:execute loop` equivalents.
- Support durable background agents with sandboxed workspace clones.
- Support PR/MR creation and review loops across GitHub, GitLab, Bitbucket, and self-hosted forges through adapters.
- Support catalog-backed initiatives across many repositories.
- Add capability publishing, compatibility testing, and migration guidance.
- Build toward a full spec-driven engineering control plane: define, plan, execute, review, enforce, observe.

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

- *Where does repo context generation live?* — `specify context generate` / `specify context check` (see §1). The plugin skill (`/spec:context`) can wrap the CLI later if useful, but the deterministic generator belongs in the CLI.
- *What are the names for framework versus consumer enforcement?* — `specify check` is the framework-repo linter (RFC-5); `specify review` is the consumer-project reviewer (§4). They share rule-id vocabulary and finding shape, never the same scanner.
