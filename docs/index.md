# Specify Developer Guide

Specify turns ad-hoc AI prompting into a repeatable, auditable workflow. Every change you make through Specify produces durable artifacts -- proposals, behavioral specs, technical designs, and task lists -- that accumulate as your project's living specification.

## What you get

- **Durable reasoning.** The thinking that connects intent to implementation is captured in version-controlled artifacts, not lost when the chat session ends.
- **Baseline accumulation.** Each merged change adds to a growing specification. Future changes build on what came before rather than starting from scratch.
- **Drift detection.** `/spec:verify` compares your code against the specification and flags where they have diverged.
- **Automated execution.** For multi-change initiatives, `/spec:execute` drives the define-build-merge loop change by change, in dependency order.

## See it in action

```text
/spec:init https://github.com/augentic/specify/schemas/omnia

/spec:define "Add a greeting endpoint that accepts a name and returns a message"
  --> generates proposal.md, spec.md, design.md, tasks.md

/spec:build
  --> implements tasks, marks checkboxes as complete

/spec:merge
  --> merges specs into baseline at .specify/specs/
```

That is the entire core workflow. Everything else in Specify builds on this loop.

## Start here

- **New to Specify?** Read [What is Specify?](orientation/index.md), install the [Prerequisites](orientation/prerequisites.md), then follow the [Quick Start](tutorials/quick-start.md).

- **Have an existing codebase?** Start with the [Brownfield Onboarding](tutorials/brownfield-onboarding.md) tutorial to extract specs from your source code, then return to [Your First Change](tutorials/first-change.md).

- **Planning a large initiative?** Read [Your First Change](tutorials/first-change.md) and [Iterating on a Baseline](tutorials/iterating-on-baseline.md) first, then skip to [A Multi-Change Initiative](tutorials/single-repo-initiative.md).

## Guide structure

- **[Getting Started](orientation/index.md)** builds the mental model and gets you set up.
- **[Tutorials](tutorials/index.md)** walk you through progressively complex scenarios.
- **[How-To Guides](how-to/index.md)** answer specific task-oriented questions.
- **[Reference](reference/index.md)** provides the complete lookup table for every skill, CLI command, artifact format, and configuration file.
- **[The Layered Stack](explanation/three-layer-stack.md)** explains the architecture and design decisions.
