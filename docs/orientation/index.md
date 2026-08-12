# What is Emery?

> [!NOTE]
> Emery was previously distributed as **Specify**. If your installed Cursor plugin still registers `/spec:*` skills backed by a `specify` CLI, update to the current Augentic plugin release — every command in this guide uses the `emery` names.

Emery is a delivery system for **accountable software change**. It is designed for work where intent is spread across people, documentation, existing code, and observed behaviour — and where uncertainty must remain visible rather than being guessed away. Emery reconciles those sources into reviewable specifications, records the exact inputs an operator authorizes, then drives implementation and verification from durable artifacts.

Operators use Emery inside [Cursor](https://cursor.com) or through the CLI. The `emery` runtime owns the workflow and target adapters implement each slice; `/emery:*` skills are thin slash-command wrappers that each run one `emery` command and relay its output.

## A graduated path

You do not need to read this guide front to back. Pick the row that matches where you are and follow it left to right — each step builds on the one before.

<div class="audience-grid">
  <div class="audience">
    <div class="who">Day 1 — run it</div>
    <div class="path"><a href="prerequisites.md">Prerequisites</a> → <a href="../tutorials/quick-start.md">Quick start</a> → <a href="../tutorials/first-change.md">First multi-slice change</a></div>
  </div>
  <div class="audience">
    <div class="who">Day 2 — understand it</div>
    <div class="path"><a href="../explanation/concepts.md">Core concepts</a> → <a href="../explanation/artifacts.md">Artifacts</a> → <a href="../explanation/reconciliation.md">From sources to slices</a> → <a href="../how-to/index.md">How-to guides</a></div>
  </div>
  <div class="audience">
    <div class="who">Going deeper</div>
    <div class="path"><a href="../explanation/layered-stack.md">Layered stack</a> → <a href="../explanation/adapter-anatomy.md">Adapter anatomy</a> → <a href="../reference/index.md">Reference</a></div>
  </div>
</div>

## The core idea

Every change flows through one rhythm:

1. **Plan** — `/emery:plan` surveys sources and writes `plan.yaml`. Exits for review.
2. **Refine** — after topology review, `emery plan refine` drains refinement per slice and writes each slice's specification bundle. Exits for review.
3. **Execute** — you review the refined specifications, then run `emery plan execute` (opens the authorization epoch); it loops per slice: build → merge.
4. **Finalize** — after operator-owned publication is complete, `/emery:finalize` archives the plan.

<div class="pipeline">

![Emery change rhythm](../assets/diagrams/orientation/workflow-rhythm.svg)

<p class="pipeline-caption">plan → review → refine → review → execute → finalize — a one-slice change uses the same steps as a twelve-slice migration.</p>
</div>

<div class="callout">
  <strong>Review.</strong> Two human review seams: after <code>/emery:plan</code> authors the topology, and after <code>emery plan refine</code> writes the specifications. Invoking <code>emery plan execute</code> opens the authorization epoch over the exact refinement digests and drives build → merge under gap gates. Nothing privileged runs until you invoke it.
</div>

## Why evidence and artifacts matter

Without Emery, a coding agent can produce an answer without preserving which source supported it, where sources disagreed, what remained unknown, or which exact result an operator authorized. Reasoning lives in the chat and disappears when the session ends.

Emery carries that reasoning in version-controlled files under `.emery/`. Evidence records what each source says; specifications preserve provenance and visible gaps; plan-time artifacts (`change.md`, `plan.yaml`, `discovery.md`) coordinate the change; per-slice artifacts (`proposal.md`, `spec.md`, `design.md`, `tasks.md`) capture requirements and implementation sequencing. See [Artifacts in depth](../explanation/artifacts.md) for the full dependency chain.

## Emery and git

Artifacts are regular files — you commit and review them like source code. The merge phase inside `emery plan execute` applies spec deltas on disk but does not create git commits. You control when to commit.

## What you interact with

**`/emery:*` skills** — slash-commands in Cursor (`/emery:init`, `/emery:plan`, `/emery:refine`, `/emery:execute`, `/emery:status`, `/emery:finalize`). Each skill elicits arguments, invokes one `emery` verb, and relays its output. The full list lives in the [Quick reference card](../reference/quick-reference.md).

If execute stops, the stop card names the reason and the resume command — fix the input it points at, then re-run `emery plan execute`; the loop resumes at the parked phase.

Behind the skills, the `emery` CLI owns lifecycle, validation, synthesis, and target build. Target adapters own domain-specific generation.

## Going deeper

- [Quick start tutorial](../tutorials/quick-start.md) — hands-on first change
- [Core concepts](../explanation/concepts.md) — vocabulary tour
- [The layered stack](../explanation/layered-stack.md) — architecture
