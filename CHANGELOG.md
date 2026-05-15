# Changelog

<!-- NOTE: exact version pegs need backfilling. Only v0.23 (baseline), v0.25.0 (RFC-10), and specify-cli v0.2.0 (v2 layout) are pinned from prior release-notes prose; other RFC sections ship under v0.x placeholders below. -->

All notable changes to the Specify framework are documented here.

The format is loosely based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Breaking

- `specify change plan create` removed. Use `specify change create <name> [--source <key>=<path-or-url> ...]` instead — the merged verb scaffolds `change.md` and `plan.yaml` together atomically (refuses with `already-exists` if either file is present, writing neither in that case). The diagnostic code emitted by the new verb's name validator is `change-name-not-kebab` (renamed from the brief-only `change-brief-name-not-kebab`); the `plan-source-duplicate-key` diagnostic for repeated `--source` keys is unchanged. The old `change-plan-create` JSON envelope (carrying only a `plan` ref) is replaced by the new `change-create` envelope `{ name, brief: { path }, plan: { path } }`. The plan skill's step 2, the orchestration umbrella, and every per-shape transcript fixture now collapse the prior two-step bootstrap into a single `specify change create` call; orchestration step 3 forwards `--extend` to `/change:plan` whenever the brief and plan are already on disk.

### Other

- Code & skill review consolidation: trimmed `change/skills/plan` and `spec/skills/init` SKILLs to live under their 250-line cap by relocating long sections into `plugins/{change,spec}/references/`; trimmed the `spec/skills/extract` description and moved its `Principles` block to a reference doc; standardised `argument-hint:` quoting across every plugin; added a `## What this skill does NOT do` table to `/spec:{define,extract,init,build}` mirroring the canonical one in `/change:execute`; added a "Related coding standards" cross-link to the CLI repo's AGENTS.md.

## v0.x — RFC-16 — Vectis WASI tools and `specify-vectis` retirement

- Standalone `specify-vectis` binary removed; operators install only `specify`.
- Vectis ships two declared WASI tools through `capabilities/vectis/tools.yaml`: `vectis validate` (read-only) and `vectis scaffold` (render-only).
- Frozen v1 invocations: `specify tool run vectis -- validate <mode> [path]` and `specify tool run vectis -- scaffold {core,ios,android} <app-name> ...`.
- Host post-processing (Cargo, Gradle, Xcode, NDK detection, registry queries, cap-matrix verification) moves into Vectis skills as shell commands the agent runs and journals.

## v0.x — RFC-15 — declared WASI capability tools

- New `specify tool {list, fetch, show, run}` CLI surface for declared WASI command components.
- Tools are declared either in `.specify/project.yaml` (project scope) or in a `tools.yaml` sidecar next to `capability.yaml` (capability scope); project scope wins on collision.
- Permissions are directory preopens with `$PROJECT_DIR` (both scopes) and `$CAPABILITY_DIR` (capability scope only). The host canonicalises paths and rejects `..`, glob metacharacters, symlink escapes, and writes to Specify lifecycle state.
- Released first-party tool declarations require `sha256` so cache fills verify exact component bytes.
- The contracts capability ships a `contract` WASI tool replacing the retired in-binary `specify contract { list, validate }` family; the JSON envelope remains byte-compatible.
- Reserves lint ids `tool.write-permission-too-broad`, `tool.lifecycle-state-write-denied`, and `skill.invokes-host-binary-with-declared-tool-equivalent`.

## v0.x — RFC-14 — workspace branch and PR ownership

- `/change:execute` materialises the selected workspace slot, prepares `specify/<change-name>` before phase writes, and commits non-baseline residue as `specify: residue <slice-name>` after `/spec:merge`.
- `/spec:merge` auto-commits stage only `.specify/specs/` and `.specify/archive/` with message `specify: merge <slice-name>`; generated code, contracts, and tests are left for the residue commit.
- `specify workspace push` is transport-only: verifies each slot is already on `specify/<change-name>`, pushes that branch, and creates or updates the PR. It never branches on the fly, commits, pushes default branches, or merges PRs.
- A checkout on `main`, `master`, `origin/HEAD`, or any other branch reports `no-branch`.
- `specify workspace merge` removed. Merge PRs through the forge UI or `gh pr merge`, then run `specify change finalize`.

## v0.x — RFC-13 — capability rename and platform-component split

- The extension noun is renamed to **capability** across the framework; first-party capabilities live at `capabilities/<name>/capability.yaml` with a closed JSON Schema.
- New CLI surface `specify capability {resolve, check, pipeline}`; `/spec:init <capability>` accepts a bare name, `https://` URL, or `file://` URI with optional `@ref` for git pinning.
- `specify init` invoked with neither a capability positional nor `--hub` errors with `init-requires-capability-or-hub`.
- `capability.yaml` drops the legacy `domain` and `extends` fields and rejects `pipeline.plan`; planning briefs live with the change-planning skill under `plugins/change/skills/plan/briefs/<capability>/`.
- The lifecycle nouns stabilise: **slice** (`.specify/slices/<name>/`, driven by `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop`) and **change** (operator umbrella via `change.md` + `plan.yaml`, driven by `/change:plan`, `/change:execute`).
- The registry and change orchestrator are first-party **platform components** — not capabilities — with a hard-coded dependency invariant: `specify-core` does not depend on `specify-registry` or `specify-change`, and `specify-registry` does not depend on `specify-change`.
- Hubs carry `project.yaml { hub: true, ... }` with the `capability:` field omitted, disabling capability resolution and phase pipelines.
- The in-binary `specify contract { list, validate }` family is retired (chunk 2.7); contract validation now runs through `specify tool run contract`.

## v0.x — RFC-12 — contract-versioning refinement

- `info.version` MUST be SemVer on every top-level OpenAPI 3.1 / AsyncAPI 3.0 document under `contracts/`. Existing date-style or bare-major values need a one-line edit before the validator passes after upgrade.
- Optional `info.x-specify-id` rename-stable identifier (`^[a-z][a-z0-9-]*$`, ≤64 characters, repo-unique) survives file moves and version bumps.
- The `contracts.imports` registry role is removed; the role set collapses to `produces` and `consumes`. `specify registry validate` rejects the unknown key after upgrade.

## v0.x — Cross-project compatibility classification

- `specify compatibility check [--change <name>] [--report-only]` classifies producer-to-consumer contract deltas as `additive`, `breaking`, `ambiguous`, or `unverifiable`. The bare verb exits validation-failed for every non-additive risk; `--report-only` prints the same payload and always exits `0` for read-only audits.
- `/change:execute` no longer owns journal or transcript warning side effects for this report.

## v0.x — Platform-first operator surface (RFC-9)

- `/change:plan <name> orchestrate` umbrella drives brief → registry validate → plan → execute loop → workspace push → operator PR merge → `specify change finalize` as a single operator action across three change shapes (`migrate-legacy`, `new-feature`, `update-existing`).
- `specify init --hub --name <name>` scaffolds a registry-only platform hub; `Registry::validate_shape` gains a `hub-only` mode rejecting any registry entry whose `url` is `.`.
- New CLI verbs: `specify registry add/remove`, the four health diagnostics on `specify change plan validate` (`cycle-in-depends-on`, `orphan-source-key`, `stale-workspace-clone`, `unreachable-entry`), and `specify change finalize` (`--clean`, `dry-run`) for atomic plan closure.
- The two-tier workspace model is codified: tier 1 (legacy-source clones under `.specify/plans/<name>/analyze/<key>/`) is ephemeral and read-only; tier 2 (registered project clones under `.specify/workspace/<name>/`) is durable and read-write.
- New `registry-amendment-required` phase outcome carries a structured proposal payload (`{ proposed-name, proposed-url, proposed-capability, proposed-description, rationale }`) for changes that target a capability needing a new registry project. The framework never auto-modifies the registry.

## v0.x — Vectis capability v3 (RFC-11)

- The standalone `vectis:design-system-writer` skill and the `design-system` platform enum value are removed. Shell writers (`ios-writer`, `android-writer`) read `tokens.yaml` and `assets.yaml` directly and emit shell-local theme + asset code under `iOS/<App>/Theme/` and `Android/.../ui/theme/` respectively.
- Vectis capability manifest version bumps from `2` to `3`.

## v0.2.0 (specify-cli) — v2 on-disk layout

- Operator-facing platform artifacts (`registry.yaml`, `plan.yaml`, `change.md`, `contracts/`) and the generated `AGENTS.md` move to the repo root; framework state (`project.yaml`, `context.lock`, `slices/`, `specs/`, `archive/`, `.cache/`, `workspace/`, `plans/`, `plan.lock`) stays under `.specify/`.
- The CLI ships one-shot migrations and refuses every project-aware verb on a v1-layout project with the stable `legacy-layout` error code. Hard cutover, no transition window.

## v0.25.0 — RFC-10 plugin namespace renormalisation

- **Breaking change** to the Cursor plugin / slash-command namespace; marketplace ship marker bumped from `0.24.3` to `0.25.0`.
- SoW-writer skill moved from the `plan` plugin to the `client` plugin (`/client:sow-writer`).
- The former `contracts` plugin is split by format into `/contract:openapi`, `/contract:asyncapi`, and `/contract:json-schema`, each carrying author / import / verify intents internally.
- The former `rt:git-cloner` skill is deleted; callers inline a guarded `git clone` snippet directly.
- Every `SKILL.md` `name:` is plugin-qualified (skills under `plugins/spec/` use the `specify-` prefix) and fits inside a 500-line ceiling enforced by `make checks`.
- The skill schema rejects `license`, `compatibility`, `metadata`, `disable-model-invocation`, `when_to_use`, `user-invocable`, and `paths` frontmatter keys; `argument-hint` simplifies to the primary positional only (`<dir>` required, `[name]` optional).
- No persisted artifact, schema, brief id, validation rule, or registry role changed.

## v0.x — API contracts as first-class platform artifacts (RFC-8)

- API contracts live at `contracts/` alongside `registry.yaml` and `plan.yaml`, using JSON Schema for payloads with OpenAPI 3.1 (HTTP) and AsyncAPI 3.0 (messaging) bindings.
- The `contracts` brief in the define pipeline runs alignment validation against the baseline; `/change:plan` automatically inserts a contract slice before implementation work when it detects an API boundary between projects.

## v0.23 baseline

- v1 CLI cleanup; routing-only reshape (renamed verbs, no new behaviour). See the project history for predecessor entries.
