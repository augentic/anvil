# The Layered Stack

Specify is organised in three layers. Each layer is independently useful, and each builds on the one below it. Underneath all of them is the `specify` CLI — the deterministic substrate that exposes verbs at every layer. The CLI is not itself a layer; it is the medium through which every layer enforces correctness.

```d2
direction: down

Layer2: "Layer 2 — Planning a change" {
  plan: "/change:plan"
  execute: "/change:execute"
  orchestrate: "/change:plan <name> orchestrate"
  analyze: "/change:analyze"
  plan -> execute
}

Layer1: "Layer 1 — Executing a change" {
  define: "/spec:define"
  build: "/spec:build"
  mergeSkill: "/spec:merge"
  drop: "/spec:drop"
  extract: "/spec:extract"
}

Layer0: "Layer 0 — Configuration" {
  projectYaml: "project.yaml"
  capabilityYaml: "capability.yaml"
  schemas: "schemas/"
  toolsYaml: "tools.yaml"
  initVerb: "specify init"
  capabilityVerb: "specify capability"
}

Layer2 -> Layer1
Layer1 -> Layer0
```

## Layer 0: Configuration

Layer 0 is the static project configuration that every higher layer reads. It declares **what** a project is — which capability it uses, what schemas are in scope, what tools are available — without describing **how** any change is planned or executed. Layer 0 is read by Layer 1 and Layer 2 verbs; Layer 0 itself does not run a workflow.

The configuration surfaces:

- **`.specify/project.yaml`** — per-project manifest: `capability:` (or `hub: true` for a registry-only platform hub), `specify_version`, declared `tools:`.
- **`capability.yaml`** — capability manifest declaring the brief pipelines (`define`, `build`, `merge`) consumed by Layer 1.
- **`schemas/`** — JSON Schema files distributed with the binary and consumed by validation.
- **`AGENTS.md` Specify-owned block** — generated guidance the framework owns inside an otherwise operator-owned file.
- **`tools.yaml`** — declared WASI command components (capability or project scoped).

The CLI verbs that read or change Layer 0 state:

- **`specify init`** / **`specify init --hub`** — one-time scaffold of `.specify/`, writes `project.yaml`.
- **`specify capability {resolve, check, pipeline}`** — inspect the active capability manifest and its pipeline shape.
- **`specify status`** — surfaces a summary that includes Layer 0 state.

Layer 0 settles before any change starts. Once `project.yaml` exists and the capability resolves, Layer 1 and Layer 2 can run.

## Layer 1: Executing a change

Layer 1 is the single-slice define-build-merge loop. It operates on **one slice** inside `.specify/slices/<name>/` and is the primary interaction surface for every Specify operator.

```text
/spec:define  -->  /spec:build  -->  /spec:merge
```

Each skill is an agent-driven orchestrator. It elicits intent from the user, reads the brief pipeline declared by the active capability (resolved from Layer 0), writes artifacts, invokes specialist plugin skills (e.g. `/omnia:crate-writer`), and renders summaries. Deterministic work is delegated to the `specify` CLI underneath.

The full set of Layer 1 skills:

| Skill | Role |
|-------|------|
| `/spec:define` | Generate all artifacts for a new slice |
| `/spec:build` | Implement tasks from a defined slice |
| `/spec:merge` | Merge a completed slice into the baseline |
| `/spec:drop` | Discard a slice without merging |
| `/spec:extract` | Produce specs and design from existing source code |

The matching CLI surface is the **`specify slice ...`** family: per-slice CRUD, validation, merge (`slice merge {preview, conflict-check, run}`), task tracking (`slice task {progress, mark}`), phase outcome (`slice outcome {set, show}`), and journal entries (`slice journal {append, show}`). Operators rarely call these directly; the skills wrap them.

**Climb to Layer 2 when:** you have three or more related slices with dependencies, you want a tracked plan to coordinate the work, or you want the framework to drive the slice-by-slice loop automatically. Two slices with no dependencies stay at Layer 1; three or more typically benefit from a plan. See the rubric in [A Multi-Slice Change -- When you need a plan](../tutorials/single-repo-change.md#when-you-need-a-plan).

## Layer 2: Planning a change

Layer 2 coordinates **multi-slice changes** through `plan.yaml` and (for cross-repo work) `registry.yaml`. It is the authoring and execution counterpart of a slice-scoped program.

| Skill | Role |
|-------|------|
| `/change:plan` | Author `plan.yaml` from inputs (legacy code, docs, or both) |
| `/change:execute` | Drive the plan through the Layer 1 define-build-merge loop |
| `/change:plan <name> orchestrate` | Umbrella mode: brief → registry validate → plan → execute → push → operator PR merge → finalize |
| `/change:analyze` | Plan-time capability inference (used internally by `/change:plan`) |

The plan is the change's table of contents. `/change:plan` produces it by analysing inputs and proposing slices. `/change:execute` consumes it by picking the next eligible slice, running the Layer 1 loop, and updating the plan's status.

```text
/change:plan <name> source legacy=./path  -->  /change:execute loop
```

The matching CLI surface spans **`specify change plan ...`** (scaffold, populate, validate, transition, archive a plan), **`specify change ...`** (operator brief at `change.md`, `change finalize`), **`specify registry ...`** (`registry.yaml` CRUD + validate), and **`specify workspace ...`** (materialise, inspect, push workspace clones for multi-repo changes).

### The `orchestrate` umbrella mode

When the change spans multiple registered projects (`registry.yaml` declares more than one project) and the operator wants the entire cross-repo loop driven as a single action, `/change:plan <name> orchestrate` strings together brief → registry validate → `/change:plan` (default mode) → `/change:execute loop` → `specify workspace push` → operator PR merge through forge UI / `gh pr merge` → `specify change finalize`.

The umbrella is **composition only**: every step shells out to a CLI verb or a Layer 2 skill in default mode; the umbrella adds no new logic. It honours every halt the underlying skills surface (self-heal, stuck, `registry-amendment-required`) and is **idempotent on re-entry** — running it again after a halt resumes from the appropriate step.

Single-project changes do not need the umbrella; running `/change:plan` and `/change:execute` directly is sufficient. Power users running CI pipelines or partial reruns also use the default modes because the umbrella's value is single-command convenience, not a new capability.

## The layers compose

A key design principle: higher layers invoke lower layers, but lower layers are unaware of what sits above them. `/change:plan <name> orchestrate` calls `/change:plan` (default mode) and `/change:execute`; `/change:execute` calls `/spec:define`, `/spec:build`, and `/spec:merge` -- the same skills you would invoke manually. The phase skills themselves do not know whether they are running inside an automated loop or being driven by a human.

This means you can always drop down a layer:

- If `/change:plan <name> orchestrate` halts on a step, you can pick up by hand at the next action (`specify workspace push`, operator PR merge, `specify change finalize`).
- If `/change:execute` fails on a slice, you can finish it manually with `/spec:build` and `/spec:merge`.
- If `/change:plan` produces a plan you want to adjust, you can edit it with `specify change plan amend` and drive it yourself with `specify change plan next`.
- If a skill does something unexpected, you can inspect the underlying state with `specify slice status` or `specify change plan status`.

See [Drop down a layer](../how-to/drop-down-a-layer.md) for worked examples of each escape hatch.
