# The Layered Stack

Specify is organised in four layers. Each layer is independently useful, and each builds on the one below it.

```d2
direction: down

Layer4: "Layer 4 — Change Orchestration" {
  orchestrate: "/change:plan <name> orchestrate"
}

Layer3: "Layer 3 — Plan & Drive" {
  plan: "/change:plan"
  execute: "/change:execute"
  analyze: "/spec:analyze"
  plan -> execute
}

Layer2: "Layer 2 — Slice Lifecycle" {
  define: "/spec:define"
  build: "/spec:build"
  mergeSkill: "/spec:merge"
  drop: "/spec:drop"
  extract: "/spec:extract"
}

Layer1: "Layer 1 — CLI Primitives" {
  sliceCli: "specify slice"
  planCli: "specify change plan"
  changeCli: "specify change"
  registryCli: "specify registry"
  workspaceCli: "specify workspace"
  capabilityCli: "specify capability"
  statusCli: "specify status"
}

Layer4 -> Layer3
Layer3 -> Layer2
Layer2 -> Layer1
```

## Layer 1: CLI primitives

The `specify` CLI is the foundation. It owns every deterministic operation: creating and transitioning slices, validating artifacts, parsing tasks, merging specs, managing plans. Skills never hand-edit `.metadata.yaml`, never `mkdir -p .specify/...`, and never `mv` anything into the archive. All writes flow through the CLI.

The primary command families are:

- **`specify slice ...`** -- per-slice CRUD, validation, merge (`slice merge {preview, conflict-check, run}`), task tracking (`slice task {progress, mark}`), phase outcome (`slice outcome {set, show}`), and journal entries (`slice journal {append, show}`).
- **`specify change plan ...`** -- scaffold, populate, validate, transition, and archive a slice plan.
- **`specify change ...`** -- manage the operator-authored change brief at `change.md` and finalize a slice once every PR has merged (`change {create, show, finalize}`).
- **`specify registry ...`** -- manage the platform registry at `registry.yaml` (multi-repo changes) — `registry {add, remove, show, validate}`.
- **`specify workspace ...`** -- materialise, inspect, and push workspace clones for multi-repo changes. Workspace clones are durable and read-write; the separate read-only legacy-source clones used by `/spec:analyze` live elsewhere -- see [Workspace Tiers](workspace-tiers.md) for the distinction.
- **`specify status`** -- project dashboard summarising registry, plan, and active slices.

**Who uses it:** Power users who want fine-grained control, CI pipelines, and anyone debugging the state of `.specify/`. Layer 1 is always available as a manual fallback beneath the higher layers.

**Climb to Layer 2 when:** you are about to author or implement a single slice. The Layer 2 skills wrap the CLI primitives in agent-driven orchestrators that elicit intent, read briefs, and write artifacts on your behalf. Reaching for `specify slice create` directly is rare outside CI scripts and recovery paths.

## Layer 2: Slice lifecycle

Layer 2 skills operate on a **single slice** inside `.specify/slices/<name>/`. They form the define-build-merge loop:

```text
/spec:define  -->  /spec:build  -->  /spec:merge
```

Each skill is an agent-driven orchestrator. It elicits intent from the user, reads brief pipelines declared by the active capability, writes artifacts, invokes specialist plugin skills (e.g. `/omnia:crate-writer`), and renders summaries. Deterministic work is delegated to the Layer 1 CLI underneath.

The full set of Layer 2 skills:

| Skill | Role |
|-------|------|
| `/spec:init` | One-time project setup |
| `/spec:define` | Generate all artifacts for a new slice |
| `/spec:build` | Implement tasks from a defined slice |
| `/spec:merge` | Merge completed slice into baseline |
| `/spec:drop` | Discard a slice without merging |
| `/spec:extract` | Produce specs and design from existing source code |

**Who uses it:** Every Specify operator, every day. This is the primary interaction layer.

**Climb to Layer 3 when:** you have three or more related slices with dependencies, you want a tracked plan to coordinate the work, or you want the framework to drive the slice-by-slice loop automatically. Two slices with no dependencies stay at Layer 2; three or more typically benefit from a plan. See the rubric in [A Multi-Slice Change -- When you need a plan](../tutorials/single-repo-change.md#when-you-need-a-plan).

## Layer 3: Plan & Drive

Layer 3 skills coordinate **multi-slice changes** through `plan.yaml` -- an ordered, dependency-aware list of slices with status tracking. They are the authoring and execution counterparts of a slice-scoped program.

| Skill | Role |
|-------|------|
| `/change:plan` | Author `plan.yaml` from inputs (legacy code, docs, or both) |
| `/change:execute` | Drive the plan through the define-build-merge loop |
| `/spec:analyze` | Plan-time capability inference (used internally by plan) |

The plan is the slice's table of contents. `/change:plan` produces it by analysing inputs and proposing slices. `/change:execute` consumes it by picking the next eligible slice, running define-build-merge, and updating the plan's status.

```text
/change:plan <name> source legacy=./path  -->  /change:execute loop
```

**Who uses it:** Change leads coordinating multi-slice programs -- greenfield builds, legacy migrations, platform modernisations -- when they want fine-grained control over the plan/execute loop or only need a subset of the platform-first flow.

**Climb to Layer 4 when:** the slice spans multiple registered projects (i.e. `registry.yaml` declares more than one project) and you want the automated half of the cross-repo loop -- brief, registry validate, plan, execute, push, then finalize after operator PR merge -- driven as a single operator action. Single-project changes stay at Layer 3 because there is no cross-repo work for the umbrella to compose. Power users running CI pipelines or partial reruns also stay at Layer 3 because the umbrella's value is single-command convenience, not a new capability.

## Layer 4: Change orchestration

Layer 4 is a flag-gated mode of `/change:plan` — `/change:plan <name> orchestrate` — that strings the entire platform-first loop into one operator action. It is **composition only**: every step shells out to a Layer 1 CLI verb or a Layer 3 skill; the orchestration mode adds no new logic.

| Skill | Role |
|-------|------|
| `/change:plan <name> orchestrate` | Brief → registry validate → `/change:plan` (default mode) → `/change:execute loop` → `specify workspace push` → operator PR merge through forge UI / `gh pr merge` → `specify change finalize` |

```text
/change:plan <name> orchestrate [shape ...] [from ...] [source ...]
```

The orchestration mode honours all the halts the underlying skills surface (self-heal, stuck, `registry-amendment-required`) and is **idempotent on re-entry** — running it again after a halt resumes from the appropriate step.

**Who uses it:** Operators driving a cross-repo change end-to-end without leaving the platform hub. Power users still call `/change:plan` (default mode) and `/change:execute` directly when they want partial reruns or CI-pipeline composability.

## The layers compose

A key design principle: higher layers invoke lower layers, but lower layers are unaware of what sits above them. `/change:plan <name> orchestrate` calls `/change:plan` (default mode) and `/change:execute`; `/change:execute` calls `/spec:define`, `/spec:build`, and `/spec:merge` -- the same skills you would invoke manually. The phase skills themselves do not know whether they are running inside an automated loop or being driven by a human.

This means you can always drop down a layer:

- If `/change:plan <name> orchestrate` halts on a step, you can pick up by hand at the next action (`specify workspace push`, operator PR merge, `specify change finalize`).
- If `/change:execute` fails on a slice, you can finish it manually with `/spec:build` and `/spec:merge`.
- If `/change:plan` produces a plan you want to adjust, you can edit it with `specify change plan amend` and drive it yourself with `specify change plan next`.
- If a skill does something unexpected, you can inspect the underlying state with `specify slice status` or `specify change plan status`.

See [Drop down a layer](../how-to/drop-down-a-layer.md) for worked examples of each escape hatch.
