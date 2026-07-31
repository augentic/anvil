# What is Emery?

Emery orchestrates **spec-driven software development** inside [Cursor](https://cursor.com). It replaces ad-hoc prompting with a structured workflow: you describe what you want to build (or point Emery at existing documentation, intent, or legacy code), Emery generates a plan and durable artifacts, then the `emery` runtime and target adapters implement each slice from those artifacts. Operators drive the loop with `/emery:*` skills — ultrathin wrappers over CLI verbs.

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

1. **Plan** — `/emery:plan` surveys sources and writes `plan.yaml`. Exits at `pending`.
2. **Operator review (Gate 1)** — you review, then approve by running `emery plan execute` (its first run stamps `approved`).
3. **Execute** — the same `emery plan execute` loops per slice: refine → build → merge.
4. **Finalize** — after operator-owned publication is complete, `/emery:finalize` archives the plan.

<div class="pipeline">

![Emery change rhythm](../assets/diagrams/orientation/workflow-rhythm.svg)

<p class="pipeline-caption">plan → operator review (Gate 1) → execute → finalize — a one-slice change uses the same steps as a twelve-slice migration.</p>
</div>

<div class="callout">
  <strong>Gate 1.</strong> The operator review step between plan and execute. <code>/emery:plan</code> exits at <code>pending</code>; invoking <code>emery plan execute</code> is your approval act — its first run stamps <code>approved</code>. Nothing executes until you invoke it.
</div>

## Why artifacts matter

Without Emery, reasoning lives in the chat and is lost when the session ends. Emery makes it durable in version-controlled files under `.emery/`. Plan-time artifacts (`change.md`, `plan.yaml`, `discovery.md`) coordinate the change; per-slice artifacts (`proposal.md`, `spec.md`, `design.md`, `tasks.md`) capture requirements and implementation sequencing. See [Artifacts in depth](../explanation/artifacts.md) for the full dependency chain.

## Emery and git

Artifacts are regular files — you commit and review them like source code. `/emery:merge` applies spec deltas on disk but does not create git commits. You control when to commit.

## What you interact with

**`/emery:*` skills** — slash-commands in Cursor (`/emery:init`, `/emery:plan`, `/emery:finalize`, and the per-slice breakouts). Each skill elicits arguments, invokes one `emery` verb, and relays its output. The full list lives in the [Quick reference card](../reference/quick-reference.md).

You can also run one phase by hand (a **breakout**) — `/emery:refine`, `/emery:build`, `/emery:merge`, `/emery:drop` — when execute parks or you want manual control. See [Drive a slice manually](../how-to/drive-slice-manually.md).

Behind the skills, the `emery` CLI and its guest orchestrations own lifecycle, validation, synthesis, and target build. Target adapters own domain-specific generation.

## Going deeper

- [Quick start tutorial](../tutorials/quick-start.md) — hands-on first change
- [Core concepts](../explanation/concepts.md) — vocabulary tour
- [The layered stack](../explanation/layered-stack.md) — architecture
