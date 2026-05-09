# Glossary

Canonical definitions for terms used throughout Specify.

## A

**Alignment report**
The output of an `/contract:*` skill's author intent (`/contract:openapi`, `/contract:asyncapi`, or `/contract:json-schema` — picked from the brief context) after running the 6-step algorithm. Summarises coverage (interactions already defined in the baseline), alignment warnings (spec-vs-baseline mismatches), generated delta (new contract files), and normalisation changes. A clean report with zero delta is the expected outcome for implementation changes in a contract-first workflow.

**API contract**
A machine-readable interface definition at `contracts/`. Uses three formats: JSON Schema for payload definitions, OpenAPI 3.1 for HTTP endpoint bindings, and AsyncAPI 3.0 for messaging bindings. Contracts capture the *shape* of interfaces -- endpoint paths, methods, payload schemas, error codes, channel names, message structures. They complement behavioral specs, which capture *what* the system does.

**Artifact**
A structured document that defines part of a change. The core artifacts are `proposal.md`, `spec.md`, `contracts/**/*.yaml`, `design.md`, and `tasks.md`. Capability-specific artifacts extend this set -- the Vectis capability adds `composition.yaml` for screen layout. Artifacts are the contract between human intent and agent execution.

**Archive**
The `.specify/archive/` directory where finalized changes (merged or dropped) and completed plans are stored for audit.

## B

**Baseline**
The accumulated set of merged specs at `.specify/specs/` and merged contracts at `contracts/`. For Vectis projects, also includes the merged `composition.yaml` for screen layout. Represents the current known behavioral and interface state of the system. Future changes produce deltas against the baseline.

**Brief**
A markdown prompt file provided by a capability that drives artifact generation. Briefs are organized into pipelines for each phase (define, build, merge).

**Brief pipeline**
An ordered sequence of briefs declared by a capability for a given phase. The Omnia define pipeline runs: proposal, specs, contracts, design, tasks. The Vectis define pipeline runs: proposal, specs, contracts, composition, design, tasks.

## C

**Capability** (extension primitive)
A versioned Specify extension that tells the core how to generate artifacts and build code for a specific outcome domain. Selected at `/spec:init <capability>` time. Each first-party capability lives at `capabilities/<name>/capability.yaml` and contributes brief pipelines for the fixed `define → build → merge` slice loop. Renamed from "schema" by [RFC-13](../../rfcs/archive/rfc-13-extensibility.md). See also: the unit-of-behaviour reading below.

**Capability** (unit of behaviour)
A discrete unit of system behavior that gets its own spec file. In the Omnia capability, capabilities (in this sense) typically correspond to crates. In the Vectis capability, they correspond to features. The same word is overloaded inside the spec / baseline directory layout (`specs/<capability>/spec.md`); context disambiguates.

**Composition artifact**
A schema-validated YAML document (`composition.yaml`) that describes the spatial layout of each screen in a Vectis application. Organises content into named regions (`header`, `body`, `footer`, `fab`) with a container tree of items and groups carrying flexbox-like layout properties, enriched with the `bind`, `event`, `maps_to`, overlay `trigger`, navigation, and `*-when` wiring keys that connect the layout to ViewModels and specs. Produced by the Vectis define pipeline (the composition brief) between specs and design from a `layout.yaml` input (when present) or from existing baseline composition; consumed by shell writers for deterministic layout generation. The unwired pre-define input is a sibling artifact, [`layout.yaml`](#l) — RFC-11 made that boundary explicit by reserving `composition.yaml` for the wired Specify lifecycle artifact and `layout.yaml` for the input layout intent. See [RFC-11](https://github.com/augentic/specify/blob/main/rfcs/rfc-11-ui-spec.md) (which superseded RFC-7's skeleton/wired duality).

**Change**
The operator-defined umbrella that coordinates one or more slices through `change.md` and `plan.yaml`. A change may be a single planned effort in one repo or a cross-repo program driven through `/change:plan`, `/change:execute`, and `specify change *` CLI verbs.

**Change branch**
The Git branch used to publish a multi-repo change from a registry workspace slot. Its exact form is `specify/<change-name>`, where `<change-name>` comes from `plan.yaml` / `change.md`. `/change:execute` prepares remote-backed slots on this branch before mutation; `specify workspace push` refuses any slot that is not already on this exact branch (`no-branch`) and never creates the branch on the fly.

**Change finalize**
The canonical closure verb for a multi-repo change. `specify change finalize` verifies that plan entries are terminal, required per-project PRs on `specify/<change-name>` are operator-merged, and workspace clones are clean; then it archives `plan.yaml`, `change.md`, and `.specify/plans/<name>/`. With `--clean`, it may remove clean workspace clones after archive succeeds. It never merges PRs.

**Context (plan entry)**
The optional `context` field on a plan entry -- a list of baseline paths (relative to `.specify/`) that are relevant to the change. Briefs use these as a focus hint when scanning baseline directories. Populated automatically by `/change:plan` (e.g. contract paths from a preceding contract change) or manually via `specify change plan add --context`.

**Coordinator root**
The repository where an operator runs a coordinated change. It owns `registry.yaml`, `plan.yaml`, `change.md`, `.specify/plans/`, and the registry workspace under `.specify/workspace/`. For a hub topology, the coordinator root may contain no product code; for platform-as-project, it is also one of the registered projects.

**Contract-first**
Authorship pattern where a dedicated contract change defines interface shapes before implementation begins. `/change:plan` inserts these automatically when it detects an API boundary between projects. The contract change uses `schema: contracts@v1` and has no `project`. Implementation changes depend on the contract change.

**Contract-given**
Authorship pattern where API contracts are imported from an external system or legacy API. The operator places the external files into the change's `contracts/` directory. `/change:plan` inserts import changes when a source is flagged as external.

**Cross-project compatibility classification**
The RM-04 CLI report produced by `specify compatibility report --change <name>` or `specify compatibility check`. It walks `registry.yaml`, matches `contracts.produces` to `contracts.consumes`, compares root producer contracts with consumer workspace views, and classifies findings as `additive`, `breaking`, `ambiguous`, or `unverifiable`.

## D

**Define**
The first phase of the slice lifecycle. Generates all artifacts from a description, optionally enriched by source code extraction.

**Delta spec**
A spec that describes modifications to an existing capability using `ADDED`, `MODIFIED`, `REMOVED`, and `RENAMED` sections. Delta specs merge into the baseline by matching on stable `REQ-XXX` IDs.

**Discovery**
The output of `/spec:analyze` during plan authoring. A `discovery.md` file containing capability summaries (name, description, source files, dependencies, confidence) derived from input analysis.

## E

**Execute**
The Layer 3 driver skill (`/change:execute`) that automates the define-build-merge loop for each entry in a plan, in dependency order. For multi-repo plans, routes each change to its target project's workspace clone via CWD-based routing.

**Extract**
The process of deriving behavioral specs and design from existing source code, performed by `/spec:extract`. Produces language-agnostic artifacts.

## G

**Greenfield bootstrapping**
The `specify workspace sync` fallback for registry projects whose remote repos do not yet exist. Creates the workspace slot, runs `git init`, sets the remote, and scaffolds `.specify/project.yaml` via `specify init` using the initiating repo's capability cache.

## H

**Hub** (also: **Platform hub**)
A registry-only platform repo. Identified by `project.yaml: hub: true` (with the `capability:` field omitted) (RFC-9 Section 1D, refined by [RFC-13 §Migration "Hub project shape"](../../rfcs/archive/rfc-13-extensibility.md#migration)). Holds platform state -- `registry.yaml`, `change.md`, `plan.yaml`, `workspace/` -- but is never itself a code project. Code projects live in their own repos and are materialised under `.specify/workspace/<name>/` by `specify workspace sync`. Scaffolded via `specify init --hub`. Contrast with the [platform-as-project](#p) shape where the initiating repo is both the platform repo and a code project (`url: .` in `registry.yaml`). See [Platform repo topologies](../explanation/platform-repo.md).

## I

**Initiative**
Legacy term for a change before RFC-13 renormalised the lifecycle nouns. Current docs use **change** for the operator umbrella and **slice** for each define-build-merge unit.

**Contract id**
The optional `info.x-specify-id` field on a top-level OpenAPI 3.1 / AsyncAPI 3.0 contract (RFC-12). Kebab-case (`^[a-z][a-z0-9-]*$`), ≤ 64 characters, unique across every top-level contract in the repo. The id is a **rename-stable hint** that survives file moves and `info.version` bumps — once set on a contract, never change it. Path-based references in `registry.yaml` remain canonical; the id is not a substitute. Format and uniqueness are enforced by the declared `contract` WASI tool (`specify tool run contract`, the contracts capability's post-merge baseline gate, RFC-13 §"Merge and adoption contract") and by the `/contract:openapi` / `/contract:asyncapi` verifier intents only when the field is present — contracts without one remain valid indefinitely.

**Initiative finalize**
Legacy name for `specify change finalize`. The command verifies operator-merged PRs and archives `plan.yaml`, `change.md`, and `.specify/plans/<name>/`; it does not merge pull requests.

**Change shapes (three)**
The three input topologies the platform-first loop handles uniformly: `migrate-legacy` (sources via `--source <key>=<git-url-or-path>`, targets are existing or newly-minted registered projects), `new-feature` (sources via `--from <docs>`, targets are existing registered projects with new ones spawned at assignment time via the registry-proposal sub-step), and `update-existing` (no input flags, targets are existing registered projects, baseline accumulation in workspace clones is the dominant signal). All three flow through the same `/change:plan <name> orchestrate` sequence.

## L

**Layout artifact**
A schema-validated YAML document (`layout.yaml`, Vectis only) that captures the spatial layout intent for each screen *before* `/spec:define` runs — regions, group hierarchy, gap / padding / align / size, token references, asset references, and the optional cross-shell `component: <slug>` directive, with no `bind` / `event` / `maps_to` / overlay `trigger` / navigation / `*-when` wiring keys yet. Produced by layout inferers (the screenshot-fronted [`vectis:image-layout-inferer`](../../plugins/vectis/skills/image-layout-inferer/SKILL.md) today; future Figma and source-code inferers per RFC-11 §B/D) or hand-authored by the operator. Validated by `specify tool run vectis-validate -- layout`, which rejects the wiring keys and enforces the §G structural-identity rule. Consumed by the composition brief during `/spec:define`, which produces the wired [composition artifact](#c). RFC-11 introduced the layout / composition split; RFC-7 conflated both into a single `composition.yaml` with a "skeleton" / "wired" mode distinction. See [RFC-11](https://github.com/augentic/specify/blob/main/rfcs/rfc-11-ui-spec.md).

**Layout boundary (operator vs framework)**
The `0.2.0` v2 layout split Specify's on-disk shape along a clear line: **operator-facing platform artifacts** (`registry.yaml`, `plan.yaml`, `change.md`, `contracts/`) live at the repo root; generated `AGENTS.md` guidance also lives at the root with Specify owning only its fenced block; **framework-managed state** (`project.yaml`, `context.lock`, `slices/`, `specs/`, `archive/`, `.cache/`, `workspace/`, `plans/`, `plan.lock`) lives under `.specify/`. The CLI refuses the legacy v1 layout (where everything sat under `.specify/`) with the stable `legacy-layout` error code; `specify migrate v2-layout` is the one-shot mover that upgrades a v1-layout project in place. See [Migrating to the v2 layout](../how-to/migrate-to-v2-layout.md).

**Legacy-layout error**
The diagnostic the CLI emits (stable code `legacy-layout`, exit 1) when a project-aware verb encounters a v1-layout project (operator artifacts still under `.specify/`). The remediation is always `specify migrate v2-layout`; see the [troubleshooting entry](troubleshooting.md#legacy-layout-error-from-every-cli-verb).

**Layer 1 (CLI primitives)**
The `specify` CLI commands that handle all deterministic operations: slice lifecycle, plan CRUD, registry mutation, workspace sync/status/push, change finalization, capability resolution, validation. The foundation that skills build on. The old workspace merge automation is no longer an active primitive; `specify workspace merge` is only a non-zero deprecation shim.

**Layer 2 (Slice lifecycle)**
The `/spec:define`, `/spec:build`, `/spec:merge` loop and supporting skills (`/spec:init`, `/spec:drop`, `/spec:extract`). Each skill operates on a single slice inside `.specify/slices/<name>/` and delegates deterministic work to the Layer 1 CLI.

**Layer 3 (Plan & Drive)**
The skills that coordinate multi-slice changes through `plan.yaml`: `/change:plan` (authors the plan via discovery, propose, and assignment), `/change:execute` (automates the define-build-merge loop per slice with CWD-based routing for multi-repo plans), and `/spec:analyze` (plan-time capability inference). Includes sync-peers for multi-repo registries and project assignment (RFC-3b).

**Layer 4 (Change orchestration)**
The orchestration mode of `/change:plan` (`/change:plan <name> orchestrate`, RFC-9 Section 2C) that strings the platform-first loop -- brief, registry validate, plan, execute, push, operator PR merge, finalize -- into one operator action. Composition only: every Specify step shells out to a Layer 1 CLI verb or a Layer 3 skill; the operator or forge owns the PR merge decision. Honours every halt the underlying skills surface and is idempotent on re-entry.

**Lifecycle state**
The current status of a slice: `created`, `defining`, `defined`, `building`, `complete`, `merged`, or `dropped`. `defining` and `building` are transient states indicating a phase is in-flight. Managed by the CLI via `.metadata.yaml`.

## M

**Merge**
The third phase of the slice lifecycle. Applies spec deltas, contract deltas, and composition deltas (Vectis) to the baseline and archives the slice. When running inside a workspace clone, `/spec:merge` auto-commits the merged baseline (RFC-3b).

**Merge key**
The stable `ID: REQ-XXX` line in a spec requirement. Used to match delta spec operations to baseline requirements during merge.

## O

**Opaque replacement**
The merge semantics used for contract files. Unlike spec files (which use the ADDED/MODIFIED/REMOVED delta format), contract files are replaced wholesale during merge -- `specify merge` copies the change's `contracts/` files into `contracts/`, replacing files that share a path. Files absent from the change are left untouched.

## P

**Phase outcome**
A classification (`success`, `failure`, `deferred`, or `registry-amendment-required`) written to `.metadata.yaml` after a phase completes. Used by `/change:execute` to determine whether to transition a plan entry to `done`, `failed`, or `blocked`. The `registry-amendment-required` variant (RFC-9 Section 2B) carries a structured payload `{ proposed-name, proposed-url, proposed-schema, proposed-description, rationale }` and triggers the operator-driven recovery sequence -- the framework never auto-modifies the registry.

**Plan**
An ordered, dependency-aware list of slices stored in `plan.yaml`. The change's table of contents.

**Plan doctor**
`specify change plan doctor` (RFC-9 Section 4B). A strict superset of `specify change plan validate` that runs every check `validate` runs and then layers four health diagnostics on top: `cycle-in-depends-on` (dependency cycles in `depends-on`), `orphan-source-key` (top-level `sources:` keys no entry references), `stale-workspace-clone` (clones whose registry signature has drifted), and `unreachable-entry` (pending entries blocked by `failed`/`skipped` predecessors). The first triage step when `/change:execute loop` reports `stuck`.

**Platform-as-project**
The single-repo platform topology where the initiating repo is both the platform repo and a code project. Identified by `url: .` on the repo's own registry entry. Phase pipelines run normally because `project.yaml:capability:` resolves to a real capability (`hub:` is absent or `false`). Still permitted for single-repo and small-team cases. Contrast with [Hub](#h). See [Platform repo topologies](../explanation/platform-repo.md).

**Plugin**
A Cursor marketplace package that provides skills, rules, and references for a specific domain (Specify, Change, Omnia, Vectis, Contract, RT, Client).

**Project (plan routing)**
The `project` field on a plan entry that names the registry project a change targets. Required on every entry when `registry.yaml` declares multiple projects; optional (or absent) for single-repo plans. Drives CWD-based routing during execution (RFC-3b).

**Project assignment**
The step during `/change:plan` (multi-repo only, step 3(d)) that infers which registry project each plan entry targets. Uses description match, baseline-spec affinity, and capability compatibility as signals. Assignments are presented to the operator for review and written via `specify change plan amend --project`.

**Proposal**
The first artifact generated during define. Captures why the change exists, what is in scope, and which capabilities are affected.

## R

**Registry**
`registry.yaml` -- a platform catalogue declaring the repos in a multi-repo system. Each entry has a name, URL, capability identifier, and domain description.

**Registry amendment** (also: **`registry-amendment-required`**)
The phase outcome variant added by RFC-9 Section 2B for cases where a phase skill discovers that a capability needs a new registry project (e.g. `/spec:extract` surfacing tangled code that should split into a new repo). The driver classifies the outcome as `blocked`, records the structured payload in the dropped change's `journal.yaml`, and surfaces the proposal to the operator. The canonical recovery sequence is `specify registry add <proposed-name> --url <proposed-url> --schema <proposed-schema> --description "<proposed-description>"` -> `specify workspace sync` -> `specify change plan amend <change> --project <proposed-name>` -> `specify change plan transition <change> pending` -> re-run `/change:execute`. The framework never auto-modifies the registry.

**Registry workspace**
The derived local view of registry projects under `.specify/workspace/`. `specify workspace sync` creates or refreshes slots from `registry.yaml`; without selectors it syncs all registry projects, and with selectors it materialises only the selected slots. The registry workspace is scratch execution state, not durable source state.

**Requirement ID**
A stable identifier (`REQ-001`, `REQ-002`, ...) assigned to each behavioral requirement in a spec. Serves as the merge key across delta specs.

**Routing (CWD-based)**
The mechanism by which `/change:execute` routes each multi-repo plan entry to its target project. The driver changes working directory to the target project's workspace clone before invoking phase skills; phase skills run unmodified in whatever directory the driver places them in (RFC-3b).

## S

**Skill**
An agent-driven orchestrator invoked with a slash-command prefix (e.g. `/spec:define`, `/omnia:crate-writer`). Skills delegate deterministic work to the CLI and use judgment for everything else.

**Skill directive tag**
An HTML comment in `tasks.md` (e.g. `<!-- skill: omnia:crate-writer -->`) that routes a task to a specific specialist skill during build.

**Slice**
The single unit that flows through the fixed `define -> build -> merge` loop. Each slice has its own proposal, specs, design, tasks, metadata, and merge step, and lives under `.specify/slices/<name>/`.

**Spec**
A behavioral specification at `specs/<capability>/spec.md`. Contains requirements with stable IDs, scenarios (WHEN/THEN), error conditions, and optional metrics.

**Spec-first (inline derivation)**
Authorship pattern where contracts are derived inline from specs during a single slice's define phase. Used as a convenience fallback for single-repo services with no external consumers and no API boundary. The baseline is empty, so the delta is the full contract set.

**Sync peers**
The phase during `/change:plan` (multi-repo only) that clones registry projects into `.specify/workspace/` and inventories their baseline specs. Produces `workspace.md`.

## T

**Top-level contract**
A YAML file under root `contracts/` whose root carries `openapi:` (OpenAPI 3.1 document) or `asyncapi:` (AsyncAPI 3.0 document) (RFC-12). Format detection decides what counts — never directory layout, file name, or a custom marker. Top-level contracts are the only files subject to the RFC-12 §Validation rules (SemVer `info.version`; format + cross-repo uniqueness on `info.x-specify-id` when present). Standalone JSON Schemas under `contracts/schemas/` are payload vocabulary referenced via `$ref` from a top-level contract — they are **not** top-level themselves.

## W

**Workspace**
The registry workspace under `.specify/workspace/`: a derived local view of registered projects. Each child is a workspace slot. It is read-only during planning (sync-peers phase) and writable during execution (`/change:execute` routes define-build-merge into the selected slot via CWD-based routing). Local commits are published through `specify workspace push`; PR merge remains an operator action outside Specify.

**Workspace merge**
Deprecated RFC-14 compatibility shim. `specify workspace merge` no longer automates PR landing: it exits non-zero, performs no PR lookup or forge merge, and points operators to merge through the forge UI or `gh pr merge`, then run `specify change finalize`.

**Workspace slot**
One project-specific child of the registry workspace, normally `.specify/workspace/<project>/`. A slot is a Git clone for remote registry URLs or a symlink for local targets. `workspace status` reports its path, materialisation type, configured target, actual origin or symlink target, branch, HEAD, dirty state, exact change-branch match, `.specify/project.yaml` presence, and active slices.

**Workspace tier 1** (also: **Legacy-source clone**)
The ephemeral, read-only clone materialised under `.specify/plans/<name>/analyze/<key>/` by `/spec:analyze` (using the inlined guarded `git clone` snippet documented at [`plugins/spec/skills/analyze/SKILL.md` §*Cloning a source tree*](../../plugins/spec/skills/analyze/SKILL.md) when the source is a git URL) so the discovery brief can read source code that is not on the operator's local disk. Belongs to a single change and is swept into `.specify/archive/plans/<YYYYMMDD>-<name>/` by `specify change plan archive`. Anything an operator edits inside a tier-1 clone moves into the archive when the change ends -- it never propagates back to the original source. See [Workspace tiers](../explanation/workspace-tiers.md).

**Workspace tier 2** (also: **Registered project clone**)
The durable, read-write slot materialised under `.specify/workspace/<name>/` by `specify workspace sync` from an entry in `registry.yaml`. Belongs to the platform, not to any one change; persists across changes. `/change:execute` `chdir`s into this slot before invoking the phase skills, so the slice directory, the merged baseline, and the workspace's git history accumulate here. `specify workspace push` is the explicit publication gate that opens or updates PRs from `specify/<change-name>`. See [Workspace tiers](../explanation/workspace-tiers.md).
