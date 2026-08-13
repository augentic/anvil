<div class="hero">
<div class="eyebrow">Understanding Emery</div>
<h1 class="hero-title">Core concepts</h1>

Recognise every term that appears throughout the guide after a quick skim of [What is Emery?](../orientation/index.md) or the [Quick start](../tutorials/quick-start.md).

<div class="meta-row">

<span class="meta-chip"><strong>Read time</strong> ~12 min</span>

<span class="meta-chip"><strong>Depth</strong> Conceptual</span>

</div>

</div>


<div class="audience-grid">


<div class="audience">
  <div class="who">Operator</div>
  <div class="path">

<a href="#the-plan--review--refine--review--execute--finalize-rhythm">Change rhythm</a> → <a href="#the-per-slice-loop">Slice loop</a> → <a href="../reference/quick-reference.md">Quick reference</a>
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

`/emery:plan` surveys sources and writes `plan.yaml`. Exits for review.
</div>


<div class="rhythm-step" data-step="02">
<div class="rhythm-num">02</div>
<div class="rhythm-label">Review</div>
<h4 class="rhythm-title">Human review</h4>

Operator reviews topology, then runs `emery plan refine` to generate every slice's specifications, reviews them, and runs `emery plan execute` to open the authorization epoch. Nothing privileged runs until then.
</div>


<div class="rhythm-step" data-step="03">
<div class="rhythm-num">03</div>
<div class="rhythm-label">Execute</div>
<h4 class="rhythm-title">Build in the loop</h4>

`emery plan execute` journals `plan.execute.started` over the exact refinement digests, enforces gap gates before build, and drives build → merge per slice until drained — it never refines.
</div>


</div>


## The plan → review → refine → review → execute → finalize rhythm

Every change flows through one rhythm. Full command detail: [Quick reference card](../reference/quick-reference.md).

<div class="pipeline">


![Change rhythm](../assets/diagrams/concepts/change-rhythm.svg)

<p class="pipeline-caption">/emery:plan exits for review; emery plan refine writes every specification; emery plan execute opens the authorization epoch and drives slices; /emery:finalize closes the change.</p>
</div>


`/emery:plan` surveys each bound source, proposes `slices[]`, and exits for operator review (topology-only — no refine). `emery plan refine` then drains specification refinement over the closed plan — extract, synthesize, validate, and a `refinement.yaml` manifest per slice — and stops before any code work. The operator starts privileged work with the guest-routed `emery plan execute` — which journals `plan.execute.started` over the exact refinement digests and drives the build → merge loop until every entry projects `done`. The operator publishes the resulting repository changes outside Emery; `/emery:finalize` archives only after that publication is complete.

A one-slice change uses the same steps as a twelve-slice change: `intent.survey` produces one lead and `emery plan refine` / `emery plan execute` run the same single-slice rhythm.

## The per-slice loop

Each slice runs through three phases. The refine phase — inside `emery plan refine` — extracts evidence per bound source and synthesizes the artifacts. The build phase — inside `emery plan execute` — works through the task list and writes code. The merge phase folds the slice's specs into the baseline.

<div class="pipeline">


![Per-slice loop](../assets/diagrams/concepts/slice-loop.svg)

<p class="pipeline-caption">refine inside emery plan refine; build → merge inside emery plan execute; merge folds specs into .emery/specs/ baseline.</p>
</div>


If refine or execute parks on a failure, fix the input the stop card points at and re-run the same command — refine skips fresh slices; the execute loop resumes at the parked phase. There are no per-slice phase-breakout verbs.

## The four slice artifacts

Refinement generates four documents in dependency order. Each one answers a different question and feeds the next:

| Artifact      | Question it answers                                                                | Location                            |
| ------------- | ---------------------------------------------------------------------------------- | ----------------------------------- |
| `proposal.md` | *Why* does this slice exist? What is in scope?                                     | `.emery/change/slices/<name>/proposal.md` |
| `spec.md`     | *What* must the system do? (behavioural requirements with `ID:`/`Sources:`/`Status:`) | `.emery/change/slices/<name>/specs/<domain>/spec.md` |
| `design.md`   | *How* will the behaviour be implemented?                                            | `.emery/change/slices/<name>/design.md`   |
| `tasks.md`    | In what *sequence* should it be built?                                              | `.emery/change/slices/<name>/tasks.md`    |

Synthesis is owned by **core**, not by adapters. Source adapters supply `Evidence`; target adapters supply a `guidance` prompt that core synthesis reads as idiom guidance. The four canonical artifacts are written by core in a fixed substep order (`proposal → specs → design → tasks`).

## The baseline

The **baseline** is the accumulated set of merged specs at `.emery/specs/`. It represents the current known behaviour of your system. Every time a slice merges, its spec deltas (`ADDED`, `MODIFIED`, `REMOVED`, `RENAMED` blocks keyed by stable `REQ-XXX` ids) are applied to the baseline files. The slice itself is then archived for audit.

Future slices read from the baseline. When you describe a new piece of work, refinement consults the baseline to keep new specs consistent with what already exists. Specs are version-controlled alongside your code, so the baseline is reviewable, diffable, and revertable like any other source file.

## Slice vs change

A **slice** is one trip through the refine → build → merge rhythm. It lives at `.emery/change/slices/<name>/`, owns its own proposal, specs, design, tasks, and metadata, and ends either merged (folded into the baseline) or dropped (discarded).

A **change** is the operator-defined umbrella that coordinates one or more slices through `change.md` and `plan.yaml`. The change owns the dependency order; each slice still goes through the same per-slice loop. `change` is on-disk vocabulary, not a slash-command namespace; every change is driven through `/emery:plan`, `emery plan refine`, `emery plan execute`, `/emery:finalize`.

## Source and target adapters

Emery splits adapters by direction.

A **source adapter** is the input role. It reads external material (operator intent, written documentation, legacy code, screenshots) and emits `Evidence`. Operations: `survey` (plan-time, produces `Lead[]`) and `extract` (slice-time, produces `Evidence`). First-party defaults: `intent`, `documentation`, `typescript`, `captures`, `screenshots`.

A **target adapter** is the output role. It consumes `spec.md` + `design.md` and produces code. Operations: `guidance` (idiom guidance read by core synthesis), the build-loop operations `build` / `verify` / `repair` / `review` (one pass per dispatch, driven by the engine's build phase machine), and `merge` (lands the slice). First-party defaults: `omnia` (Rust WASM service crates), `vectis` (cross-platform UI applications), `contracts` (API contracts).

Both ship as a single WebAssembly component exporting the matching axis interface from the WIT contract; metadata comes from the component's `metadata` export. See [Anatomy of an adapter](adapter-anatomy.md).

You pick the target at scaffolding time (`/emery:init <target>`). You bind sources per change (`/emery:plan <name> source legacy=typescript:./repo source docs=documentation:./design-notes`).

## Evidence, provenance, authority

When refinement runs, each bound source produces an `Evidence` document at `.emery/change/slices/<name>/evidence/<source>.yaml`. Each `Evidence` carries `authority:` (closed enum `intent` > `documentation` > `behaviour` — canonical: [Authority hierarchy](../../crates/slice/prompts/synthesis/authority.md)) and a list of `claims:` with structured kinds.

Core synthesis reconciles `Evidence[]` into the slice's per-domain `specs/<domain>/spec.md` (the full leads → evidence → `model.yaml` → spec trail is walked in [From sources to slices](reconciliation.md)). Every requirement header carries:

```markdown
ID: REQ-001
Sources: [identity-design-notes, legacy-monolith]
Status: agreed
```

`Sources:` is the **provenance** — which sources contributed the requirement. `Status:` is the closed enum `agreed` | `unknown` | `conflict` | `divergence`. **Authority** controls who wins a disagreement; ties at the top authority produce `[conflict]`, authority-resolved disagreements produce `[divergence]`. Tags surface inline on the requirement header and **never park the slice** — synthesis tags the requirement and proceeds. The `ID:` / `Sources:` / `Status:` lines are machine-rendered and never hand-edited; the operator reconciles through overrides and a re-refine — the full rule and recovery steps live in [Resolve spec conflicts](../how-to/resolve-spec-conflicts.md).

## Skills

A **skill** is a slash-command you invoke in Cursor's agent chat. Skills are how you drive Emery — the agent owns judgement, the skill owns the workflow, and the `emery` CLI does the deterministic work (validation, lifecycle transitions, spec merging, plan writes) underneath.

The default rhythm:

> [!NOTE]
> **Commands.** `/emery:init <target>` → `/emery:plan <name> source …` → `emery plan refine` (specifications) → `emery plan execute` (authorization + loop) → `/emery:finalize <name>`

`/emery:status` projects the next action at any point; `emery plan drop <entry>` abandons a slice without merging.

<div class="see-also">
<strong>See also</strong>

- [From sources to slices](reconciliation.md) — the two reconciliation moments (leads → slices at plan time, evidence → spec at slice time) end to end
- [Anatomy of an adapter](adapter-anatomy.md) — how source and target adapters compose with core synthesis
- [The layered stack](layered-stack.md) — the architectural framing
- [Quick reference card](../reference/quick-reference.md) — every verb at a glance
</div>

