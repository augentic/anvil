# The Layered Stack

Specify is organised in four layers. Each layer is independently useful, and each builds on the one below it.

> **Naming note.** The pre-RFC-9 stack was a "three-layer stack" topped by `/spec:plan` and `/spec:execute`. RFC-9 §2C added the umbrella verb at Layer 4 (Initiative Orchestration), promoting plan-and-drive to its own dedicated layer. The umbrella was originally a separate `/spec:initiative` skill; it now lives as the `--orchestrate` mode of `/spec:plan`. The filename of this page (`three-layer-stack.md`) is preserved so existing cross-references keep resolving — see the [decision log entry](decision-log.md#independently-useful-layers) for the rationale.

```d2
direction: down

Layer4: "Layer 4 — Initiative Orchestration" {
  initiative: "/spec:plan --orchestrate"
}

Layer3: "Layer 3 — Plan & Drive" {
  plan: "/spec:plan"
  execute: "/spec:execute"
  analyze: "/spec:analyze"
  plan -> execute
}

Layer2: "Layer 2 — Change Lifecycle" {
  define: "/spec:define"
  build: "/spec:build"
  mergeSkill: "/spec:merge"
  drop: "/spec:drop"
  extract: "/spec:extract"
}

Layer1: "Layer 1 — CLI Primitives" {
  changeCli: "specify change"
  planCli: "specify plan"
  initCli: "specify initiative"
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

The `specify` CLI is the foundation. It owns every deterministic operation: creating and transitioning changes, validating artifacts, parsing tasks, merging specs, managing plans. Skills never hand-edit `.metadata.yaml`, never `mkdir -p .specify/...`, and never `mv` anything into the archive. All writes flow through the CLI.

The primary command families are:

- **`specify change ...`** -- per-change CRUD, validation, merge (`change merge {preview, conflict-check, run}`), task tracking (`change task {progress, mark}`), phase outcome (`change outcome {set, show}`), and journal entries (`change journal {append, show}`).
- **`specify plan ...`** -- scaffold, populate, validate, transition, and archive an initiative plan.
- **`specify initiative ...`** -- manage the operator-authored initiative brief at `initiative.md` and finalize an initiative once every PR has merged (`initiative {create, show, finalize}`).
- **`specify registry ...`** -- manage the platform registry at `registry.yaml` (multi-repo initiatives) — `registry {add, remove, show, validate}`.
- **`specify workspace ...`** -- materialise, inspect, push, and merge workspace clones for multi-repo initiatives. Workspace clones are durable and read-write; the separate read-only legacy-source clones used by `/spec:analyze` live elsewhere -- see [Workspace Tiers](workspace-tiers.md) for the distinction.
- **`specify status`** -- project dashboard summarising registry, plan, and active changes.

**Who uses it:** Power users who want fine-grained control, CI pipelines, and anyone debugging the state of `.specify/`. Layer 1 is always available as a manual fallback beneath the higher layers.

**Climb to Layer 2 when:** you are about to author or implement a single change. The Layer 2 skills wrap the CLI primitives in agent-driven orchestrators that elicit intent, read briefs, and write artifacts on your behalf. Reaching for `specify change create` directly is rare outside CI scripts and recovery paths.

## Layer 2: Change lifecycle

Layer 2 skills operate on a **single change** inside `.specify/changes/<name>/`. They form the define-build-merge loop:

```text
/spec:define  -->  /spec:build  -->  /spec:merge
```

Each skill is an agent-driven orchestrator. It elicits intent from the user, reads brief pipelines declared by the active capability, writes artifacts, invokes specialist plugin skills (e.g. `/omnia:crate-writer`), and renders summaries. Deterministic work is delegated to the Layer 1 CLI underneath.

The full set of Layer 2 skills:

| Skill | Role |
|-------|------|
| `/spec:init` | One-time project setup |
| `/spec:define` | Generate all artifacts for a new change |
| `/spec:build` | Implement tasks from a defined change |
| `/spec:merge` | Merge completed change into baseline |
| `/spec:drop` | Discard a change without merging |
| `/spec:extract` | Produce specs and design from existing source code |

**Who uses it:** Every Specify operator, every day. This is the primary interaction layer.

**Climb to Layer 3 when:** you have three or more related changes with dependencies, you want a tracked plan to coordinate the work, or you want the framework to drive the change-by-change loop automatically. Two changes with no dependencies stay at Layer 2; three or more typically benefit from a plan. See the rubric in [A Multi-Change Initiative -- When you need a plan](../tutorials/single-repo-initiative.md#when-you-need-a-plan).

## Layer 3: Plan & Drive

Layer 3 skills coordinate **multi-change programs** through `plan.yaml` -- an ordered, dependency-aware list of changes with status tracking. They are the authoring and execution counterparts of an initiative-scoped program.

| Skill | Role |
|-------|------|
| `/spec:plan` | Author `plan.yaml` from inputs (legacy code, docs, or both) |
| `/spec:execute` | Drive the plan through the define-build-merge loop |
| `/spec:analyze` | Plan-time capability inference (used internally by plan) |

The plan is the initiative's table of contents. `/spec:plan` produces it by analysing inputs and proposing changes. `/spec:execute` consumes it by picking the next eligible change, running define-build-merge, and updating the plan's status.

```text
/spec:plan <name> --source legacy=./path  -->  /spec:execute --loop
```

**Who uses it:** Initiative leads coordinating multi-change programs -- greenfield builds, legacy migrations, platform modernisations -- when they want fine-grained control over the plan/execute loop or only need a subset of the platform-first flow.

**Climb to Layer 4 when:** the initiative spans multiple registered projects (i.e. `registry.yaml` declares more than one project) and you want the cross-repo loop -- brief, registry validate, plan, execute, push, optional merge, finalize -- driven as a single operator action. Single-project initiatives stay at Layer 3 because there is no cross-repo work for the umbrella to compose. Power users running CI pipelines or partial reruns also stay at Layer 3 because the umbrella's value is single-command convenience, not a new capability.

## Layer 4: Initiative orchestration

Layer 4 is a flag-gated mode of `/spec:plan` — `/spec:plan --orchestrate` (RFC-9 §2C, formerly the dedicated `/spec:initiative` skill) — that strings the entire platform-first loop into one operator action. It is **composition only**: every step shells out to a Layer 1 CLI verb or a Layer 3 skill; the orchestration mode adds no new logic.

| Skill | Role |
|-------|------|
| `/spec:plan --orchestrate` | Brief → registry validate → `/spec:plan` (default mode) → `/spec:execute --loop` → `specify workspace push` → optional `specify workspace merge` → `specify initiative finalize` |

```text
/spec:plan --orchestrate <name> [--shape ...] [--from ...] [--source ...] [--auto-merge]
```

The orchestration mode honours all the halts the underlying skills surface (self-heal, stuck, `registry-amendment-required`) and is **idempotent on re-entry** — running it again after a halt resumes from the appropriate step.

**Who uses it:** Operators driving a cross-repo initiative end-to-end without leaving the platform hub. Power users still call `/spec:plan` (default mode) and `/spec:execute` directly when they want partial reruns or CI-pipeline composability.

## The layers compose

A key design principle: higher layers invoke lower layers, but lower layers are unaware of what sits above them. `/spec:plan --orchestrate` calls `/spec:plan` (default mode) and `/spec:execute`; `/spec:execute` calls `/spec:define`, `/spec:build`, and `/spec:merge` -- the same skills you would invoke manually. The phase skills themselves do not know whether they are running inside an automated loop or being driven by a human.

This means you can always drop down a layer:

- If `/spec:plan --orchestrate` halts on a step, you can pick up by hand at the next CLI verb (`specify workspace push`, `specify workspace merge`, `specify initiative finalize`).
- If `/spec:execute` fails on a change, you can finish it manually with `/spec:build` and `/spec:merge`.
- If `/spec:plan` produces a plan you want to adjust, you can edit it with `specify plan amend` and drive it yourself with `specify plan next`.
- If a skill does something unexpected, you can inspect the underlying state with `specify change status` or `specify plan status`.

See [Drop down a layer](../how-to/drop-down-a-layer.md) for worked examples of each escape hatch.
