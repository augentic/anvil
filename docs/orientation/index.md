# What is Specify?

Specify is a plugin system that orchestrates **spec-driven software development** inside [Cursor](https://cursor.com). It replaces ad-hoc prompting with a structured workflow: you describe what you want to build (or point Specify at existing documentation, intent, or legacy code), Specify generates a plan and durable artifacts, then specialist AI skills implement each slice from those artifacts.

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

1. **Plan** — `/spec:plan` surveys sources and writes `plan.yaml`. Exits at `pending`.
2. **Operator review (Gate 1)** — you stamp `approved`: `specrun plan transition <name> approved`.
3. **Execute** — `/spec:execute` loops per slice: refine → build → merge.
4. **Finalize** — `/spec:finalize` pushes branches, observes PRs, archives the plan.

<div class="pipeline">

![Specify change rhythm](../assets/diagrams/orientation/workflow-rhythm.svg)

<p class="pipeline-caption">plan → operator review (Gate 1) → execute → finalize — a one-slice change uses the same steps as a twelve-slice migration.</p>
</div>

<div class="callout">
  <strong>Gate 1.</strong> The operator review step between plan and execute. <code>/spec:plan</code> exits at <code>pending</code>; you stamp <code>approved</code> explicitly. Nothing executes until that transition.
</div>

## Why artifacts matter

Without Specify, reasoning lives in the chat and is lost when the session ends. Specify makes it durable in version-controlled files under `.specify/`. Plan-time artifacts (`change.md`, `plan.yaml`, `discovery.md`) coordinate the change; per-slice artifacts (`proposal.md`, `spec.md`, `design.md`, `tasks.md`) capture requirements and implementation sequencing. See [Artifacts in depth](../explanation/artifacts.md) for the full dependency chain.

## Specify and git

Artifacts are regular files — you commit and review them like source code. `/spec:merge` applies spec deltas on disk but does not create git commits. You control when to commit.

## What you interact with

**Skills** — slash-commands in Cursor (`/spec:init`, `/spec:plan`, `/spec:execute`, `/spec:finalize`). The full skill list lives in the [Quick reference card](../reference/quick-reference.md).

You can also run one phase by hand (a **breakout**) — `/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop` — when execute parks or you want manual control. See [Drive a slice manually](../how-to/drive-slice-manually.md).

Behind the skills, the `specrun` CLI handles deterministic work: validation, lifecycle transitions, spec merging. The agent keeps judgment; the CLI keeps correctness.

## Going deeper

- [Quick start tutorial](../tutorials/quick-start.md) — hands-on first change
- [Core concepts](../explanation/concepts.md) — vocabulary tour
- [The layered stack](../explanation/layered-stack.md) — architecture
