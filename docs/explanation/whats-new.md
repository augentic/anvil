# What's New Since v0.23

This page captures the **additive** changes to the Specify framework since the v1 CLI cleanup landed in v0.23. The v1 cleanup was a routing-only reshape (renamed verbs, no new behaviour); the work below adds new capabilities.

The bulk of the additions ship under [RFC-9: Platform-First Operator Experience](https://github.com/augentic/specify/blob/main/rfcs/rfc-9-platform.md) and [RFC-8: API Contracts](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-8-api-contracts.md). RFC-9 closes the operator-experience gaps in the cross-repo loop; RFC-8 introduces contracts as platform-level artifacts. The most recent additions land [RFC-13](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-13-extensibility.md) (capability rename, platform-component split, change/slice vocabulary), [RFC-14](../../rfcs/archive/rfc-14-workspace.md) (workspace branch and PR ownership), [RFC-15](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-15-wasm-plugins.md) (declared WASI capability tools), and [RFC-16](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-16-wasi-vectis.md) (Vectis WASI tools and `specify-vectis` retirement).

## RFC-13 — capability rename and platform-component split

[RFC-13](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-13-extensibility.md) reframes Specify's extensibility model. The "schema" noun is renamed to **capability** throughout the framework, the `change` / `initiative` lifecycle nouns are renormalised, and the registry and change orchestration become first-party **platform components** rather than capabilities.

### Capability vocabulary

- **`schemas/` → `capabilities/`.** First-party capabilities now live at `capabilities/<name>/capability.yaml` with an explicit JSON Schema at `capabilities/capability.schema.json`. The legacy `schemas/<name>/schema.yaml` layout is gone.
- **`specify schema {resolve, check, pipeline}` → `specify capability {resolve, check, pipeline}`.** The CLI surface follows the noun rename.
- **`/spec:init <capability>`.** The init positional is now a capability identifier (a bare name like `omnia`, an `https://…` URL, or a `file:///…` URI), with optional `@ref` suffixes for git pinning. The `--schema-uri` flag is gone; `specify init` invoked with neither a capability positional nor `--hub` errors with `init-requires-capability-or-hub`.
- **`capability.yaml` is closed and minimal.** The post-RFC manifest drops the legacy `domain` and `extends` fields. Tech-stack guidance, architectural notes, and testing context belong in capability references and skills, not in always-loaded manifest metadata.
- **`pipeline.plan` is rejected.** Both `capabilities/capability.schema.json` (this repo) and `schemas/capability.schema.json` (CLI) reject `pipeline.plan` outright; planning is platform-component orchestration, not capability-owned per-slice work. Planning briefs live with the change-planning skill at [`plugins/change/skills/plan/briefs/<capability>/`](../../plugins/change/skills/plan/briefs/).

### Lifecycle vocabulary: slice ↔ change

The two lifecycle nouns are now stable:

- **Slice** — the single unit that flows through the fixed `define → build → merge` loop. Each slice has its own proposal, specs, design, tasks, and merge step; lives at `.specify/slices/<name>/`. Driven by `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop` and the `specify slice *` CLI verbs.
- **Change** — the operator-defined umbrella that coordinates one or more slices through `change.md` + `plan.yaml`. Driven by `/change:plan`, `/change:execute`, and the `specify change *` CLI verbs (which include the `specify change plan *` subresource).

Pre-RFC-13 the per-loop unit was called "change" and the umbrella was called "initiative". Both were renamed in Phase 3 of the RFC; "the change loop" no longer exists as a phrase — call it the *slice loop*. Current releases expect `.specify/slices/` and `change.md`; the temporary RFC-13 migration shims have been removed.

### Platform components are not capabilities

The registry and the change component are first-party Specify components — they have commands, libraries, and files, but they do not appear in any `capability.yaml`, do not participate in the manifest protocol, and are never activated through a capability-name switch. The dependency invariant is hard-coded: **`specify-core` does not depend on `specify-registry` or `specify-change`, and `specify-registry` does not depend on `specify-change`.** Platform components compose downward; they never re-enter the core. See [Registry reference](../reference/registry.md), [Change component reference](../reference/change-component.md), and [Capabilities and Plugins](capabilities-and-plugins.md).

### Hub project shape simplified

A hub now carries `project.yaml { hub: true, … }` with the `capability:` field omitted (its absence is what disables capability resolution and the per-project phase pipelines). The legacy `schema: hub` sentinel is removed in the same release that lands the capability rename.

## RFC-14 — workspace branch and PR ownership

[RFC-14](../../rfcs/archive/rfc-14-workspace.md) tightens the cross-repo landing contract so Specify prepares and publishes work, but operators own the PR merge decision.

- **`/change:execute` prepares workspace branches.** For each routed plan entry, the driver materialises only the selected workspace slot when needed, prepares `specify/<change-name>` before phase writes, runs define-build-merge in that slot, and commits non-baseline residue as `specify: residue <slice-name>` after `/spec:merge` succeeds.
- **`/spec:merge` owns only the baseline commit.** In workspace clones, the merge auto-commit stages `.specify/specs/` and `.specify/archive/` only, with message `specify: merge <slice-name>`. Generated code, contracts, tests, and other project outputs are left for `/change:execute`'s residue commit.
- **`specify workspace push` is transport-only.** It verifies each selected workspace is already on `specify/<change-name>`, pushes that branch, and creates or updates the PR. It does not create branches on the fly, does not create commits, never pushes default branches, and never merges PRs. A checkout on `main`, `master`, `origin/HEAD`, or any other branch reports `no-branch`; drive the slot through `/change:execute` or check out `specify/<change-name>` explicitly before pushing.
- **PR merge is operator-owned.** Merge through the forge UI, `gh pr merge`, or the team's normal review queue. `specify change finalize` only verifies that each PR is already merged and that workspace clones are clean before archiving the plan.
- **`specify workspace merge` is removed.** Merge PRs through the forge UI, `gh pr merge`, or the team's normal merge queue, then run `specify change finalize`.

## RFC-15 — declared WASI capability tools

[RFC-15](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-15-wasm-plugins.md) introduces a deterministic helper-tool model. Capability authors and project authors declare WASI command components, and the `specify` host resolves, caches, permissions, and runs them through a single CLI surface — `specify tool`.

### Two declaration sites

- **Project scope** — `.specify/project.yaml` carries an optional `tools:` array. Use it for repo-private helpers, local development overrides, or hub projects that have no capability.
- **Capability scope** — capabilities ship a `tools.yaml` sidecar next to `capability.yaml`. The `capability.yaml` schema remains closed (no `tools:` field on the manifest itself). Use capability scope when the helper is part of the capability's promised behavior — e.g. a merge validator or deterministic artifact checker.

Project scope wins on collision, so an operator can redirect a capability-shipped tool to a local build or pinned mirror without editing the capability. Within a single declaration site, tool names must be unique. The cache is segmented by declaration scope (`project--<name>/<tool>/<version>/` vs `capability--<name>/<tool>/<version>/`) so ownership stays explicit.

### Permission model

Permissions are directory preopens, not globs. The host canonicalises every path and rejects `..` segments, glob metacharacters, symlink escapes, and direct writes to Specify lifecycle state. Permission entries may use `$PROJECT_DIR` in either scope and `$CAPABILITY_DIR` only in capability-scope declarations. Released first-party tool declarations require `sha256` so cache fills verify the exact component bytes.

### CLI surface

```bash
specify tool list
specify tool fetch <name>
specify tool show <name>
specify tool run <name> -- <args...>
```

See [`specify tool`](../reference/cli/tool.md) for the full surface and [Tool Declarations](tool-declarations.md) for the declaration-site, precedence, cache, permission, and lint model.

### Contract validator becomes the first declared tool

The pre-RFC-13 in-binary `specify contract { list, validate }` family was retired in chunk 2.7 when contracts became a first-party capability owning its own validation behavior. The contracts capability now ships `capabilities/contracts/tools.yaml` declaring a `contract` WASI tool; the merge brief at [`capabilities/contracts/briefs/merge.md`](../../capabilities/contracts/briefs/merge.md) shells out through `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json` as the post-merge baseline gate. The validation rules (SemVer `info.version`, `info.x-specify-id` format, cross-repo id-uniqueness) survived intact; the JSON envelope remains byte-compatible with the retired in-binary validator. See [`specify tool run contract`](../reference/cli/contract.md).

### Future lints

RFC-15 reserves three rule ids that compose with the RFC-5 framework linter once the broader linter has enough context:

- `tool.write-permission-too-broad` — warn on broad writes (including `$PROJECT_DIR`) where root-file scaffolding is not justified.
- `tool.lifecycle-state-write-denied` — reject writes to Specify lifecycle state.
- `skill.invokes-host-binary-with-declared-tool-equivalent` — warn when a brief or skill shells out to a host helper after an equivalent declared tool exists.

## RFC-16 — Vectis WASI tools and `specify-vectis` retirement

[RFC-16](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-16-wasi-vectis.md) applies the RFC-15 declared-tool model to Vectis. Operators install one binary, `specify`; the deterministic Vectis helpers ship as WASI command components declared by `capabilities/vectis/tools.yaml`.

### Two declared tools

- **`vectis-validate`** — read-only validation for Vectis UI input artifacts (`tokens`, `assets`, `layout`, `composition`, `all`). Replaces `specify-vectis validate <mode> [path]`.
- **`vectis-scaffold`** — render-only scaffolding for the Vectis core, iOS, and Android shells. Writes template output under `PROJECT_DIR` using the permissions declared by `capabilities/vectis/tools.yaml`. Replaces `specify-vectis init` and `specify-vectis add-shell`.

Frozen v1 tool arguments:

```bash
specify tool run vectis-validate -- <mode> [path]
specify tool run vectis-scaffold -- core <app-name> [--caps <csv>] [--android-package <package>] [--version-file <path>]
specify tool run vectis-scaffold -- ios <app-name> [--caps <csv>] [--version-file <path>]
specify tool run vectis-scaffold -- android <app-name> [--caps <csv>] [--android-package <package>] [--version-file <path>]
```

`vectis-scaffold` is render-only. It does not run Cargo, Xcode, Gradle, SDK installers, registry updates, or cap-matrix verification. Those host workflow steps belong to the Vectis writer, reviewer, and template-updater skills.

### Host post-processing is skill-owned

The previous `specify-vectis` binary mixed pure rendering with host-toolchain work. RFC-16 splits them: WASI tools handle the deterministic, file-IO-only work; host commands (Cargo, Gradle wrapper bootstrap, `make typegen` / `make package` / `make xcode`, `local.properties`, Java home and NDK detection, prerequisite checks, registry queries, cap-matrix verification) live in Vectis skills as ordinary shell commands the agent runs and journals.

### `specify-vectis` retired

The standalone `specify-vectis` binary is retired in `specify-cli`. The repo evidence supported deletion rather than a deprecation wrapper: `crates/vectis/Cargo.toml` set `publish = false`, release archives only ever packaged the `specify` binary, and `release.md` never listed `specify-vectis` in the public crates.io publish order. Operators no longer need a second binary for Vectis. Historical RFCs may still mention `specify-vectis verify`, `update-versions`, and `versions`; in v1 those concerns live in skill-owned host workflows and the [template-updater skill](../../plugins/vectis/skills/template-updater/SKILL.md), not in a WASI wrapper.

See [Vectis WASI tools](../reference/cli/vectis.md) for the operator-facing surface and migration map.

## v2 layout — platform artifacts at the repo root (specify-cli `0.2.0`)

The on-disk layout split along a clear boundary: **operator-facing platform artifacts** (`registry.yaml`, `plan.yaml`, `change.md`, `contracts/`) live at the repo root; generated `AGENTS.md` guidance also lives at the root with Specify owning only its fenced block; **framework-managed state** (`project.yaml`, `context.lock`, `slices/`, `specs/`, `archive/`, `.cache/`, `workspace/`, `plans/`, `plan.lock`) stays under `.specify/`. The boundary makes the responsibilities explicit — operators own root artifacts and prose outside generated fences; Specify owns `.specify/`.

`project.yaml` stays under `.specify/`. The `contracts@v1` schema id, the `contracts` brief, the merge semantics, the produces/consumes registry roles, and the workspace flow are all unchanged — only the file locations moved. The decision-log entry "Platform artifacts at the repo root, framework state under `.specify/`" carries the design rationale.

## RFC-12 contract-versioning refinement

[RFC-12](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-12-refine-rfc-8.md) refines RFC-8 along three axes — none of them change the artifact tree, the `contracts` brief, the `contracts@v1` schema id, the merge semantics, or the workspace flow. They are recorded as an RFC because two of the three are breaking.

- **`info.version` MUST be SemVer.** Every top-level OpenAPI 3.1 / AsyncAPI 3.0 document under `contracts/` MUST set `info.version` to a value that parses per [semver.org](https://semver.org), including optional prerelease labels (`1.0.0-draft.1`). Existing contracts whose value is a `YYYY-MM-DD` date or a bare major (`"2"`) need a one-line edit before the validator will pass after upgrade. Bump rules (when to advance major / minor / patch) remain skill-side judgement.
- **Optional `info.x-specify-id` rename-stable identifier.** Every top-level contract MAY set `info.x-specify-id` to a kebab-case slug (`^[a-z][a-z0-9-]*$`, ≤ 64 characters; uniqueness enforced across the repo). The id survives file moves and `info.version` bumps. New top-level contracts SHOULD set it; existing contracts MAY add it any time. Path-based references in `registry.yaml` remain canonical — the id is a hint, not a substitute.
- **`contracts.imports` removed from the registry.** The role set on each `registry.yaml` project entry collapses to two: `produces` and `consumes`. Contracts that no project produces are, by definition, externally authored — no separate field is needed to flag them. `specify registry validate` rejects the unknown `imports` key after upgrade, so any surviving usage surfaces immediately.
- **In-binary `specify contract` verbs retired in RFC-13 chunk 2.7.** RFC-12 originally landed `specify contract { list, validate }` as in-binary CLI verbs; RFC-13 retired the family when contracts became a first-party capability owning its own validation behavior. The validation rules survived intact: the contracts capability now declares a [`contract` WASI tool](../reference/cli/contract.md) run through `specify tool run contract -- <BASELINE_DIR> --format json`, and the contracts capability merge brief ([`capabilities/contracts/briefs/merge.md`](../../capabilities/contracts/briefs/merge.md)) is where the post-merge gate runs. There is no replacement for `contract list`; consult the merged `contracts/` directory directly when projecting top-level contracts.

## Vectis capability v3 — design-system-writer removed (RFC-11)

[RFC-11](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-11-ui-spec.md) dissolves the standalone `vectis:design-system-writer` skill and the `design-system` platform enum value. Each shell writer (`ios-writer`, `android-writer`) now reads `tokens.yaml` and `assets.yaml` directly and emits shell-local theme + asset code under its own tree (`iOS/<App>/Theme/` for iOS, `Android/.../ui/theme/` for Android). The Vectis capability bumps from `2` to `3` (the manifest moved from `schemas/vectis/schema.yaml` to `capabilities/vectis/capability.yaml` as part of RFC-13 — see §RFC-13 below) and the `plugins/vectis/skills/design-system-writer/` directory is deleted. Projects that reference `design-system` in their Platforms list or import `VectisDesign` / `:vectis-design` should migrate to the shell-local theming model.

## RFC-10 plugin namespace renormalisation (v0.25.0)

[RFC-10](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-10-skills.md) renormalises the Cursor plugin / slash-command surface so each skill name is plugin-qualified, each plugin is a coherent capability domain, and each SKILL.md body fits inside Anthropic's 500-line ceiling. **It is a breaking change to the slash-command namespace** — the ship marker is the marketplace bump from `0.24.3` to `0.25.0`. **No persisted artifact, schema, brief id, validation rule, or registry role changed by RFC-10**: the `contracts` brief id, the `contracts@v1` schema, the `contracts/` baseline directory, the `contracts.*` validation rule ids, and the `contracts.{produces,consumes,imports}` registry roles all kept their RFC-10-era names. (RFC-12 has since dropped `contracts.imports`; see the §RFC-12 entry above.) For the full migration map (with both old and new slash forms) see [`rfcs/archive/rfc-10-skills.md`](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-10-skills.md).

### Renamed

- The SoW-writer skill moved from the former `plan` plugin to the `client` plugin. The new slash form is `/client:sow-writer`.
- The contracts plugin was reorganised by format. Every former `/contracts:*` invocation is now `/contract:*` (specifics under *Split* below). The `contracts` brief id, the `contracts@v1` schema, and the `contracts/` baseline directory keep their original names — only the Cursor plugin / slash-command surface changed.

### Split

The former `contracts` plugin shipped three intent-named skills (`writer`, `validator`, `importer`) that each branched on format internally. The `contract` plugin inverts that axis: three format-named skills, each carrying author / import / verify intents internally and dispatching via a per-skill intent table.

| Old (former lifecycle skills) | New (`contract` plugin) |
|---|---|
| `writer`, `validator`, `importer` | `/contract:openapi`, `/contract:asyncapi`, `/contract:json-schema` |

Each new skill handles author / import / verify intents internally. The former validator's `--mode {single, cross-project}` flag becomes an internal verifier option per format. The `cross-project` verifier mode is now the merge-time baseline validator delegate over `specify tool run contract`; consumer-impact classification lives under `specify compatibility`.

### Removed

- The former `rt` plugin's `git-cloner` skill has been deleted. Cloning a remote source tree is no longer a dedicated skill; the two callers (`plugins/spec/skills/analyze/SKILL.md` and `plugins/rt/skills/wiretapper/SKILL.md`) now inline a guarded `git clone` snippet directly.

### Other notable changes

- **Plugin-qualified skill names.** Every SKILL.md `name:` is now globally unique by construction. Skills under `plugins/<plugin>/` use the directory name as their prefix (e.g. `omnia-crate-writer`, `vectis-core-reviewer`, `contract-openapi`, `client-sow-writer`); skills under `plugins/spec/` use the `specify-` prefix instead (so the operator-facing product name surfaces in discovery, e.g. `specify-init`, `specify-plan`).
- **500-line ceiling per SKILL.md.** Every skill body now fits under Anthropic's 500-line guidance; depth pushes one level out into siblings (`references/`, `examples/`, or topical files such as `author.md` / `importer.md` / `verifier.md`). `make checks` enforces the ceiling.
- **Phase-outcome contract authored once.** The four phase skills (`define`, `build`, `merge`, `drop`) used to repeat the four-of-one outcome contract inline. They now link to a single source of truth at `plugins/spec/references/phase-outcome-contract.md`, eliminating drift.
- **Forbidden frontmatter keys.** The skill schema (`.cursor/schemas/skill.schema.json`) now rejects `license`, `compatibility`, `metadata`, `disable-model-invocation`, `when_to_use`, `user-invocable`, and `paths` in SKILL.md frontmatter; license is declared once in the plugin manifest and the repo `LICENSE` file, and the rest belong in the body.
- **`argument-hint` simplified.** The hint now names the **primary positional** argument only, with angle brackets for required (`<slice-dir>`) and square brackets for optional (`[crate-name]`); flags and secondary positionals move into the body's *Invocation* section. The earlier `?` / `--` / `|` syntax is gone.

For the full rationale and migration plan, see [rfcs/archive/rfc-10-skills.md](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-10-skills.md) and [rfcs/rfc-10-plan.md](https://github.com/augentic/specify/blob/main/rfcs/rfc-10-plan.md).

## Hub topology

A **registry-only platform hub** (RFC-9 ?1D) is now the canonical starting shape for a multi-repo change. The hub holds platform state -- `registry.yaml`, `change.md`, `plan.yaml`, `workspace/` -- and is never itself a code project.

```bash
specify init --hub --name shop-platform
```

The flag scaffolds a sentinel `project.yaml { hub: true, ... }` (the `capability:` field is omitted on hubs) that disables phase pipelines on the hub itself, plus an empty `registry.yaml`. `change.md` and `plan.yaml` are created later by `specify change create` and `specify change plan create`. `Registry::validate_shape` extends with a `hub-only` mode that rejects any registry entry whose `url` is `.`.

The platform-as-project shape (initiating repo with `url: .`) is still permitted for single-repo and small-team cases.

- Reference: [`specify init`](../reference/cli/init.md)
- Explanation: [Platform repo topologies](platform-repo.md)
- How-to: [Bootstrap a Platform Hub](../how-to/bootstrap-a-platform-hub.md)

## Two-tier workspace model

Specify now distinguishes two kinds of clones with very different lifecycles:

- **Tier 1 (legacy-source clone)**: ephemeral, read-only, lives at `.specify/plans/<name>/analyze/<key>/`. Materialised by `/spec:analyze`; swept by `specify change plan archive`.
- **Tier 2 (registered project clone)**: durable, read-write, lives at `.specify/workspace/<name>/`. Materialised by `specify workspace sync`; pushed by `specify workspace push`.

The distinction was always implicit; RFC-9 ?1E codifies it so operators stop losing tier-1 writes or expecting tier-2 clones to disappear after a change.

- Explanation: [Workspace Tiers](workspace-tiers.md)

## `/change:plan <name> orchestrate` umbrella mode (Layer 4)

A Layer 4 mode of `/change:plan` (RFC-9 ?2C) drives the cross-repo loop end-to-end as a single operator action:

```text
/change:plan <name> orchestrate [shape ...] [from ...] [source ...]
```

> **Note.** This was originally a separate `/spec:initiative` skill; it was folded into `/change:plan` as a flag-gated `orchestrate` mode in a progressive-disclosure pass. The seven-step umbrella sequence is unchanged.

The mode composes: brief -> registry validate -> `/change:plan` (default mode) -> `/change:execute loop` -> `specify workspace push` -> operator PR merge -> `specify change finalize`. Every automated step is a shell-out to a Layer 1 verb or a Layer 3 skill; the orchestration mode adds no new logic. Halts (self-heal, `stuck`, `registry-amendment-required`, unmerged PRs) surface verbatim, and re-running `--orchestrate` against an in-progress change resumes at the first incomplete step.

Three change shapes flow through the same uniform sequence: `migrate-legacy` (sources via `--source`), `new-feature` (docs via `--from`), `update-existing` (no input flags).

- Reference: [`/change:plan <name> orchestrate`](../reference/change-skills/change.md)
- Tutorial: [Cross-Repo Changes](../tutorials/cross-repo-change.md) -> [Landing a Change](../tutorials/landing-a-change.md)
- Explanation: [The Layered Stack](three-layer-stack.md) (Layer 4 row)

## `specify registry add/remove`

Registry mutation is now a CLI verb instead of a hand-edit (RFC-9 ?2A):

```bash
specify registry add <name> --url <url> --capability <capability> --description "..."
specify registry remove <name>
```

`add` validates kebab-case names, URL classification, and the `description-missing-multi-repo` invariant after the write. `remove` warns when plan entries reference the removed project. `/change:plan`'s registry-proposal sub-step (RFC-9 ?2B) shells out to `add` automatically when assignment names a project not yet in `registry.yaml`.

- Reference: [`specify registry`](../reference/cli/registry.md)
- How-to: [Manage Registry Projects](../how-to/manage-registry-projects.md)

## `specify workspace merge` removed

RFC-14 removed the cross-repo PR-landing verb. Specify does not inspect checks, call `gh pr merge`, or merge PRs for the operator; merge PRs through the forge UI or an explicit `gh pr merge`, then run `specify change finalize`.

- How-to: [Land a Change](../how-to/land-a-change.md)

## `specify change plan doctor`

A strict superset of `specify change plan validate` with four additional health diagnostics (RFC-9 ?4B):

| Code | Severity | Meaning |
|------|----------|---------|
| `cycle-in-depends-on` | error | Dependency cycle in `depends-on`. |
| `orphan-source-key` | warning | Top-level `sources:` key not referenced by any entry. |
| `stale-workspace-clone` | warning | Workspace clone signature drifted from registry. |
| `unreachable-entry` | error | Pending entry blocked by `failed`/`skipped` predecessors. |

`plan doctor` is the canonical first triage step when `/change:execute loop` reports `stuck`.

- Reference: [`specify change plan doctor`](../reference/cli/plan.md#specify-plan-doctor)
- Troubleshooting: [Plan doctor diagnostics](../appendices/troubleshooting.md#plan-doctor-diagnostics)

## `specify change finalize`

The canonical closure verb for the platform-first loop (RFC-9 ?4C):

```bash
specify change finalize [--clean] [dry-run]
```

Runs four guards in order (plan-presence, plan terminal-state, per-project PR-state, workspace-cleanliness) before atomically archiving `plan.yaml`, `change.md`, and `.specify/plans/<name>/`. Any guard refusal leaves the on-disk state untouched. `--clean` prunes `.specify/workspace/<peer>/` after the archive completes. Idempotent: re-running after a successful finalize returns `plan-not-found` (the explicit "already finalized" signal).

- Reference: [`specify change finalize`](../reference/cli/change.md#specify-change-finalize)
- How-to: [Land a Change](../how-to/land-a-change.md)

## API contracts (RFC-8)

API contracts are now first-class platform artifacts at `contracts/`, alongside `registry.yaml` and `plan.yaml`. The contract format uses JSON Schema (payload definitions) plus OpenAPI 3.1 (HTTP bindings) and AsyncAPI 3.0 (messaging bindings) -- no proprietary IDL.

The `contracts` brief in the define pipeline runs alignment validation against the baseline; `/change:plan` automatically inserts a contract change before implementation changes when it detects an API boundary between projects (the contract-first authorship pattern). The Contract plugin ships three format-first skills, each carrying author / import / verify intents internally: `/contract:openapi` (HTTP / resource APIs), `/contract:asyncapi` (evented / pub-sub / streaming), and `/contract:json-schema` (shared payload schemas). The `contracts` brief, capability id, and `contracts/` baseline directory keep their original names; only the Cursor plugin / slash-command surface is renamed.

- Reference: [Contract plugin](../reference/plugins/contract.md), [Contracts capability](../reference/capabilities/contracts.md), [Artifact Format -> Contracts](../reference/artifact-format.md#contract-artifacts-api-shape)
- How-to: [Work with Contracts Across Repos](../how-to/cross-repo-contracts.md)

## Cross-project compatibility classification (RM-04)

`specify compatibility report --change <name>` and `specify compatibility check` classify producer-to-consumer contract deltas from `registry.yaml`, root `contracts/`, and `.specify/workspace/<consumer>/contracts/`. Findings are `additive`, `breaking`, `ambiguous`, or `unverifiable`; `compatibility check` exits validation-failed for every non-additive risk. `/change:execute` no longer owns journal or transcript warning side effects for this report.

- How-to: [Resolve Cross-Project Compatibility Findings](../how-to/resolve-cross-project-contract-warnings.md)
- CLI: [specify compatibility](../reference/cli/compatibility.md)

## `registry-amendment-required` outcome

A new phase outcome variant (RFC-9 ?2B) for cases where a phase skill discovers that a change targets a capability needing a new registry project. The outcome carries a structured payload (`{ proposed-name, proposed-url, proposed-schema, proposed-description, rationale }`); the executor classifies it as `blocked`, records the payload in the dropped change's `journal.yaml`, and surfaces the proposal to the operator. The framework never auto-modifies the registry.

The canonical recovery sequence: `specify registry add` -> `specify workspace sync` -> `specify change plan amend <change> --project <new>` -> `specify change plan transition <change> pending` -> re-run `/change:execute`.

- How-to: [Recover from `registry-amendment-required`](../how-to/recover-from-registry-amendment.md)
- Troubleshooting: [Registry amendment required](../appendices/troubleshooting.md#registry-amendment-required)

## v1.x verb renames (RFC-9 ??1F, 1G)

Three renames landed on top of the v1 cleanup so every noun-create verb now uses `create`:

| Old verb (v1) | New verb (v1.x) |
|---|---|
| `specify change init <name>` | `specify change create <name>` |
| `specify change plan init <name>` | `specify change plan create <name>` |
| `specify change plan create <name>` | `specify change plan add <name>` |

The renames ship together so the `plan` group never spent an interim release with `init` and `create` for the same noun.

## See also

- [The Layered Stack](three-layer-stack.md) -- updated for Layer 4.
- [Cross-Repo Changes](../tutorials/cross-repo-change.md) and [Landing a Change](../tutorials/landing-a-change.md) -- the worked example exercising all of the above.
- [Quick Reference](../reference/quick-reference.md) -- single-page cheat sheet for the post-RFC-9 surface.
