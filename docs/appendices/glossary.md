# Glossary

Canonical definitions for terms used throughout Specify.

## A

**Alignment report**
The output of an `/contract:*` skill's author intent (`/contract:openapi`, `/contract:asyncapi`, or `/contract:json-schema` — picked from the brief context) after running the 6-step algorithm. Summarises coverage (interactions already defined in the baseline), alignment warnings (spec-vs-baseline mismatches), generated delta (new contract files), and normalisation changes. A clean report with zero delta is the expected outcome for implementation changes in a contract-first workflow.

**API contract**
A machine-readable interface definition at `contracts/`. Uses three formats: JSON Schema for payload definitions, OpenAPI 3.1 for HTTP endpoint bindings, and AsyncAPI 3.0 for messaging bindings. Contracts capture the *shape* of interfaces -- endpoint paths, methods, payload schemas, error codes, channel names, message structures. They complement behavioral specs, which capture *what* the system does.

**Artifact**
A structured document that defines part of a change. The core artifacts are `proposal.md`, `spec.md`, `contracts/**/*.yaml`, `design.md`, and `tasks.md`. Adapter-specific artifacts extend this set -- the Vectis adapter adds `composition.yaml` for screen layout. Artifacts are the contract between human intent and agent execution.

**Archive**
The `.specify/archive/` directory where finalized changes (merged or dropped) and completed plans are stored for audit.

## B

**Baseline**
The accumulated set of merged specs at `.specify/specs/` and merged contracts at `contracts/`. For Vectis projects, also includes the merged `composition.yaml` for screen layout. Represents the current known behavioral and interface state of the system. Future changes produce deltas against the baseline.

**Brief**
A markdown prompt file provided by a adapter that drives artifact generation. Briefs are organized into pipelines for each phase (define, build, merge).

**Brief pipeline**
An ordered sequence of briefs declared by a adapter for a given phase. The Omnia define pipeline runs: proposal, specs, contracts, design, tasks. The Vectis define pipeline runs: proposal, specs, contracts, composition, design, tasks.

## C

**Adapter** (extension primitive)
A versioned Specify extension that tells the core how to generate artifacts and build code for a specific outcome domain. Selected at `/spec:init <adapter>` time. Each first-party adapter lives at `adapters/<name>/adapter.yaml` and contributes brief pipelines for the fixed `define → build → merge` slice loop. See also: the unit-of-behaviour reading below.

**Adapter** (unit of behaviour)
A discrete unit of system behavior that gets its own spec file. In the Omnia adapter, adapters (in this sense) typically correspond to crates. In the Vectis adapter, they correspond to features. The same word is overloaded inside the spec / baseline directory layout (`specs/<adapter>/spec.md`); context disambiguates.

**Composition artifact**
A schema-validated YAML document (`composition.yaml`) that describes the spatial layout of each screen in a Vectis application. Organises content into named regions (`header`, `body`, `footer`, `fab`) with a container tree of items and groups carrying flexbox-like layout properties, enriched with the `bind`, `event`, `maps_to`, overlay `trigger`, navigation, and `*-when` wiring keys that connect the layout to ViewModels and specs. Produced by the Vectis define pipeline (the composition brief) between specs and design from a `layout.yaml` input (when present) or from existing baseline composition; consumed by shell writers for deterministic layout generation. The unwired pre-define input is a sibling artifact, [`layout.yaml`](#l): `composition.yaml` carries the wired Specify lifecycle artifact and `layout.yaml` carries the input layout intent.

**Change**
An operator-defined unit of work that coordinates one or more slices through `change.md` and `plan.yaml`. A change may be a single planned effort in one repo or a cross-repo program driven through the three-skill change lifecycle (`/change:draft`, `/change:execute`, `/change:finalize`) and the `specify change *` CLI verbs.

**Change branch**
The Git branch used to publish a multi-repo change from a registry workspace slot. Its exact form is `specify/<change-name>`, where `<change-name>` comes from `plan.yaml` / `change.md`. `/change:execute` prepares remote-backed slots on this branch before mutation; `specify workspace push` refuses any slot that is not already on this exact branch (`no-branch`) and never creates the branch on the fly.

**Change finalize**
The canonical closure verb for a multi-repo change. `specify change finalize` verifies that plan entries are terminal, required per-project PRs on `specify/<change-name>` are operator-merged, and workspace clones are clean; then it archives `plan.yaml`, `change.md`, and `.specify/plans/<name>/`. With `--clean`, it may remove clean workspace clones after archive succeeds. It never merges PRs.

**Context (plan entry)**
The optional `context` field on a plan entry -- a list of baseline paths (relative to `.specify/`) that are relevant to the change. Briefs use these as a focus hint when scanning baseline directories. Populated automatically by `/change:draft` (e.g. contract paths from a preceding contract change) or manually via `specify plan add --context`.

**Coordinator root**
The repository where an operator runs a coordinated change. It owns `registry.yaml`, `plan.yaml`, `change.md`, `.specify/plans/`, and the registry workspace under `.specify/workspace/`. For a hub topology, the coordinator root may contain no product code; for platform-as-project, it is also one of the registered projects.

**Contract-first**
Authorship pattern where a dedicated contract change defines interface shapes before implementation begins. `/change:draft` inserts these automatically when it detects an API boundary between projects. The contract change uses `adapter: contracts@v1` and has no `project`. Implementation changes depend on the contract change.

**Contract-given**
Authorship pattern where API contracts are imported from an external system or legacy API. The operator places the external files into the change's `contracts/` directory. `/change:draft` inserts import changes when a source is flagged as external.

**Cross-project compatibility classification**
The RM-04 CLI report produced by `specify compatibility check` (strict gate) or `specify compatibility check --change <name> --report-only` (read-only). It walks `registry.yaml`, matches `contracts.produces` to `contracts.consumes`, compares root producer contracts with consumer workspace views, and classifies findings as `additive`, `breaking`, `ambiguous`, or `unverifiable`.

## D

**Define**
The first phase of the slice lifecycle. Generates all artifacts from a description, optionally enriched by source code extraction.

**Delta spec**
A spec that describes modifications to an existing adapter using `ADDED`, `MODIFIED`, `REMOVED`, and `RENAMED` sections. Delta specs merge into the baseline by matching on stable `REQ-XXX` IDs.

**Discovery**
The output of `/change:analyze` during plan authoring. A `discovery.md` file containing adapter summaries (name, description, source files, dependencies, confidence) derived from input analysis.

**Draft**
The authoring skill (`/change:draft`) at the head of the change lifecycle. Mints `change.md` and `plan.yaml` (via `specify change draft`), runs `specify registry validate`, walks the brief pipeline (discovery → optional sync-workspace → propose → optional assignment), and stops at a hand-off summary so the operator can review `plan.yaml` before any per-slice work runs. The deliberate pause between draft and `/change:execute` is the design — the framework does not auto-transition between authoring and execution. Re-entry is via `extend` mode.

## E

**Execute**
The driver skill (`/change:execute`) that automates the define-build-merge loop for each entry in a plan, in dependency order. For multi-repo plans, routes each change to its target project's workspace clone via CWD-based routing. Second of the three peer skills in the change lifecycle (`/change:draft → /change:execute → /change:finalize`).

**Extract**
The process of deriving behavioral specs and design from existing source code, performed by `/spec:extract`. Produces language-agnostic artifacts.

## F

**Finalize**
The closure skill (`/change:finalize`) at the tail of the change lifecycle. Wraps the post-execute steps: `specify workspace push` to publish each `specify/<change-name>` branch as a PR, `gh pr list` (read-only) to confirm every PR is `MERGED`, and `specify change finalize` to archive `plan.yaml`, `change.md`, and `.specify/plans/<name>/`. The skill never merges PRs itself — when any PR is open, it halts with `pr-not-merged` and re-enters cleanly once the operator merges through the forge UI or `gh pr merge`. Third of the three peer skills in the change lifecycle.

## G

**Greenfield bootstrapping**
The `specify workspace sync` fallback for registry projects whose remote repos do not yet exist. Creates the workspace slot, runs `git init`, sets the remote, and scaffolds `.specify/project.yaml` via `specify init` using the initiating repo's adapter cache.

## H

**Hub** (also: **Platform hub**)
A registry-only platform repo. Identified by `project.yaml: hub: true` (with the `adapter:` field omitted). Holds platform state -- `registry.yaml`, `change.md`, `plan.yaml`, `workspace/` -- but is never itself a code project. Code projects live in their own repos and are materialised under `.specify/workspace/<name>/` by `specify workspace sync`. Scaffolded via `specify init --hub`. Contrast with the [platform-as-project](#p) shape where the initiating repo is both the platform repo and a code project (`url: .` in `registry.yaml`). See [Platform repo topologies](../explanation/platform-repo.md).

## I

**Contract id**
The optional `info.x-specify-id` field on a top-level OpenAPI 3.1 / AsyncAPI 3.0 contract. Kebab-case (`^[a-z][a-z0-9-]*$`), ≤ 64 characters, unique across every top-level contract in the repo. The id is a **rename-stable hint** that survives file moves and `info.version` bumps — once set on a contract, never change it. Path-based references in `registry.yaml` remain canonical; the id is not a substitute. Format and uniqueness are enforced by the declared `contract` WASI tool (`specify tool run contract`, the contracts adapter's post-merge baseline gate) and by the `/contract:openapi` / `/contract:asyncapi` verifier intents only when the field is present — contracts without one remain valid indefinitely.

**Change shapes (three)**
The three input topologies the platform-first loop handles uniformly: `migrate-legacy` (sources via `--source <key>=<git-url-or-path>`, targets are existing or newly-minted registered projects), `new-feature` (sources via `--from <docs>`, targets are existing registered projects with new ones spawned at assignment time via the registry-proposal sub-step), and `update-existing` (no input flags, targets are existing registered projects, baseline accumulation in workspace clones is the dominant signal). All three flow through the same three-skill change lifecycle (`/change:draft → /change:execute → /change:finalize`).

## L

**Layout artifact**
A schema-validated YAML document (`layout.yaml`, Vectis only) that captures the spatial layout intent for each screen *before* `/spec:define` runs — regions, group hierarchy, gap / padding / align / size, token references, asset references, and the optional cross-shell `component: <slug>` directive, with no `bind` / `event` / `maps_to` / overlay `trigger` / navigation / `*-when` wiring keys yet. Produced by layout inferers (the [`screenshots` source adapter](../../sources/screenshots/adapter.yaml) is the first-party producer; future Figma and source-code inferers reuse the same contract) or hand-authored. Validated by `specify tool run vectis -- validate layout`, which rejects the wiring keys and enforces the structural-identity rule. Consumed by the composition brief during `/spec:define`, which produces the wired [composition artifact](#c).

**Layered stack**
Specify is organised in three layers above the `specify` CLI substrate: Layer 0 — configuration (`project.yaml`, `adapter.yaml`, `specify init`, `specify adapter`); Layer 1 — executing a change (the single-slice define-build-merge loop: `/spec:define`, `/spec:build`, `/spec:merge`, plus supporting skills); and Layer 2 — planning a change (the three peer skills `/change:draft`, `/change:execute`, `/change:finalize`, plus `/change:analyze`, all of which read or write `registry.yaml` and `plan.yaml`). See [The Layered Stack](../explanation/layered-stack.md) for the full picture.

**Lifecycle state**
The current status of a slice: `created`, `defining`, `defined`, `building`, `complete`, `merged`, or `dropped`. `defining` and `building` are transient states indicating a phase is in-flight. Managed by the CLI via `.metadata.yaml`.

## M

**Merge**
The third phase of the slice lifecycle. Applies spec deltas, contract deltas, and composition deltas (Vectis) to the baseline and archives the slice. When running inside a workspace clone, `/spec:merge` auto-commits the merged baseline.

**Merge key**
The stable `ID: REQ-XXX` line in a spec requirement. Used to match delta spec operations to baseline requirements during merge.

## O

**Opaque replacement**
The merge semantics used for contract files. Unlike spec files (which use the ADDED/MODIFIED/REMOVED delta format), contract files are replaced wholesale during merge -- `specify slice merge run` copies the slice's `contracts/` files into `contracts/`, replacing files that share a path. Files absent from the slice are left untouched.

**Orchestrate (mode)**
*Historical:* the umbrella mode `/change:plan <name> orchestrate` that strung together brief → registry validate → plan → execute loop → workspace push → operator PR merge → `specify change finalize` as a single command. Removed in favour of the explicit three-skill change lifecycle. The seven-step body survives, redistributed across `/change:draft` (steps 1–3), `/change:execute` (step 4), and `/change:finalize` (steps 5–7). See the [decision log](../explanation/decision-log.md#three-skill-change-lifecycle-rfc-23) for the rename trail.

## P

**Phase outcome**
A classification (`success`, `failure`, `deferred`, or `registry-amendment-required`) written to `.metadata.yaml` after a phase completes. Used by `/change:execute` to determine whether to transition a plan entry to `done`, `failed`, or `blocked`. The `registry-amendment-required` variant carries a structured payload `{ proposed-name, proposed-url, proposed-adapter, proposed-description, rationale }` and triggers the operator-driven recovery sequence -- the framework never auto-modifies the registry.

**Plan**
An ordered, dependency-aware list of slices stored in `plan.yaml`. The change's table of contents.

**Plan health diagnostics**
The four extra checks `specify plan validate` layers on top of its base shape rules: `cycle-in-depends-on` (dependency cycles in `depends-on`), `orphan-source-key` (top-level `sources:` keys no entry references), `stale-workspace-clone` (clones whose registry signature has drifted), and `unreachable-entry` (pending entries blocked by `failed`/`skipped` predecessors). The first triage step when `/change:execute loop` reports `stuck`. Previously surfaced through the retired `specify plan doctor` verb.

**Plan (skill)**
*Historical:* the planning skill `/change:plan` that authored `plan.yaml` (default mode) and, with the `orchestrate` positional, drove brief → registry validate → plan → execute → push → PR merge → finalize as one command. Replaced by the three-skill change lifecycle: authoring is owned by `/change:draft`, per-slice execution by `/change:execute`, and post-execute close by `/change:finalize`. The `orchestrate` umbrella mode is removed outright. The matching CLI verb `specify change create` is renamed to `specify change draft`. See the [decision log](../explanation/decision-log.md#three-skill-change-lifecycle-rfc-23) for the rename trail.

**Platform-as-project**
The single-repo platform topology where the initiating repo is both the platform repo and a code project. Identified by `url: .` on the repo's own registry entry. Phase pipelines run normally because `project.yaml:adapter:` resolves to a real adapter (`hub:` is absent or `false`). Still permitted for single-repo and small-team cases. Contrast with [Hub](#h). See [Platform repo topologies](../explanation/platform-repo.md).

**Plugin**
A Cursor marketplace package that provides skills, rules, and references for a specific domain (Specify, Change, Omnia, Vectis, Contract, RT, Client).

**Project (plan routing)**
The `project` field on a plan entry that names the registry project a change targets. Required on every entry when `registry.yaml` declares multiple projects; optional (or absent) for single-repo plans. Drives CWD-based routing during execution.

**Project assignment**
The step during `/change:draft` (multi-repo only, brief-pipeline step 4(d)) that infers which registry project each plan entry targets. Uses description match, baseline-spec affinity, and adapter compatibility as signals. Assignments are presented to the operator for review and written via `specify plan amend --project`.

**Proposal**
The first artifact generated during define. Captures why the change exists, what is in scope, and which adapters are affected.

## R

**Registry**
`registry.yaml` -- a platform catalogue declaring the repos in a multi-repo system. Each entry has a name, URL, adapter identifier, and domain description.

**Registry amendment** (also: **`registry-amendment-required`**)
The phase outcome variant raised when a phase skill discovers that a adapter needs a new registry project (e.g. `/spec:extract` surfacing tangled code that should split into a new repo). The driver classifies the outcome as `blocked`, records the structured payload in the dropped change's `journal.yaml`, and surfaces the proposal. The canonical recovery sequence is `specify registry add <proposed-name> --url <proposed-url> --adapter <proposed-adapter> --description "<proposed-description>"` -> `specify workspace sync` -> `specify plan amend <change> --project <proposed-name>` -> `specify plan transition <change> pending` -> re-run `/change:execute`. The framework never auto-modifies the registry.

**Registry workspace**
The derived local view of registry projects under `.specify/workspace/`. `specify workspace sync` creates or refreshes slots from `registry.yaml`; without selectors it syncs all registry projects, and with selectors it materialises only the selected slots. The registry workspace is scratch execution state, not durable source state.

**Requirement ID**
A stable identifier (`REQ-001`, `REQ-002`, ...) assigned to each behavioral requirement in a spec. Serves as the merge key across delta specs.

**Routing (CWD-based)**
The mechanism by which `/change:execute` routes each multi-repo plan entry to its target project. The driver changes working directory to the target project's workspace clone before invoking phase skills; phase skills run unmodified in whatever directory the driver places them in.

## S

**Skill**
An agent-driven orchestrator invoked with a slash-command prefix (e.g. `/spec:define`, `/omnia:crate-writer`). Skills delegate deterministic work to the CLI and use judgment for everything else.

**Skill directive tag**
An HTML comment in `tasks.md` (e.g. `<!-- skill: omnia:crate-writer -->`) that routes a task to a specific specialist skill during build.

**Slice**
The single unit that flows through the fixed `define -> build -> merge` loop. Each slice has its own proposal, specs, design, tasks, metadata, and merge step, and lives under `.specify/slices/<name>/`.

**Spec**
A behavioral specification at `specs/<adapter>/spec.md`. Contains requirements with stable IDs, scenarios (WHEN/THEN), error conditions, and optional metrics.

**Spec-first (inline derivation)**
Authorship pattern where contracts are derived inline from specs during a single slice's define phase. Used as a convenience fallback for single-repo services with no external consumers and no API boundary. The baseline is empty, so the delta is the full contract set.

**Sync workspace**
The phase during `/change:draft` (multi-repo only) that clones registry projects into `.specify/workspace/` and inventories their baseline specs. Produces `workspace.md`.

## T

**Top-level contract**
A YAML file under root `contracts/` whose root carries `openapi:` (OpenAPI 3.1 document) or `asyncapi:` (AsyncAPI 3.0 document). Format detection decides what counts — never directory layout, file name, or a custom marker. Top-level contracts are the only files subject to the contract validation rules (SemVer `info.version`; format + cross-repo uniqueness on `info.x-specify-id` when present). Standalone JSON Schemas under `contracts/schemas/` are payload vocabulary referenced via `$ref` from a top-level contract — they are **not** top-level themselves.

## W

**Workspace**
The registry workspace under `.specify/workspace/`: a derived local view of registered projects. Each child is a workspace slot. It is read-only during planning (sync-workspace phase) and writable during execution (`/change:execute` routes define-build-merge into the selected slot via CWD-based routing). Local commits are published through `specify workspace push`; PR merge remains an operator action outside Specify.

**Workspace merge**
Retired PR-landing automation. `specify workspace merge` is no longer an active CLI subcommand. Merge through the forge UI, `gh pr merge`, or your normal merge queue, then run `specify change finalize`.

**Workspace slot**
One project-specific child of the registry workspace, normally `.specify/workspace/<project>/`. A slot is a Git clone for remote registry URLs or a symlink for local targets. `workspace status` reports its path, materialisation type, configured target, actual origin or symlink target, branch, HEAD, dirty state, exact change-branch match, `.specify/project.yaml` presence, and active slices.

**Workspace tier 1** (also: **Legacy-source clone**)
The ephemeral, read-only clone materialised under `.specify/plans/<name>/analyze/<key>/` by `/change:analyze` (using the inlined guarded `git clone` snippet documented at [`plugins/change/skills/analyze/SKILL.md` §*Cloning a source tree*](../../plugins/change/skills/analyze/SKILL.md) when the source is a git URL) so the discovery brief can read source code that is not on your local disk. Belongs to a single change and is swept into `.specify/archive/plans/<YYYYMMDD>-<name>/` by `specify plan archive`. Anything edited inside a tier-1 clone moves into the archive when the change ends -- it never propagates back to the original source. See [Workspace tiers](../explanation/workspace-tiers.md).

**Workspace tier 2** (also: **Registered project clone**)
The durable, read-write slot materialised under `.specify/workspace/<name>/` by `specify workspace sync` from an entry in `registry.yaml`. Belongs to the platform, not to any one change; persists across changes. `/change:execute` `chdir`s into this slot before invoking the phase skills, so the slice directory, the merged baseline, and the workspace's git history accumulate here. `specify workspace push` is the explicit publication gate that opens or updates PRs from `specify/<change-name>`. See [Workspace tiers](../explanation/workspace-tiers.md).
