# What's New Since v0.23

This page captures the **additive** changes to the Specify framework since the v1 CLI cleanup landed in v0.23. The v1 cleanup was a routing-only reshape (renamed verbs, no new behaviour); the work below adds new capabilities. For pure rename mappings see [Migrating to CLI v1](migrating-cli-v1.md). The two pages compose: this one tells you **what is new**, the migration map tells you **what was renamed**.

The bulk of the additions ship under [RFC-9: Platform-First Operator Experience](https://github.com/augentic/specify/blob/main/rfcs/rfc-9-platform.md) and [RFC-8: API Contracts](https://github.com/augentic/specify/blob/main/rfcs/archive/rfc-8-api-contracts.md). RFC-9 closes the operator-experience gaps in the cross-repo loop; RFC-8 introduces contracts as platform-level artifacts.

## Hub topology

A **registry-only platform hub** (RFC-9 §1D) is now the canonical starting shape for a multi-repo initiative. The hub holds platform state -- `registry.yaml`, `initiative.md`, `plan.yaml`, `workspace/` -- and is never itself a code project.

```bash
specify init hub --schema-dir . --name shop-platform --hub
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

The distinction was always implicit; RFC-9 §1E codifies it so operators stop losing tier-1 writes or expecting tier-2 clones to disappear after an initiative.

- Explanation: [Workspace Tiers](workspace-tiers.md)

## `/spec:initiative` umbrella (Layer 4)

A new Layer 4 skill (RFC-9 §2C) drives the cross-repo loop end-to-end as a single operator action:

```text
/spec:initiative create <name> [--shape ...] [--from ...] [--source ...] [--auto-merge]
```

The umbrella composes: brief -> registry validate -> `/spec:plan` -> `/spec:execute --loop` -> `specify workspace push` -> optional `specify workspace merge` -> `specify initiative finalize`. Every step is a shell-out to a Layer 1 verb or a Layer 3 skill; the umbrella adds no new logic. Halts (self-heal, `stuck`, `registry-amendment-required`, `pending-checks`, unmerged PRs) surface verbatim, and re-running `create` against an in-progress initiative resumes at the first incomplete step.

Three initiative shapes flow through the same uniform sequence: `migrate-legacy` (sources via `--source`), `new-feature` (docs via `--from`), `update-existing` (no input flags).

- Reference: [`/spec:initiative`](../reference/initiative-skills/initiative.md)
- Tutorial: [Cross-Repo Initiatives](../tutorials/cross-repo-initiative.md) -> [Landing an Initiative](../tutorials/landing-an-initiative.md)
- Explanation: [The Layered Stack](three-layer-stack.md) (Layer 4 row)

## `specify registry add/remove`

Registry mutation is now a CLI verb instead of a hand-edit (RFC-9 §2A):

```bash
specify registry add <name> --url <url> --schema <schema> --description "..."
specify registry remove <name>
```

`add` validates kebab-case names, URL classification, and the `description-missing-multi-repo` invariant after the write. `remove` warns when plan entries reference the removed project. `/spec:plan`'s registry-proposal sub-step (RFC-9 §2B) shells out to `add` automatically when assignment names a project not yet in `registry.yaml`.

- Reference: [`specify registry`](../reference/cli/registry.md)
- How-to: [Manage Registry Projects](../how-to/manage-registry-projects.md)

## `specify workspace merge`

The cross-repo PR-landing verb (RFC-9 §4A):

```bash
specify workspace merge [<project>...]
```

Per project, checks `gh pr checks` against `specify/<initiative-name>` and runs `gh pr merge --squash` when every check is `pass` or `skipping`. Refuses any PR whose `headRefName` is not `specify/<initiative-name>` exactly (the `branch-pattern-mismatch` guard); never `--admin`, never `--auto`. Best-effort across projects.

- Reference: [`specify workspace merge`](../reference/cli/workspace.md#specify-workspace-merge)
- How-to: [Land an Initiative](../how-to/land-an-initiative.md)

## `specify plan doctor`

A strict superset of `specify plan validate` with four additional health diagnostics (RFC-9 §4B):

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

The canonical closure verb for the platform-first loop (RFC-9 §4C):

```bash
specify initiative finalize [--clean] [--dry-run]
```

Runs four guards in order (plan-presence, plan terminal-state, per-project PR-state, workspace-cleanliness) before atomically archiving `plan.yaml`, `initiative.md`, and `.specify/plans/<name>/`. Any guard refusal leaves the on-disk state untouched. `--clean` prunes `.specify/workspace/<peer>/` after the archive completes. Idempotent: re-running after a successful finalize returns `plan-not-found` (the explicit "already finalized" signal).

- Reference: [`specify initiative finalize`](../reference/cli/initiative.md#specify-initiative-finalize)
- How-to: [Land an Initiative](../how-to/land-an-initiative.md)

## API contracts (RFC-8)

API contracts are now first-class platform artifacts at `.specify/contracts/`, alongside `registry.yaml` and `plan.yaml`. The contract format uses JSON Schema (payload definitions) plus OpenAPI 3.1 (HTTP bindings) and AsyncAPI 3.0 (messaging bindings) -- no proprietary IDL.

The `contracts` brief in the define pipeline runs alignment validation against the baseline; `/spec:plan` automatically inserts a contract change before implementation changes when it detects an API boundary between projects (the contract-first authorship pattern). The Contracts plugin ships three skills: `/contracts:writer`, `/contracts:validator`, `/contracts:importer`.

- Reference: [Contracts plugin](../reference/plugins/contracts.md), [Contracts schema](../reference/schemas/contracts.md), [Artifact Format -> Contracts](../reference/artifact-format.md#contract-artifacts-api-shape)
- How-to: [Work with Contracts Across Repos](../how-to/cross-repo-contracts.md)

## Cross-project contract validation (RFC-9 §3B)

Post-merge, `/spec:execute` runs a cross-project compatibility check: for each contract the producer `produces`, find every consumer that `consumes` it and run `/contracts:validator --mode cross-project` against each consumer's workspace clone. Incompatibilities surface as warnings on the merge transcript and on the merged change's `journal.yaml` (`cross-project-warning:` entries). **Warnings never halt the loop** -- the operator triages.

- How-to: [Resolve Cross-Project Contract Warnings](../how-to/resolve-cross-project-contract-warnings.md)
- Troubleshooting: [Cross-project contract warnings on the merge transcript](../appendices/troubleshooting.md#cross-project-contract-warnings-on-the-merge-transcript)

## `registry-amendment-required` outcome

A new phase outcome variant (RFC-9 §2B) for cases where a phase skill discovers that a change targets a capability needing a new registry project. The outcome carries a structured payload (`{ proposed-name, proposed-url, proposed-schema, proposed-description, rationale }`); the executor classifies it as `blocked`, records the payload in the dropped change's `journal.yaml`, and surfaces the proposal to the operator. The framework never auto-modifies the registry.

The canonical recovery sequence: `specify registry add` -> `specify workspace sync` -> `specify plan amend <change> --project <new>` -> `specify plan transition <change> pending` -> re-run `/spec:execute`.

- How-to: [Recover from `registry-amendment-required`](../how-to/recover-from-registry-amendment.md)
- Troubleshooting: [Registry amendment required](../appendices/troubleshooting.md#registry-amendment-required)

## Fixture-backed verification (design)

RFC-9 §4D adds a second mode to `/spec:verify` that replays captured fixtures against a live, deployed service and reports response drift. **Implementation is pending** (`rfc9-4d2-impl`); the design note documents the inputs (TestDef-style fixtures, `transport.yaml` binding, `tolerances.yaml` policy), the diff semantics, and the `--fixtures <dir>` operator surface so a follow-up implementation change can land it without re-deriving the model.

- Explanation: [Fixture-backed verification mode](verify-fixture-mode.md)

## v1.x verb renames (RFC-9 §§1F, 1G)

Three renames landed on top of the v1 cleanup so every noun-create verb now uses `create`:

| Old verb (v1) | New verb (v1.x) |
|---|---|
| `specify initiative init <name>` | `specify initiative create <name>` |
| `specify plan init <name>` | `specify plan create <name>` |
| `specify plan create <name>` | `specify plan add <name>` |

The renames ship together so the `plan` group never spent an interim release with `init` and `create` for the same noun.

- See: [Migrating to CLI v1 -- v1.x renames](migrating-cli-v1.md#v1x-renames)

## Skill frontmatter standard

The SKILL.md frontmatter schema (`schemas/skill.schema.json`) now matches the upstream Anthropic / Cursor / Claude Code spec more closely:

- `argument-hint` adopts a deliberate Specify house style: bare names for required arguments, `?` suffix for optional, literal pipes for choices, `...` for repeatables. No `--` prefix, no angle or square brackets.
- `allowed-tools` is space-separated (matches Cursor and Claude Code spec).
- `license` accepts any SPDX ID, custom name, or path to a bundled `LICENSE` file (no longer enum-restricted).
- New optional fields: `compatibility`, `metadata`, `disable-model-invocation`, `when_to_use`, `user-invocable`, `paths`.

- Reference: [Anatomy of a Skill -- Frontmatter fields](../contributing/skill-anatomy.md#frontmatter-fields)

## See also

- [Migrating to CLI v1](migrating-cli-v1.md) -- the v1 rename map (companion page).
- [The Layered Stack](three-layer-stack.md) -- updated for Layer 4.
- [Cross-Repo Initiatives](../tutorials/cross-repo-initiative.md) and [Landing an Initiative](../tutorials/landing-an-initiative.md) -- the worked example exercising all of the above.
- [Quick Reference](../reference/quick-reference.md) -- single-page cheat sheet for the post-RFC-9 surface.
