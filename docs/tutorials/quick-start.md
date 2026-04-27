# Quick Start (5 Minutes)

This gets you from zero to a merged change with no explanation. For the "why" behind each step, see [Your First Change](first-change.md).

**Prerequisites:** [Cursor, Augentic plugins, and the `specify` CLI installed](../orientation/prerequisites.md).

## 1. Initialise

Open your project in Cursor and type in the agent chat:

```text
/spec:init https://github.com/augentic/specify/schemas/omnia
```

<details>
<summary>Expected output</summary>

```text
Specify Initialized
  Schema: omnia@latest
  Project config: .specify/project.yaml
  Cache: .specify/.cache/omnia/
```

</details>

> Use `schemas/vectis` instead if you are building a cross-platform Crux application.

## 2. Define

```text
/spec:define "Add a greeting endpoint that accepts a name and returns a personalised message"
```

<details>
<summary>Expected output</summary>

```text
Change created: add-greeting-endpoint

Generating artifacts...
  ✓ proposal.md
  ✓ specs/greeting/spec.md
  ✓ design.md
  ✓ tasks.md

Change defined. Run /spec:build to implement.
```

</details>

## 3. Build

```text
/spec:build
```

<details>
<summary>Expected output</summary>

```text
Building add-greeting-endpoint...
  ✓ 1.1 Generate the domain crate
  ✓ 1.2 Generate test suites
  ✓ 1.3 Verify output

All tasks complete. Run /spec:merge to finalise.
```

</details>

## 4. Merge

```text
/spec:merge
```

<details>
<summary>Expected output</summary>

```text
Merge preview:
  + specs/greeting/spec.md (new capability)

Merge complete.
  Baseline updated: .specify/specs/greeting/spec.md
  Archived: .specify/archive/2026-04-27-add-greeting-endpoint/
```

</details>

## Done

Your project now has a baseline specification at `.specify/specs/`. Future changes will build on it.

**Next steps:**

- [Your First Change](first-change.md) -- understand what just happened and why each artifact exists.
- [Iterating on a Baseline](iterating-on-baseline.md) -- modify an existing capability with delta specs.
- [Quick Reference Card](../reference/quick-reference.md) -- all skills and commands on one page.
