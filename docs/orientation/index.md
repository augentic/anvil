# What is Specify?

Specify is a plugin system that orchestrates **spec-driven software development** inside [Cursor](https://cursor.com). It replaces ad-hoc prompting with a structured workflow: you describe what you want to build, Specify generates a set of interdependent artifacts that capture intent, requirements, design, and implementation sequencing, then specialist AI skills implement the change from those artifacts.

## The core idea

Every change flows through three phases:

1. **Define** -- Describe what you want to build. Specify generates a proposal, behavioral specs, technical design, and an implementation task list.
2. **Build** -- The agent works through the task list, delegating to specialist skills that generate code from the artifacts.
3. **Merge** -- The change's specs merge into your project's baseline, building a cumulative record of what the system does.

```text
/spec:define  -->  /spec:build  -->  /spec:merge
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

## Scaling up

For work that spans multiple changes, Specify adds an orchestration layer:

- A **plan** (`plan.yaml`) sequences changes with dependency tracking.
- `/spec:execute` automates the define-build-merge loop change by change.
- `/spec:plan` derives the plan from inputs -- legacy code, documentation, or both.

For work that spans multiple repositories, a **registry** (`registry.yaml`) declares the repos in your platform, and Specify routes each change to the correct repo with the correct schema.

The same three-phase loop runs at every scale. The coordination layers above it are optional -- you can use Specify for a single change in a single repo and never touch plans or registries.

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
