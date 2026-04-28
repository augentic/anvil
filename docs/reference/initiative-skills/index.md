# Initiative Skills (Layers 3 & 4)

Initiative-scoped skills coordinate multi-change programs through `.specify/plan.yaml`. They sit above the [change lifecycle skills](../change-skills/index.md) and invoke them per-change. The Layer 3 skills (`/spec:plan`, `/spec:execute`, `/spec:analyze`) author and drive a plan; the Layer 4 umbrella (`/spec:initiative`, RFC-9 §2C) composes plan + execute + push + merge + finalize into a single operator action — see the [layered stack](../../explanation/three-layer-stack.md) for the relationship.

## The plan-execute flow

```text
/spec:plan <name> --source legacy=./path  -->  /spec:execute --loop
```

`/spec:plan` produces the plan. `/spec:execute` consumes it by running the define-build-merge loop for each change in dependency order.

## The Layer 4 loop

`/spec:initiative` strings the full platform-first sequence into one operator action. Layer 3 skills are still callable directly for partial reruns and CI pipelines.

```d2
direction: right

umbrella: "/spec:initiative create" {shape: rectangle}

brief: "specify initiative create" {shape: rectangle}
registry: "specify registry validate" {shape: rectangle}
plan: "/spec:plan" {shape: rectangle}
execute: "/spec:execute --loop" {shape: rectangle}
push: "specify workspace push" {shape: rectangle}
mergeStep: "specify workspace merge" {shape: rectangle}
finalize: "specify initiative finalize" {shape: rectangle}

operator: "Operator merges PRs by hand\n(without --auto-merge)" {shape: rectangle}

umbrella -> brief: "step 1"
brief -> registry: "step 2"
registry -> plan: "step 3"
plan -> execute: "step 4"
execute -> push: "step 5"
push -> mergeStep: "step 6 (with --auto-merge)"
push -> operator: "step 6 halt (no --auto-merge)"
operator -> finalize: "re-enter umbrella"
mergeStep -> finalize: "step 7"
```

Every halt -- registry validation failure, `stuck`, `registry-amendment-required`, `pending-checks`, an unmerged PR -- surfaces verbatim and stops the umbrella. Re-running `/spec:initiative create <name>` resumes at the first incomplete step. See [`/spec:initiative`](initiative.md) for the full algorithm.

## Skill summary

| Skill | Layer | Purpose | Reads | Writes |
|-------|-------|---------|-------|--------|
| [/spec:initiative](initiative.md) | 4 | Drive the cross-repo loop end-to-end (brief -> registry -> plan -> execute -> push -> optional merge -> finalize) | `.specify/initiative.md`, `registry.yaml`, `plan.yaml`, workspace clones | Composition only -- shells out; never writes directly |
| [/spec:plan](plan.md) | 3 | Author `plan.yaml` from inputs | Sources, docs, registry, baseline specs | `plan.yaml`, `discovery.md`, `proposal.md`, optional `workspace.md`; for multi-project plans, amends entries with `--project` via the assignment step |
| [/spec:execute](execute.md) | 3 | Drive the plan through define-build-merge | `plan.yaml` | Plan status transitions (via CLI); CWD-routes into workspace clones for multi-project plans; merge may auto-commit `.specify/` in clones |
| [/spec:analyze](analyze.md) | 3 | Plan-time capability inference (used internally by `/spec:plan`) | Source code or documentation | `discovery.md`, optional `metadata.json` |

## Layered composition

These skills are optional. You can use the define-build-merge loop without ever touching plans. But when you do need them, they compose with the lower layers:

- **Layer 4 (`/spec:initiative`)** -- single command for a cross-repo initiative end-to-end.
- **Layer 3 alone (`/spec:plan`)** -- author a plan, then drive it manually with Layer 1 CLI commands.
- **Layer 3 + Layer 2 (`/spec:plan` then `/spec:execute`)** -- author a plan, then automate execution.
- **Layer 2 alone** -- skip plans entirely, define and build changes one at a time.

The Layer 1 CLI commands (`specify plan ...`, `specify workspace ...`, `specify initiative ...`) remain available as manual fallback at every level.
