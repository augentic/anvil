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

**Decision:** The system is structured in three layers, each independently useful. Higher layers invoke lower layers but lower layers are unaware of what sits above them. Underneath all of them is the `specify` CLI — the deterministic substrate that exposes verbs at every layer; the CLI is not itself a layer.

1. **Layer 0 — Configuration.** Static project settings and the verbs that change them: `.specify/project.yaml`, `capability.yaml`, `schemas/`, `tools.yaml`, `specify init`, `specify capability`.
2. **Layer 1 — Executing a change.** The single-slice define-build-merge loop: `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`, `/spec:extract`, and the `specify slice *` verbs they wrap.
3. **Layer 2 — Planning a change.** Anything that impacts or uses `registry.yaml` and `plan.yaml`: `/change:plan`, `/change:execute`, the `/change:plan <name> orchestrate` umbrella mode, `/spec:analyze`, and the `specify change *` / `specify change plan *` / `specify registry *` / `specify workspace *` verbs they wrap.

**Rationale:** Not every use case needs automation. A single slice needs only Layer 1. A small change can be driven manually with the matching CLI verbs. Plan/execute automation (Layer 2) composes on top of Layer 1, and the cross-repo umbrella mode is a composition inside Layer 2 — every step shells out to a CLI verb or a Layer 2 skill in default mode. This means you can always drop down a layer when automation fails — see [Drop down a layer](../how-to/drop-down-a-layer.md).

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

**Decision:** The four operator-facing platform artifacts -- `registry.yaml`, `plan.yaml`, `change.md`, `contracts/` -- live at the repo root. Generated `AGENTS.md` guidance also lives at the root, with Specify owning only its fenced block. `.specify/` retains framework-managed state: `project.yaml`, `context.lock`, `slices/`, `specs/`, `archive/`, `.cache/`, `workspace/`, `plans/`, and the advisory `plan.lock`.

**Rationale:** The operator-facing artifacts (the registry, the operator brief, the plan, contracts) are durable, PR-reviewed, human-edited material. Putting them under a dot-prefixed framework directory understates their importance and forces operators to navigate framework internals to inspect or hand-edit them. Keeping them at the root makes the boundary explicit: framework owns `.specify/`; operators own everything else.

**Source:** specify-cli [`DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) (v2 layout entry).

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

## Planning briefs ship with the skill, not the capability manifest

**Decision:** The planning briefs (`discovery`, `propose`) live alongside the `/change:plan` skill under `plugins/change/skills/plan/briefs/<capability>/` rather than under `capability.yaml:pipeline.plan`. The capability manifest schema actively rejects a `pipeline.plan` block.

**Rationale:** Planning is orchestration, not capability-owned slice work. A capability decides what define/build/merge produces inside an individual slice; it does not decide how a *change* (potentially spanning many slices and projects) gets composed. Putting plan briefs in the capability manifest blurred that boundary and forced every capability to ship near-duplicate plan briefs. Keeping the briefs with the plan skill keeps the framework concern at the framework, with capabilities free to ship their own plan-time variants by name (`briefs/<capability>/`).

**Source:** [RFC-13: Extensibility](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-13-extensibility.md)

## Capability vs `--hub` is mutually exclusive at init

**Decision:** `specify init` accepts either a capability positional or `--hub`, never both and never neither. A regular project carries a `capability:` in `.specify/project.yaml`; a hub carries `hub: true` and never carries a `capability:`. The CLI rejects the two pathological combinations (no positional + no `--hub`, or both supplied) with the stable `init-requires-capability-or-hub` diagnostic.

**Rationale:** Hubs are registry-only repositories that never run phase pipelines, so they have no capability to resolve. Allowing an empty capability would force every downstream verb to special-case the missing field; allowing both would double the topology surface. The mutual-exclusion is mechanically enforced at init so every later verb can rely on the invariant without re-checking.

**Source:** [RFC-9: Platform](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-9-platform.md), [RFC-13: Extensibility](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-13-extensibility.md)

## Platform components are not capabilities

**Decision:** The registry and the change orchestrator are first-party **platform components**, not capabilities. They have commands, libraries, and files, but they never appear in any `capability.yaml`, never participate in the manifest protocol, and are never activated through a capability-name switch. The dependency direction is fixed at the crate level: `specify-core` does not depend on `specify-registry` or `specify-change`, and `specify-registry` does not depend on `specify-change`.

**Rationale:** Treating the registry and change orchestration as capabilities created a circular activation problem — the surface that decides which capabilities are active was itself a capability. Promoting them to platform components keeps capability composition strictly downward and means a capability author never has to think about whether the registry or change loop is "available." The hard-coded crate dependency direction makes the invariant a build-time guarantee rather than a convention.

**Source:** [RFC-13: Extensibility](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-13-extensibility.md)

## Operator owns PR merge; Specify prepares and publishes

**Decision:** Specify materialises workspace slots, prepares the `specify/<change-name>` branch before phase writes, accumulates a baseline commit from `/spec:merge` and a residue commit from `/change:execute`, and pushes the branch through `specify workspace push`. PR review and merge happen through the forge UI, `gh pr merge`, or the team's normal merge queue. The framework never inspects checks or calls `gh pr merge` itself, and `specify change finalize` only verifies that each PR is already merged before archiving the plan. `specify workspace merge` is removed.

**Rationale:** Automated PR merge couples the framework to forge APIs, check-suite semantics, and team-specific review rules that vary across operators. Holding the framework at "prepare and publish" lets every team layer its own merge policy — checks, reviewers, merge queue, manual approval — without the framework modelling any of it. The split also gives a natural rollback surface: an unmerged PR can be closed or rebased without rewinding any framework state.

**Source:** [RFC-14: Workspace](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-14-workspace.md)

## Declared WASI capability tools

**Decision:** Helper tools shipped by capabilities or projects are declared as WASI command components in a `tools.yaml` sidecar (capability scope) or in `.specify/project.yaml` (project scope), and run through a single CLI surface — `specify tool {list, fetch, show, run}`. Project scope wins on collision, so an operator can redirect a capability-shipped tool to a local build or pinned mirror without editing the capability. Permissions are directory preopens, not globs; the host canonicalises every path and rejects `..` segments, glob metacharacters, symlink escapes, and direct writes to Specify lifecycle state. Released first-party tool declarations use exact `specify:*@<semver>` package requests resolved through wasm-pkg metadata.

**Rationale:** Capabilities used to extend the framework either by adding more in-binary CLI verbs or by shelling out to host binaries the operator had to install separately. Both paths broke on every CLI release: in-binary verbs grew the host surface unboundedly; host binaries diverged in version, permissions, and discoverability across machines. WASI command components keep the helpers sandboxed and deterministic while making them data — the host fetches them, the host enforces the preopens, the host caches them — so a capability can ship behavior without growing the host.

**Source:** [RFC-15: WASM Plugins](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-15-wasm-plugins.md)

## One `specify` binary; capability-specific helpers ship as declared tools

**Decision:** Operators install one binary — `specify`. The deterministic Vectis helpers (validation and scaffold rendering) ship as WASI tools declared by `capabilities/vectis/tools.yaml`. Host post-processing for Vectis projects (Cargo, Gradle wrapper bootstrap, Xcode and `make typegen` / `make package` / `make xcode`, `local.properties`, Java home and NDK detection, prerequisite checks, registry queries, cap-matrix verification) lives in Vectis skills as ordinary shell commands the agent runs and journals.

**Rationale:** A separate capability-specific binary would double the install, packaging, release, and version-coordination surface for every capability that needs helpers. Applying the declared-tool model from RFC-15 keeps the surface to one binary and keeps the "deterministic rendering" layer cleanly separated from the "host toolchain" layer, which never belongs inside a WASI wrapper.

**Source:** [RFC-16: WASI Vectis](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-16-wasi-vectis.md)

## SemVer `info.version` and rename-stable `info.x-specify-id`

**Decision:** Every top-level OpenAPI 3.1 and AsyncAPI 3.0 document under `contracts/` MUST set `info.version` to a value that parses per [semver.org](https://semver.org), including optional prerelease labels. Every top-level contract MAY set `info.x-specify-id` to a kebab-case slug (`^[a-z][a-z0-9-]*$`, ≤64 characters, repo-unique) that survives file moves and `info.version` bumps. Path-based references in `registry.yaml` remain canonical — the id is a hint, not a substitute. Bump rules (when to advance major / minor / patch) remain skill-side judgement.

**Rationale:** Pre-RFC-12 contracts used a mix of `YYYY-MM-DD` dates and bare majors as `info.version`, which prevented any tooling from comparing two contract versions programmatically. Requiring SemVer aligns contract evolution with the broader ecosystem (`progenitor`, `typify`, `schemars`) and makes producer/consumer compatibility classification (`specify compatibility`) decidable. The optional rename-stable id captures the identity of a contract independent of its file location, so a file move or a major-version rename does not look like deletion-plus-creation to baseline diff tools.

**Source:** [RFC-12: Refine RFC-8](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-12-refine-rfc-8.md)

## SKILL.md discipline cleanup (2026-05)

**Decision:** Five mechanical predicates lock down the SKILL.md shape that operators read every day:

- **Description grammar** (S1) — `checkDescriptionStartsWithVerb` + `checkDescriptionHasUseWhen` enforce a leading imperative verb (curated allow-list in `scripts/checks/skill_frontmatter.ts`) and a `Use when …` clause, in addition to the pre-existing `checkDescriptionLength` ≤ 512 char cap.
- **Section line cap** — `checkBodyAndSectionLineCounts` caps each H2 section at 45 lines (non-blank, non-comment). Depth migrates into `references/<topic>.md` instead of letting individual sections sprawl.
- **Argument-hint grammar** (S3) — `checkArgumentHintGrammar` accepts only `<name>`, `[name]`, trailing `...`, `<a|b>` / `[a|b]`, and `--flag` tokens (kebab-case names). Bare prose, mixed punctuation, and short flags are rejected.
- **Envelope-example forbid** (S5) — `checkNoEnvelopeExamples` flags fenced ```json``` blocks whose body looks like a CLI envelope wrapper. Envelope shapes live with stable anchors in `plugins/references/cli-output-shapes.md`; SKILL.md bodies link instead of embed.
- **Vocabulary / guardrails consolidation** (S4) — cross-cutting guardrails (the recurring `.metadata.yaml` / slice-dir / plan-write rules) live in `plugins/references/guardrails.md`; SKILL.md files link, not restate. Pre-1.0 sweep dropped "previously / migrate / backward-compat" prose that did not document a real legacy-migration feature, and stale CLI names were replaced in docs (`initiative` → `change`, `JSON_SCHEMA_VERSION` → `ENVELOPE_VERSION`).

**Rationale:** The skill-discovery surface needs to match on intent ("does this skill apply to my task?") rather than vocabulary, which means the description and argument-hint shapes have to be mechanically tight. The body and section caps push depth into `references/` so a SKILL.md stays an orientation artifact. The envelope-example forbid pins one place where the wire shape can drift. The pre-1.0 vocabulary stance — no backward compatibility constraints, no migration prose unless documenting a real legacy-migration feature — keeps the skill bodies honest about what the system does today rather than how it was built.

**Source:** Cleanup chunks S1–S5 on the `code-review` branch.

## CLI verb renames (RFC-9 / RFC-13)

**Decision:** Several CLI verb groups were renamed during the RFC-9 → RFC-13 cutover so the top-level surface matches the operator-facing nouns (slice / change / registry / workspace / capability):

- `specify change *` (per-slice verbs) → `specify slice *` (RFC-13 §3.2).
- `specify change *` (umbrella verbs) → `specify change *` (RFC-13 §3.5); `specify change create` was renamed from the v1 `specify change init`.
- `specify schema {resolve, check, pipeline}` → `specify capability {resolve, check, pipeline}` (RFC-13 §Migration).
- `specify registry {add, remove}` were added by RFC-9 §2A and both validate the resulting shape after the write.
- The pre-RFC-13 in-binary `specify contract { list, validate }` family was retired in chunk 2.7 when contracts became a first-party capability owning its own validation behaviour; the contracts merge brief now shells out through `specify tool run contract` as the post-merge baseline gate (RFC-15).
- `specify init --hub` (RFC-9 §1D) is the mutually exclusive alternative to `specify init <capability>` — it scaffolds a registry-only platform hub whose `project.yaml` carries only `hub: true`.
- `specify workspace merge` has been removed; operators merge through the forge UI or `gh pr merge`, then `specify change finalize` verifies remote PR state.

**Rationale:** Specify is pre-1.0 and the wire/CLI surface is allowed to evolve. Capturing the rename trail here keeps `AGENTS.md` free of "renamed from … by RFC-N" parentheticals while preserving the trail for anyone tracing a stale call site.

**Source:** [RFC-9](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-9-platform.md), [RFC-13](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-13-extensibility.md), [RFC-15](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-15-wasi-tools.md).
