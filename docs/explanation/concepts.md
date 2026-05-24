{{#template ../templates/hero-open.md eyebrow=Understanding Specify title=Core concepts}}
Recognise every term that appears throughout the guide after a quick skim of [What is Specify?](../orientation/index.md) or the [Quick start](../tutorials/quick-start.md).

{{#template ../templates/meta-row-open.md}}
{{#template ../templates/meta-chip.md label=Read time value=~12 min}}
{{#template ../templates/meta-chip.md label=Depth value=Conceptual}}
{{#template ../templates/meta-row-close.md}}
{{#template ../templates/hero-close.md}}

{{#template ../templates/audience-grid-open.md}}

{{#template ../templates/audience-open.md who=Operator}}
<a href="#the-plan--gate-1--execute--finalize-rhythm">Change rhythm</a> → <a href="#the-per-slice-loop">Slice loop</a> → <a href="../reference/quick-reference.md">Quick reference</a>
{{#template ../templates/audience-close.md}}

{{#template ../templates/audience-open.md who=Adapter author}}
<a href="#source-and-target-adapters">Adapters</a> → <a href="adapter-anatomy.md">Anatomy</a>
{{#template ../templates/audience-close.md}}

{{#template ../templates/audience-open.md who=Spec reader}}
<a href="#the-four-slice-artifacts">Artifacts</a> → <a href="#evidence-provenance-authority">Evidence</a>
{{#template ../templates/audience-close.md}}

{{#template ../templates/audience-grid-close.md}}

{{#template ../templates/rhythm-open.md}}

{{#template ../templates/rhythm-step-open.md num=01 label=Plan title=Define the change}}
`/spec:plan` enumerates sources and writes `plan.yaml`. Exits at `pending`.
{{#template ../templates/rhythm-step-close.md}}

{{#template ../templates/rhythm-step-open.md num=02 label=Gate 1 title=Human approval}}
Operator stamps `reviewed`. Nothing executes until this transition.
{{#template ../templates/rhythm-step-close.md}}

{{#template ../templates/rhythm-step-open.md num=03 label=Execute title=Build in the loop}}
`/spec:execute` drives refine → build → merge per slice until drained.
{{#template ../templates/rhythm-step-close.md}}

{{#template ../templates/rhythm-close.md}}

## The plan → operator review (Gate 1) → execute → finalize rhythm

Every change flows through one rhythm. Full command detail: [Quick reference card](../reference/quick-reference.md).

{{#template ../templates/pipeline-open.md}}

![Change rhythm](../assets/diagrams/concepts/change-rhythm.svg)

{{#template ../templates/pipeline-close.md caption=/spec:plan exits pending; operator stamps Gate 1; /spec:execute drives slices; /spec:finalize closes the change.}}

`/spec:plan` enumerates each bound source, proposes `slices[]`, and exits at `plan.lifecycle: pending`. The operator stamps the review step explicitly: `specify plan transition <name> reviewed` (Gate 1). `/spec:execute` then drives the per-slice loop until every entry is `done`. `/spec:finalize` pushes branches, observes PRs, and archives.

A one-slice change uses the same steps as a twelve-slice change: `intent.enumerate` produces one candidate and `/spec:execute` runs the same single-slice rhythm.

## The per-slice loop

Each slice runs through three phases inside `/spec:execute`. `/spec:refine` extracts evidence per bound source and synthesizes the artifacts. `/spec:build` works through the task list and writes code. `/spec:merge` folds the slice's specs into the baseline.

{{#template ../templates/pipeline-open.md}}

![Per-slice loop](../assets/diagrams/concepts/slice-loop.svg)

{{#template ../templates/pipeline-close.md caption=refine → build → merge inside /spec:execute; merge folds specs into .specify/specs/ baseline.}}

The same skills are available as breakouts — run one phase by hand — when execute parks on a failure or when you want manual control. See [Drive a slice manually](../how-to/drive-slice-manually.md).

## The four slice artifacts

Refine generates four documents in dependency order. Each one answers a different question and feeds the next:

| Artifact      | Question it answers                                                                | Location                            |
| ------------- | ---------------------------------------------------------------------------------- | ----------------------------------- |
| `proposal.md` | *Why* does this slice exist? What is in scope?                                     | `.specify/slices/<name>/proposal.md` |
| `spec.md`     | *What* must the system do? (behavioural requirements with `ID:`/`Sources:`/`Status:`) | `.specify/slices/<name>/specs/<unit>/spec.md` |
| `design.md`   | *How* will the behaviour be implemented?                                            | `.specify/slices/<name>/design.md`   |
| `tasks.md`    | In what *sequence* should it be built?                                              | `.specify/slices/<name>/tasks.md`    |

Synthesis is owned by **core**, not by adapters. Source adapters supply `Evidence`; target adapters supply a `shape` brief that core synthesis reads as idiom guidance. The four canonical artifacts are written by core in a fixed substep order (`proposal → specs → design → tasks`).

## The baseline

The **baseline** is the accumulated set of merged specs at `.specify/specs/`. It represents the current known behaviour of your system. Every time you run `/spec:merge`, the slice's spec deltas (`ADDED`, `MODIFIED`, `REMOVED`, `RENAMED` blocks keyed by stable `REQ-XXX` ids) are applied to the baseline files. The slice itself is then archived for audit.

Future slices read from the baseline. When you describe a new piece of work, refine consults the baseline to keep new specs consistent with what already exists. Specs are version-controlled alongside your code, so the baseline is reviewable, diffable, and revertable like any other source file.

## Slice vs change

A **slice** is one trip through the refine → build → merge loop. It lives at `.specify/slices/<name>/`, owns its own proposal, specs, design, tasks, and metadata, and ends either merged (folded into the baseline) or dropped (discarded).

A **change** is the operator-defined umbrella that coordinates one or more slices through `change.md` and `plan.yaml`. The change owns the dependency order; each slice still goes through the same per-slice loop. `change` is on-disk vocabulary, not a slash-command namespace — 2.0 drives every change through `/spec:plan`, `/spec:execute`, `/spec:finalize`.

## Source and target adapters

Specify 2.0 splits adapters by direction.

A **source adapter** is the input role. It reads external material (operator intent, written documentation, legacy code, screenshots) and emits `Evidence`. Operations: `enumerate` (plan-time, produces `Candidate[]`) and `extract` (slice-time, produces `Evidence`). First-party defaults: `intent`, `documentation`, `code-typescript`, `screenshots`.

A **target adapter** is the output role. It consumes `spec.md` + `design.md` and produces code. Operations: `shape` (idiom guidance read by core synthesis), `build` (writes code), `merge` (lands the slice). First-party defaults: `omnia` (Rust WASM service crates), `vectis` (cross-platform UI applications), `contracts` (API contracts).

Both ship `adapter.yaml` validated by an axis-specific schema (`source.schema.json` for sources, `target.schema.json` for targets). See [Anatomy of an adapter](adapter-anatomy.md).

You pick the target at scaffolding time (`/spec:init <target>`). You bind sources per change (`/spec:plan <name> source legacy=./repo source docs=./design-notes`).

## Evidence, provenance, authority

When refine runs, each bound source produces an `Evidence` document at `.specify/slices/<name>/evidence/<source-key>.yaml`. Each `Evidence` carries `authority:` (closed enum `intent` > `documentation` > `behaviour`) and a list of `claims:` with structured kinds.

Core synthesis fuses `Evidence[]` into one `spec.md`. Every requirement header carries:

```markdown
ID: REQ-001
Sources: [identity-design-notes, legacy-monolith]
Status: agreed
```

`Sources:` is the **provenance** — which sources contributed the requirement. `Status:` is the closed enum `agreed` | `unknown` | `conflict` | `divergence`. **Authority** controls who wins a disagreement; ties at the top authority produce `[conflict]`, authority-resolved disagreements produce `[divergence]`. Tags surface inline on the requirement header and **never park the slice** — synthesis tag-and-proceeds, and the operator reconciles by hand-editing `spec.md` or by amending the plan to drop a source.

## Skills

A **skill** is a slash-command you invoke in Cursor's agent chat. Skills are how you drive Specify — the agent owns judgement, the skill owns the workflow, and the `specify` CLI does the deterministic work (validation, lifecycle transitions, spec merging, plan writes) underneath.

The default rhythm:

{{#template ../templates/callout-open.md}}
**Commands.** `/spec:init <target>` → `/spec:plan <name> source …` → `specify plan transition <name> reviewed` (Gate 1) → `/spec:execute` → `/spec:finalize <name>`
{{#template ../templates/callout-close.md}}

Breakouts (`/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`) run one phase by hand when execute parks or you want manual control.

{{#template ../templates/see-also-open.md}}
- [Anatomy of an adapter](adapter-anatomy.md) — how source and target adapters compose with core synthesis
- [The layered stack](layered-stack.md) — the architectural framing
- [Quick reference card](../reference/quick-reference.md) — every verb at a glance
{{#template ../templates/see-also-close.md}}
