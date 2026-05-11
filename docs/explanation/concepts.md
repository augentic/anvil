# Core concepts

This primer is for developers who have skimmed [What is Specify?](../orientation/index.md) and want a friendly tour of the vocabulary before running anything. After reading it you will recognise every term that appears in the Quick Start and know where each piece lives on disk.

## The define → build → merge loop

Every piece of work in Specify flows through three phases. **Define** turns a description into a set of artifacts (structured documents that capture intent, requirements, design, and sequencing). **Build** works through those artifacts and writes the code. **Merge** folds the slice's specs into your project's baseline so future work can build on them.

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

This loop is the heartbeat of Specify. It works the same way whether you are adding a single endpoint or coordinating dozens of services -- the only thing that changes is what sits above it.

## The four artifacts

Define generates four documents in dependency order. Each one answers a different question and feeds the next:

| Artifact | Question it answers | Location |
|----------|---------------------|----------|
| `proposal.md` | *Why* does this slice exist? What is in scope? | `.specify/slices/<name>/proposal.md` |
| `spec.md` | *What* must the system do? (behavioural requirements) | `.specify/slices/<name>/specs/<unit>/spec.md` |
| `design.md` | *How* will the behaviour be implemented? | `.specify/slices/<name>/design.md` |
| `tasks.md` | In what *sequence* should it be built? | `.specify/slices/<name>/tasks.md` |

The proposal scopes the work, the spec turns that scope into testable requirements, the design turns those requirements into a concrete shape (domain models, APIs, business logic), and the tasks turn the design into an ordered checklist that build can tick off.

## The baseline

The **baseline** is the accumulated set of merged specs at `.specify/specs/`. It represents the current known behaviour of your system -- the durable answer to "what does this codebase actually do?".

The baseline grows over time. Every time you run `/spec:merge`, the slice's spec deltas (`ADDED`, `MODIFIED`, `REMOVED`, `RENAMED` blocks keyed by stable `REQ-XXX` ids) are applied to the baseline files. The slice itself is then archived for audit.

The baseline matters because future slices read from it. When you describe a new piece of work, the define phase consults the baseline to understand what already exists, which keeps new specs consistent with the system you've built so far. Specs are version-controlled alongside your code, so the baseline is reviewable, diffable, and revertable like any other source file.

## Slice vs change

A **slice** is one trip through the define → build → merge loop. It lives at `.specify/slices/<name>/`, owns its own proposal, specs, design, tasks, and metadata, and ends either merged (folded into the baseline) or dropped (discarded).

A **change** is the umbrella that coordinates one or more slices. It sits in `change.md` and `plan.yaml` at the project root and is useful when a single piece of work needs to land in a deliberate order -- for example, a contract change that other implementation slices depend on, or a piece of work that spans several repos. The change owns the dependency order; each slice still goes through the same loop.

Most beginners never need a change -- a single slice is enough for the vast majority of day-to-day work. Reach for a change when you have multiple slices that must land together, or when the work crosses repository boundaries.

## Capabilities

A **capability** is the extension that tells Specify how to generate artifacts and build code for a particular outcome domain. The first-party capabilities today are **Omnia** (Rust WASM service crates), **Vectis** (cross-platform UI applications), and **Contracts** (standalone API contract changes). You pick one when you scaffold a project with `/spec:init <capability>`, and that choice configures which artifacts the define pipeline produces and which specialist skills the build phase delegates to.

> **Note:** The word "capability" has a second meaning inside Specify. Inside a slice's specs (`specs/<capability>/spec.md`) it refers to a unit of behaviour with its own spec file -- a crate in Omnia, a feature in Vectis. This primer uses "capability" only in the first sense (the project-level extension). The two meanings are kept distinct in the [glossary](../appendices/glossary.md).

## Skills

A **skill** is a slash-command you invoke in Cursor's agent chat. Skills are how you drive Specify -- the agent owns the judgement, the skill owns the workflow, and a Rust CLI binary called `specify` does the deterministic work (validation, lifecycle transitions, spec merging, task tracking) underneath.

The skills you'll use most are `/spec:init`, `/spec:define`, `/spec:build`, `/spec:merge`, and `/spec:drop`. A typical first interaction looks like this:

```
/spec:define "Add a greeting endpoint that returns hello to a named user"
```

The agent runs the define pipeline, generates the four artifacts, and stops. You review them, then run `/spec:build` to implement the tasks and `/spec:merge` to fold the specs into the baseline.

## Where to next

- [Quick Start](../tutorials/quick-start.md) -- run the loop end to end in five minutes.
- [Your first slice](../tutorials/first-change.md) -- the same loop, annotated.
- [The layered stack](three-layer-stack.md) -- the architectural framing for readers who want to know how the pieces compose.
