# Glossary

Canonical definitions for terms used throughout Emery.

## A

**Adapter**
A versioned Emery extension. Emery splits adapters by direction: **source adapters** (operations `survey` + `extract`) and **target adapters** (operations `guidance`, the build loop `build` / `verify` / `repair` / `review`, and `merge`). Both ship as a single WebAssembly component exporting the matching axis interface from the WIT contract; metadata comes from the component's `metadata` export (no manifest file). See [Anatomy of an adapter](../explanation/adapter-anatomy.md).

**Action**
A grammar leaf in the `emery` CLI — the executable command path (`plan execute`, `journal show`). Registered in the typed command router with one concrete clap `Args` type and workflow operation.

**Active slice**
A plan entry that currently projects `in-progress` from claim facts. The execute loop claims the slice before doing per-slice work; a stopped run resumes at the same active entry.

**API contract**
A machine-readable interface definition at `contracts/`. Uses three formats: JSON Schema for payload definitions, OpenAPI 3.1 for HTTP endpoint bindings, and AsyncAPI 3.0 for messaging bindings. Authored, imported, or verified through the contracts target adapter's `build` sub-flows; validated by the contracts adapter's in-guest validator.

**Archive**
The `.emery/change/archive/` directory where finalized plans (one per change) and merged or dropped slices are stored for audit.

**Authority**
Closed enum that decides who wins when two `Evidence` rows disagree about the same claim. Order: `intent` > `documentation` > `behaviour` (canonical: [Authority hierarchy](../../crates/slice/prompts/synthesis/authority.md)). Set on each `Evidence` document during `extract`, applied during slice-time synthesis. See `Provenance`, `Divergence`, `Conflict`.

**Artifact**
A structured document that defines part of a slice. The core slice artifacts are `proposal.md`, `spec.md`, `design.md`, and `tasks.md`, all written by core synthesis. The change-level artifacts are `change.md`, `plan.yaml`, and `discovery.md`. Target-specific structured outputs (e.g. Vectis `composition.yaml`) are produced by the target adapter's `build` operation, not by core synthesis.

## B

**Baseline**
The accumulated set of merged specs at `.emery/specs/` and merged contracts at `contracts/`. Represents the current known behavioural and interface state of the system. Future changes produce deltas against the baseline.

**Brief**
A markdown prompt file shipped by a source or target adapter that drives one operation, compiled into the adapter guest. Prompts live under `sources/<name>/prose/prompts/` (survey, extract) or `targets/<name>/prose/prompts/` (guidance, the build-loop operations, merge) in the adapters repo; each performs exactly one pass — the engine owns retry loops.

## C

**Change**
The operator-defined umbrella that coordinates one or more slices through `change.md` and `plan.yaml`. On-disk vocabulary, not a slash-command namespace. Driven through `/emery:plan`, `emery plan refine`, `emery plan execute`, `/emery:finalize`.

**Command**
One operator-facing `emery` invocation (`emery plan status`). Implemented by exactly one command **operation** and exposed through the typed command router. Distinct from a shell command, a source/target adapter operation, and a slash **skill**. See [Operation shape](../standards/handler-shape.md).

**Command group**
The resource prefix that namespaces CLI actions (`slice`, `plan`, `journal`). `emery slice *` is the slice command group.

**Claim**
One row inside an `Evidence` document. Closed `kind` enum: `intent`, `requirement`, `criterion`, `decision`, `section`, `diagram`, `contract`, `excerpt`, `type`, `call`, `region`, `container`, `leaf`. `requirement` and `criterion` carry a `id` for deterministic reconciliation across sources.

**Conflict**
Unresolvable disagreement between two `Evidence` rows at the same authority class. Surfaces as `Status: conflict` and a `[conflict]` tag on the requirement header. The operator reconciles by recording a per-slice authority override (`emery plan amend --authority-override`) or amending sources, then re-running `emery plan refine` (the amendment stales the slice's refinement manifest and the drain re-refines it) — never by hand-editing the kernel-rendered `Status:` / `Sources:` lines. Tags never park the slice.

**Contract id**
The optional `info.x-emery-id` field on a top-level OpenAPI 3.1 / AsyncAPI 3.0 contract. Kebab-case (`^[a-z][a-z0-9-]*$`), ≤ 64 characters, unique across every top-level contract in the repo. Rename-stable hint that survives file moves and `info.version` bumps.

**Crux**
An Augentic product: a cross-platform application framework (Rust core, native iOS and Android shells). The [Vectis](#v) target adapter generates Crux applications. Not part of the core Emery contract.

**Cursor plugin**
A marketplace package under `plugins/<name>/` that registers slash-command skill wrappers with Cursor. Invisible to the `emery` CLI. See [Cursor operator plugins](../contributing/operator-plugins.md).

## D

**Diagnostic**
The neutral finding currency every check surface emits (`emery slice validate`, build reports). Each carries a `source` (`deterministic` / `model-assisted` / `hybrid` / `human` / `tool`) and a `kind`: `violation` (a structural defect; open critical/important violations block a gate) or `review` (a deterministically-raised request for agent judgment, never blocking). A `DiagnosticReport` is a collection of them.

**Discovery**
The plan-time discovery artifact at `.emery/change/discovery.md`. Three required sections: `## Summary`, `## Source inventory`, `## Lead inventory`. Written by `/emery:plan` through CLI helpers.

**Divergence**
Authority-resolved disagreement between two `Evidence` rows. The higher-authority claim wins as the operative requirement; the loser is preserved as inline commentary; the requirement header gets a `[divergence]` tag and `Status: divergence`. The slice-level `divergence:` enum (`none` / `likely` / `accepted` / `rejected`) carries the operator's plan-review acknowledgement; the field is advisory in v1.

**Drained**
The state of a plan in which no entry is `pending` or `in-progress` — every entry is `done`. Not a stored field: `emery plan status` computes it from per-entry status and projects `drained`. `/emery:finalize` becomes legal at that point.

**Drop**
The lifecycle target that abandons a slice without merging its specs into the baseline. Stamped via `emery plan drop <name> --reason "..."`; the entry stays on the plan and projects the `slice-dropped` stop.

## E

**emery**
The single CLI binary produced by the Rust workspace at the repo root that backs every `/emery:*` skill: validation, lifecycle transitions, spec merging, and plan and slice management. Contributor consistency for this repo is the mdBook links gate (`cargo make links`), not a CLI verb. See [Workflow, standards, and artifacts](../explanation/standards-layer.md).

**Evidence**
The per-source result of `extract`. A structured document with `claims:` persisted to `.emery/change/slices/<slice>/evidence/<source>.yaml`. Parsed and validated through the typed `artifacts::evidence::Document`. Top-level `authority:` is required.

**Execute**
The guest-routed driver loop (`emery plan execute`) that advances each entry and runs build → merge until the plan drains, consuming the exact refinement manifests `emery plan refine` wrote (it never refines — a missing or stale manifest is the typed `plan-refinement-required`). Running it opens the authorization epoch over the covered refinement digests. Resumes from on-disk state — no `--continue` flag.

**Extract**
The slice-time operation declared by a source adapter. Reads one `Lead` plus the bound source and returns `Evidence` content the CLI persists.

## F

**Finalize**
The closure skill (`/emery:finalize`) that verifies the plan is drained, confirms operator-owned publication is complete, then runs `emery plan archive`.

## G

**Gate (quality)**
One of the two engine test rungs and its cadence: repository correctness (`cargo make ci`, every push) or prompt evaluation (`cargo make eval`, operator-invoked). The WASM seam has no automated gate — it is exercised by the operator-run wasm example (`cargo make wasm-run`). See [Quality gates](../contributing/quality-gates.md).

**Guest**
The **engine guest** is the WebAssembly component embedded in the `emery` binary that owns the orchestrations behind "guest-routed" verbs (`plan author`, `plan refine`, `plan execute`) and the refine / build / merge phases they run. An **adapter guest** is a source or target adapter's own component, dispatched by the engine. A running driver orchestration holds the create-exclusive `.emery/guest.lock` marker; a second driver session exits with `guest-marker-held`.

## H

**Hard assertion**
A mechanically decidable test result, such as lifecycle state, exit status, schema validity, journal cadence, or filesystem shape. Every engine test — including the live `eval` rung — is graded by hard assertions only; there is no semantic grading machinery.

## I

**Intent**
The operator-supplied free-form description that backs single-slice, intent-only work and outranks every other source in authority. Declared as a source adapter (`sources/intent/` in the adapters repo). Authority class: `intent`.

## J

**Journal**
Per-writer append-only fact logs at `.emery/change/events/<writer>.jsonl` (union via `emery journal show`). Carries authorization (`plan.execute.started`), claims, phase events, waves, and the outcome ledger (`slice.archive.created`, …).

**Journal writer**
A stable identity with exclusive append authority over one `.emery/change/events/<writer>.jsonl` log and its sequence namespace. Claims use that writer ID to identify the current slice owner.

## L

**Lead**
A slice-sized unit of work emitted by a source adapter's `survey`. One block per lead under `## Lead inventory` in `discovery.md`, identified by its `(source, lead)` pair. Re-surveying the same source replaces that source's blocks. Cross-source lead matching happens later, in `propose`. See [From sources to slices](../explanation/reconciliation.md).

**Lifecycle**
Two stacked projected ladders: per-entry (`pending → in-progress → done` — no per-entry `dropped`) from claims / merge-archive facts, and per-slice (`refining → refined → built → merged`, or terminal `dropped` via `emery plan drop`) from phase timestamps and artifacts. Neither is a stored status field. Starting `emery plan execute` opens the authorization epoch — there is no projected `approved` rung. See [Lifecycle](../reference/lifecycle.md).

## M

**Merge**
The slice phase that wave-commits requirement identity, applies spec deltas to the baseline, archives the slice, and projects per-entry `done`.

**Merge key**
The stable `ID: REQ-XXX` line in a spec requirement. Used to match delta spec operations to baseline requirements during merge.

**Model backend**
The implementation serving judgment requests in a test: `omnia-testkit` scripted responses on the native and WASM rungs, or the configured live model on the explicit live rung.

**model.yaml**
The single structured artifact per refined slice, at `.emery/change/slices/<slice>/model.yaml`. Holds the requirement set with **inline provenance** (per requirement: contributing claims and the winning one), the task list, and a small header. Validated by `emery slice validate`; the audit `provenance` view is projected from it on demand (there is no persisted `provenance.yaml`). See [From sources to slices](../explanation/reconciliation.md).

## O

**Omnia**
An Augentic product: a runtime for sandboxed Rust WebAssembly (WASM) services. The [`omnia`](../reference/targets/omnia.md) target adapter generates Omnia service crates. Not part of the core Emery contract.

**Operator**
The human driving Emery: binds sources, reviews the plan before execute, resolves conflicts through overrides, and owns everything Emery deliberately leaves outside its scope — Git commits and publication.

**Operation**
The transport-neutral `omnia_guest::api::operation::Operation<P>` implementation for one **command**: a flat `Input` DTO, typed `Output`, operation-layer `Error`, and `call(input, context)`. Operations live in `<crate>::<domain>::handlers` submodules (in the `project`, `slice`, and `change` crates) beside their kernels and are invoked through `Invoker<P>` by the explicit typed command and HTTP routers. See [Operation shape](../standards/handler-shape.md).

## P

**Parked**
Informal term for `emery plan execute` having stopped mid-loop on a stop condition (`refine-failed`, `build-failed`, `merge-conflict`, `merge-postflight-failed`, `slice-dropped`, `merge-incomplete`, `stuck`). The plan keeps its on-disk state; `emery plan status` names the stop reason and the literal resume command — usually re-running `emery plan execute` after fixing the reported problem.

**Plan**
The change's table of contents in `plan.yaml`. Contains `sources:` (top-level source bindings) and `slices[]` (per-slice rows with `project`, `sources[]`, `status`, optional `divergence`; the target adapter is resolved on demand from the bound `project`, not stored). Written through `emery plan {author, add, amend, remove, drop, archive}` and the merge stamp only.

**Plugin** (adapter vocabulary)
The shared shape for either adapter role. Loader `crates/project/src/adapter/`; metadata comes from the component's `metadata` export (no adapter manifest file). Source and target adapters share the same resolver; the axis decides which WIT operations the component exports. The vocabulary noun "plugin" survives where source + target authors share an audience tag. Distinct from [Cursor plugins](#c) under `plugins/` (the IDE distribution surface for `/emery:*` skill wrappers).

**Project (plan routing)**
The `project` field on a slice entry that names the project a slice targets. The reconcile leg auto-binds the sole project; the field exists so a slice's `target` adapter can be derived from its bound project.

**Propose**
The `/emery:plan` sub-step that reconciles `Lead[]` from each source's `survey` into `slices[]` rows in `plan.yaml` via the reconcile leg inside `emery plan author`. The agent returns `slices[]`, each row carrying an explicit kebab-case `name`, its matched `sources[]` (at most one lead per source), and a bound `project`. Coverage is at-least-once: a lead may appear in more than one slice — cross-project work becomes multiple slices joined by `depends-on`, and a cross-cutting lead is multi-homed across the slices it informs (surfaced in `change.md` under `## Cross-cutting leads`). Agent-default with operator override during plan review. Uncertain cross-source matches surface in `change.md` under `## Tentative merges`; materially-disagreeing synopsis pairs set `slices[].divergence: likely` via `emery plan amend`.

**Provenance**
The `Sources:` list on a requirement block — one or more source keys, highest authority first. Records which sources contributed the requirement.

## R

**Refine**
The first phase the execute loop runs per slice: slice create (re-entry safe, inside the orchestration), serial `extract` per bound source, synthesize `proposal.md` / `spec.md` / `design.md` / `tasks.md`, validate, transition to `refined`.

**Requirement ID**
A stable identifier (`REQ-001`, `REQ-002`, …) assigned to each behavioral requirement in a spec. Serves as the merge key across delta specs.

## S

**Shape**
The idiom-guidance prompt shipped by a target adapter. Read by core synthesis as context; not executed. Empty `guidance` is valid.

**Skill**
A thin Cursor slash-command wrapper (e.g. `/emery:plan`, `/emery:execute`) that elicits arguments, invokes one `emery` verb, and relays its output. Skills do not own orchestration, synthesis, or code generation — those live in the engine guest and target-adapter prompts.

**Slice**
The single unit that flows through the fixed `refine → build → merge` rhythm — refinement inside the `emery plan refine` drain, build and merge inside `emery plan execute`. Each slice has its own proposal, spec, design, tasks, metadata, evidence rows, and refinement manifest, and lives under `.emery/change/slices/<name>/`.

**Source adapter**
Input adapter role. Operations: `survey` + `extract`. First-party defaults: `intent`, `documentation`, `typescript`, `screenshots`, `captures`. Published as `emery:<name>@<semver>`; the guest crate lives at `sources/<name>/` in the adapters repo. See the [Source adapters](../reference/sources/index.md) reference.

**Source binding**
An entry under `plan.yaml.sources.<key>` that pairs a source key (operator-chosen) with an adapter and a `path:` or `value:`. The source key is what `slices[].sources[]` references.

**Spec**
A behavioral specification at `specs/<domain>/spec.md`. Contains requirements with stable IDs, `Sources:` and `Status:` provenance lines, scenarios (WHEN/THEN), error conditions, and optional metrics.

**Survey**
The plan-time operation declared by a source adapter. Reads the operator-bound source and emits one `Lead` block per slice-sized unit under `## Lead inventory` in `discovery.md`. Runs inside `/emery:plan`.

## T

**Target adapter**
Output adapter role. Operations: `guidance`, the build loop `build` / `verify` / `repair` / `review` (one pass per dispatch under the engine's phase machine), and `merge`. First-party defaults: `omnia`, `vectis`, `contracts`. Published as `emery:<name>@<semver>`; the guest crate lives at `targets/<name>/` in the adapters repo.

**Top-level contract**
A YAML file under root `contracts/` whose root carries `openapi:` (OpenAPI 3.1 document) or `asyncapi:` (AsyncAPI 3.0 document). Format detection decides what counts — never directory layout, file name, or a custom marker. Subject to the contract validation rules (SemVer `info.version`; format + cross-repo uniqueness on `info.x-emery-id` when present).

## V

**Vectis**
An Augentic product: a target that applies spec-first generation to cross-platform UI, producing [Crux](#c) applications (Rust core plus native iOS and Android shells). The [`vectis`](../reference/targets/vectis.md) target adapter drives it. Not part of the core Emery contract.

## W

**WASI**
The [WebAssembly System Interface](https://wasi.dev/) — the sandbox model Emery runs adapter operations under. A WASI component gets explicit, narrow filesystem preopens, no inherited host environment, and no network access, which is how source adapters read a source tree without reaching the rest of the machine.
