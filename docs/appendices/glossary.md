# Glossary

Canonical definitions for terms used throughout Specify 2.0.

## A

**Adapter**
A versioned Specify extension. Specify 2.0 splits adapters by direction: **source adapters** (`axis: source`, operations `survey` + `extract`) and **target adapters** (`axis: target`, operations `shape` + `build` + `merge`). Both ship `adapter.yaml` validated by an axis-specific schema (`source.schema.json` for sources, `target.schema.json` for targets). See [Anatomy of an adapter](../explanation/adapter-anatomy.md).

**Active slice**
The plan entry currently `in-progress` per `plan.yaml.slices[].status`. `specrun plan next` writes `in-progress`; `/spec:refine` and the breakouts resolve the active slice before doing per-slice work.

**API contract**
A machine-readable interface definition at `contracts/`. Uses three formats: JSON Schema for payload definitions, OpenAPI 3.1 for HTTP endpoint bindings, and AsyncAPI 3.0 for messaging bindings. Authored, imported, or verified through the contracts target adapter's `build` sub-flows; validated by the declared `contract` WASI tool (`specrun tool run contract`).

**Archive**
The `.specify/archive/` directory where finalized plans (one per change) and merged or dropped slices are stored for audit.

**Authority**
Closed enum that decides who wins when two `Evidence` rows disagree about the same claim. Order: `intent` > `documentation` > `behaviour`. Declared per source adapter, applied during slice-time synthesis. See `Provenance`, `Divergence`, `Conflict`.

**Artifact**
A structured document that defines part of a slice. The core slice artifacts are `proposal.md`, `spec.md`, `design.md`, and `tasks.md`, all written by core synthesis. The change-level artifacts are `change.md`, `plan.yaml`, and `discovery.md`. Target-specific structured outputs (e.g. Vectis `composition.yaml`) are produced by the target adapter's `build` operation, not by core synthesis.

## B

**Baseline**
The accumulated set of merged specs at `.specify/specs/` and merged contracts at `contracts/`. Represents the current known behavioural and interface state of the system. Future changes produce deltas against the baseline.

**Brief**
A markdown prompt file shipped by a source or target adapter that drives one operation. Briefs live under `adapters/sources/<name>/briefs/{survey,extract}.md` or `adapters/targets/<name>/briefs/{shape,build,merge}.md`.

**Breakout verb**
`/spec:refine`, `/spec:build`, or `/spec:merge` invoked outside the `/spec:execute` loop — typically after execute parks or when an operator wants to drive one slice by hand. Shares the same skill body as the in-loop call.

## C

**Lead**
A slice-sized unit emitted by a source adapter's `survey`. One block per lead under `## Lead inventory` in `discovery.md`, with stable `id` and `sources[]`. Re-surveying the same source replaces blocks by `id`.

**Change**
The operator-defined umbrella that coordinates one or more slices through `change.md` and `plan.yaml`. On-disk vocabulary in 2.0, not a slash-command namespace. Driven through `/spec:plan`, `/spec:execute`, `/spec:finalize`.

**Change branch**
The Git branch used to publish a multi-repo change from a workspace slot. Form: `specify/<change-name>`. `/spec:execute` prepares remote-backed slots on this branch before mutation; `specrun workspace push` publishes them.

**Claim**
One row inside an `Evidence` document. Closed `kind` enum: `intent`, `requirement`, `criterion`, `decision`, `section`, `diagram`, `contract`, `excerpt`, `type`, `call`, `region`, `container`, `leaf`. `requirement` and `criterion` carry a `claim-id` for deterministic reconciliation across sources.

**Conflict**
Unresolvable disagreement between two `Evidence` rows at the same authority class. Surfaces as `Status: conflict` and a `[conflict]` tag on the requirement header. The operator reconciles by hand-editing `spec.md` or by amending sources. Tags never park the slice.

**Contract id**
The optional `info.x-specify-id` field on a top-level OpenAPI 3.1 / AsyncAPI 3.0 contract. Kebab-case (`^[a-z][a-z0-9-]*$`), ≤ 64 characters, unique across every top-level contract in the repo. Rename-stable hint that survives file moves and `info.version` bumps.

## D

**Discovery**
The plan-time discovery artifact at `.specify/discovery.md` (workspace mode: at the workspace root). Three required sections: `## Summary`, `## Source inventory`, `## Lead inventory`. Written by `/spec:plan` through CLI helpers.

**Divergence**
Authority-resolved disagreement between two `Evidence` rows. The higher-authority claim wins as the operative requirement; the loser is preserved as inline commentary; the requirement header gets a `[divergence]` tag and `Status: divergence`. The slice-level `divergence:` enum (`none` / `likely` / `accepted` / `rejected`) carries the operator's Gate-1 acknowledgement; the field is advisory in v1.

**Drop**
The lifecycle target that abandons a slice without merging its specs into the baseline. Stamped via `specrun slice transition <name> dropped --reason "..."`.

## E

**Survey**
The plan-time operation declared by a source adapter. Reads the operator-bound source and emits one `Lead` block per slice-sized unit under `## Lead inventory` in `discovery.md`.

**Evidence**
The per-source result of `extract`. A structured document with `claims:` persisted to `.specify/slices/<slice>/evidence/<source-key>.yaml`. Validates against `schemas/evidence.schema.json`. Top-level `authority:` is required.

**Execute**
The supervised driver skill (`/spec:execute`) that loops per slice: `specrun plan next` → `/spec:refine` → `/spec:build` → `/spec:merge` → repeat. Refuses unless the plan is `approved`. Resumes from on-disk state — no `--continue` flag.

**Extract**
The slice-time operation declared by a source adapter. Reads one `Lead` plus the bound source and returns `Evidence` content the CLI persists.

## F

**Finalize**
The closure skill (`/spec:finalize`) that pushes branches, observes PR state with `gh pr view` (read-only), and runs `specrun plan archive` once every PR is `MERGED`. Never merges PRs itself.

## G

**Gate 1**
The operator-stamped lifecycle transition `plan.lifecycle: pending → approved`. The only review gate Specify 2.0 ships in v1. Written by `specrun plan transition <name> approved`; `/spec:plan` exits at `pending` and prints the literal command in its closing hint.

## I

**Intent**
The operator-supplied free-form description that backs N=1 work and overrides higher-authority sources. Declared as a source adapter (`adapters/sources/intent/`). Authority class: `intent`.

## L

**Lifecycle**
Three stacked lifecycles in `plan.yaml`: the plan lifecycle (`pending → approved`, two stored states), the per-entry lifecycle (`pending → in-progress → done`, with `dropped` available as a slice transition target), and the slice lifecycle inside `.metadata.yaml` (`refining → refined → built → merged`).

## M

**Merge**
The slice phase that applies spec deltas to the baseline, archives the slice, and stamps per-entry `done`. The only writer of `done`.

**Merge key**
The stable `ID: REQ-XXX` line in a spec requirement. Used to match delta spec operations to baseline requirements during merge.

## P

**Plan**
The change's table of contents in `plan.yaml`. Contains `sources:` (top-level source-key bindings), `slices[]` (per-slice rows with `target`, `project`, `sources[]`, `status`, optional `divergence`), and `lifecycle`. Written through `specrun plan {create, add, amend, transition, next, archive}` only.

**Plugin**
The shared shape for either adapter role. Schemas `source.schema.json` / `target.schema.json` (axis-specific, distributed with the CLI); loader `crates/workflow/src/adapter/`. Source and target adapters share the same loader; the axis decides which operations a manifest declares. The vocabulary noun "plugin" survives where source + target authors share an audience tag.

**Project (plan routing)**
The `project` field on a slice entry that names the workspace project a slice targets. Required when `registry.yaml` declares multiple projects; absent for single-repo plans.

**Propose**
The `/spec:plan` sub-step that reconciles `Lead[]` from each source's `survey` into `slices[]` rows in `plan.yaml` via `specrun plan propose`. The agent returns `slices[]`, each row carrying a `scope` id, its matched `sources[]` (at most one lead per source), and a bound `project` (one row per `(scope, project)` binding). Agent-default with operator override at Gate 1. Uncertain cross-source matches surface in `change.md` under `## Tentative merges`; materially-disagreeing summary pairs set `slices[].divergence: likely` via `specrun plan amend`.

**Scope (reconciliation)**
The reconciled unit of work behind a slice: the set of leads the agent judges to be the same piece of work, at most one lead per source. Expressed as a shared `scope` id across one or more `specrun plan propose` response `slices[]` rows that carry identical `sources[]`. The per-scope source sets declare every `(source-key, lead-id)` exactly once; one scope may fan out to multiple `plan.yaml.slices[]` rows across projects. The agent never fuses two leads from the same source — same-source re-sizing is an operator action at Gate 1. Distinct from proposal "in scope" wording and from the [Core concepts](../explanation/concepts.md) doc title.

**Provenance**
The `Sources:` list on a requirement block — one or more source keys, highest authority first. Records which sources contributed the requirement.

## R

**Refine**
The breakout skill (`/spec:refine`) that runs per slice: `specrun slice create`, serial `extract` per bound source, synthesize `proposal.md` / `spec.md` / `design.md` / `tasks.md`, validate, transition to `refined`. Replaces 1.x `/spec:define` and `/spec:extract`.

**Registry**
`registry.yaml` — a workspace catalogue declaring the repos in a multi-repo system. Each entry has a name, URL, target adapter identifier, and domain description.

**Requirement ID**
A stable identifier (`REQ-001`, `REQ-002`, …) assigned to each behavioral requirement in a spec. Serves as the merge key across delta specs.

## S

**Shape**
The idiom-guidance brief shipped by a target adapter. Read by core synthesis as context; not executed. Empty `shape` is valid.

**Skill**
An agent-driven orchestrator invoked with a slash-command prefix (e.g. `/spec:plan`, `/omnia:crate-writer`). Skills delegate deterministic work to the CLI and use judgment for everything else.

**Slice**
The single unit that flows through the fixed `refine → build → merge` loop. Each slice has its own proposal, spec, design, tasks, metadata, and evidence rows, and lives under `.specify/slices/<name>/`.

**Source adapter**
Input adapter role. Operations: `survey` + `extract`. First-party defaults: `intent`, `documentation`, `code-typescript`, `screenshots`. Lives at `adapters/sources/<name>/adapter.yaml`.

**Source binding**
An entry under `plan.yaml.sources.<key>` that pairs a source key (operator-chosen) with an adapter and a `path:` or `value:`. The source key is what `slices[].sources[]` references.

**Spec**
A behavioral specification at `specs/<unit>/spec.md`. Contains requirements with stable IDs, `Sources:` and `Status:` provenance lines, scenarios (WHEN/THEN), error conditions, and optional metrics.

## T

**Target adapter**
Output adapter role. Operations: `shape` + `build` + `merge`. First-party defaults: `omnia`, `vectis`, `contracts`. Lives at `adapters/targets/<name>/adapter.yaml`. Replaces the unqualified 1.x "adapter".

**Top-level contract**
A YAML file under root `contracts/` whose root carries `openapi:` (OpenAPI 3.1 document) or `asyncapi:` (AsyncAPI 3.0 document). Format detection decides what counts — never directory layout, file name, or a custom marker. Subject to the contract validation rules (SemVer `info.version`; format + cross-repo uniqueness on `info.x-specify-id` when present).

## W

**Workspace**
The directory under `.specify/workspace/` holding per-project slots in a multi-repo change. Each child is a workspace slot — a Git clone for remote registry URLs or a symlink for local targets. Materialised by `specrun workspace sync` from `registry.yaml`. Local commits are published through `specrun workspace push`; PR merge remains an operator action outside Specify.

**Workspace mode**
The project topology declared by `project.yaml: workspace: true`. The repository holds `registry.yaml`, plan artifacts at the repository root, and project slots under `.specify/workspace/<project>/`. Contrast with single-repo mode (`workspace: false`).
