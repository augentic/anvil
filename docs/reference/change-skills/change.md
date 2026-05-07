# /change:plan --orchestrate (formerly /spec:initiative)

> **Renamed.** The Layer 4 umbrella was originally a separate `/spec:initiative` skill. It has been folded into `/change:plan` as a flag-gated `--orchestrate` mode in a progressive-disclosure pass; the seven-step sequence and every halt / re-entry semantic is unchanged. Replace `/spec:initiative create <name>` with `/change:plan --orchestrate <name>` everywhere; the skill body now lives at [`plugins/spec/skills/plan/orchestration.md`](../../../plugins/change/skills/plan/orchestration.md) (with [shapes](../../../plugins/change/skills/plan/shapes.md) and [re-entry](../../../plugins/change/skills/plan/re-entry.md) details in adjacent siblings). The CLI verbs (`specify change create`, `specify change finalize`) are unchanged.

Drive a cross-repo Specify initiative end-to-end from a single operator action: brief -> registry validate -> `/change:plan` (default mode) -> `/change:execute --loop` -> `specify workspace push` -> optional `specify workspace merge` -> `specify change finalize`.

`/change:plan --orchestrate` is the Layer 4 umbrella mode (RFC-9 Section 2C). It is **composition only** -- every step shells out to a Layer 1 CLI verb or a Layer 3 skill; the orchestration mode adds no new logic, owns no new on-disk state, and never invents a CLI verb.

## Synopsis

```text
/change:plan --orchestrate <name> \
    [--shape migrate-legacy | new-feature | update-existing] \
    [--from <path>[:<kind>]...] \
    [--against <path>[:<kind>]] \
    [--source <key>=<path-or-url>[:<kind>]...] \
    [--auto-merge] \
    [--dry-run]
```

Re-running `--orchestrate` against an existing initiative is the canonical resume path -- see [Re-entry / idempotency](#re-entry--idempotency).

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `<name>` | Yes | Kebab-case identifier matching `^[a-z][a-z0-9-]*$`. Becomes the change name in `change.md`, the plan name in `plan.yaml`, and the PR branch suffix `specify/<name>` for `specify workspace push`. |
| `--shape` | No | Explicit shape override; one of `migrate-legacy`, `new-feature`, `update-existing`. Inferred from the input flags when omitted (see [Shape inference](#shape-inference)). |
| `--from <path>` | No | Documentation input forwarded to `/change:plan`. Repeatable. Default kind is `documentation`; override per-input via `:<kind>` suffix. |
| `--against <path>` | No | Refactor-target codebase forwarded to `/change:plan`. Single-valued. Default kind is `legacy-code`. |
| `--source <key>=<path-or-url>` | No | Named legacy source forwarded to `/change:plan` and threaded through `/change:execute` per-slice. Repeatable. Default kind is `legacy-code`. Git URLs flow into `/spec:analyze` clones (tier-1 workspace); local paths are passed through verbatim. |
| `--auto-merge` | No | When set, step 6 invokes `specify workspace merge` (RFC-9 Section 4A) on every open PR with green CI. Without it, step 6 lists the open PRs and stops. |
| `--dry-run` | No | Observation-only end-to-end. Runs read-side checks for steps 1-3 and invokes `/change:plan --dry-run`; never invokes `/change:execute`, `specify workspace {push, merge}`, or `specify change finalize`. |

## When to use

- You are driving a cross-repo initiative from a platform hub and want a single command to take it from "I have an idea" to "every PR is merged."
- You want the framework to honour every halt the underlying skills surface (self-heal, `stuck`, `registry-amendment-required`) and resume idempotently when you re-invoke it.
- You want shape-aware input handling (`migrate-legacy`, `new-feature`, `update-existing`) without remembering which sub-verb each shape needs.

For partial reruns, CI pipelines, or fine-grained control over a single phase, call `/change:plan` and `/change:execute` directly. For single-repo work, the bare define-build-merge loop is simpler.

## Internal sequence

The umbrella drives the canonical platform-first loop:

| Step | Invocation | Halts on |
|------|------------|----------|
| 1. Brief | `specify change create <name>` (when `change.md` is absent) | Kebab-case violation, partial scaffold |
| 2. Registry | `specify registry validate` | `description-missing-multi-repo`, `hub-cannot-be-project`, kebab-case / URL / schema violations |
| 3. Plan | `/change:plan <name> [--from ...] [--against ...] [--source ...]` | Operator `abort` in propose loop, `specify change plan validate` failure |
| 4. Execute | `/change:execute --loop` | `stuck`, `halted`, `driver-interrupted`, `registry-amendment-required` |
| 5. Push | `specify workspace push` | Per-project `failed` status (auth, missing remote) |
| 6. Land | `specify workspace merge` (with `--auto-merge`) or list open PRs and stop | `pending-checks`, `failed-checks`, `closed`, `branch-pattern-mismatch` |
| 7. Finalize | `specify change finalize` | Non-terminal plan entry, unmerged PR, dirty workspace clone |

Every state mutation is a shell-out -- the umbrella never writes any of these files itself. Manual-fallback equivalents for each step are documented in the SKILL body so an operator can drop down a layer at any point. See [Drop down a layer](../../how-to/drop-down-a-layer.md#from-layer-4-to-layer-3-skip-the-umbrella) for the canonical sequence.

## Shape inference

When `--shape` is omitted the skill infers the shape from the CLI flags:

| Flags supplied | Inferred shape |
|----------------|----------------|
| `--source <k>=<v>` (one or more) | `migrate-legacy` |
| `--from <path>` (only) | `new-feature` |
| `--against <path>` (only) | `new-feature` |
| neither `--source`, `--from`, nor `--against` | `update-existing` |

When `--shape` is explicit, the flags are validated against a closed table:

| Explicit shape | Required | Forbidden |
|----------------|----------|-----------|
| `migrate-legacy` | at least one `--source` | -- |
| `new-feature` | at least one `--from` OR `--against` OR a populated `change.md:inputs` | -- |
| `update-existing` | -- | `--from`, `--against`, `--source` (any of the three is a hard exit) |

A shape conflict is a hard exit before any side-effect.

## Halt semantics

The umbrella stops on:

- Any failure during pre-flight (missing `.specify/`, missing `specify` binary, unknown sub-verb, invalid `<name>`, shape conflict).
- Step-1-7 failures (table above) -- the offending diagnostic surfaces verbatim, and the operator runs the manual-fallback sequence for that step before re-running the umbrella.
- Without `--auto-merge`, step 6 always halts after listing open PRs -- the operator merges by hand or via `specify workspace merge` and re-runs.

## Re-entry / idempotency

Running `/change:plan --orchestrate <name>` against a populated initiative is the canonical resume path. The mode walks the on-disk state (`change.md` present? `plan.yaml` present and terminal? PRs merged on remote?) and resumes at the first incomplete step:

| State on entry | Resumes at |
|----------------|------------|
| Brief absent | Step 1 |
| Brief present, plan absent | Step 3 |
| Plan present, entries non-terminal | Step 4 |
| Plan terminal, PRs unpushed | Step 5 |
| Plan terminal, PRs open | Step 6 (lists PRs unless `--auto-merge`) |
| Plan terminal, PRs merged, plan still on disk | Step 7 |
| Plan archived (`plan-not-found` from `finalize`) | Reports already-closed and exits |

This is what makes the umbrella safe to re-invoke after any halt.

## Three initiative shapes

Each shape uses the same seven-step sequence. Only the inputs to step 3 (Plan) differ.

| Shape | Sources | Targets |
|-------|---------|---------|
| `migrate-legacy` | `--source <k>=<git-url-or-path>` (one or more); cloned into `.specify/plans/<name>/analyze/<key>/` (tier 1) for shallow inventory; deep `/spec:extract` runs at define time | Existing or newly-minted registered projects |
| `new-feature` | `--from <docs>` (and/or `change.md:inputs`) | Existing registered projects, possibly with new ones spawned at assignment time via the 2B registry-proposal sub-step |
| `update-existing` | None | Existing registered projects; baseline accumulation in `.specify/workspace/<peer>/specs/` is the dominant signal |

## Lifecycle transitions

The umbrella is a composition of existing skills and CLI verbs. It does not introduce new lifecycle states. Plan-entry transitions are written by `/change:execute` via `specify change plan transition`; change lifecycle transitions are written by the phase skills via `specify slice transition`.

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| Missing `.specify/` | Pre-flight failed -- this is not a project | Run `/spec:init` first |
| `specify` binary not found | Pre-flight failed | Install per [Prerequisites](../../orientation/prerequisites.md) |
| Unknown sub-verb | Only `create` is supported | Use `specify change finalize` directly for closure, or re-run `create` for idempotent resume |
| Shape conflict | Explicit `--shape` does not match supplied flags | Drop the offending flag or change the shape |
| `description-missing-multi-repo` | Multi-project registry has an entry without a description | `specify registry add <name> --description "..."` for each missing entry, then re-run |
| `registry-amendment-required` | `/change:execute` halt -- a phase skill proposed a new registry entry | See [Recover from `registry-amendment-required`](../../how-to/recover-from-registry-amendment.md) |

## Examples

```text
# migrate-legacy: full autonomy
/change:plan --orchestrate migrate-foo \
    --shape migrate-legacy \
    --source monolith=git@github.com:org/legacy-foo.git \
    --auto-merge

# new-feature: supervised land (operator merges PRs by hand)
/change:plan --orchestrate dark-mode \
    --shape new-feature \
    --from ./docs/dark-mode-spec.md

# update-existing: baseline-driven polish
/change:plan --orchestrate polish-pass \
    --shape update-existing \
    --auto-merge

# Dry-run: read-side checks + plan preview, no writes
/change:plan --orchestrate migrate-foo \
    --shape migrate-legacy \
    --source monolith=git@github.com:org/legacy-foo.git \
    --dry-run
```

## See also

- [/change:plan](plan.md) -- the Layer 3 plan-authoring skill the umbrella invokes at step 3.
- [/change:execute](execute.md) -- the Layer 2 driver the umbrella invokes at step 4.
- [specify change](../cli/change.md) -- the `create` / `show` / `finalize` CLI verbs.
- [specify workspace](../cli/workspace.md) -- `push` and `merge` (steps 5 and 6).
- [Cross-Repo Initiatives](../../tutorials/cross-repo-change.md) -- end-to-end worked example.
- [Land a change](../../how-to/land-a-change.md) -- autonomous vs supervised landing.
- [Recover from `registry-amendment-required`](../../how-to/recover-from-registry-amendment.md) -- the canonical recovery sequence.
- [Drop down a layer](../../how-to/drop-down-a-layer.md) -- manual-fallback for every umbrella step.
- [Platform repo topologies](../../explanation/platform-repo.md) -- the hub topology the umbrella composes against.
