# Glossary

Canonical definitions for terms used throughout Specify.

## A

**Adapter**
A versioned Specify extension. Specify splits adapters by direction: **source adapters** (operations `survey` + `extract`) and **target adapters** (operations `guidance` + `build` + `merge`). Both ship as a single WebAssembly component exporting the matching axis interface from the WIT contract; metadata comes from the component's `metadata` export (no manifest file). See [Anatomy of an adapter](../explanation/adapter-anatomy.md).

**Action**
A grammar leaf in the `specify` CLI — the executable command path (`slice build`, `journal emit`). Registered in the typed command router with one concrete clap `Args` type and workflow operation.

**Active slice**
The plan entry currently `in-progress` per `plan.yaml.slices[].status`. `specify plan next` writes `in-progress`; `/spec:refine` and the breakouts resolve the active slice before doing per-slice work.

**API contract**
A machine-readable interface definition at `contracts/`. Uses three formats: JSON Schema for payload definitions, OpenAPI 3.1 for HTTP endpoint bindings, and AsyncAPI 3.0 for messaging bindings. Authored, imported, or verified through the contracts target adapter's `build` sub-flows; validated by the contracts adapter's in-guest validator.

**Archive**
The `.specify/archive/` directory where finalized plans (one per change) and merged or dropped slices are stored for audit.

**Authority**
Closed enum that decides who wins when two `Evidence` rows disagree about the same claim. Order: `intent` > `documentation` > `behaviour` (canonical: [Authority hierarchy](../../crates/slice/prompts/synthesis/authority.md)). Set on each `Evidence` document during `extract`, applied during slice-time synthesis. See `Provenance`, `Divergence`, `Conflict`.

**Artifact**
A structured document that defines part of a slice. The core slice artifacts are `proposal.md`, `spec.md`, `design.md`, and `tasks.md`, all written by core synthesis. The change-level artifacts are `change.md`, `plan.yaml`, and `discovery.md`. Target-specific structured outputs (e.g. Vectis `composition.yaml`) are produced by the target adapter's `build` operation, not by core synthesis.

## B

**Baseline**
The accumulated set of merged specs at `.specify/specs/` and merged contracts at `contracts/`. Represents the current known behavioural and interface state of the system. Future changes produce deltas against the baseline.

**Brief**
A markdown prompt file shipped by a source or target adapter that drives one operation, compiled into the adapter guest. Prompts live under `adapters/sources/<name>/prose/prompts/{survey,extract}.md` or `adapters/targets/<name>/prose/prompts/{guidance,build,merge}.md`.

**Breakout verb**
`/spec:refine`, `/spec:build`, or `/spec:merge` invoked outside the `specify plan execute` loop — typically after execute parks or when an operator wants to drive one slice by hand. Invokes the same guest orchestration as the in-loop call.

## C

**Change**
The operator-defined umbrella that coordinates one or more slices through `change.md` and `plan.yaml`. On-disk vocabulary, not a slash-command namespace. Driven through `/spec:plan`, `specify plan execute`, `/spec:finalize`.

**Change branch**
The Git branch an operator may use to publish a multi-repo change from a workspace slot. Branch preparation and publication are repository operations outside Specify.

**Command**
One operator-facing `specify` invocation (`specify slice build my-slice`). Implemented by exactly one command **operation** and exposed through the typed command router. Distinct from a shell command, a source/target adapter operation, and a slash **skill**. See [Operation shape](../standards/handler-shape.md).

**Command group**
The resource prefix that namespaces CLI actions (`slice`, `plan`, `journal`). `specify slice *` is the slice command group.

**Claim**
One row inside an `Evidence` document. Closed `kind` enum: `intent`, `requirement`, `criterion`, `decision`, `section`, `diagram`, `contract`, `excerpt`, `type`, `call`, `region`, `container`, `leaf`. `requirement` and `criterion` carry a `id` for deterministic reconciliation across sources.

**Conflict**
Unresolvable disagreement between two `Evidence` rows at the same authority class. Surfaces as `Status: conflict` and a `[conflict]` tag on the requirement header. The operator reconciles by recording a per-slice authority override (`specify plan amend --authority-override`) or amending sources, then re-running `/spec:refine` — never by hand-editing the kernel-rendered `Status:` / `Sources:` lines. Tags never park the slice.

**Contract id**
The optional `info.x-specify-id` field on a top-level OpenAPI 3.1 / AsyncAPI 3.0 contract. Kebab-case (`^[a-z][a-z0-9-]*$`), ≤ 64 characters, unique across every top-level contract in the repo. Rename-stable hint that survives file moves and `info.version` bumps.

**Crux**
An Augentic product: a cross-platform application framework (Rust core, native iOS and Android shells). The [Vectis](#v) target adapter generates Crux applications. Not part of the core Specify contract.

**Cursor plugin**
A marketplace package under `plugins/<name>/` that registers slash-command skill wrappers with Cursor. Invisible to the `specify` CLI. See [Cursor operator plugins](../contributing/operator-plugins.md).

## D

**Diagnostic**
The neutral finding currency every check surface emits (`specify slice validate`, build reports). Each carries a `source` (`deterministic` / `model-assisted` / `hybrid` / `human` / `tool`) and a `kind`: `violation` (a structural defect; open critical/important violations block a gate) or `review` (a deterministically-raised request for agent judgment, never blocking). A `DiagnosticReport` is a collection of them.

**Discovery**
The plan-time discovery artifact at `discovery.md` in the project root (workspace mode: at the workspace root). Three required sections: `## Summary`, `## Source inventory`, `## Lead inventory`. Written by `/spec:plan` through CLI helpers.

**Divergence**
Authority-resolved disagreement between two `Evidence` rows. The higher-authority claim wins as the operative requirement; the loser is preserved as inline commentary; the requirement header gets a `[divergence]` tag and `Status: divergence`. The slice-level `divergence:` enum (`none` / `likely` / `accepted` / `rejected`) carries the operator's Gate-1 acknowledgement; the field is advisory in v1.

**Drop**
The lifecycle target that abandons a slice without merging its specs into the baseline. Stamped via `specify slice drop <name> --reason "..."`.

## E

**Evidence**
The per-source result of `extract`. A structured document with `claims:` persisted to `.specify/slices/<slice>/evidence/<source>.yaml`. Validates against `schemas/evidence.schema.json`. Top-level `authority:` is required.

**Execute**
The guest-routed driver loop (`specify plan execute`) that claims each entry and runs refine → build → merge until the plan drains. Refuses unless the plan is `approved`. Resumes from on-disk state — no `--continue` flag.

**Extract**
The slice-time operation declared by a source adapter. Reads one `Lead` plus the bound source and returns `Evidence` content the CLI persists.

## F

**Finalize**
The closure skill (`/spec:finalize`) that verifies the plan is drained, confirms operator-owned publication is complete, then runs `specify plan archive`.

## G

**Gate 1**
The operator-stamped lifecycle transition `plan.lifecycle: pending → approved`. The only review gate Specify ships. Written by `specify plan transition <name> approved`; `/spec:plan` exits at `pending` and prints the literal command in its closing hint.

**Gate (quality)**
One of the three engine test rungs and its cadence: repository correctness (`cargo make ci`, every push), WASM boundary smoke (`cargo make test-wasm`, weekly/path-filtered/manual; required for release), or the explicit live-model trial (`cargo make test-live`, operator-invoked). Distinct from the operator's plan Gate 1. See [Quality gates](../contributing/quality-gates.md).

## H

**Hard assertion**
A mechanically decidable test result, such as lifecycle state, exit status, schema validity, journal cadence, or filesystem shape. Every engine test — including the live-model trial — is graded by hard assertions only; there is no semantic grading machinery.

## I

**Intent**
The operator-supplied free-form description that backs N=1 work and overrides higher-authority sources. Declared as a source adapter (`adapters/sources/intent/`). Authority class: `intent`.

## L

**Lead**
A slice-sized unit of work emitted by a source adapter's `survey`. One block per lead under `## Lead inventory` in `discovery.md`, identified by its `(source, lead)` pair. Re-surveying the same source replaces that source's blocks. Cross-source lead matching happens later, in `propose`. See [From sources to slices](../explanation/reconciliation.md).

**Lifecycle**
Three stacked lifecycles in `plan.yaml`: the plan lifecycle (`pending → approved`, two stored states), the per-entry lifecycle (`pending → in-progress → done`, with `dropped` stamped by `specify slice drop`), and the slice lifecycle inside `metadata.yaml` (`refining → refined → built → merged`).

## M

**Merge**
The slice phase that applies spec deltas to the baseline, archives the slice, and stamps per-entry `done`. The only writer of `done`.

**Merge key**
The stable `ID: REQ-XXX` line in a spec requirement. Used to match delta spec operations to baseline requirements during merge.

**Model backend**
The implementation serving judgment requests in a test: `omnia-testkit` scripted responses on the native and WASM rungs, or the configured live model on the explicit live rung.

## O

**Operation**
The transport-neutral `omnia_guest::api::operation::Operation<P>` implementation for one **command**: a flat `Input` DTO, typed `Output`, operation-layer `Error`, and `call(input, context)`. Operations live in `<crate>::<domain>::handlers` submodules (in the `project`, `slice`, and `change` crates) beside their kernels and are invoked through `Invoker<P>` by the explicit typed command and HTTP routers. See [Operation shape](../standards/handler-shape.md).

**model.yaml**
The single structured artifact per refined slice, at `.specify/slices/<slice>/model.yaml`. Holds the requirement set with **inline provenance** (per requirement: contributing claims and the winning one), the task list, and a small header. Validated by `specify slice validate`; the audit `provenance` view is projected from it on demand (there is no persisted `provenance.yaml`). See [From sources to slices](../explanation/reconciliation.md).

## O

**Omnia**
An Augentic product: a runtime for sandboxed Rust WebAssembly (WASM) services. The [`omnia`](../reference/targets/omnia.md) target adapter generates Omnia service crates. Not part of the core Specify contract.

## P

**Plan**
The change's table of contents in `plan.yaml`. Contains `sources:` (top-level source bindings), `slices[]` (per-slice rows with `project`, `sources[]`, `status`, optional `divergence`; the target adapter is resolved on demand from the bound `project`, not stored), and `lifecycle`. Written through `specify plan {create, add, amend, transition, next, archive}` only.

**Plugin** (adapter vocabulary)
The shared shape for either adapter role. Loader `crates/project/src/adapter/`; metadata comes from the component's `metadata` export (no adapter manifest file). Source and target adapters share the same resolver; the axis decides which WIT operations the component exports. The vocabulary noun "plugin" survives where source + target authors share an audience tag. Distinct from [Cursor plugins](#c) under `plugins/` (the IDE distribution surface for `/spec:*` skill wrappers).

**Project (plan routing)**
The `project` field on a slice entry that names the workspace project a slice targets. Required when `registry.yaml` declares multiple projects; absent for single-repo plans.

**Propose**
The `/spec:plan` sub-step that reconciles `Lead[]` from each source's `survey` into `slices[]` rows in `plan.yaml` via the reconcile leg inside `specify plan author`. The agent returns `slices[]`, each row carrying an explicit kebab-case `name`, its matched `sources[]` (at most one lead per source), and a bound `project`. Coverage is at-least-once: a lead may appear in more than one slice — cross-project work becomes multiple slices joined by `depends-on`, and a cross-cutting lead is multi-homed across the slices it informs (surfaced in `change.md` under `## Cross-cutting leads`). Agent-default with operator override at Gate 1. Uncertain cross-source matches surface in `change.md` under `## Tentative merges`; materially-disagreeing synopsis pairs set `slices[].divergence: likely` via `specify plan amend`.

**Provenance**
The `Sources:` list on a requirement block — one or more source keys, highest authority first. Records which sources contributed the requirement.

## R

**Refine**
The breakout skill (`/spec:refine`) that runs per slice: slice create (re-entry safe, inside the orchestration), serial `extract` per bound source, synthesize `proposal.md` / `spec.md` / `design.md` / `tasks.md`, validate, transition to `refined`.

**Registry**
`registry.yaml` — a workspace catalogue declaring the repos in a multi-repo system. Each entry carries a `name` and `url` (plus optional contract wiring and a greenfield adapter seed); a project's description, capabilities, and target live in its own `project.yaml` and are projected into `topology.lock`.

**Requirement ID**
A stable identifier (`REQ-001`, `REQ-002`, …) assigned to each behavioral requirement in a spec. Serves as the merge key across delta specs.

## S

**Shape**
The idiom-guidance prompt shipped by a target adapter. Read by core synthesis as context; not executed. Empty `guidance` is valid.

**Skill**
An ultrathin Cursor slash-command wrapper (e.g. `/spec:plan`, `/spec:build`) that elicits arguments, invokes one `specify` verb, and relays its output. Skills do not own orchestration, synthesis, or code generation — those live in guest orchestrations and target-adapter prompts.

**Slice**
The single unit that flows through the fixed `refine → build → merge` loop. Each slice has its own proposal, spec, design, tasks, metadata, and evidence rows, and lives under `.specify/slices/<name>/`.

**Source adapter**
Input adapter role. Operations: `survey` + `extract`. First-party defaults: `intent`, `documentation`, `typescript`, `screenshots`, `captures`. Published as `specify:<name>@<semver>`; the guest crate lives at `adapters/sources/<name>/` in the adapters repo. See the [Source adapters](../reference/sources/index.md) reference.

**Source binding**
An entry under `plan.yaml.sources.<key>` that pairs a source key (operator-chosen) with an adapter and a `path:` or `value:`. The source key is what `slices[].sources[]` references.

**Spec**
A behavioral specification at `specs/<domain>/spec.md`. Contains requirements with stable IDs, `Sources:` and `Status:` provenance lines, scenarios (WHEN/THEN), error conditions, and optional metrics.

**specify**
The single CLI binary produced by the Rust workspace at the repo root that backs every `/spec:*` skill: validation, lifecycle transitions, spec merging, and plan and slice management. Framework **authoring** checks for contributors to the `augentic/specify` repo run as plain cargo tests (`tests/framework/`), not as a CLI verb. See [Workflow, standards, and artifacts](../explanation/standards-layer.md).

**Survey**
The plan-time operation declared by a source adapter. Reads the operator-bound source and emits one `Lead` block per slice-sized unit under `## Lead inventory` in `discovery.md`. Runs inside `/spec:plan`.

## T

**Target adapter**
Output adapter role. Operations: `guidance` + `build` + `merge`. First-party defaults: `omnia`, `vectis`, `contracts`. Published as `specify:<name>@<semver>`; the guest crate lives at `adapters/targets/<name>/` in the adapters repo.

**Top-level contract**
A YAML file under root `contracts/` whose root carries `openapi:` (OpenAPI 3.1 document) or `asyncapi:` (AsyncAPI 3.0 document). Format detection decides what counts — never directory layout, file name, or a custom marker. Subject to the contract validation rules (SemVer `info.version`; format + cross-repo uniqueness on `info.x-specify-id` when present).

**topology.lock**
A committed file at `.specify/topology.lock` (workspace mode only). A machine-written projection of each member project's `project.yaml` topology facets and baseline routing identity. Plan-time reconciliation reads it so the agent can route slices to the right project; regeneration is operator-owned.

## V

**Vectis**
An Augentic product: a target that applies spec-first generation to cross-platform UI, producing [Crux](#c) applications (Rust core plus native iOS and Android shells). The [`vectis`](../reference/targets/vectis.md) target adapter drives it. Not part of the core Specify contract.

## W

**WASI**
The [WebAssembly System Interface](https://wasi.dev/) — the sandbox model Specify runs adapter operations under. A WASI component gets explicit, narrow filesystem preopens, no inherited host environment, and no network access, which is how source adapters read a source tree without reaching the rest of the machine.

**Workspace**
The top-level `workspace/` directory holding per-project slots in a multi-repo change. Each child is a workspace slot — typically a Git checkout/worktree for a remote registry URL or a symlink for a local target. Materialization and publication are operator-owned outside Specify.

**Workspace mode**
The project topology declared by `project.yaml: workspace: true`. The repository holds `registry.yaml`, plan artifacts at the repository root, and project slots under top-level `workspace/<project>/`. Contrast with single-repo mode (`workspace: false`).
