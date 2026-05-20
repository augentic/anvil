# The Layered Stack

Specify is organised in three layers. Each layer is independently useful, and each builds on the one below it. Underneath all of them is the `specify` CLI — the deterministic substrate that exposes verbs at every layer. The CLI is not itself a layer; it is the medium through which every layer enforces correctness.

```d2
direction: down

Layer2: "Layer 2 — Planning a change" {
  draft: "/change:draft"
  review: "(operator review of plan.yaml)" {shape: hexagon}
  execute: "/change:execute"
  finalize: "/change:finalize"
  analyze: "/change:analyze"
  draft -> review
  review -> execute
  execute -> finalize
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
  adapterYaml: "adapter.yaml"
  schemas: "schemas/"
  toolsYaml: "tools.yaml"
  initVerb: "specify init"
  adapterVerb: "specify adapter"
}

Layer2 -> Layer1
Layer1 -> Layer0
```

## Layer 0: Configuration

Layer 0 is the static project configuration that every higher layer reads. It declares **what** a project is — which adapter it uses, what schemas are in scope, what tools are available — without describing **how** any change is planned or executed. Layer 0 is read by Layer 1 and Layer 2 verbs; Layer 0 itself does not run a workflow.

The configuration surfaces:

- **`.specify/project.yaml`** — per-project manifest: `adapter:` (or `hub: true` for a registry-only platform hub), `specify_version`, declared `tools:`.
- **`adapter.yaml`** — adapter manifest declaring the brief pipelines (`define`, `build`, `merge`) consumed by Layer 1.
- **`schemas/`** — JSON Schema files distributed with the binary and consumed by validation.
- **`AGENTS.md` Specify-owned block** — generated guidance the framework owns inside an otherwise operator-owned file.
- **`tools.yaml`** — declared WASI command components (adapter or project scoped).

The CLI verbs that read or change Layer 0 state:

- **`specify init`** / **`specify init --hub`** — one-time scaffold of `.specify/`, writes `project.yaml`.
- **`specify adapter {resolve, check, pipeline}`** — inspect the active adapter manifest and its pipeline shape.
- **`specify status`** — surfaces a summary that includes Layer 0 state.

Layer 0 settles before any change starts. Once `project.yaml` exists and the adapter resolves, Layer 1 and Layer 2 can run.

## Layer 1: Executing a change

Layer 1 is the single-slice define-build-merge loop. It operates on **one slice** inside `.specify/slices/<name>/` and is the primary interaction surface for every Specify operator.

```text
/spec:define  -->  /spec:build  -->  /spec:merge
```

Each skill is an agent-driven orchestrator. It elicits intent from the user, reads the brief pipeline declared by the active adapter (resolved from Layer 0), writes artifacts, invokes specialist plugin skills (e.g. `/omnia:crate-writer`), and renders summaries. Deterministic work is delegated to the `specify` CLI underneath.

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

Layer 2 coordinates **multi-slice changes** through `plan.yaml` and (for cross-repo work) `registry.yaml`. It is the authoring, execution, and close-out counterpart of a slice-scoped program. Three peer skills carry the change lifecycle, with a deliberate operator review pause between authoring and execution:

| Skill | Role |
|-------|------|
| `/change:draft` | Author `plan.yaml` from inputs (legacy code, docs, or both); stop at the operator review seam |
| `/change:execute` | Drive the plan through the Layer 1 define-build-merge loop |
| `/change:finalize` | Push branches, observe PR state, and run `specify change finalize` once every PR is merged |
| `/change:analyze` | Plan-time adapter inference (used internally by `/change:draft`) |

The plan is the change's table of contents. `/change:draft` produces it by analysing inputs and proposing slices, then halts so the operator can review (and, if needed, edit with `specify plan amend`). `/change:execute` consumes the reviewed plan by picking the next eligible slice, running the Layer 1 loop, and updating the plan's status. `/change:finalize` closes the change once execution is done by pushing branches, confirming each PR is `MERGED`, and archiving `plan.yaml`.

```text
/change:draft <name> source legacy=./path
        |
        v
(operator reviews plan.yaml; edits with `specify plan amend` if needed)
        |
        v
/change:execute loop
        |
        v
/change:finalize <name>
```

The matching CLI surface spans **`specify plan ...`** (scaffold, populate, validate, transition, archive a plan), **`specify change ...`** (`change draft` mints `change.md` and `plan.yaml`; `change finalize` archives), **`specify registry ...`** (`registry.yaml` CRUD + validate), and **`specify workspace ...`** (materialise, inspect, push workspace clones for multi-repo changes).

### The operator review seam

The pause between `/change:draft` and `/change:execute` is the design, not a missing automation. `/change:draft` ends at "plan validated, hand back to operator," and `/change:execute` starts when the operator decides it does — there is no automatic transition between them. This gives operators a deliberate point to inspect `plan.yaml`, run `specify plan status` or `specify plan show`, and amend entries with `specify plan amend` before any per-slice work runs.

The framework does not ship a single "do everything" command for the change layer. Teams that want one-command flow can compose the three skills in their own shell wrapper, accepting that the wrapper opts out of the review pause. The seam is internal to Layer 2; both stages still belong to the planning layer.

## The layers compose

A key design principle: higher layers invoke lower layers, but lower layers are unaware of what sits above them. `/change:execute` calls `/spec:define`, `/spec:build`, and `/spec:merge` -- the same skills you would invoke manually. The phase skills themselves do not know whether they are running inside `/change:execute` or being driven by a human.

This means you can always drop down a layer:

- If `/change:draft` produces a plan you want to adjust, you can edit it with `specify plan amend` and drive it yourself with `specify plan next` instead of `/change:execute`.
- If `/change:execute` fails on a slice, you can finish it manually with `/spec:build` and `/spec:merge`.
- If `/change:finalize` halts on an unmerged PR, you can pick up by hand at the next action (merge through the forge UI, then re-run `specify change finalize`).
- If a skill does something unexpected, you can inspect the underlying state with `specify slice status` or `specify plan status`.

See [Drop down a layer](../how-to/drop-down-a-layer.md) for worked examples of each escape hatch.
