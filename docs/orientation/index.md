# What is Specify?

Specify is a plugin system that orchestrates **spec-driven software development** inside [Cursor](https://cursor.com). It replaces ad-hoc prompting with a structured workflow: you describe what you want to build, Specify generates a set of interdependent artifacts that capture intent, requirements, design, and implementation sequencing, then specialist AI skills implement the change from those artifacts.

## The core idea

Every change flows through three phases:

1. **Define** -- Describe what you want to build. Specify generates a proposal, behavioral specs, technical design, and an implementation task list.
2. **Build** -- The agent works through the task list, delegating to specialist skills that generate code from the artifacts.
3. **Merge** -- The change's specs merge into your project's baseline, building a cumulative record of what the system does.

```d2
direction: right
define: "/spec:define" {shape: rectangle}
build: "/spec:build" {shape: rectangle}
merge: "/spec:merge" {shape: rectangle}
baseline: "Baseline\n(.specify/specs/)" {shape: cylinder}

define -> build: "artifacts"
build -> merge: "complete"
merge -> baseline: "specs merged"
```

This loop is the heartbeat of Specify. It works the same way whether you are adding a single endpoint or modernising a 200-service platform -- the only difference is what sits above it.

## Why artifacts matter

Without Specify, an AI coding agent receives a prompt and produces code. The reasoning that connects intent to implementation is ephemeral -- it lives in the conversation and is lost when the session ends.

Specify makes that reasoning durable:

- **`proposal.md`** captures *why* the change exists and what is in scope.
- **`spec.md`** captures *what* the system must do -- behavioral requirements with scenarios.
- **`design.md`** captures *how* the behavior will be implemented -- domain models, APIs, business logic.
- **`tasks.md`** captures the *sequence* -- what to build first, what depends on what.

These artifacts are version-controlled alongside your code. They serve as the contract between human intent and agent execution, and they accumulate as a baseline that future changes build on.

## Specify and git

Specify artifacts live in a `.specify/` directory at your project root. They are regular files -- you commit them, branch them, and review them like any other source file. `/spec:merge` modifies files on disk (applying spec deltas to the baseline) but does not create git commits. You control when and how to commit.

## What you interact with

You interact with Specify through **skills** -- commands prefixed with `/spec:` that you invoke in Cursor's agent chat:

| Skill | Purpose |
|-------|---------|
| `/spec:init` | One-time project setup |
| `/spec:define` | Generate artifacts for a new change |
| `/spec:build` | Implement tasks from a defined change |
| `/spec:merge` | Merge completed specs into baseline |
| `/spec:drop` | Discard a change |
| `/spec:status` | Check progress |
| `/spec:verify` | Detect drift between code and specs |
| `/spec:explore` | Think through a problem before defining |
| `/spec:plan` | Author a multi-change initiative plan |
| `/spec:execute` | Drive a plan through the define-build-merge loop |

Behind these skills, a Rust CLI binary (`specify`) handles every deterministic operation -- validation, lifecycle transitions, spec merging, task tracking. The agent keeps judgment; the CLI keeps correctness.

## Going deeper

For a detailed understanding of Specify's layered architecture, artifact system, and schema/plugin model, see [Understanding Specify](../explanation/three-layer-stack.md).
