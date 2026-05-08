# Decision Log

Key architectural decisions in Specify, distilled from the design RFCs. Each entry explains the *why* behind a design choice. For full context, follow the links to the original RFCs.

## CLI owns correctness, agent owns judgment

**Decision:** All deterministic operations (validation, lifecycle transitions, spec merging, task parsing, plan management) run through the `specify` CLI. Skills never hand-edit `.metadata.yaml` or manipulate the `.specify/` directory directly.

**Rationale:** LLM-interpreted prose rules for structured operations (validation, task parsing, directory manipulation) produced unreliable results. A binary that returns structured JSON and exit codes gives deterministic correctness where it matters, while the agent retains judgment for semantic decisions.

**Litmus test:** "Would this operation need to understand `.specify/` directory structure or spec format?" If yes, it belongs in the CLI. If no (like running `cargo test`), it stays with the agent.

**Source:** [RFC-1: specify CLI](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-1-cli.md)

## Pass/Fail/Deferred validation

**Decision:** The validation engine classifies checks into three outcomes: Pass (check passed), Fail (must fix), Deferred (requires semantic judgment, flagged for agent review).

**Rationale:** Some checks are purely structural (file exists, format correct) and can be answered definitively by the CLI. Others require understanding context ("is this design adequate?"). The three-way classification lets the CLI handle what it can and explicitly flags what needs agent judgment, rather than pretending everything is binary.

**Source:** [RFC-1a: Deferred Validation](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-1a-validation.md)

## Independently useful layers

**Decision:** The system is structured in four layers, each independently useful. Higher layers invoke lower layers but lower layers are unaware of what sits above them.

1. **Layer 1 — CLI primitives.** Deterministic verbs (`specify slice`, `specify change plan`, `specify change`, `specify registry`, `specify workspace`, `specify capability`).
2. **Layer 2 — Slice lifecycle.** The define-build-merge skills that operate on a single slice.
3. **Layer 3 — Plan & Drive.** `/change:plan` authors `plan.yaml`; `/change:execute` runs it.
4. **Layer 4 — Change orchestration.** `/change:plan <name> orchestrate` composes Layers 1-3 plus `specify workspace push`, operator PR merge, and `specify change finalize` into a single operator action.

**Rationale:** Not every use case needs automation. A single slice needs only Layer 2. A small change can be driven manually with Layer 1 CLI commands. Plan/execute automation (Layer 3) composes on top, and the cross-repo umbrella (Layer 4) composes on top of that. This means you can always drop down a layer when automation fails — see [Drop down a layer](../how-to/drop-down-a-layer.md).

The original three-layer stack (Layers 1–3) was introduced by RFC-2; Layer 4 was promoted from "an aggregator inside Layer 3" to its own layer by RFC-9 §2C because the umbrella verb is a strict superset of the plan/execute layer and giving it a dedicated layer keeps the operator-facing entry-point per layer canonical.

**Source:** [RFC-2: Execution](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-2-execution.md), [RFC-9 §2C: Change umbrella](https://github.com/augentic/specify/blob/main/rfcs/rfc-9-platform.md)

## Plan as a data file, not a configuration

**Decision:** The plan (`plan.yaml`) is an ordered list of changes with status, not a pipeline configuration. There is no planning configuration file. The internal flow of `/change:plan` is fixed.

**Rationale:** Configurability adds a debugging surface ("why did step X run?") before the system is well-understood. A fixed flow with no config is easier to reason about, and configurability can be added later without migration.

**Source:** [RFC-3a: Monolith Migration Planning](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-3a-monoliths.md)

## Analyze/extract split

**Decision:** Plan-time capability discovery (`/spec:analyze`) is separate from define-time deep extraction (`/spec:extract`). Analyze scans the whole source cheaply; extract runs deeply against a per-slice slice.

**Rationale:** A large monolith cannot be fully extracted in one pass -- it would be too slow and expensive. The two-skill split makes large migrations tractable: cheap scanning builds the inventory, deep extraction happens per-slice where it is focused and affordable.

**Source:** [RFC-3a: Monolith Migration Planning, Large-Monolith Decomposition](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-3a-monoliths.md)

## Registry-driven multi-repo planning

**Decision:** Multi-repo coordination uses a `registry.yaml` platform catalogue and an automatic sync-peers phase, not a configuration DSL or federation protocol.

**Rationale:** The same `/change:plan <name>` command should work unchanged from one repo to 100+. The registry adds the minimum information needed (what repos exist, what capability they use, what domain they own). Sync-peers runs automatically when the registry has multiple projects, and not at all for single-repo work. No new user-facing concepts for the common case.

**Source:** [RFC-3a: Monolith Migration Planning](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-3a-monoliths.md), [RFC-3b: Platform Changes](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-3b-platform.md)

## CWD-based routing for multi-repo execution

**Decision:** The execute driver routes each change to its target project by changing working directory to the workspace clone before invoking phase skills. Phase skills (`/spec:define`, `/spec:build`, `/spec:merge`) are completely unaware of multi-repo routing -- they run unmodified in whatever directory the driver places them in.

**Rationale:** The alternative (passing a `--project` flag through to every phase skill) would have required changes to every skill and every brief pipeline. CWD-based routing keeps the routing decision in one place (the driver) and preserves the invariant that phase skills operate on "the current project." Phase skills discover the capability via their normal `.specify/project.yaml` walk from CWD.

**Source:** [RFC-3b: Platform Changes, §Execution routing](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-3b-platform.md)

## One plan entry, one project

**Decision:** Each plan entry targets exactly one registry project. Capabilities that span multiple repos are decomposed into separate slices (one per project) linked by `depends-on` edges.

**Rationale:** Allowing a single slice to span repos would require the execution loop to manage multiple project roots, multiple capabilities, and multiple baseline merge targets within one define-build-merge cycle. Decomposing cross-cutting capabilities into per-project entries keeps the loop simple and matches the existing baseline-accumulation model where each merge has a single target.

**Source:** [RFC-3b: Platform Changes, §One change, one project](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-3b-platform.md)

## Project assignment is a framework concern

**Decision:** Inferring which registry project each plan entry targets (the assignment step) runs in the plan skill at the framework level, not inside capability-owned propose briefs. Propose creates entries without `--project`; the plan skill's assignment step writes the routing after propose completes.

**Rationale:** A multi-repo plan spans projects with different capabilities, so assignment is inherently a cross-capability concern. Placing it in individual propose briefs would duplicate the logic across capabilities and create an ordering problem (the brief would need to know about projects it does not own). Keeping it in the plan skill also means propose briefs are unchanged -- a single-repo propose brief works identically in a multi-repo plan.

**Source:** [RFC-3b: Platform Changes, §Assignment algorithm](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-3b-platform.md)

## Workspace-centric execution with explicit push

**Decision:** All multi-repo execution happens inside workspace clones under the initiating repo's `.specify/workspace/`. Local commits from merge accumulate in the clones. Changes are published to remotes only when the operator explicitly runs `specify workspace push`.

**Rationale:** Automatic pushes during execution would make the driver non-idempotent and create a rollback problem -- a failed change that was already pushed cannot be cleanly undone. Keeping pushes explicit gives the operator a review gate between "execution produced artifacts" and "artifacts are published." The workspace is the staging area; `workspace push` is the release gate.

**Source:** [RFC-3b: Platform Changes, §Workspace-centric execution](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-3b-platform.md)

## Composition as a separate artifact, not embedded in specs or design

**Decision:** Introduce `composition.yaml` as a new Vectis-specific artifact that describes spatial screen layout, rather than extending specs or design with layout concerns.

**Rationale:** Specs define observable behavior ("the user sees their todo items"); they should not specify how items are arranged on screen. Design defines the type system; embedding layout in design would make it responsible for both data shape and visual arrangement. A separate artifact preserves the existing separation of concerns: specs drive the core, design defines the type contract, and composition drives the shell. This also enables multi-source authoring -- Figma adapters, legacy extractors, and manual editing can all produce composition artifacts without touching specs or design.

**Source:** [RFC-7: View Layout Artifact](https://github.com/augentic/specify/blob/main/rfcs/rfc-7-ui.md)

## YAML for composition, markdown for specs

**Decision:** The composition artifact uses YAML (`composition.yaml`) rather than markdown, despite all other define-phase artifacts being markdown.

**Rationale:** Layout is fundamentally structural data -- a tree of components with properties. Shell writers and the validation CLI consume it programmatically against a JSON Schema. A markdown representation would require pattern-matching on indented lists to reconstruct the component tree -- fragile and impossible to schema-validate. YAML also aligns with `tokens.yaml` as a structured design-layer artifact and enables same-format diffing for re-imports from design tools.

**Source:** [RFC-7: View Layout Artifact, §Why YAML](https://github.com/augentic/specify/blob/main/rfcs/rfc-7-ui.md)

## Screen-level delta merge for composition

**Decision:** Composition deltas operate at the screen level (`added`/`modified`/`removed` per screen), with `modified` performing full screen replacement rather than region-level or item-level merging.

**Rationale:** Merging independently edited region structures at the item level would require positional diff logic with ambiguous conflict resolution. Full-screen replacement is simple, predictable, and sufficient because the define pipeline always produces complete screen entries. Per-screen SHA-256 checksums in `.composition-checksums.yaml` provide conflict detection when two changes modify the same screen.

**Source:** [RFC-7: View Layout Artifact, §Delta Operations](https://github.com/augentic/specify/blob/main/rfcs/rfc-7-ui.md)

## Contracts as platform-level artifacts, not per-project

**Decision:** API contracts live at `contracts/` alongside `registry.yaml` and `plan.yaml`, not nested inside any project's capability tree or spec directory.

**Rationale:** An API contract is a shared agreement between parties -- it does not belong to the producer any more than to the consumer. Nesting contracts inside a single project's capability tree misattributes ownership and forces consumers to navigate workspace clones to find the producer's contract files. Co-locating contracts with `registry.yaml` makes the neutrality structural: `registry.yaml` declares *who* the participants are, `plan.yaml` declares *what* changes are planned, and `contracts/` declares *how* participants communicate. This mirrors established industry practice (proto repos, shared OpenAPI spec repos, contract-first design).

**Source:** [RFC-8: API Contracts](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-8-api-contracts.md)

## Platform artifacts at the repo root, framework state under `.specify/`

**Decision:** The four operator-facing platform artifacts -- `registry.yaml`, `plan.yaml`, `change.md`, `contracts/` -- live at the repo root. `.specify/` retains only framework-managed state: `project.yaml`, `slices/`, `specs/`, `archive/`, `.cache/`, `workspace/`, `plans/`, and the advisory `plan.lock`. The CLI ships one-shot migrations to upgrade existing projects in place and refuses every project-aware verb on a v1-layout project with the stable `legacy-layout` error code (hard cutover, no transition window).

**Rationale:** `.specify/` started life as workflow scratch -- cache, archive, working changes, lifecycle metadata. The artifacts that have accreted there since (the registry, the operator brief, the plan, contracts) are durable, PR-reviewed, human-edited material. Putting them under a dot-prefixed framework directory understated their importance and forced operators to navigate framework internals to inspect or hand-edit them. Pulling them up to the root makes the boundary explicit: framework owns `.specify/`; operators own everything else. The hard-cutover stance avoids carrying a dual-read code path indefinitely; the migrate verb is a one-line operator action that addresses the upgrade in a single step.

**Source:** specify-cli [`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) (v2 layout entry); operator walkthrough at [`docs/how-to/migrate-to-v2-layout.md`](../how-to/migrate-to-v2-layout.md).

## JSON Schema + OpenAPI + AsyncAPI, not a new IDL

**Decision:** The contract format uses JSON Schema as the shared payload vocabulary with OpenAPI 3.1 and AsyncAPI 3.0 as protocol-specific bindings. No proprietary schema language is introduced.

**Rationale:** JSON Schema is the common denominator -- both OpenAPI 3.1 and AsyncAPI 3.0 use it for payload definitions. Defining domain types as JSON Schema files means both protocol bindings reference a single source of truth. The Rust code generation ecosystem (`schemars` + `typify`, `progenitor`) can consume these artifacts directly. Introducing a proprietary format or a less common IDL (Smithy, Protobuf) would narrow the ecosystem without clear benefit.

**Source:** [RFC-8: API Contracts](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-8-api-contracts.md)

## Opaque replacement for contract merge

**Decision:** Contract files use opaque file replacement during merge -- the entire file is replaced rather than delta-merged. Unlike spec files (which use ADDED/MODIFIED/REMOVED sections), contract files are replaced wholesale.

**Rationale:** JSON Schema and OpenAPI/AsyncAPI files have their own versioning semantics (`$id`, `info.version`). Introducing a second delta-merge algorithm for YAML contract files would add complexity without clear benefit over replacement. Two concurrent changes that modify the same contract file are caught by `specify slice merge conflict-check` (baseline modification after the change's `defined-at` timestamp), and the resolution is to re-run the define phase against the updated baseline.

**Source:** [RFC-8: API Contracts](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-8-api-contracts.md)

## Stable requirement IDs as merge keys

**Decision:** Each behavioral requirement has a stable `ID: REQ-XXX` line that serves as the merge key across delta specs. Requirement titles may change; IDs must not.

**Rationale:** When specs evolve over multiple changes, the system needs a way to match "this modification applies to that requirement." Titles are human-facing and change frequently. Stable IDs give the merge engine a reliable key while keeping the spec format readable.

## Capability-agnostic lifecycle, capability-specific briefs

**Decision:** The lifecycle (states, transitions, core artifacts, baseline accumulation) is invariant across capabilities. Capabilities control the *content* of brief pipelines, may add capability-specific stages (e.g. Vectis adds `composition` to the define pipeline), and determine which specialist skills are invoked during build.

**Rationale:** The workflow is the value -- define-build-merge, baseline accumulation, drift detection. Making this capability-agnostic means every project gets the same tooling regardless of target platform. Capabilities customise the generation content and may extend the pipeline without fragmenting the workflow.
