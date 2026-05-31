# Decision Log

Key architectural decisions in Specify, distilled from the design RFCs. Each entry explains the *why* behind a design choice. For full context, follow the links to the original RFCs.

## CLI owns correctness, agent owns judgment

**Decision:** All deterministic operations (validation, lifecycle transitions, spec merging, task parsing, plan management) run through the `specify` CLI. Skills never hand-edit `.metadata.yaml` or manipulate the `.specify/` directory directly.

**Rationale:** LLM-interpreted prose rules for structured operations (validation, task parsing, directory manipulation) produced unreliable results. A binary that returns structured JSON and exit codes gives deterministic correctness where it matters, while the agent retains judgment for semantic decisions.

**Litmus test:** "Would this operation need to understand `.specify/` directory structure or spec format?" If yes, it belongs in the CLI. If no (like running `cargo test`), it stays with the agent.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Pass/Fail/Deferred validation (superseded by the shared `Diagnostic` substrate)

**Decision (superseded):** The validation engine classified checks into three outcomes: Pass (check passed), Fail (must fix), Deferred (requires semantic judgment, flagged for agent review).

**Superseded by:** Every check surface now emits one neutral `Diagnostic` / `DiagnosticReport` currency with two orthogonal axes — `source` (`deterministic | model-assisted | hybrid | human | tool`) and `kind` (`violation` vs `review`). A clean check simply emits no `violation` finding; the former `Deferred` outcome is now `kind: review` (a deterministically-raised request for agent judgment), and lint's `lint-mode: model-assisted` rules surface as `review` too. "Needs judgment" became a first-class, queryable concept across both surfaces rather than a per-validate enum value. See the CLI repo's [DECISIONS.md §"Drained `Error::Validation` and the `Diagnostic` substrate"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#drained-errorvalidation-and-the-diagnostic-substrate).

**Rationale (unchanged):** Some checks are purely structural (file exists, format correct) and can be answered definitively by the CLI. Others require understanding context ("is this design adequate?"). The split lets the CLI handle what it can and explicitly flags what needs agent judgment, rather than pretending everything is binary — now expressed via the `kind` axis instead of a dedicated validation-only enum.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Lint and validate share a substrate but stay distinct surfaces

**Decision:** `lint` and `validate` are unified onto one `Diagnostic` substrate (data type, fingerprint, validator, renderer, blocking predicate) but remain conceptually distinct surfaces with different authority. `validate` gates a lifecycle transition (`refining → refined`): workflow-owned, non-negotiable, non-silenceable. `lint` is standards/policy compliance: codex-owned, versioned, lifecycle-neutral (may block CI, never transitions a slice), silenceable with an in-source rationale.

**Rationale:** The analogy is LSP — `rustc`, `clippy`, and `rust-analyzer` all emit one `Diagnostic` with a `source` without becoming the same tool. Convergence applies to the substrate, never to the concepts or their gate policies. The neutrality is encoded one layer down at the crate graph: the shared machinery lives in a neutral `specify-diagnostics` leaf, and the litmus test is that `validate` (or any non-lint producer) must not depend on anything named `lint`. A uniform blocking predicate (`kind == violation && status == open && severity ∈ {critical, important}`) serves both surfaces; they differ only in whether ignore directives apply (lint: yes; validate: no).

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Independently useful layers

**Decision:** The system is structured in three layers, each independently useful. Higher layers invoke lower layers but lower layers are unaware of what sits above them. Underneath all of them is the `specify` CLI — the deterministic substrate that exposes verbs at every layer; the CLI is not itself a layer.

1. **Layer 0 — Configuration.** Static project settings and the verbs that change them: `.specify/project.yaml`, `adapter.yaml`, `schemas/`, `tools.yaml`, `specrun init`, `specify adapter`.
2. **Layer 1 — Executing a change.** The single-slice refine-build-merge loop: `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`, and the `specrun slice *` verbs they wrap.
3. **Layer 2 — Planning a change.** Anything that impacts or uses `registry.yaml` and `plan.yaml`: `/spec:plan`, `/spec:execute`, `/spec:finalize`, and the `specrun plan *` / `specrun registry *` / `specrun workspace *` verbs they wrap.

**Rationale:** Not every use case needs automation. A single slice needs only Layer 1. A small change can be driven manually with the matching CLI verbs. Plan/execute automation (Layer 2) composes on top of Layer 1, and the cross-repo umbrella mode is a composition inside Layer 2 — every step shells out to a CLI verb or a Layer 2 skill in default mode. This means you can always drop down a layer when automation fails — see [Drop down a layer](../how-to/drop-down-a-layer.md).

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Workflow, standards, and artifacts

**Decision:** Specify separates three concerns that must not collapse into one surface:

1. **Workflow** — phase skills and lifecycle CLI verbs (`/spec:plan` … `/spec:finalize`, `specrun slice *`, `specrun plan *`). Mutates `.specify/` state through a closed verb set.
2. **Artifacts** — slice-local and baseline product intent (`proposal.md`, `spec.md`, `design.md`, `tasks.md`, baseline specs). Records what to build; does not encode durable engineering policy.
3. **Engineering standards** — rules under `adapters/**/rules/` plus future `specrun rules export` and `specrun lint`. Durable policy that outlives any slice; read-only enforcement — findings may block CI but never transition plans or slices.

**Authoring standards** (`docs/standards/`, enforced by `specdev lint` on the plugin repo) govern skill and doc house style. **Engineering standards** govern generated and hand-written code in consumer projects. The word "standards" appears in both paths; the enforcement surfaces do not overlap.

**Rationale:** CI-native standards enforcement (`specrun lint`, RM-10) must run continuously on consumer repos without entering the interactive slice loop. Build-time `REVIEW.md` applies standards with model-assisted judgment during `/spec:build`. Plan Gate 1 `approved` is operator approval of a plan — a third, lifecycle-only meaning of "review." Keeping workflow, artifacts, and engineering standards explicit prevents `specrun lint` from being mistaken for a phase skill.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Plan as a data file, not a configuration

**Decision:** The plan (`plan.yaml`) is an ordered list of changes with status, not a pipeline configuration. There is no planning configuration file. The internal flow of `/change:plan` is fixed.

**Rationale:** Configurability adds a debugging surface ("why did step X run?") before the system is well-understood. A fixed flow with no config is easier to reason about, and configurability can be added later without migration.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Analyze/extract split

**Decision:** Plan-time adapter discovery (`/change:analyze`) is separate from define-time deep extraction (`/spec:extract`). Analyze scans the whole source cheaply; extract runs deeply against a per-slice slice.

**Rationale:** A large monolith cannot be fully extracted in one pass -- it would be too slow and expensive. The two-skill split makes large migrations tractable: cheap scanning builds the inventory, deep extraction happens per-slice where it is focused and affordable.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Registry-driven multi-repo planning

**Decision:** Multi-repo coordination uses a `registry.yaml` platform catalogue and an automatic sync-workspace phase, not a configuration DSL or federation protocol.

**Rationale:** The same `/change:plan <name>` command should work unchanged from one repo to 100+. The registry adds the minimum information needed (what repos exist, what adapter they use, what domain they own). Sync-workspace runs automatically when the registry has multiple projects, and not at all for single-repo work. No new user-facing concepts for the common case.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## CWD-based routing for multi-repo execution

**Decision:** The execute driver routes each change to its target project by changing working directory to the workspace clone before invoking phase skills. Phase skills (`/spec:define`, `/spec:build`, `/spec:merge`) are completely unaware of multi-repo routing -- they run unmodified in whatever directory the driver places them in.

**Rationale:** The alternative (passing a `--project` flag through to every phase skill) would have required changes to every skill and every brief pipeline. CWD-based routing keeps the routing decision in one place (the driver) and preserves the invariant that phase skills operate on "the current project." Phase skills discover the adapter via their normal `.specify/project.yaml` walk from CWD.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## One plan entry, one project

**Decision:** Each plan entry targets exactly one registry project. Adapters that span multiple repos are decomposed into separate slices (one per project) linked by `depends-on` edges.

**Rationale:** Allowing a single slice to span repos would require the execution loop to manage multiple project roots, multiple adapters, and multiple baseline merge targets within one define-build-merge cycle. Decomposing cross-cutting adapters into per-project entries keeps the loop simple and matches the existing baseline-accumulation model where each merge has a single target.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Project binding happens in the propose response

**Decision:** Each slice's registry project is bound by the **agent inside the `specrun plan propose --from` response**, not by a separate post-propose assignment step. The dry-run request (`specrun plan propose --dry-run`) carries a `projects[]` topology (always at least one project; a single regular project synthesized from `project.yaml`, each entry carrying its normalized `target` adapter), and the agent names a `project` on each response slice. When exactly one project exists the agent may omit `project` and the kernel auto-binds it; the kernel then derives each slice's `target` from the bound project. Propose is the slice writer — no project-less entries linger for a later assignment pass.

**Rationale:** Cross-source lead matching and project binding are the same agent judgment over the same request envelope, so splitting binding into a second skill step would re-read state the agent already holds and reintroduce an ordering problem. Folding binding into the propose response keeps the projection kernel the single writer of `plan.yaml.slices[].project` and `.target`, and keeps the N=1 case ergonomic via auto-bind. The earlier "assignment runs after propose in the plan skill" framing predates the agent-led `propose --from` kernel and is superseded.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Workspace-centric execution with explicit push

**Decision:** All multi-repo execution happens inside workspace clones under the initiating repo's `.specify/workspace/`. Local commits from merge accumulate in the clones. Changes are published to remotes only when the operator explicitly runs `specrun workspace push`.

**Rationale:** Automatic pushes during execution would make the driver non-idempotent and create a rollback problem -- a failed change that was already pushed cannot be cleanly undone. Keeping pushes explicit gives the operator a review gate between "execution produced artifacts" and "artifacts are published." The workspace is the staging area; `workspace push` is the release gate.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Composition as a separate artifact, not embedded in specs or design

**Decision:** Introduce `composition.yaml` as a new Vectis-specific artifact that describes spatial screen layout, rather than extending specs or design with layout concerns.

**Rationale:** Specs define observable behavior ("the user sees their todo items"); they should not specify how items are arranged on screen. Design defines the type system; embedding layout in design would make it responsible for both data shape and visual arrangement. A separate artifact preserves the existing separation of concerns: specs drive the core, design defines the type contract, and composition drives the shell. This also enables multi-source authoring -- Figma adapters, legacy extractors, and manual editing can all produce composition artifacts without touching specs or design.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## YAML for composition, markdown for specs

**Decision:** The composition artifact uses YAML (`composition.yaml`) rather than markdown, despite all other define-phase artifacts being markdown.

**Rationale:** Layout is fundamentally structural data -- a tree of components with properties. Shell writers and the validation CLI consume it programmatically against a JSON Schema. A markdown representation would require pattern-matching on indented lists to reconstruct the component tree -- fragile and impossible to schema-validate. YAML also aligns with `tokens.yaml` as a structured design-layer artifact and enables same-format diffing for re-imports from design tools.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Screen-level delta merge for composition

**Decision:** Composition deltas operate at the screen level (`added`/`modified`/`removed` per screen), with `modified` performing full screen replacement rather than region-level or item-level merging.

**Rationale:** Merging independently edited region structures at the item level would require positional diff logic with ambiguous conflict resolution. Full-screen replacement is simple, predictable, and sufficient because the define pipeline always produces complete screen entries. Per-screen SHA-256 checksums in `.composition-checksums.yaml` provide conflict detection when two changes modify the same screen.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Contracts as platform-level artifacts, not per-project

**Decision:** API contracts live at `contracts/` alongside `registry.yaml` and `plan.yaml`, not nested inside any project's adapter tree or spec directory.

**Rationale:** An API contract is a shared agreement between parties -- it does not belong to the producer any more than to the consumer. Nesting contracts inside a single project's adapter tree misattributes ownership and forces consumers to navigate workspace clones to find the producer's contract files. Co-locating contracts with `registry.yaml` makes the neutrality structural: `registry.yaml` declares *who* the participants are, `plan.yaml` declares *what* changes are planned, and `contracts/` declares *how* participants communicate. This mirrors established industry practice (proto repos, shared OpenAPI spec repos, contract-first design).

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Platform artifacts at the repo root, framework state under `.specify/`

**Decision:** The four operator-facing platform artifacts -- `registry.yaml`, `plan.yaml`, `change.md`, `contracts/` -- live at the repo root. Generated `AGENTS.md` guidance also lives at the root, with Specify owning only its fenced block. `.specify/` retains framework-managed state: `project.yaml`, `context.lock`, `slices/`, `specs/`, `archive/`, `.cache/`, `workspace/`, `plans/`, and the advisory `plan.lock`.

**Rationale:** The operator-facing artifacts (the registry, the operator brief, the plan, contracts) are durable, PR-reviewed, human-edited material. Putting them under a dot-prefixed framework directory understates their importance and forces operators to navigate framework internals to inspect or hand-edit them. Keeping them at the root makes the boundary explicit: framework owns `.specify/`; operators own everything else.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## JSON Schema + OpenAPI + AsyncAPI, not a new IDL

**Decision:** The contract format uses JSON Schema as the shared payload vocabulary with OpenAPI 3.1 and AsyncAPI 3.0 as protocol-specific bindings. No proprietary schema language is introduced.

**Rationale:** JSON Schema is the common denominator -- both OpenAPI 3.1 and AsyncAPI 3.0 use it for payload definitions. Defining domain types as JSON Schema files means both protocol bindings reference a single source of truth. The Rust code generation ecosystem (`schemars` + `typify`, `progenitor`) can consume these artifacts directly. Introducing a proprietary format or a less common IDL (Smithy, Protobuf) would narrow the ecosystem without clear benefit.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Opaque replacement for contract merge

**Decision:** Contract files use opaque file replacement during merge -- the entire file is replaced rather than delta-merged. Unlike spec files (which use ADDED/MODIFIED/REMOVED sections), contract files are replaced wholesale.

**Rationale:** JSON Schema and OpenAPI/AsyncAPI files have their own versioning semantics (`$id`, `info.version`). Introducing a second delta-merge algorithm for YAML contract files would add complexity without clear benefit over replacement. Two concurrent changes that modify the same contract file are caught by `specrun slice merge conflict-check` (baseline modification after the change's `defined-at` timestamp), and the resolution is to re-run the define phase against the updated baseline.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Stable requirement IDs as merge keys

**Decision:** Each behavioral requirement has a stable `ID: REQ-XXX` line that serves as the merge key across delta specs. Requirement titles may change; IDs must not.

**Rationale:** When specs evolve over multiple changes, the system needs a way to match "this modification applies to that requirement." Titles are human-facing and change frequently. Stable IDs give the merge engine a reliable key while keeping the spec format readable.

## Adapter-agnostic lifecycle, adapter-specific briefs

**Decision:** The lifecycle (states, transitions, core artifacts, baseline accumulation) is invariant across adapters. Adapters control the *content* of brief pipelines, may add adapter-specific stages (e.g. Vectis adds `composition` to the define pipeline), and determine which specialist skills are invoked during build.

**Rationale:** The workflow is the value -- define-build-merge, baseline accumulation, drift detection. Making this adapter-agnostic means every project gets the same tooling regardless of target platform. Adapters customise the generation content and may extend the pipeline without fragmenting the workflow.

## Planning briefs ship with the skill, not the adapter manifest

**Decision:** The planning briefs (`discovery`, `propose`) live alongside the `/change:plan` skill under `plugins/change/skills/plan/briefs/<adapter>/` rather than under `adapter.yaml:pipeline.plan`. The adapter manifest schema actively rejects a `pipeline.plan` block.

**Rationale:** Planning is orchestration, not adapter-owned slice work. A adapter decides what define/build/merge produces inside an individual slice; it does not decide how a *change* (potentially spanning many slices and projects) gets composed. Putting plan briefs in the adapter manifest blurred that boundary and forced every adapter to ship near-duplicate plan briefs. Keeping the briefs with the plan skill keeps the framework concern at the framework, with adapters free to ship their own plan-time variants by name (`briefs/<adapter>/`).

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Adapter vs `--hub` is mutually exclusive at init

**Decision:** `specrun init` accepts either a adapter positional or `--hub`, never both and never neither. A regular project carries a `adapter:` in `.specify/project.yaml`; a hub carries `hub: true` and never carries a `adapter:`. The CLI rejects the two pathological combinations (no positional + no `--hub`, or both supplied) through clap, exiting `2` with its standard parse-error diagnostic.

**Rationale:** Hubs are registry-only repositories that never run phase pipelines, so they have no adapter to resolve. Allowing an empty adapter would force every downstream verb to special-case the missing field; allowing both would double the topology surface. The mutual-exclusion is mechanically enforced at init so every later verb can rely on the invariant without re-checking.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Platform components are not adapters

**Decision:** The registry and the change orchestrator are first-party **platform components**, not adapters. They have commands, libraries, and files, but they never appear in any `adapter.yaml`, never participate in the manifest protocol, and are never activated through a adapter-name switch. The dependency direction is fixed at the crate level: `specify-core` does not depend on `specify-registry` or `specify-change`, and `specify-registry` does not depend on `specify-change`.

**Rationale:** Treating the registry and change orchestration as adapters created a circular activation problem — the surface that decides which adapters are active was itself a adapter. Promoting them to platform components keeps adapter composition strictly downward and means a adapter author never has to think about whether the registry or change loop is "available." The hard-coded crate dependency direction makes the invariant a build-time guarantee rather than a convention.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Operator owns PR merge; Specify prepares and publishes

**Decision:** Specify materialises workspace slots, prepares the `specify/<change-name>` branch before phase writes, accumulates a baseline commit from `/spec:merge` and a residue commit from `/change:execute`, and pushes the branch through `specrun workspace push`. PR review and merge happen through the forge UI, `gh pr merge`, or the team's normal merge queue. The framework never inspects checks or calls `gh pr merge` itself, and `specify change finalize` only verifies that each PR is already merged before archiving the plan. `specrun workspace merge` is removed.

**Rationale:** Automated PR merge couples the framework to forge APIs, check-suite semantics, and team-specific review rules that vary across operators. Holding the framework at "prepare and publish" lets every team layer its own merge policy — checks, reviewers, merge queue, manual approval — without the framework modelling any of it. The split also gives a natural rollback surface: an unmerged PR can be closed or rebased without rewinding any framework state.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Declared WASI adapter tools

**Decision:** Helper tools shipped by adapters or projects are declared as WASI command components in a `tools.yaml` sidecar (adapter scope) or in `.specify/project.yaml` (project scope), and run through a single CLI surface — `specrun tool {list, fetch, show, run}`. Project scope wins on collision, so an operator can redirect a adapter-shipped tool to a local build or pinned mirror without editing the adapter. Permissions are directory preopens, not globs; the host canonicalises every path and rejects `..` segments, glob metacharacters, symlink escapes, and direct writes to Specify lifecycle state. Released first-party tool declarations use exact `specify:*@<semver>` package requests resolved through wasm-pkg metadata.

**Rationale:** Adapters used to extend the framework either by adding more in-binary CLI verbs or by shelling out to host binaries the operator had to install separately. Both paths broke on every CLI release: in-binary verbs grew the host surface unboundedly; host binaries diverged in version, permissions, and discoverability across machines. WASI command components keep the helpers sandboxed and deterministic while making them data — the host fetches them, the host enforces the preopens, the host caches them — so a adapter can ship behavior without growing the host.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## One `specify` binary; adapter-specific helpers ship as declared tools

**Decision:** Operators install one binary — `specify`. The deterministic Vectis helpers (validation and scaffold rendering) ship as WASI tools declared by [`adapters/targets/vectis/adapter.yaml`](../../adapters/targets/vectis/adapter.yaml) (`tools[]`). Host post-processing for Vectis projects (Cargo, Gradle wrapper bootstrap, Xcode and `make typegen` / `make package` / `make xcode`, `local.properties`, Java home and NDK detection, prerequisite checks, registry queries, cap-matrix verification) lives in the Vectis target's [`build`](../../adapters/targets/vectis/briefs/build.md) and [`merge`](../../adapters/targets/vectis/briefs/merge.md) briefs as ordinary shell commands the agent runs and journals.

**Rationale:** A separate adapter-specific binary would double the install, packaging, release, and version-coordination surface for every adapter that needs helpers. Applying the declared-tool model keeps the surface to one binary and keeps the "deterministic rendering" layer cleanly separated from the "host toolchain" layer, which never belongs inside a WASI wrapper.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## SemVer `info.version` and rename-stable `info.x-specify-id`

**Decision:** Every top-level OpenAPI 3.1 and AsyncAPI 3.0 document under `contracts/` MUST set `info.version` to a value that parses per [semver.org](https://semver.org), including optional prerelease labels. Every top-level contract MAY set `info.x-specify-id` to a kebab-case slug (`^[a-z][a-z0-9-]*$`, ≤64 characters, repo-unique) that survives file moves and `info.version` bumps. Path-based references in `registry.yaml` remain canonical — the id is a hint, not a substitute. Bump rules (when to advance major / minor / patch) remain skill-side judgement.

**Rationale:** Earlier contract documents used a mix of `YYYY-MM-DD` dates and bare majors as `info.version`, which prevented any tooling from comparing two contract versions programmatically. Requiring SemVer aligns contract evolution with the broader ecosystem (`progenitor`, `typify`, `schemars`) and keeps adapter-owned contract validation reports machine-readable. The optional rename-stable id captures the identity of a contract independent of its file location, so a file move or a major-version rename does not look like deletion-plus-creation to baseline diff tools.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## SKILL.md discipline cleanup (2026-05)

**Decision:** Five mechanical predicates lock down the SKILL.md shape that operators read every day:

- **Description grammar** (S1) — `checkDescriptionStartsWithVerb` + `checkDescriptionHasUseWhen` enforce a leading imperative verb (curated allow-list in `scripts/checks/skill_frontmatter.ts`) and a `Use when …` clause, in addition to the pre-existing `checkDescriptionLength` ≤ 512 char cap.
- **Section line cap** — `checkBodyAndSectionLineCounts` caps each H2 section at 45 lines (non-blank, non-comment). Depth migrates into `references/<topic>.md` instead of letting individual sections sprawl.
- **Argument-hint grammar** (S3) — `checkArgumentHintGrammar` accepts only `<name>`, `[name]`, trailing `...`, `<a|b>` / `[a|b]`, and `--flag` tokens (kebab-case names). Bare prose, mixed punctuation, and short flags are rejected.
- **Envelope-example forbid** (S5) — `checkNoEnvelopeExamples` flags fenced ```json``` blocks whose body looks like a CLI envelope wrapper. Envelope shapes live with stable anchors in `docs/reference/cli-output-shapes.md`; SKILL.md bodies link instead of embed.
- **Vocabulary / guardrails consolidation** (S4) — cross-cutting guardrails (the recurring `.metadata.yaml` / slice-dir / plan-write rules) live in `docs/standards/skill-guardrails.md`; SKILL.md files link, not restate. Pre-1.0 sweep dropped "previously / migrate / backward-compat" prose that did not document a real legacy-migration feature, and stale CLI names were replaced in docs (`initiative` → `change`, `JSON_SCHEMA_VERSION` → `ENVELOPE_VERSION`).

**Rationale:** The skill-discovery surface needs to match on intent ("does this skill apply to my task?") rather than vocabulary, which means the description and argument-hint shapes have to be mechanically tight. The body and section caps push depth into `references/` so a SKILL.md stays an orientation artifact. The envelope-example forbid pins one place where the wire shape can drift. The pre-1.0 vocabulary stance — no backward compatibility constraints, no migration prose unless documenting a real legacy-migration feature — keeps the skill bodies honest about what the system does today rather than how it was built.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## CLI verb renames

**Decision:** Several CLI verb groups were renamed during the historical design record → historical design record cutover so the top-level surface matches the operator-facing nouns (slice / change / registry / workspace / adapter):

- `specify change *` (per-slice verbs) → `specrun slice *`.
- `specify change *` (umbrella verbs) → `specify change *`; `specify change create` was renamed from the v1 `specify change init`.
- `specify schema {resolve, check, pipeline}` → `specify adapter {resolve, check, pipeline}`.
- `specrun registry {add, remove}` were added by historical design record.
- The pre-historical design record in-binary `specify contract { list, validate }` family was retired in chunk 2.7 when contracts became a first-party adapter owning its own validation behaviour; the contracts merge brief now shells out through `specrun tool run contract` as the post-merge baseline gate.
- `specrun init --hub` is the mutually exclusive alternative to `specrun init <adapter>` — it scaffolds a registry-only platform hub whose `project.yaml` carries only `hub: true`.
- `specrun workspace merge` has been removed; operators merge through the forge UI or `gh pr merge`, then `specify change finalize` verifies remote PR state.

**Rationale:** Specify is pre-1.0 and the wire/CLI surface is allowed to evolve. Capturing the rename trail here keeps `AGENTS.md` free of "renamed from earlier surfaces" parentheticals while preserving the trail for anyone tracing a stale call site.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Three-skill change lifecycle

**Decision:** A May 2026 workflow cleanup split `/change:plan` (default mode + `orchestrate` umbrella) into three peer skills (`/change:draft`, `/change:execute`, `/change:finalize`); CLI verb `specify change create` renamed to `specify change draft`.

**Rationale:** `/change:plan` doubled as authoring skill and orchestration umbrella, hiding the operator review pause between authoring and execution and breaking lifecycle symmetry with `/spec:define → /spec:build → /spec:merge`. Three peer skills with an explicit human seam between draft and execute restore the rhythm and make the review pause a property of the framework rather than an opt-in manual flow. The seven-step orchestration body survives, redistributed across the three skills; the umbrella mode is removed outright.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Source/target split

**Decision:** Replace the unqualified 1.x "adapter" with two qualified roles. **Source adapters** declare `axis: source` and ship `survey` + `extract` briefs at `adapters/sources/<name>/adapter.yaml`. **Target adapters** declare `axis: target` and ship `shape` + `build` + `merge` briefs at `adapters/targets/<name>/adapter.yaml`. The adapter loader (`crates/workflow/src/adapter/`) routes by axis and the manifest cache splits as `.specify/.cache/manifests/{sources,targets}/<name>/` (the workflow §D8 per-source extraction cache lives in a disjoint sibling tree at `.specify/.cache/extractions/<adapter>/`).

**Rationale:** `/change:analyze` and `/change:survey` were two evidence sources for the same operation; `/spec:define` and `/spec:extract` repeated the pattern at slice time. Unqualified `adapter` only named outputs, leaving no symmetrical term for inputs. Qualifying by direction makes the input/output asymmetry explicit, gives third-party legacy migration a first-class home (source adapters), and lets one resolver module replace the bifurcated define/analyze surface.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Core owns synthesis

**Decision:** Source adapters emit `Evidence` only; target adapters supply `shape` only; **core owns canonical artifacts** (`proposal.md`, `spec.md`, `design.md`, `tasks.md`). `/spec:refine` runs extraction, synthesis, validation, and lifecycle transition. Synthesis authors from `plugins/spec/references/synthesis/`; the CLI validates structure and stamps lifecycle.

**Rationale:** 1.x let each adapter author its own `spec.md` and `design.md`, which blocked multi-source synthesis (no canonical document to fold two sources into) and forced every adapter to ship near-duplicate define briefs. Pulling synthesis into core means every adapter inherits multi-source reconciliation, requirement provenance, and tag-and-proceed disagreement handling without re-implementing them.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Multi-source slices

**Decision:** `Slice.sources` is a list of `{ key, lead }` bindings with cardinality ≥ 1. Each binding pairs a source key (referencing `plan.yaml.sources.<key>`) with the lead id from `discovery.md` that contributed to the slice. The reader accepts a bare `<key>` shorthand when the lead id equals the slice's `name`; the CLI always writes the structured form.

**Rationale:** Combined evidence (code + documentation, intent + design notes, screenshots + product brief) is the common case for non-trivial work. Single-source slices become a degenerate one-binding case rather than the structural default. Carrying the lead id on every binding preserves the back-reference into `discovery.md` so re-survey can replace by id without disturbing the slice row.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Provenance and disagreement tags

**Decision:** Every requirement block in `spec.md` carries three header lines: `ID:` (stable merge key), `Sources:` (one or more source keys, highest authority first), `Status:` (closed enum `agreed` | `unknown` | `conflict` | `divergence`). Authority hierarchy is the closed enum `intent` > `documentation` > `behaviour`. Tags `[conflict]`, `[divergence]`, `[unknown]` surface inline on the requirement header; they never park the slice.

**Rationale:** Multi-source synthesis routinely surfaces disagreements that cannot be auto-resolved. Tag-and-proceed keeps the slice lifecycle moving (`refining → refined → built → merged`) regardless of tag content; the operator hand-edits `spec.md` between refine and build when they want to reconcile, or amends the plan to drop a source. Parking the slice on every tag would have made multi-source slices unusable in practice.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Always plan

**Decision:** Every change runs through `survey` and `plan.yaml`, including N=1. `/spec:define` retires; trivial work uses the degenerate `intent.survey` path — one operator-supplied intent value produces one lead, which becomes one slice.

**Rationale:** 1.x's `/spec:define` shortcut for "single trivial slice" duplicated the plan-time machinery and produced an orphan path that bypassed Gate 1. Collapsing to one rhythm — `/spec:plan` → Gate 1 → `/spec:execute` → `/spec:finalize` — means N=1 and N=12 share every skill body, every CLI verb, and every artifact. Operator ergonomics at N=1 became the release blocker for the collapse.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Gate 1 only

**Decision:** Human review happens at exactly one place in v1: between planning and execution, via `plan.lifecycle == approved`. The operator runs `specrun plan transition <name> approved` explicitly; `/spec:plan` exits at `pending` and prints the literal command but never stamps `approved` itself. No Gate 2 (post-synthesis park) and no synthesis review state ship in v1.

**Rationale:** Multiple gates compound operator cost and incentivise skipping. One observable gate written by the operator at one observable moment makes the review pause unambiguous and unbypassable. Synthesis-time disagreements use tag-and-proceed; the `slices[].divergence` field carries the Gate-1 acknowledgement signal a future Gate 2 would consume, so the park can be wired in later without a schema change.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Supervised execute

**Decision:** `/spec:execute` is the only v1 driver. It resumes from on-disk state — `plan.yaml.lifecycle`, per-entry `status`, slice `.metadata.yaml`. The skill ships with no automation flags (no `yes-plan`, `one`, `until`, `dry-run`, or `continue` variants). Build failures and merge conflicts park execute and surface the failure; the operator fixes and re-runs `/spec:execute` or invokes a breakout (`/spec:build`, `/spec:merge`).

**Rationale:** Automation flags multiplied the number of resume paths the loop had to reason about and made every failure a question of "did execute stop because it finished, because a flag tripped, or because something broke?". Stripping the flags makes re-entry trivial: read on-disk state, pick up where the active entry left off. When a real automation consumer appears, the flags can be reinstated with a clear contract.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## CLI owns workflow writes

**Decision:** The CLI is the single writer for lifecycle and deterministic workflow files: `plan.yaml`, `.metadata.yaml`, archive paths, `discovery.md`, `sources.yaml`, `targets.yaml`. Skills and adapters never hand-edit these. Skills do drive content writes (evidence bodies, synthesized artifacts, implementation code) where the contract allows; adapters retain authorship of their briefs.

**Rationale:** LLM-interpreted prose rules for lifecycle transitions produced unreliable results — partial writes, inconsistent state, missing validation. A binary that returns structured JSON and exit codes gives deterministic correctness for the files everyone has to trust. Skills stay agent-driven for the parts that need judgment (synthesis, code generation, review) and shell out for the parts that need determinism.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Uniform workspace routing

**Decision:** `/spec:execute` and the breakout verbs share the same routing: plan lock at workspace root → resolve the active slice's project → `workspace sync` of that slot → `chdir` → phase work → return. Phase skills remain unaware of multi-repo routing; the driver handles it identically whether invoked from the loop or as a breakout.

**Rationale:** 1.x had separate routing paths for the loop and the manual breakouts, which meant a workspace breakout after a build failure required the operator to navigate to the right slot by hand. Sharing the routing rule means an operator can park execute, run `/spec:build` from the workspace root, and have the driver place them in the correct project slot automatically.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Hard cut at 2.0

**Decision:** 1.x manifests, verbs, brief paths, and `/change:*` retire together at 2.0. No interim release. No compatibility aliases.

**Rationale:** Compatibility shims for an in-flight pre-1.0 redesign multiply the surface area of every change without serving real consumers (there is no production install base yet). A single cut avoids any code path having to support both shapes.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.

## Automated propose

**Decision:** `/spec:plan`'s `propose` sub-step reconciles `Lead[]` from each source's `survey` into `slices[]` rows in `plan.yaml` through `specrun plan propose --from`. Uncertain cross-source merges surface in a `## Tentative merges` block in `change.md` (the agent never edits `discovery.md`); materially-disagreeing synopsis pairs set `slices[].divergence: likely` via `specrun plan amend` and surface in a `## Likely divergences` block. The operator overrides at Gate 1 with `specrun plan amend`, `specrun plan add`, and `specrun plan remove` (split, merge, relabel, rebind, defer, accept/reject divergence). Authority hierarchy does not apply at propose — reconciliation runs on lead headlines alone; authority activates at slice-time synthesis once `Evidence` lands.

**Rationale:** Operator-driven lead reconciliation at the planning step would have added a second review ceremony before Gate 1 with no automation hook. Tag-and-proceed at propose mirrors tag-and-proceed at slice synthesis: the workflow keeps moving, uncertainty surfaces as review signals the operator inspects at Gate 1, and the operator's amendment is the override path. The `slices[].divergence` enum (`none` / `likely` / `accepted` / `rejected`) is advisory in v1 — no halt is wired against any value — but gives a durable record of "operator was warned at Gate 1" that future workflow gates can consume without a schema change.

**Source:** Current maintained docs, schemas, and CLI implementation surfaces.
