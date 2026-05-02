# What's New Since v0.23

This page captures the **additive** changes to the Specify framework since the v1 CLI cleanup landed in v0.23. The v1 cleanup was a routing-only reshape (renamed verbs, no new behaviour); the work below adds new capabilities. For pure rename mappings see [Migrating to CLI v1](migrating-cli-v1.md). The two pages compose: this one tells you **what is new**, the migration map tells you **what was renamed**.

The bulk of the additions ship under [RFC-9: Platform-First Operator Experience](https://github.com/augentic/specify/blob/main/rfcs/rfc-9-platform.md) and [RFC-8: API Contracts](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-8-api-contracts.md). RFC-9 closes the operator-experience gaps in the cross-repo loop; RFC-8 introduces contracts as platform-level artifacts.

## v2 layout — platform artifacts at the repo root (specify-cli `0.2.0`)

The on-disk layout split along a clear boundary: **operator-facing platform artifacts** (`registry.yaml`, `plan.yaml`, `initiative.md`, `contracts/`) live at the repo root; **framework-managed state** (`project.yaml`, `changes/`, `specs/`, `archive/`, `.cache/`, `workspace/`, `plans/`, `plan.lock`) stays under `.specify/`. The boundary makes the responsibilities explicit — operators own everything at the root; Specify owns `.specify/`.

This is a **hard cutover**. The CLI no longer reads the v1 layout. Any project-aware verb on a v1-layout project errors with the stable `legacy-layout` code (exit 1) and points the operator at:

```bash
specify migrate v2-layout
```

The migrate verb is idempotent, refuses to clobber an existing destination, and refuses to run inside a workspace clone (peer clones get migrated explicitly per-clone). See [`specify migrate v2-layout`](../reference/cli/migrate.md) for the wire shape and [Migrating to the v2 layout](../how-to/migrate-to-v2-layout.md) for the operator-facing walkthrough.

Per-artifact migration map:

| Artifact | v1 path | v2 path |
|---|---|---|
| Platform catalogue | `.specify/registry.yaml` | `registry.yaml` |
| Initiative plan | `.specify/plan.yaml` | `plan.yaml` |
| Operator brief | `.specify/initiative.md` | `initiative.md` |
| API contracts | `.specify/contracts/` | `contracts/` |

`project.yaml` stays under `.specify/`. The `contracts@v1` schema id, the `contracts` brief, the merge semantics, the produces/consumes registry roles, and the workspace flow are all unchanged — only the file locations moved. The decision-log entry "Platform artifacts at the repo root, framework state under `.specify/`" carries the design rationale.

## RFC-12 contract-versioning refinement

[RFC-12](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-12-refine-rfc-8.md) refines RFC-8 along three axes — none of them change the artifact tree, the `contracts` brief, the `contracts@v1` schema id, the merge semantics, or the workspace flow. They are recorded as an RFC because two of the three are breaking.

- **`info.version` MUST be SemVer.** Every top-level OpenAPI 3.1 / AsyncAPI 3.0 document under `contracts/` MUST set `info.version` to a value that parses per [semver.org](https://semver.org), including optional prerelease labels (`1.0.0-draft.1`). Existing contracts whose value is a `YYYY-MM-DD` date or a bare major (`"2"`) need a one-line edit before `specify contract validate` will pass after upgrade. Bump rules (when to advance major / minor / patch) remain skill-side judgement.
- **Optional `info.x-specify-id` rename-stable identifier.** Every top-level contract MAY set `info.x-specify-id` to a kebab-case slug (`^[a-z][a-z0-9-]*$`, ≤ 64 characters; uniqueness enforced across the repo). The id survives file moves and `info.version` bumps. New top-level contracts SHOULD set it; existing contracts MAY add it any time. Path-based references in `registry.yaml` remain canonical — the id is a hint, not a substitute.
- **`contracts.imports` removed from the registry.** The role set on each `registry.yaml` project entry collapses to two: `produces` and `consumes`. Contracts that no project produces are, by definition, externally authored — no separate field is needed to flag them. `specify registry validate` rejects the unknown `imports` key after upgrade, so any surviving usage surfaces immediately.
- **New `specify contract { list, validate }` verbs.** `contract list` projects every top-level OpenAPI / AsyncAPI document under `contracts/` (`(file, format, info.title, info.version, info.x-specify-id)`). `contract validate` runs the SemVer + id-format + cross-repo id-uniqueness checks across the same set and exits non-zero on any finding. Both verbs no-op with exit 0 when `contracts/` is absent.

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

Each new skill handles author / import / verify intents internally. The former validator's `--mode {single, cross-project}` flag becomes an internal verifier option per format. `/spec:execute`'s post-merge cross-project compatibility check now picks the format-appropriate skill (`/contract:openapi`, `/contract:asyncapi`, or `/contract:json-schema`) and threads the verifier intent with `--mode cross-project`.

### Removed

- The former `rt` plugin's `git-cloner` skill has been deleted. Cloning a remote source tree is no longer a dedicated skill; the two callers (`plugins/spec/skills/analyze/SKILL.md` and `plugins/rt/skills/wiretapper/SKILL.md`) now inline a guarded `git clone` snippet directly.

### Other notable changes

- **Plugin-qualified skill names.** Every SKILL.md `name:` is now globally unique by construction. Skills under `plugins/<plugin>/` use the directory name as their prefix (e.g. `omnia-crate-writer`, `vectis-core-reviewer`, `contract-openapi`, `client-sow-writer`); skills under `plugins/spec/` use the `specify-` prefix instead (so the operator-facing product name surfaces in discovery, e.g. `specify-init`, `specify-plan`).
- **500-line ceiling per SKILL.md.** Every skill body now fits under Anthropic's 500-line guidance; depth pushes one level out into siblings (`references/`, `examples/`, or topical files such as `author.md` / `importer.md` / `verifier.md`). `make checks` enforces the ceiling.
- **Phase-outcome contract authored once.** The four phase skills (`define`, `build`, `merge`, `drop`) used to repeat the four-of-one outcome contract inline. They now link to a single source of truth at `plugins/spec/references/phase-outcome-contract.md`, eliminating drift.
- **Forbidden frontmatter keys.** The skill schema (`.cursor/schemas/skill.schema.json`) now rejects `license`, `compatibility`, `metadata`, `disable-model-invocation`, `when_to_use`, `user-invocable`, and `paths` in SKILL.md frontmatter; license is declared once in the plugin manifest and the repo `LICENSE` file, and the rest belong in the body.
- **`argument-hint` simplified.** The hint now names the **primary positional** argument only, with angle brackets for required (`<change-dir>`) and square brackets for optional (`[crate-name]`); flags and secondary positionals move into the body's *Invocation* section. The earlier `?` / `--` / `|` syntax is gone.

For the full rationale and migration plan, see [rfcs/archive/rfc-10-skills.md](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-10-skills.md) and [rfcs/rfc-10-plan.md](https://github.com/augentic/specify/blob/main/rfcs/rfc-10-plan.md).

## Hub topology

A **registry-only platform hub** (RFC-9 ?1D) is now the canonical starting shape for a multi-repo initiative. The hub holds platform state -- `registry.yaml`, `initiative.md`, `plan.yaml`, `workspace/` -- and is never itself a code project.

```bash
specify init --hub --name shop-platform
```

The flag scaffolds a sentinel `project.yaml { schema: hub, hub: true, ... }` that disables phase pipelines on the hub itself, plus an empty `registry.yaml` and an `initiative.md` template. `Registry::validate_shape` extends with a `hub-only` mode that rejects any registry entry whose `url` is `.`.

The platform-as-project shape (initiating repo with `url: .`) is still permitted for single-repo and small-team cases.

- Reference: [`specify init`](../reference/cli/init.md)
- Explanation: [Platform repo topologies](platform-repo.md)
- How-to: [Bootstrap a Platform Hub](../how-to/bootstrap-a-platform-hub.md)

## Two-tier workspace model

Specify now distinguishes two kinds of clones with very different lifecycles:

- **Tier 1 (legacy-source clone)**: ephemeral, read-only, lives at `.specify/plans/<name>/analyze/<key>/`. Materialised by `/spec:analyze`; swept by `specify plan archive`.
- **Tier 2 (registered project clone)**: durable, read-write, lives at `.specify/workspace/<name>/`. Materialised by `specify workspace sync`; pushed by `specify workspace push`.

The distinction was always implicit; RFC-9 ?1E codifies it so operators stop losing tier-1 writes or expecting tier-2 clones to disappear after an initiative.

- Explanation: [Workspace Tiers](workspace-tiers.md)

## `/spec:plan --orchestrate` umbrella mode (Layer 4)

A Layer 4 mode of `/spec:plan` (RFC-9 ?2C) drives the cross-repo loop end-to-end as a single operator action:

```text
/spec:plan --orchestrate <name> [--shape ...] [--from ...] [--source ...] [--auto-merge]
```

> **Note.** This was originally a separate `/spec:initiative` skill; it was folded into `/spec:plan` as a flag-gated `--orchestrate` mode in a progressive-disclosure pass. The seven-step umbrella sequence is unchanged.

The mode composes: brief -> registry validate -> `/spec:plan` (default mode) -> `/spec:execute --loop` -> `specify workspace push` -> optional `specify workspace merge` -> `specify initiative finalize`. Every step is a shell-out to a Layer 1 verb or a Layer 3 skill; the orchestration mode adds no new logic. Halts (self-heal, `stuck`, `registry-amendment-required`, `pending-checks`, unmerged PRs) surface verbatim, and re-running `--orchestrate` against an in-progress initiative resumes at the first incomplete step.

Three initiative shapes flow through the same uniform sequence: `migrate-legacy` (sources via `--source`), `new-feature` (docs via `--from`), `update-existing` (no input flags).

- Reference: [`/spec:plan --orchestrate`](../reference/initiative-skills/initiative.md)
- Tutorial: [Cross-Repo Initiatives](../tutorials/cross-repo-initiative.md) -> [Landing an Initiative](../tutorials/landing-an-initiative.md)
- Explanation: [The Layered Stack](three-layer-stack.md) (Layer 4 row)

## `specify registry add/remove`

Registry mutation is now a CLI verb instead of a hand-edit (RFC-9 ?2A):

```bash
specify registry add <name> --url <url> --schema <schema> --description "..."
specify registry remove <name>
```

`add` validates kebab-case names, URL classification, and the `description-missing-multi-repo` invariant after the write. `remove` warns when plan entries reference the removed project. `/spec:plan`'s registry-proposal sub-step (RFC-9 ?2B) shells out to `add` automatically when assignment names a project not yet in `registry.yaml`.

- Reference: [`specify registry`](../reference/cli/registry.md)
- How-to: [Manage Registry Projects](../how-to/manage-registry-projects.md)

## `specify workspace merge`

The cross-repo PR-landing verb (RFC-9 ?4A):

```bash
specify workspace merge [<project>...]
```

Per project, checks `gh pr checks` against `specify/<initiative-name>` and runs `gh pr merge --squash` when every check is `pass` or `skipping`. Refuses any PR whose `headRefName` is not `specify/<initiative-name>` exactly (the `branch-pattern-mismatch` guard); never `--admin`, never `--auto`. Best-effort across projects.

- Reference: [`specify workspace merge`](../reference/cli/workspace.md#specify-workspace-merge)
- How-to: [Land an Initiative](../how-to/land-an-initiative.md)

## `specify plan doctor`

A strict superset of `specify plan validate` with four additional health diagnostics (RFC-9 ?4B):

| Code | Severity | Meaning |
|------|----------|---------|
| `cycle-in-depends-on` | error | Dependency cycle in `depends-on`. |
| `orphan-source-key` | warning | Top-level `sources:` key not referenced by any entry. |
| `stale-workspace-clone` | warning | Workspace clone signature drifted from registry. |
| `unreachable-entry` | error | Pending entry blocked by `failed`/`skipped` predecessors. |

`plan doctor` is the canonical first triage step when `/spec:execute --loop` reports `stuck`.

- Reference: [`specify plan doctor`](../reference/cli/plan.md#specify-plan-doctor)
- Troubleshooting: [Plan doctor diagnostics](../appendices/troubleshooting.md#plan-doctor-diagnostics)

## `specify initiative finalize`

The canonical closure verb for the platform-first loop (RFC-9 ?4C):

```bash
specify initiative finalize [--clean] [--dry-run]
```

Runs four guards in order (plan-presence, plan terminal-state, per-project PR-state, workspace-cleanliness) before atomically archiving `plan.yaml`, `initiative.md`, and `.specify/plans/<name>/`. Any guard refusal leaves the on-disk state untouched. `--clean` prunes `.specify/workspace/<peer>/` after the archive completes. Idempotent: re-running after a successful finalize returns `plan-not-found` (the explicit "already finalized" signal).

- Reference: [`specify initiative finalize`](../reference/cli/initiative.md#specify-initiative-finalize)
- How-to: [Land an Initiative](../how-to/land-an-initiative.md)

## API contracts (RFC-8)

API contracts are now first-class platform artifacts at `contracts/`, alongside `registry.yaml` and `plan.yaml`. The contract format uses JSON Schema (payload definitions) plus OpenAPI 3.1 (HTTP bindings) and AsyncAPI 3.0 (messaging bindings) -- no proprietary IDL.

The `contracts` brief in the define pipeline runs alignment validation against the baseline; `/spec:plan` automatically inserts a contract change before implementation changes when it detects an API boundary between projects (the contract-first authorship pattern). The Contract plugin ships three format-first skills, each carrying author / import / verify intents internally: `/contract:openapi` (HTTP / resource APIs), `/contract:asyncapi` (evented / pub-sub / streaming), and `/contract:json-schema` (shared payload schemas). The `contracts` brief, schema id, and `contracts/` baseline directory keep their original names; only the Cursor plugin / slash-command surface is renamed.

- Reference: [Contract plugin](../reference/plugins/contract.md), [Contracts schema](../reference/schemas/contracts.md), [Artifact Format -> Contracts](../reference/artifact-format.md#contract-artifacts-api-shape)
- How-to: [Work with Contracts Across Repos](../how-to/cross-repo-contracts.md)

## Cross-project contract validation (RFC-9 ?3B)

Post-merge, `/spec:execute` runs a cross-project compatibility check: for each contract the producer `produces`, find every consumer that `consumes` it and run the format-appropriate verifier against each consumer's workspace clone (`/contract:openapi`, `/contract:asyncapi`, or `/contract:json-schema`, picking the verifier intent and threading `--mode cross-project`). Incompatibilities surface as warnings on the merge transcript and on the merged change's `journal.yaml` (`cross-project-warning:` entries). **Warnings never halt the loop** -- the operator triages.

- How-to: [Resolve Cross-Project Contract Warnings](../how-to/resolve-cross-project-contract-warnings.md)
- Troubleshooting: [Cross-project contract warnings on the merge transcript](../appendices/troubleshooting.md#cross-project-contract-warnings-on-the-merge-transcript)

## `registry-amendment-required` outcome

A new phase outcome variant (RFC-9 ?2B) for cases where a phase skill discovers that a change targets a capability needing a new registry project. The outcome carries a structured payload (`{ proposed-name, proposed-url, proposed-schema, proposed-description, rationale }`); the executor classifies it as `blocked`, records the payload in the dropped change's `journal.yaml`, and surfaces the proposal to the operator. The framework never auto-modifies the registry.

The canonical recovery sequence: `specify registry add` -> `specify workspace sync` -> `specify plan amend <change> --project <new>` -> `specify plan transition <change> pending` -> re-run `/spec:execute`.

- How-to: [Recover from `registry-amendment-required`](../how-to/recover-from-registry-amendment.md)
- Troubleshooting: [Registry amendment required](../appendices/troubleshooting.md#registry-amendment-required)

## v1.x verb renames (RFC-9 ??1F, 1G)

Three renames landed on top of the v1 cleanup so every noun-create verb now uses `create`:

| Old verb (v1) | New verb (v1.x) |
|---|---|
| `specify initiative init <name>` | `specify initiative create <name>` |
| `specify plan init <name>` | `specify plan create <name>` |
| `specify plan create <name>` | `specify plan add <name>` |

The renames ship together so the `plan` group never spent an interim release with `init` and `create` for the same noun.

- See: [Migrating to CLI v1 -- v1.x renames](migrating-cli-v1.md#v1x-renames)

## See also

- [Migrating to CLI v1](migrating-cli-v1.md) -- the v1 rename map (companion page).
- [The Layered Stack](three-layer-stack.md) -- updated for Layer 4.
- [Cross-Repo Initiatives](../tutorials/cross-repo-initiative.md) and [Landing an Initiative](../tutorials/landing-an-initiative.md) -- the worked example exercising all of the above.
- [Quick Reference](../reference/quick-reference.md) -- single-page cheat sheet for the post-RFC-9 surface.
