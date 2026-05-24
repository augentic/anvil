<div class="hero">
<div class="eyebrow">Understanding Specify</div>
<h1 class="hero-title">Core concepts</h1>

Recognise every term that appears throughout the guide after a quick skim of [What is Specify?](../orientation/index.md) or the [Quick start](../tutorials/quick-start.md).

<div class="meta-row">

<span class="meta-chip"><strong>Read time</strong> ~12 min</span>

<span class="meta-chip"><strong>Depth</strong> Conceptual</span>

</div>

</div>


<div class="audience-grid">


<div class="audience">
  <div class="who">Operator</div>
  <div class="path">

<a href="#the-plan--gate-1--execute--finalize-rhythm">Change rhythm</a> → <a href="#the-per-slice-loop">Slice loop</a> → <a href="../reference/quick-reference.md">Quick reference</a>
  </div>
</div>


<div class="audience">
  <div class="who">Adapter author</div>
  <div class="path">

<a href="#source-and-target-adapters">Adapters</a> → <a href="adapter-anatomy.md">Anatomy</a>
  </div>
</div>


<div class="audience">
  <div class="who">Spec reader</div>
  <div class="path">

<a href="#the-four-slice-artifacts">Artifacts</a> → <a href="#evidence-provenance-authority">Evidence</a>
  </div>
</div>


</div>


<div class="rhythm">


<div class="rhythm-step" data-step="01">
<div class="rhythm-num">01</div>
<div class="rhythm-label">Plan</div>
<h4 class="rhythm-title">Define the change</h4>

`/spec:plan` enumerates sources and writes `plan.yaml`. Exits at `pending`.
</div>


<div class="rhythm-step" data-step="02">
<div class="rhythm-num">02</div>
<div class="rhythm-label">Gate 1</div>
<h4 class="rhythm-title">Human approval</h4>

Operator stamps `reviewed`. Nothing executes until this transition.
</div>


<div class="rhythm-step" data-step="03">
<div class="rhythm-num">03</div>
<div class="rhythm-label">Execute</div>
<h4 class="rhythm-title">Build in the loop</h4>

`/spec:execute` drives refine → build → merge per slice until drained.
</div>


</div>


## The plan → operator review (Gate 1) → execute → finalize rhythm

Every change flows through one rhythm. Full command detail: [Quick reference card](../reference/quick-reference.md).

<div class="pipeline">


![Change rhythm](../assets/diagrams/concepts/change-rhythm.svg)

<p class="pipeline-caption">/spec:plan exits pending; operator stamps Gate 1; /spec:execute drives slices; /spec:finalize closes the change.</p>
</div>


`/spec:plan` enumerates each bound source, proposes `slices[]`, and exits at `plan.lifecycle: pending`. The operator stamps the review step explicitly: `specify plan transition <name> reviewed` (Gate 1). `/spec:execute` then drives the per-slice loop until every entry is `done`. `/spec:finalize` pushes branches, observes PRs, and archives.

A one-slice change uses the same steps as a twelve-slice change: `intent.enumerate` produces one candidate and `/spec:execute` runs the same single-slice rhythm.

## The per-slice loop

Each slice runs through three phases inside `/spec:execute`. `/spec:refine` extracts evidence per bound source and synthesizes the artifacts. `/spec:build` works through the task list and writes code. `/spec:merge` folds the slice's specs into the baseline.

<div class="pipeline">


![Per-slice loop](../assets/diagrams/concepts/slice-loop.svg)

<p class="pipeline-caption">refine → build → merge inside /spec:execute; merge folds specs into .specify/specs/ baseline.</p>
</div>


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

> [!NOTE]
> **Commands.** `/spec:init <target>` → `/spec:plan <name> source …` → `specify plan transition <name> reviewed` (Gate 1) → `/spec:execute` → `/spec:finalize <name>`

Breakouts (`/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`) run one phase by hand when execute parks or you want manual control.

<div class="see-also">
<strong>See also</strong>

- [Anatomy of an adapter](adapter-anatomy.md) — how source and target adapters compose with core synthesis
- [The layered stack](layered-stack.md) — the architectural framing
- [Quick reference card](../reference/quick-reference.md) — every verb at a glance
</div>

