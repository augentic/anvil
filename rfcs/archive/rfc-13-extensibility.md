# RFC-13: Extensibility

> Status: Implemented · Depends: [RFC-1](rfc-1-cli.md), [RFC-8](rfc-8-api-contracts.md), [RFC-9](rfc-9-platform.md), [RFC-12](rfc-12-refine-rfc-8.md) · Enables: [RFC-14](rfc-14-workspace.md)

## Abstract

A capability describes how Specify's `define → build → merge` loop handles an outcome domain and the artefacts that domain owns. RFC-13 defines the **immutable core** as the per-project loop engine plus capability-agnostic scaffolding (init, migrate, capability resolver, and slice driver). Platform features such as registry materialisation and change orchestration are first-party Specify components outside both the core and the capability model.

This RFC also assigns two lifecycle nouns. A **slice** is the single unit that flows through the fixed `define → build → merge` loop and a **change** is the umbrella concept that coordinates a multi-slice outcome. It holds one or more slices through its `plan.yaml`; each slice is a per-project transaction with its own proposal, specs, design, tasks, and merge step.

The `schema.yaml` surface uses the wrong noun and mixes extension metadata with project guidance and composition fields. This RFC renames the extension primitive to **capability** and makes `pipeline:` the capability manifest's declarative loop surface. Capability-specific skills own artefact creation, validation, adoption, and cleanup. The RFC also draws a line around non-capability foundation components: topology and local materialisation belong to `specify registry`, and change orchestration belongs above the core loop.

## Motivation

### The core isn't actually core

Specify's current surface promises extensibility and breaks it inside the binary:

- `specify-cli/src/cli.rs` carries `Vectis { action: VectisAction }` and `Contract { action: ContractAction }` as top-level subcommands, dispatched through `specify_vectis` and `specify::validate_baseline_contracts`. Capability-specific surfaces wearing a core coat.
- `crates/merge/src/change.rs` takes `specs_dir` and `contracts_dir` as first-class parameters, carries a `ContractPreviewEntry` type, and hard-codes "3-way for specs, opaque-replace for contracts" as the entire merge universe.
- `crates/validate/src/lib.rs` re-exports `validate_baseline_contracts` — a contracts-format validator has become part of the core's public API.
- `src/config.rs` exposes `ProjectConfig::specs_dir` and `contracts_dir` as fixed helpers.
- `schemas/schema.schema.json` admits a Specify extension as a schema, mixing product vocabulary with JSON Schema vocabulary.

Every new concern — infra, client SDKs, standards, codex rules, design tokens, fixtures — therefore requires a core patch.

### One primitive already works

`schema:` in `.specify/project.yaml` is already URL-resolvable, with project-local caching under `.specify/.cache/`. The rename changes the noun, not the distribution model: capability manifests are still remote, versioned artefacts. The migration maps `schema:` to `capability:` and `schema.yaml` to `capability.yaml` (§Migration). Follow-up RFCs that currently say "schema" for the extension primitive must be updated as part of this landing so the post-RFC vocabulary has one meaning: **capability** for Specify extensions, **schema** for validation schemas.

### Domain behavior is encoded in Rust

Today the runtime knows too much about a small set of artefacts:

- Specs are staged and merged file-by-file.
- Contracts are staged and promoted by whole-file replacement.
- Crates and Vectis shells are written directly into the project tree.

Those are valid mechanics, but they are not core truths. They should move behind capability skills and references rather than becoming a larger core type surface.

### What the status quo blocks

- A future `infra@v1` cannot validate or apply infrastructure-specific workflow without patching `specify-cli`.
- A future `standards@v1` (roadmap §3) needs skills that cite standards material during generation and review; today neighboring-domain mechanics are hard-coded for specs, contracts, and direct code generation.
- The format validators behind `specify contract validate` live in the core's public API, so a third-party capability cannot ship an equivalent without patching core.

## Design

### Principle

**A capability describes how Specify creates an outcome domain's artefacts.** The `define → build → merge` phase loop is fixed by the core; capabilities populate it with per-domain briefs and skills. The core never switches on a capability name and never carries capability-specific type surfaces. Imperative code is owned by a capability's skills, which have the tool and script mechanisms needed to execute it.

The `define → build → merge` loop's *shape* is frozen: the phase set, legal transition DAG, and per-phase outcome contract recorded in `.metadata.yaml` are part of the immutable core. Capabilities declare which briefs run inside those phases, but never the phases themselves. Variation that capabilities legitimately want lives in variable briefs per phase and capability-specific skill behavior. See §Non-Goals.

The coordinating principle is: **capabilities own outcome artefacts and their mechanics; platform components coordinate where and when those per-project slices run**.

Every mutable artefact has exactly one capability owner, every reviewed slice runs through exactly one capability/scope, and cross-capability outcomes are represented by explicit change plan entries rather than by fusing capabilities into a larger hidden capability. Outcomes are not necessarily code: they may be contracts, documentation, policy, infrastructure, fixtures, reports, generated clients, or any other capability-owned artefact.

### The immutable core boundary

The core is what's needed to run the fixed slice loop over one project root and one resolved capability — no more:


| Surface                       | Owner | What it does                                                                                                                        |
| ----------------------------- | ----- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `specify init`                | Core  | Bootstrap `.specify/`, resolve capability URL(s), cache briefs. Runs before any capability has loaded.                              |
| `specify migrate <migration>` | Core  | One-shot layout migrations.                                                                                                         |
| `specify capability` *        | Core  | Resolve, check, pipeline. Replaces `specify schema` *.                                                                              |
| `specify slice` *             | Core  | Fixed slice loop: create, list, status, validate, merge, drop, transition, archive, journal, outcome, touched-specs, overlap, task. |


The left-hand column is frozen as the core responsibility boundary; new capability behavior lands on the right. Platform components sit above this table: they may choose a project root, prepare a materialised registry checkout, or sequence several slices, but they call the core loop rather than becoming part of it.

### Platform components are not capabilities

Registry and the change component are first-party Specify components because they are substrate for multi-project operation, not outcome domains. They may have commands, libraries, and files, but they do not participate in the capability manifest protocol and they are not activated through `capability.yaml`.


| Component          | Primary file / state                    | Responsibility                                                                                                                                                                                                                                    | Must not own                                                                                                                                                                             |
| ------------------ | --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify registry` | `registry.yaml` + `.specify/workspace/` | Topology ledger plus local materialised view: project ids, repository locations, human descriptions, default capabilities, clone/symlink resolution, dirty-state reporting, and explicit push/merge operations for checked-out registry projects. | Change or plan status, contract relationships, validation findings, change execution, capability-specific validation, or PR metadata beyond the local project operation being requested. |
| `specify change`   | `change.md` + `plan.yaml`               | Coordinate an operator outcome from brief through executable plan, execution state, and close-out by consuming registry project ids, materialised project paths, and core phase outcomes.                                                         | Domain artefact ownership, topology materialisation, or hidden multi-capability transactions.                                                                                            |


The dependency direction is one-way: `specify` core knows nothing about registry or change orchestration. `specify change` may depend on `specify registry` and the core loop because orchestration composes those lower-level services.

Platform-component artefacts (`registry.yaml`, `plan.yaml`, `change.md`) live at the repo root per RFC-9 §1B; this RFC does not move them. Only the *owning component* changes — `change.md` and `plan.yaml` move from `specify-initiative` / `specify-plan` to `specify change`, and `registry.yaml` plus `.specify/workspace/` move into the new `specify registry` crate.

### What becomes a capability

Not every top-level noun becomes a capability. Capability is reserved for outcome domains whose artefacts flow through the fixed loop. Foundation and orchestration surfaces become separate Specify components.

The durable post-RFC surfaces are:


| Surface                | Owner / kind                 | Primary state / artefact                    | Notes                                                                                                                |
| ---------------------- | ---------------------------- | ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `specify init`         | Core                         | `.specify/` + `project.yaml`                | Bootstraps a project before any capability has loaded.                                                               |
| `specify capability` * | Core                         | capability manifest cache                   | Resolves, checks, and renders capability pipelines. Replaces `specify schema` *.                                     |
| `specify slice `*      | Core                         | `.specify/slices/`                          | Runs the fixed per-project slice loop against one resolved capability.                                               |
| `specify registry *`   | `specify registry` component | `registry.yaml` + `.specify/workspace/`     | Owns topology plus the local materialised view. It is validated and mutated directly, not reviewed through the loop. |
| `specify change *`     | `specify change` component   | `change.md` + `plan.yaml`                   | Owns change brief, planning graph, execution state, finalization, and archive.                                       |
| `contracts@v1`         | Capability                   | `contracts/` baseline                       | RFC-12's SemVer + `info.x-specify-id` checks become capability validation behavior.                                  |
| `vectis@v2`            | Capability                   | Shared / iOS / Android / design-system dirs | Vectis-specific validation and merge behavior moves into Vectis skills.                                              |


The renamed or folded surfaces are:


| Current surface                    | Result                       | Replacement                                     | Notes                                                                                                                                                |
| ---------------------------------- | ---------------------------- | ----------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| `specify schema *`                 | Renamed                      | `specify capability *`                          | Vocabulary cut-over from schema to capability.                                                                                                       |
| `specify change *` (per-loop unit) | Renamed                      | `specify slice *`                               | The single unit that flows through `define → build → merge` is a slice; `change` names the orchestration surface below.                              |
| `specify plan *`                   | Folded into change           | `specify plan *`                         | Plan authoring, inspection, status, locking, and transitions are change subresource behavior, not a separate top-level domain.                       |
| `specify initiative *`             | Renamed                      | `specify change *`                              | The umbrella orchestration noun moves from `initiative` to `change`. The verb set (`create`, `plan`, `execute`, `finalize`, `archive`) is preserved. |
| `/spec:plan`                       | Temporary landing alias only | change planning skill / command                 | The assisted planning skill authors or refreshes the change's executable plan during the cut-over; released docs teach the change surface.           |
| `/spec:execute`                    | Temporary landing alias only | change execution skill / command                | The execute driver moves to the change surface during the cut-over; released docs teach the change surface.                                          |
| `specify workspace *`              | Removed                      | `specify registry *`                            | No compatibility alias is retained. Materialisation commands move to the registry surface, and `.specify/workspace/` is registry state.              |
| `specify contract *`               | Folded into capability       | `contracts@v1` capability validation and skills | Contract format validation and adoption behavior move out of core command modules.                                                                   |
| `specify vectis *`                 | Folded into capability       | `vectis@v2` capability validation and skills    | Vectis-specific behavior moves out of core command modules.                                                                                          |


The durable command surface makes `change` the operator-facing orchestration noun. `plan` remains the file name (`plan.yaml`) and a subresource in command help, but not a peer top-level CLI family. The intended shape is `specify change create`, `specify plan {add,amend,next,status,doctor,lock}`, `specify change execute`, `specify change finalize`, and `specify change archive`. Temporary aliases for `/spec:plan`, `/spec:execute`, or `specify plan *` may exist only inside the landing branch to keep incremental commits testable; the released post-RFC surface teaches and supports the change-owned form.

### Capability manifest and protocol

The capability manifest is the declarative surface the core loads before running the slice loop. It declares the phase-brief pipeline:

```yaml
name: vectis
version: 2
description: Vectis Crux application workflow

pipeline:
  define:
    - id: draft-proposal
      brief: briefs/proposal.md
    - id: draft-specs
      brief: briefs/specs.md
    - id: draft-composition
      brief: briefs/composition.md
    - id: draft-design
      brief: briefs/design.md
    - id: draft-tasks
      brief: briefs/tasks.md
  build:
    - id: implement
      brief: briefs/build.md
  merge:
    - id: prepare-merge
      brief: briefs/merge.md
```

`pipeline:` maps each fixed slice phase to the ordered briefs the core renders for that phase. The capability manifest pipeline contains only slice phases (`define`, `build`, and `merge`). Today's `pipeline.plan` entries move to the `specify change` planning surface because planning is orchestration, not capability-owned slice work. Each brief entry has a stable step `id` and a path to the markdown brief template. The capability's briefs and skills decide which artefacts are created, validated, promoted, or cleaned up during each phase.

Only `name`, `version`, `description`, and `pipeline` are present. The post-RFC capability manifest intentionally drops the current `schema.yaml` `domain` and `extends` fields: durable domain guidance belongs in capability references and skills. The manifest field is:


| Field       | Meaning                                                               |
| ----------- | --------------------------------------------------------------------- |
| `pipeline:` | Ordered phase briefs used by the fixed `define → build → merge` loop. |


The protocol pieces below describe how the core interprets that manifest. Imperative behavior stays in capability skills and references, not in `capability.yaml`.

#### Merge and adoption contract

The core owns the slice state machine and generic bookkeeping around a merge: phase transitions, `.metadata.yaml` outcomes, journal entries, conflict preflight for the slice directory itself, and archival after a successful merge. It does not know which baseline paths are authoritative for a capability and it does not choose a merge strategy by capability name.

Capability merge skills own domain adoption. The merge brief validates the capability's staged artefacts, decides whether each artefact is promoted, replaced, generated, or cleaned up, and runs any capability-specific drift or format checks. The brief signals go/no-go through the existing core outcome contract: `specify slice outcome set --phase merge --outcome {success,failed,blocked}` records the decision, and `specify slice journal append --kind {failure,recovery}` records capability-owned diagnostics. The core reads the merge phase outcome and proceeds with archival on `success`; `failed` or `blocked` halts archival and surfaces the journal entries to the operator. The core does not parse capability diagnostics — they round-trip as opaque journal entries.

#### Cross-capability coexistence

Within a project or scope, capability coexistence is governed by one active domain capability. Only the active domain capability owns the slice's mutable artefacts.

A repository activates exactly one **domain** capability under this RFC. Multi-domain repositories are covered by [RFC-14](rfc-14-workspace.md), which adds a Cargo-style `package:` / `workspace:` shape.

#### Cross-capability coordination

When an outcome spans capabilities, the runtime does not fuse their pipelines. Coordination is explicit and platform-owned: `specify change` records the operator outcome, close-out criteria, executable plan, execution state, and finalization checks, while `specify registry` identifies participating projects and resolves their materialised project roots.

The change plan coordinates capability-owned slices, validations, or checks; edges express ordering (`needs:`) and blocking conditions. A change may deliver code, but it may also deliver contracts, docs, infrastructure, fixtures, reports, or policy artefacts.

This RFC does not define a core change runner. Change planning, validation, execution, re-entry, and finalization are `specify change` concerns. `/spec:execute` is therefore not a new core lifecycle command; it belongs on the change surface as the long-running orchestrator that calls core phase skills (`/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`) and change-owned deterministic helpers.

##### Example: landing a change

The end-to-end human loop has two operator checkpoints:

1. **Change definition.** The operator authors or refreshes `change.md`; `specify change` authors or updates the change's `plan.yaml`. The operator reviews the desired outcome, scope, impacted projects, feature list, close-out criteria, dependencies, target projects/scopes, and slice boundaries before execution.
2. **Change execution and close-out.** The change execution action drives eligible plan entries through the core slice loop. The finalization action verifies that the plan is terminal, required PRs have merged, and the close-out criteria in `change.md` are satisfied.

### What this enables

With `capability.yaml` owning phase briefs, new concerns ship as capabilities. None of these requires a core patch:


| Capability         | Capability behavior                                                                 |
| ------------------ | ----------------------------------------------------------------------------------- |
| `infra@v1`         | Infra skills can shell out to `terraform validate` and manage infrastructure plans. |
| `standards@v1`     | Generators and reviewers can cite adopted standards.                                |
| `design-tokens@v1` | Design-token skills can regenerate platform outputs and report drift.               |


None of these needs a core patch for domain-specific paths, merge behavior, or validators. Those mechanics live in the capability's briefs, skills, references, and helper scripts.

### Distribution: manifest plus skills

Capabilities ship a small manifest plus the skills and references that implement domain behavior. This keeps the core protocol small: the manifest declares brief flow; skills decide how to produce, validate, review, adopt, or clean up artefacts.

Skill-owned imperative code runs through the standard agent mechanisms: checked-in helper scripts, generated code, shell commands, package-manager tools, and language-specific toolchains invoked by the skill. The security posture is therefore the skill/tooling posture, not a second plugin trust model hidden behind `capability.yaml`.

#### Registry-materialised execution

When change execution materialises registry-declared projects, capability skills run relative to **the clone's project root**. `specify registry` supplies the normalized project-root mapping used by registry-aware change execution; the core receives only the project root it should run against.

## Alternatives Considered

- **Subprocess capability plugins.** Rejected because capability skills already own imperative behavior and already have mechanisms for invoking scripts, tools, and generated code. A second plugin runtime would duplicate the skill layer and introduce a separate trust model.
- **WASM-component plugins.** Rejected for the same reason as subprocess plugins; sandboxing imperative capability code belongs in the agent/tool execution model, not in `capability.yaml`.
- **In-process dynamic-library plugins.** Rejected because Rust ABI instability disqualifies them and because the capability protocol does not need a second imperative extension path.
- **Keep `specify workspace` as a core exception.** Rejected because it weakens the core boundary. Registry materialisation is first-party Specify behavior above core, not a core verb family.
- **Extract a standalone `workspace@v1` capability.** Rejected because materialisation is topology-driven substrate for change execution, not an outcome domain with capability-owned artefacts.
- **Split registry and workspace into separate components.** Rejected because they are two faces of the same domain: declared topology and its local materialised view. Keeping them separate also overloads "workspace" just as RFC-14 needs that noun for in-repo scopes.
- **Treat registry or change orchestration as capabilities.** Rejected because it recreates the "everything is extensible" monolith in a new vocabulary. Registry is topology plus local materialisation, and the change component is orchestration from operator brief through plan execution and close-out; each has a different lifecycle from capability-owned domain artefacts.
- **Split change orchestration and workflow into separate components.** Rejected because they are two faces of the same domain: operator intent and the executable graph that lands it. Keeping them separate gives the plan a false top-level identity, just as keeping workspace separate from registry gave materialisation a false top-level identity.
- **Multiple imperative escape hatches.** Rejected because capability skills are the single imperative escape hatch.

## Non-Goals

- **Replacing or capability-configuring the `define → build → merge` loop.** The loop's *shape* (phase set, transition DAG, per-phase outcome contract) is part of the immutable core. Capabilities declare which briefs run in the phases, but never the phases themselves. Variability lives in variable briefs per phase and skill-owned behavior. A capability that genuinely cannot fit any of those would justify proposing a *second* fixed loop shape as a peer to this one — never open-ended phase configuration.
- **Format-level contract evolution.** SemVer + `info.x-specify-id` + cross-repo uniqueness continue to be owned by RFC-12; this RFC only moves where the rules run from.
- **A new plugin runtime.** Imperative behavior remains in skills; this RFC only defines the declarative capability manifest.
- **A general sandboxed write-fence.** Capability skills run through the host tool execution model; this RFC does not add a core write-fence.
- **Cloud execution semantics.** Orthogonal; capability skills should run through whatever tool execution model the host provides.
- **Back-compat for capabilities without the new surface.** See §Migration — current usage footprint lets us cut over without a fallback path.
- **Third-party replacements for foundation components.** Registry and the change component are first-party Specify components in this RFC. Swapping them or making them externally pluggable is a follow-up RFC.
- **Multiple domain capabilities per repository.** Covered by [RFC-14](rfc-14-workspace.md), strictly additive on top of this RFC's capability manifest protocol.
- **Cross-capability slices in a single transaction.** Multi-capability outcomes are coordinated by change plan entries, not by one slice that writes multiple capabilities' baselines. RFC-14 applies the same rule to scopes: cross-scope work is a change plan with multiple entries, not a multi-scope slice.

## Glossary


| Term                              | Meaning                                                                                                                                                                              |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Active capability set             | The active domain capability for a project or scope. Platform components are outside this set.                                                                                       |
| Capability                        | A versioned Specify extension manifest that declares phase briefs.                                                                                                                   |
| Change                            | The umbrella orchestration concept: an operator-defined outcome that coordinates one or more slices through `change.md` + `plan.yaml`.                                               |
| Slice                             | The single unit that flows through the fixed `define → build → merge` loop: a per-project transaction with its own proposal, specs, design, tasks, and merge step.                   |
| Domain capability                 | The primary project capability such as `omnia@v1`, `contracts@v1`, or `vectis@v2`. RFC-14 adds multiple domain capabilities through scopes.                                          |
| First-party capability            | A domain capability bundled with the CLI release and resolved through the same manifest path as URL capabilities.                                                                    |
| Platform component                | A first-party Specify subsystem above core, such as `specify registry` or `specify change`. Platform components are not capabilities.                                                |
| Change component                  | `specify change`, the first-party component that owns operator brief, plan, orchestration state, execution, finalization, and archive. It is not a core runner and not a capability. |
| Registry materialisation resolver | The `specify registry` service that maps registry-declared projects to materialised project roots.                                                                                   |


## Implementation Scope

An incremental landing, each stage independently testable and shippable. Every stage preserves working `/spec:define → /spec:build → /spec:merge` for the `omnia` capability (the only capability currently in real use). The phases are delivery increments for this RFC, not separate RFCs.

Sizing guide:


| Phase                               | Expected size   | Acceptance focus                                                                                                                                         |
| ----------------------------------- | --------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1. Capability vocabulary cut-over   | ~400-700 lines  | Rename surfaces and diagnostics while preserving `pipeline:` behavior. Includes the `specify init` positional rename.                                    |
| 2. Component extraction             | ~700-1000 lines | Extract `specify registry` and `specify change` as platform components and delete concern-specific core type surfaces.                                   |
| 3. Lifecycle vocabulary cut-over    | ~400-700 lines  | Rename change → slice and initiative → change across CLI, crates, fixtures, and brief substitutions; ship the on-disk migrations operator projects need. |
| 4. First-party capability migration | ~500-900 lines  | Move concern-specific validation, generation, adoption, and cleanup behavior into first-party capability skills.                                         |


Estimated total: ~2000-3300 lines across `specify-cli`, schema updates, fixture refreshes, and plugin documentation.

### Phase 1 — Capability vocabulary cut-over

Lands the rename without changing artefact mechanics.

1. Rename the extension primitive in manifests and project config: `schema.yaml` → `capability.yaml`, `project.yaml:schema` → `project.yaml:capability`, and `specify schema {resolve,check,pipeline}` → `specify capability {resolve,check,pipeline}`.
2. Rename the schema/manifest crate and CLI help text where they refer to Specify extensions. JSON Schema remains JSON Schema.
3. Preserve `pipeline:` behavior byte-for-byte so the only behavior change in this phase is the vocabulary cut-over.
4. Update docs, fixtures, and diagnostics to use **capability** for Specify extensions and **schema** only for validation schemas.
5. Replace `specify init --schema-uri <uri>` with `specify init <capability>` as a required positional argument. Mutually exclusive with `--hub`: `specify init` with neither errors with `init-requires-capability-or-hub`; `specify init <capability> --hub` errors with the same diagnostic. The positional accepts the same value space `--schema-uri` accepts today (bare capability name or URL).

Acceptance: a canonical omnia slice still completes through `/spec:define → /spec:build → /spec:merge`; pre-cut-over manifests fail with a clear "schema has become capability" diagnostic; `specify init <capability>` and `specify init --hub` both succeed; and `specify init` invoked with neither (or with both) fails with `init-requires-capability-or-hub`.

### Phase 2 — Component extraction

Extracts the platform components without touching the change/slice noun yet, so the orchestration cut-over and the vocabulary cut-over can be reviewed independently.

1. **Extract `specify registry` as the topology and materialisation crate.** Move `registry.yaml` parsing, add/remove/show helpers, topology validation, clone/symlink resolution, dirty-state reporting, push, and merge support out of the schema/capability crate. Registry entries carry project id, repository URL, description, and default capability; they must not embed contract roles, change status, plan status, or validation findings.
2. **Keep `.specify/workspace/` as derived registry state.** The directory may continue to hold clones or symlinks, but its contents are the local materialised view of `registry.yaml`, not a separate component-owned topology.
3. **Extract `specify change` as the orchestration crate.** Change brief management, plan authoring, next-entry selection, locking, status updates, recovery, execution, finalization, and archive move here. `/spec:plan` and `/spec:execute` become change-surface commands or skills; any retained `/spec:plan` or `/spec:execute` spelling is only a temporary landing alias.
4. **Keep change helpers internal to change execution.** The change component may use skill-owned scripts or library helpers for next-entry selection, locking, status updates, and recovery. Generic per-loop-unit reads such as `specify change outcome show` stay core (renamed to `specify slice outcome show` in Phase 3).
5. **Delete concern-specific core type surfaces.** `Commands::{Vectis, Contract}` and the matching command modules stop being the place where capability validation and merge behavior live.
6. **Retire surviving hard-coded `contracts` / `specs` references** in core crates where they encode concern-specific behavior. The core may carry generic per-loop-unit layout helpers, but it must not decide contract or Vectis behavior by name.
7. **Initialization wires components, not active capabilities.** A project's `project.yaml` declares its domain capability; platform-component files (`registry.yaml`, `plan.yaml`, `change.md`) are scaffolded by their owning components, not by the capability resolver.

Acceptance: the core no longer exposes first-party capability command modules or public validation APIs, `specify registry` and `specify change` are independently testable crates, and the fixed `define → build → merge` lifecycle remains intact under the **pre-rename** noun set (`specify change` * for the per-loop unit, `.specify/changes/`, `$CHANGE_DIR`).

### Phase 3 — Lifecycle vocabulary cut-over

Renames the per-loop unit and the umbrella orchestration noun in one phase, and ships the on-disk migrations operator projects need to upgrade. Splitting this from Phase 2 keeps the orchestration extraction and the noun rename independently bisectable.

1. **Rename the per-loop-unit surface.** `specify change` * → `specify slice `*, `crates/change/` → `crates/slice/`, the `specify-change` library crate → `specify-slice`. Brief substitutions follow: `$CHANGE_DIR` → `$SLICE_DIR`. Outcome and journal helpers move with the rename: `specify change outcome show` → `specify slice outcome show`, `specify change journal append` → `specify slice journal append`.
2. **Rename the umbrella orchestration surface.** `specify initiative` * → `specify change `*, `initiative.md` → `change.md`, `specify-initiative` library crate → `specify-change`. The verb set (`create`, `plan`, `execute`, `finalize`, `archive`) is preserved across the rename.
3. **Add `specify migrate slice-layout`.** Renames `.specify/changes/` to `.specify/slices/` on disk and rewrites any in-tree `$CHANGE_DIR` substitutions in skill markdown to `$SLICE_DIR`. Idempotent; refuses to run when an in-progress per-loop-unit carries an unfinished phase (operator must finish or drop the in-progress unit before migrating).
4. **Add `specify migrate change-noun`.** Renames `initiative.md` to `change.md` at the repo root. Operator-facing platform artefacts (`registry.yaml`, `plan.yaml`, `change.md`, `contracts/`) remain at the repo root per RFC-9 §1B; this migration is purely the noun cut-over from initiative to change.
5. **Update fixtures, tests, and brief templates** to use the post-rename noun set. Post-rename docs never use "the change loop"; the per-loop unit is a *slice* and the umbrella is a *change*.

Acceptance: the new `change` orchestration surface and `slice` loop surface are unambiguous; running `specify migrate slice-layout` followed by `specify migrate change-noun` on a v1 omnia project produces a working post-RFC layout that completes a canonical slice end-to-end through `/spec:define → /spec:build → /spec:merge`.

### Phase 4 — First-party capability migration

Move domain mechanics into first-party capabilities.

1. **First-party domain capabilities publish their full surface** — `omnia`, `contracts`, and `vectis` declare `pipeline:`.
2. Move contract SemVer, `info.x-specify-id`, and cross-project validation behavior into the contracts capability's skills and helpers.
3. Move Vectis validation, generation, review, and cleanup behavior into Vectis skills and helpers.
4. Keep Omnia's canonical `/spec:define → /spec:build → /spec:merge` path working while the capability vocabulary lands.
5. Platform components publish their own file formats and command contracts separately.

Phase 4 may land as a sequence of smaller commits, but every commit keeps the `define → build → merge` lifecycle intact. Superseded concern-specific surfaces are not preserved as deprecated aliases after this phase ships.

### This repo (`augentic/specify`)

1. Add `capabilities/capability.schema.json` to cover `pipeline:`.
2. Rewrite `capabilities/{contracts,omnia,vectis}/capability.yaml` to use the capability vocabulary.
3. Keep path, validation, generation, adoption, and cleanup rules in the relevant capability skills and references.
4. Move `plugins/spec/skills/plan/` and `plugins/spec/skills/execute/` to the change surface; keep any `/spec:plan` or `/spec:execute` material as a compatibility shim only.
5. Update `plugins/contract/`, `plugins/vectis/`, and change-facing skills to own any imperative validation, generation, or review behavior that used to sit behind in-binary command modules.
6. Document the manifest protocol in `docs/reference/capabilities.md`; cross-link from each capability's README. Add companion references for `specify registry` and `specify change`, including registry materialisation behavior, change planning/execution behavior, and their dependency direction relative to core.
7. Update RFC-14 and any roadmap references that still use pre-RFC-13 extension vocabulary so follow-up work speaks in terms of capabilities, changes, and slices.

## Migration

Only the `omnia` capability and the core loop are in real-world use. `specify contract` *, `specify vectis`* , and the bulk of `specify plan|initiative|registry|workspace *` have no durable external user base to protect.

**Hard cut-over, no fallback path.** Each phase's minor version is a breaking change for the surfaces it touches. Pre-reframe capability manifests fail to load against the post-reframe CLI with a clear diagnostic pointing at this RFC and the capability rename. `/spec:plan` and `/spec:execute` are not retained as `spec` plugin responsibilities in the released post-RFC surface; temporary landing aliases, if used, delegate to the change surface and are removed before release.

**Hub project shape.** Hub projects (`specify init --hub`) write `hub: true` at the top level of `project.yaml` and **omit** `capability:`. The CLI treats `hub: true` as the sentinel that disables capability resolution; non-hub projects must declare `capability:`. The current `schema: hub, hub: true` sentinel is removed in the same release that lands Phase 1 — encoding the disabled state as the absence of a field keeps the §Migration invariant "the core never learns a capability name" intact.

### Migration TL;DR

Two vocabulary cut-overs land in this RFC: the **schema → capability** rename for the extension primitive, and the **change → slice** / **initiative → change** lifecycle rename. They may land in separate implementation phases, but each row below is a hard cut-over when its phase ships; no compatibility aliases are kept in the released surface.


| Current term / surface                                    | Post-RFC term / surface                                           |
| --------------------------------------------------------- | ----------------------------------------------------------------- |
| Schema (extension primitive)                              | Capability                                                        |
| `schema.yaml`                                             | `capability.yaml`                                                 |
| `project.yaml:schema`                                     | `project.yaml:capability`                                         |
| `specify schema {resolve,check,pipeline}`                 | `specify capability {resolve,check,pipeline}`                     |
| `specify init --schema-uri <uri>`                         | `specify init <capability>` (positional, required unless `--hub`) |
| `schemas/<name>/schema.yaml`                              | `capabilities/<name>/capability.yaml`                             |
| `project.yaml: { schema: hub, hub: true }` (hub sentinel) | `project.yaml: { hub: true }` (`capability:` omitted)             |
| Change (single per-loop unit)                             | Slice                                                             |
| Initiative (umbrella orchestration)                       | Change                                                            |
| `specify change` * (per-loop unit)                        | `specify slice` *                                                 |
| `specify initiative` *                                    | `specify change` *                                                |
| `.specify/changes/`                                       | `.specify/slices/`                                                |
| `initiative.md`                                           | `change.md`                                                       |
| `specify-initiative` (crate / component)                  | `specify change`                                                  |
| `specify change` (crate, slice loop)                      | `specify-slice`                                                   |
| `crates/change/`                                          | `crates/slice/`                                                   |
| `$CHANGE_DIR` (brief substitution)                        | `$SLICE_DIR`                                                      |


JSON Schema remains JSON Schema. `*.schema.json` continues to name validation schemas, not Specify capabilities.

The `change → slice` / `initiative → change` rows reuse the noun "change" with a new meaning. Inside a Specify project the post-cut-over reading is unambiguous: a *change* is the operator-defined umbrella, a *slice* is what flows through `define → build → merge`, and "the change loop" no longer exists as a phrase — call it the *slice loop*.

Four invariants guard the landing:

1. **Omnia keeps working.** Every phase's acceptance criterion includes running `/spec:define → /spec:build → /spec:merge` on a canonical omnia slice end-to-end.
2. **The core never learns a capability name.** `specify check` rejects hard-coded capability-name literals in core crate sources outside tests, including first-party domain capability names after extraction.
3. **Concern-specific behavior leaves core.** A companion rule rejects hard-coded first-party capability behavior in core crate sources outside tests; phase 2 retires the current canonical violations.
4. **Platform components stay outside the active capability set.** A rule verifies `specify-core` does not depend on `specify registry` or `specify change`; dependency direction flows from the change component down to registry/core, never the reverse.

The hard-coded-name lints are RFC-5 design work, not a naive string-literal ban. RFC-5 should define the crate allowlist, generated-code exemptions, test exemptions, and AST-aware matching needed to avoid flagging unrelated prose or diagnostics.

Linter rules in `specify-check` (RFC-5) enforce, additionally:

- **First-party capability parity:** bundled domain capabilities pass every rule URL-resolved capabilities must pass.

## Open Questions

1. **Structured merge diagnostics shape.** §Merge and adoption contract pins the protocol to the existing `specify slice outcome set` + `specify slice journal append` surfaces; capability diagnostics round-trip as opaque journal entries. A future RFC may add a structured-findings shape so the core can render capability diagnostics richly without parsing free-form journal text. Provisional: opaque journal entries are sufficient for the bundled `omnia`, `contracts`, and `vectis` capabilities; revisit if a third-party capability needs richer surfacing.
2. `**specify migrate slice-layout` and in-progress per-loop units.** The migration refuses to run when a per-loop unit is mid-phase (operator must finish or drop it first). A future release may extend it to migrate in-progress work by rewriting `.metadata.yaml` and re-stamping journal entries. Provisional: require operators to finish or drop in-progress work first — the canonical loop is short enough that this is rarely costly.
3. **Hub project discriminator.** This RFC pins `hub: true` (with `capability:` omitted) as the post-RFC hub sentinel. An alternative `kind: { package, hub }` discriminator would be more symmetric with RFC-14's `package: / workspace:` shape. Defer to RFC-14: if RFC-14 lands the workspace shape it can revisit the hub spelling under the same discriminator.
4. **First-party capability binary distribution.** Phase 4 moves contracts SemVer + `info.x-specify-id` validation into capability skills, but does not pin whether the validator ships as a Rust binary, a TypeScript checker, or a shell script. Provisional: capability authors choose; this RFC only requires that the validator is invokable from the merge brief and reports through the §Merge and adoption contract protocol.
   Disposition: resolved by [RFC-15](rfc-15-wasm-plugins.md), which standardizes declared WASI tools run through `specify tool`.

## References

- [RFC-1: `specify` CLI](rfc-1-cli.md) — owns the crates the reframe touches (`specify-schema`, `specify-merge`, `specify-validate`, and `specify change`; the slice loop crate is renamed to `specify-slice` in this RFC) and the `src/cli.rs` dispatcher.
- [RFC-8: API contracts](rfc-8-api-contracts.md) — `contracts@v1` capability; contract validation and adoption behavior move into capability skills.
- [RFC-2: Execution](rfc-2-execution.md) — `/spec:execute --loop`; informs the `specify change` extraction, but this RFC does not change the lifecycle model.
- [RFC-3a: Monoliths](rfc-3a-monoliths.md) — plan authoring pipeline; informs change plan authoring.
- [RFC-3b: Platform](rfc-3b-platform.md) — registry routing and materialised project clones.
- [RFC-9: Platform](rfc-9-platform.md) — moved registry, plan, initiative, and contracts to repo root; informs change-driven orchestration.
- [RFC-12: Refine RFC-8](rfc-12-refine-rfc-8.md) — SemVer + `info.x-specify-id` rules become contracts capability validation behavior.
- [RFC-5: Framework Linter](../rfc-5-lint.md) — home of the lints enforcing the reframe's invariants, including the hard-coded-name lint design.
- [Roadmap](../roadmap.md) — §5 / §6 / §7 are consumers of a stable core surface.

