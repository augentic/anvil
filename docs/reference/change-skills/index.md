# Change Skills (Layers 3 & 4)

Change-scoped skills coordinate multi-slice changes through `plan.yaml`. They sit above the [slice lifecycle skills](../slice-skills/index.md) and invoke them per-slice. The Layer 3 skills live on the `change` plugin (`/change:plan`, `/change:execute`) and the `spec` plugin (`/spec:analyze`); the Layer 4 umbrella mode (`/change:plan <name> orchestrate`) composes plan + execute + push, then resumes to finalize after the operator merges PRs — see the [layered stack](../../explanation/three-layer-stack.md) for the relationship.

## The plan-execute flow

```text
/change:plan <name> source legacy=./path  -->  /change:execute loop
```

`/change:plan` produces the plan. `/change:execute` consumes it by running the define-build-merge loop for each slice in dependency order.

## The Layer 4 loop

`/change:plan <name> orchestrate` strings the full platform-first sequence into one operator action. Layer 3 skills (and the default mode of `/change:plan`) are still callable directly for partial reruns and CI pipelines.

```d2
direction: right

umbrella: "/change:plan <name> orchestrate" {shape: rectangle}

brief: "specify change create" {shape: rectangle}
registry: "specify registry validate" {shape: rectangle}
plan: "/change:plan" {shape: rectangle}
execute: "/change:execute loop" {shape: rectangle}
push: "specify workspace push" {shape: rectangle}
mergeStep: "Operator merges PRs\n(forge UI or gh pr merge)" {shape: rectangle}
finalize: "specify change finalize" {shape: rectangle}

umbrella -> brief: "step 1"
brief -> registry: "step 2"
registry -> plan: "step 3"
plan -> execute: "step 4"
execute -> push: "step 5"
push -> mergeStep: "step 6"
mergeStep -> finalize: "re-enter umbrella"
```

Every halt -- registry validation failure, `stuck`, `registry-amendment-required`, an unmerged PR -- surfaces verbatim and stops the umbrella. Re-running `/change:plan <name> orchestrate` resumes at the first incomplete step. See [`/change:plan <name> orchestrate`](change.md) for the full algorithm.

## Skill summary

| Skill | Layer | Purpose | Reads | Writes |
|-------|-------|---------|-------|--------|
| [/change:plan <name> orchestrate](change.md) | 4 | Drive the cross-repo loop through push, then finalize after operator PR merge | `change.md`, `registry.yaml`, `plan.yaml`, workspace clones | Composition only -- shells out; never writes directly |
| [/change:plan](plan.md) | 3 | Author `plan.yaml` from inputs | Sources, docs, registry, baseline specs | `plan.yaml`, `discovery.md`, `proposal.md`, optional `workspace.md`; for multi-project plans, amends entries with the CLI project option via the assignment step |
| [/change:execute](execute.md) | 3 | Drive the plan through define-build-merge | `plan.yaml` | Plan status transitions (via CLI); prepares workspace branches, routes into workspace clones for multi-project plans, and commits non-baseline residue after merge |
| [/spec:analyze](analyze.md) | 3 | Plan-time capability inference (used internally by `/change:plan`) | Source code or documentation | `discovery.md`, optional `metadata.json` |

## Layered composition

These skills are optional. You can use the define-build-merge loop without ever touching plans. But when you do need them, they compose with the lower layers:

- **Layer 4 (`/change:plan <name> orchestrate`)** -- single command for a cross-repo change end-to-end.
- **Layer 3 alone (`/change:plan`)** -- author a plan, then drive it manually with Layer 1 CLI commands.
- **Layer 3 + Layer 2 (`/change:plan` then `/change:execute`)** -- author a plan, then automate execution.
- **Layer 2 alone** -- skip plans entirely, define and build slices one at a time.

The Layer 1 CLI commands (`specify change plan ...`, `specify workspace ...`, `specify change ...`) remain available as manual fallback at every level.
