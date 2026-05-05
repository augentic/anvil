# Glossary

Canonical definitions for terms used throughout Specify.

## A

**Alignment report**
The output of an `/contract:*` skill's author intent (`/contract:openapi`, `/contract:asyncapi`, or `/contract:json-schema` — picked from the brief context) after running the 6-step algorithm. Summarises coverage (interactions already defined in the baseline), alignment warnings (spec-vs-baseline mismatches), generated delta (new contract files), and normalisation changes. A clean report with zero delta is the expected outcome for implementation changes in a contract-first workflow.

**API contract**
A machine-readable interface definition at `contracts/`. Uses three formats: JSON Schema for payload definitions, OpenAPI 3.1 for HTTP endpoint bindings, and AsyncAPI 3.0 for messaging bindings. Contracts capture the *shape* of interfaces -- endpoint paths, methods, payload schemas, error codes, channel names, message structures. They complement behavioral specs, which capture *what* the system does.

**Artifact**
A structured document that defines part of a change. The core artifacts are `proposal.md`, `spec.md`, `contracts/**/*.yaml`, `design.md`, and `tasks.md`. Schema-specific artifacts extend this set -- the Vectis schema adds `composition.yaml` for screen layout. Artifacts are the contract between human intent and agent execution.

**Archive**
The `.specify/archive/` directory where finalized changes (merged or dropped) and completed plans are stored for audit.

## B

**Baseline**
The accumulated set of merged specs at `.specify/specs/` and merged contracts at `contracts/`. For Vectis projects, also includes the merged `composition.yaml` for screen layout. Represents the current known behavioral and interface state of the system. Future changes produce deltas against the baseline.

**Brief**
A markdown prompt file provided by a schema that drives artifact generation. Briefs are organized into pipelines for each phase (define, build, merge).

**Brief pipeline**
An ordered sequence of briefs declared by a schema for a given phase. The Omnia define pipeline runs: proposal, specs, contracts, design, tasks. The Vectis define pipeline runs: proposal, specs, contracts, composition, design, tasks.

## C

**Capability**
A discrete unit of system behavior that gets its own spec file. In the Omnia schema, capabilities typically correspond to crates. In the Vectis schema, they correspond to features.

**Composition artifact**
A schema-validated YAML document (`composition.yaml`) that describes the spatial layout of each screen in a Vectis application. Organises content into named regions (`header`, `body`, `footer`, `fab`) with a container tree of items and groups carrying flexbox-like layout properties, enriched with the `bind`, `event`, `maps_to`, overlay `trigger`, navigation, and `*-when` wiring keys that connect the layout to ViewModels and specs. Produced by the Vectis define pipeline (the composition brief) between specs and design from a `layout.yaml` input (when present) or from existing baseline composition; consumed by shell writers for deterministic layout generation. The unwired pre-define input is a sibling artifact, [`layout.yaml`](#l) — RFC-11 made that boundary explicit by reserving `composition.yaml` for the wired Specify lifecycle artifact and `layout.yaml` for the input layout intent. See [RFC-11](https://github.com/augentic/specify/blob/main/rfcs/rfc-11-ui-spec.md) (which superseded RFC-7's skeleton/wired duality).

**Change**
A unit of work in Specify, stored at `.specify/changes/<name>/`. Contains the core artifacts (`proposal.md`, `spec.md`, `design.md`, `tasks.md`), optional contract artifacts (`contracts/`), any schema-specific artifacts (e.g. `composition.yaml` for Vectis), and a `.metadata.yaml` file tracking lifecycle state.

**Context (plan entry)**
The optional `context` field on a plan entry -- a list of baseline paths (relative to `.specify/`) that are relevant to the change. Briefs use these as a focus hint when scanning baseline directories. Populated automatically by `/spec:plan` (e.g. contract paths from a preceding contract change) or manually via `specify plan add --context`.

**Contract-first**
Authorship pattern where a dedicated contract change defines interface shapes before implementation begins. `/spec:plan` inserts these automatically when it detects an API boundary between projects. The contract change uses `schema: contracts@v1` and has no `project`. Implementation changes depend on the contract change.

**Contract-given**
Authorship pattern where API contracts are imported from an external system or legacy API. The operator places the external files into the change's `contracts/` directory. `/spec:plan` inserts import changes when a source is flagged as external.

**Cross-project contract validation**
The post-merge check `/spec:execute` runs against the producer's `contracts.produces` list (RFC-9 Section 3B). For each produced contract, the driver finds consumer projects via `contracts.consumes`, runs the format-appropriate `/contract:*` skill (verifier intent, with `--mode cross-project`) against each consumer's workspace clone, and writes any incompatibilities to the merged change's `journal.yaml` as `cross-project-warning:` entries. Warnings never halt the loop; the operator triages them.

## D

**Define**
The first phase of the change lifecycle. Generates all artifacts from a description, optionally enriched by source code extraction.

**Delta spec**
A spec that describes modifications to an existing capability using `ADDED`, `MODIFIED`, `REMOVED`, and `RENAMED` sections. Delta specs merge into the baseline by matching on stable `REQ-XXX` IDs.

**Discovery**
The output of `/spec:analyze` during plan authoring. A `discovery.md` file containing capability summaries (name, description, source files, dependencies, confidence) derived from input analysis.

## E

**Execute**
The Layer 3 driver skill (`/spec:execute`) that automates the define-build-merge loop for each entry in a plan, in dependency order. For multi-repo plans, routes each change to its target project's workspace clone via CWD-based routing.

**Extract**
The process of deriving behavioral specs and design from existing source code, performed by `/spec:extract`. Produces language-agnostic artifacts.

## G

**Greenfield bootstrapping**
The `specify workspace sync` fallback for registry projects whose remote repos do not yet exist. Creates the workspace slot, runs `git init`, sets the remote, and scaffolds `.specify/project.yaml` via `specify init` using the initiating repo's schema cache.

## H

**Hub** (also: **Platform hub**)
A registry-only platform repo. Identified by `project.yaml: schema: hub, hub: true` (RFC-9 Section 1D). Holds platform state -- `registry.yaml`, `initiative.md`, `plan.yaml`, `workspace/` -- but is never itself a code project. Code projects live in their own repos and are materialised under `.specify/workspace/<name>/` by `specify workspace sync`. Scaffolded via `specify init --hub`. Contrast with the [platform-as-project](#p) shape where the initiating repo is both the platform repo and a code project (`url: .` in `registry.yaml`). See [Platform repo topologies](../explanation/platform-repo.md).

## I

**Initiative**
A multi-change program coordinated through a plan. Examples: a migration, a greenfield build, a platform modernisation.

**Contract id**
The optional `info.x-specify-id` field on a top-level OpenAPI 3.1 / AsyncAPI 3.0 contract (RFC-12). Kebab-case (`^[a-z][a-z0-9-]*$`), ≤ 64 characters, unique across every top-level contract in the repo. The id is a **rename-stable hint** that survives file moves and `info.version` bumps — once set on a contract, never change it. Path-based references in `registry.yaml` remain canonical; the id is not a substitute. Format and uniqueness are enforced by `specify contract validate` and the `/contract:openapi` / `/contract:asyncapi` verifier intents only when the field is present — contracts without one remain valid indefinitely.

**Initiative finalize**
The canonical closure verb for the platform-first loop (RFC-9 Section 4C). `specify initiative finalize` runs four guards in order -- plan-presence, plan terminal-state, per-project PR-state (`MERGED` on remote), workspace-cleanliness -- then atomically archives `plan.yaml`, `initiative.md`, and `.specify/plans/<name>/` into `.specify/archive/plans/<YYYYMMDD>-<name>/`. Idempotent: re-running after a successful finalize returns `plan-not-found`, the explicit "already finalized" signal. Optional `--clean` flag prunes `.specify/workspace/<peer>/` clones after the archive completes.

**Initiative shapes (three)**
The three input topologies the platform-first loop handles uniformly (RFC-9 Section Motivation): `migrate-legacy` (sources via `--source <key>=<git-url-or-path>`, targets are existing or newly-minted registered projects), `new-feature` (sources via `--from <docs>`, targets are existing registered projects with new ones spawned at assignment time via the registry-proposal sub-step), and `update-existing` (no input flags, targets are existing registered projects, baseline accumulation in workspace clones is the dominant signal). All three flow through the same seven-step `/spec:plan --orchestrate` sequence (was `/spec:initiative` before the orchestration mode was folded into `/spec:plan`).

## L

**Layout artifact**
A schema-validated YAML document (`layout.yaml`, Vectis only) that captures the spatial layout intent for each screen *before* `/spec:define` runs — regions, group hierarchy, gap / padding / align / size, token references, asset references, and the optional cross-shell `component: <slug>` directive, with no `bind` / `event` / `maps_to` / overlay `trigger` / navigation / `*-when` wiring keys yet. Produced by layout inferers (the screenshot-fronted [`vectis:image-layout-inferer`](../../plugins/vectis/skills/image-layout-inferer/SKILL.md) today; future Figma and source-code inferers per RFC-11 §B/D) or hand-authored by the operator. Validated by `specify vectis validate layout`, which rejects the wiring keys and enforces the §G structural-identity rule. Consumed by the composition brief during `/spec:define`, which produces the wired [composition artifact](#c). RFC-11 introduced the layout / composition split; RFC-7 conflated both into a single `composition.yaml` with a "skeleton" / "wired" mode distinction. See [RFC-11](https://github.com/augentic/specify/blob/main/rfcs/rfc-11-ui-spec.md).

**Layout boundary (operator vs framework)**
The `0.2.0` v2 layout split Specify's on-disk shape along a clear line: **operator-facing platform artifacts** (`registry.yaml`, `plan.yaml`, `initiative.md`, `contracts/`) live at the repo root; **framework-managed state** (`project.yaml`, `changes/`, `specs/`, `archive/`, `.cache/`, `workspace/`, `plans/`, `plan.lock`) lives under `.specify/`. The CLI refuses the legacy v1 layout (where everything sat under `.specify/`) with the stable `legacy-layout` error code; `specify migrate v2-layout` is the one-shot mover that upgrades a v1-layout project in place. See [Migrating to the v2 layout](../how-to/migrate-to-v2-layout.md).

**Legacy-layout error**
The diagnostic the CLI emits (stable code `legacy-layout`, exit 1) when a project-aware verb encounters a v1-layout project (operator artifacts still under `.specify/`). The remediation is always `specify migrate v2-layout`; see the [troubleshooting entry](troubleshooting.md#legacy-layout-error-from-every-cli-verb).

**Layer 1 (CLI primitives)**
The `specify` CLI commands that handle all deterministic operations: change lifecycle, plan CRUD, registry mutation, workspace sync/push/merge, schema resolution, validation. The foundation that skills build on.

**Layer 2 (Change lifecycle)**
The `/spec:define`, `/spec:build`, `/spec:merge` loop and supporting skills (`/spec:init`, `/spec:drop`, `/spec:extract`). Each skill operates on a single change inside `.specify/changes/<name>/` and delegates deterministic work to the Layer 1 CLI.

**Layer 3 (Plan & Drive)**
The skills that coordinate multi-change programs through `plan.yaml`: `/spec:plan` (authors the plan via discovery, propose, and assignment), `/spec:execute` (automates the define-build-merge loop per change with CWD-based routing for multi-repo plans), and `/spec:analyze` (plan-time capability inference). Includes sync-peers for multi-repo registries and project assignment (RFC-3b). Originally called "Initiative orchestration"; renamed to "Plan & Drive" by RFC-9 Section 2C when Layer 4 was promoted above it.

**Layer 4 (Initiative orchestration)**
The orchestration mode of `/spec:plan` (`/spec:plan --orchestrate`, RFC-9 Section 2C) that strings the platform-first loop -- brief, registry validate, plan, execute, push, optional merge, finalize -- into one operator action. Composition only: every step shells out to a Layer 1 CLI verb or a Layer 3 skill; the umbrella adds no new logic. Honours every halt the underlying skills surface and is idempotent on re-entry. Was a dedicated `/spec:initiative` skill before being folded into `/spec:plan`.

**Lifecycle state**
The current status of a change: `created`, `defining`, `defined`, `building`, `complete`, `merged`, or `dropped`. `defining` and `building` are transient states indicating a phase is in-flight. Managed by the CLI via `.metadata.yaml`.

## M

**Merge**
The third phase of the change lifecycle. Applies spec deltas, contract deltas, and composition deltas (Vectis) to the baseline and archives the change. When running inside a workspace clone, `specify merge` auto-commits the merged baseline (RFC-3b).

**Merge key**
The stable `ID: REQ-XXX` line in a spec requirement. Used to match delta spec operations to baseline requirements during merge.

## O

**Opaque replacement**
The merge semantics used for contract files. Unlike spec files (which use the ADDED/MODIFIED/REMOVED delta format), contract files are replaced wholesale during merge -- `specify merge` copies the change's `contracts/` files into `contracts/`, replacing files that share a path. Files absent from the change are left untouched.

## P

**Phase outcome**
A classification (`success`, `failure`, `deferred`, or `registry-amendment-required`) written to `.metadata.yaml` after a phase completes. Used by `/spec:execute` to determine whether to transition a plan entry to `done`, `failed`, or `blocked`. The `registry-amendment-required` variant (RFC-9 Section 2B) carries a structured payload `{ proposed-name, proposed-url, proposed-schema, proposed-description, rationale }` and triggers the operator-driven recovery sequence -- the framework never auto-modifies the registry.

**Plan**
An ordered, dependency-aware list of changes stored in `plan.yaml`. The initiative's table of contents.

**Plan doctor**
`specify plan doctor` (RFC-9 Section 4B). A strict superset of `specify plan validate` that runs every check `validate` runs and then layers four health diagnostics on top: `cycle-in-depends-on` (dependency cycles in `depends-on`), `orphan-source-key` (top-level `sources:` keys no entry references), `stale-workspace-clone` (clones whose registry signature has drifted), and `unreachable-entry` (pending entries blocked by `failed`/`skipped` predecessors). The first triage step when `/spec:execute --loop` reports `stuck`.

**Platform-as-project**
The single-repo platform topology where the initiating repo is both the platform repo and a code project. Identified by `url: .` on the repo's own registry entry. Phase pipelines run normally because `project.yaml:schema:` resolves to a real schema (not `hub`). Still permitted for single-repo and small-team cases. Contrast with [Hub](#h). See [Platform repo topologies](../explanation/platform-repo.md).

**Plugin**
A Cursor marketplace package that provides skills, rules, and references for a specific domain (Specify, Omnia, Vectis, Contracts, RT, Plan).

**Project (plan routing)**
The `project` field on a plan entry that names the registry project a change targets. Required on every entry when `registry.yaml` declares multiple projects; optional (or absent) for single-repo plans. Drives CWD-based routing during execution (RFC-3b).

**Project assignment**
The step during `/spec:plan` (multi-repo only, step 3(d)) that infers which registry project each plan entry targets. Uses description match, baseline-spec affinity, and schema compatibility as signals. Assignments are presented to the operator for review and written via `specify plan amend --project`.

**Proposal**
The first artifact generated during define. Captures why the change exists, what is in scope, and which capabilities are affected.

## R

**Registry**
`registry.yaml` -- a platform catalogue declaring the repos in a multi-repo system. Each entry has a name, URL, schema, and domain description.

**Registry amendment** (also: **`registry-amendment-required`**)
The phase outcome variant added by RFC-9 Section 2B for cases where a phase skill discovers that a capability needs a new registry project (e.g. `/spec:extract` surfacing tangled code that should split into a new repo). The driver classifies the outcome as `blocked`, records the structured payload in the dropped change's `journal.yaml`, and surfaces the proposal to the operator. The canonical recovery sequence is `specify registry add <proposed-name> --url <proposed-url> --schema <proposed-schema> --description "<proposed-description>"` -> `specify workspace sync` -> `specify plan amend <change> --project <proposed-name>` -> `specify plan transition <change> pending` -> re-run `/spec:execute`. The framework never auto-modifies the registry.

**Requirement ID**
A stable identifier (`REQ-001`, `REQ-002`, ...) assigned to each behavioral requirement in a spec. Serves as the merge key across delta specs.

**Routing (CWD-based)**
The mechanism by which `/spec:execute` routes each multi-repo plan entry to its target project. The driver changes working directory to the target project's workspace clone before invoking phase skills; phase skills run unmodified in whatever directory the driver places them in (RFC-3b).

## S

**Schema**
A configuration package that tells Specify how to generate artifacts and build code for a specific target platform. Contains brief pipelines and domain context.

**Skill**
An agent-driven orchestrator invoked with a slash-command prefix (e.g. `/spec:define`, `/omnia:crate-writer`). Skills delegate deterministic work to the CLI and use judgment for everything else.

**Skill directive tag**
An HTML comment in `tasks.md` (e.g. `<!-- skill: omnia:crate-writer -->`) that routes a task to a specific specialist skill during build.

**Spec**
A behavioral specification at `specs/<capability>/spec.md`. Contains requirements with stable IDs, scenarios (WHEN/THEN), error conditions, and optional metrics.

**Spec-first (inline derivation)**
Authorship pattern where contracts are derived inline from specs during a single change's define phase. Used as a convenience fallback for single-repo services with no external consumers and no API boundary. The baseline is empty, so the delta is the full contract set.

**Sync peers**
The phase during `/spec:plan` (multi-repo only) that clones registry projects into `.specify/workspace/` and inventories their baseline specs. Produces `workspace.md`.

## T

**Top-level contract**
A YAML file under root `contracts/` whose root carries `openapi:` (OpenAPI 3.1 document) or `asyncapi:` (AsyncAPI 3.0 document) (RFC-12). Format detection decides what counts — never directory layout, file name, or a custom marker. Top-level contracts are the only files subject to the RFC-12 §Validation rules (SemVer `info.version`; format + cross-repo uniqueness on `info.x-specify-id` when present). Standalone JSON Schemas under `contracts/schemas/` are payload vocabulary referenced via `$ref` from a top-level contract — they are **not** top-level themselves.

## W

**Workspace**
`.specify/workspace/<project>/` -- clones of registry projects materialised by `specify workspace sync`. Read-only during planning (sync-peers phase); writable during execution (`/spec:execute` routes define-build-merge into the clone via CWD-based routing). Local commits are pushed to remotes via `specify workspace push`.

**Workspace merge**
`specify workspace merge` (RFC-9 Section 4A). Squash-merges the open PRs created by `specify workspace push` once their CI is green. Per-project, the verb checks `gh pr checks` against `specify/<initiative-name>` and runs `gh pr merge --squash` when every check is `pass` or `skipping`. Refuses any PR whose `headRefName` is not `specify/<initiative-name>` exactly (the `branch-pattern-mismatch` guard). Never `--admin`, never `--auto`. Best-effort across projects; a single project's failure surfaces in its row without aborting the others.

**Workspace tier 1** (also: **Legacy-source clone**)
The ephemeral, read-only clone materialised under `.specify/plans/<name>/analyze/<key>/` by `/spec:analyze` (using the inlined guarded `git clone` snippet documented at [`plugins/spec/skills/analyze/SKILL.md` §*Cloning a source tree*](../../plugins/spec/skills/analyze/SKILL.md) when the source is a git URL) so the discovery brief can read source code that is not on the operator's local disk. Belongs to a single initiative and is swept into `.specify/archive/plans/<YYYYMMDD>-<name>/` by `specify plan archive`. Anything an operator edits inside a tier-1 clone moves into the archive when the initiative ends -- it never propagates back to the original source. See [Workspace tiers](../explanation/workspace-tiers.md).

**Workspace tier 2** (also: **Registered project clone**)
The durable, read-write clone materialised under `.specify/workspace/<name>/` by `specify workspace sync` from an entry in `registry.yaml`. Belongs to the platform, not to any one initiative; persists across initiatives. `/spec:execute` `chdir`s into this clone before invoking the phase skills, so the change directory, the merged baseline, and the workspace's git history accumulate here. `specify workspace push` is the explicit release gate that publishes those local commits. See [Workspace tiers](../explanation/workspace-tiers.md).
