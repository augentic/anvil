# What is Specify?

Specify is a plugin system that orchestrates **spec-driven software development** inside [Cursor](https://cursor.com). It replaces ad-hoc prompting with a structured workflow: you describe what you want to build (or point Specify at existing documentation, intent, or legacy code), Specify generates a plan and a set of interdependent artifacts that capture intent, requirements, design, and implementation sequencing, then specialist AI skills implement each slice from those artifacts.

## The core idea

Every change in Specify flows through one rhythm:

1. **Plan** -- `/spec:plan` reads bound sources, enumerates slice-sized candidates, fuses them across sources, and writes `plan.yaml`. It exits at `plan.lifecycle: pending`.
2. **Gate 1** -- The operator stamps `reviewed` explicitly: `specify plan transition <name> reviewed`. Nothing executes until this happens.
3. **Execute** -- `/spec:execute` loops per slice: refine (extract evidence + synthesize artifacts) → build → merge.
4. **Finalize** -- `/spec:finalize` pushes branches, observes PRs, archives the plan once every PR is merged.

```d2
direction: right
plan: "/spec:plan" {shape: rectangle}
gate: "Gate 1\n(operator stamps reviewed)" {shape: hexagon}
execute: "/spec:execute" {shape: rectangle}
finalize: "/spec:finalize" {shape: rectangle}

plan -> gate: "exits at pending"
gate -> execute: "reviewed"
execute -> finalize: "all done"
```

This rhythm is the heartbeat of Specify. It works the same way whether you are fixing a typo or migrating a 200-service platform — the only thing that changes is what sits inside the loop.

## Why artifacts matter

Without Specify, an AI coding agent receives a prompt and produces code. The reasoning that connects intent to implementation is ephemeral -- it lives in the conversation and is lost when the session ends.

Specify makes that reasoning durable:

- **`change.md`** captures *why* the change exists at the operator-author level.
- **`plan.yaml`** captures *what* slices will land and in what order.
- **`discovery.md`** captures the candidate inventory each source enumerated.
- **`proposal.md`** captures why each slice exists and what is in scope.
- **`spec.md`** captures *what* the system must do — behavioral requirements with `ID:` / `Sources:` / `Status:` provenance.
- **`design.md`** captures *how* the behavior will be implemented — domain models, APIs, business logic.
- **`tasks.md`** captures the *sequence* — what to build first, what depends on what.

These artifacts are version-controlled alongside your code. They serve as the contract between human intent and agent execution, and they accumulate as a baseline that future slices build on.

## Specify and git

Specify artifacts live in a `.specify/` directory at your project root. They are regular files -- you commit them, branch them, and review them like any other source file. `/spec:merge` modifies files on disk (applying spec deltas to the baseline) but does not create git commits. You control when and how to commit.

## What you interact with

You interact with Specify through **skills** -- commands prefixed with `/spec:` that you invoke in Cursor's agent chat. The headline skills:

| Skill            | Purpose                                                                    |
| ---------------- | -------------------------------------------------------------------------- |
| `/spec:init`     | One-time project setup; selects a target adapter (Omnia, Vectis, …)        |
| `/spec:plan`     | Enumerate sources, propose `slices[]`, exit at Gate 1                      |
| `/spec:execute`  | Drive the per-slice refine → build → merge loop                            |
| `/spec:finalize` | Push branches, observe PRs, archive plan                                   |

Breakouts (`/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`) are used when execute parks or when an operator wants to drive one slice by hand.

For the architectural framing of how skills compose with sources, targets, and core synthesis, see [The layered stack](../explanation/layered-stack.md) and [Anatomy of an adapter](../explanation/adapter-anatomy.md).

Behind these skills, a Rust CLI binary (`specify`) handles every deterministic operation -- manifest validation, lifecycle transitions, spec merging, task tracking. The agent keeps judgment; the CLI keeps correctness.

## Going deeper

For a detailed understanding of Specify's layered architecture, artifact system, and adapter model, see [The Layered Stack](../explanation/layered-stack.md).
