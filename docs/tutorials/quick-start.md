# Quick start

This tutorial walks you through a complete Specify change: one slice, intent-only source, Omnia target. When you finish, you will have merged specs in your baseline and an archived plan.

## What you will build

A minimal Omnia project where Specify plans, specifies, implements, and merges a single slice driven entirely by operator intent — fixing a typo in `user.rs`. A one-slice change uses the same steps as a twelve-slice migration; only the plan row count differs.

## Prerequisites

Complete [Prerequisites](../orientation/prerequisites.md):

- Cursor with Augentic plugins installed
- `specify` CLI (`specify --version` succeeds)
- Rust toolchain with `wasm32-wasip2` target for Omnia

Open your project in Cursor Agent chat. This tutorial assumes a fresh or disposable repo.

## Step 1 — Initialise the project

Run once per project:

```text
/spec:init omnia
```

The skill runs `specify init omnia`, which scaffolds:

```text
.specify/
├── project.yaml
├── slices/
├── specs/          ← baseline accumulates here after merge
└── archive/
AGENTS.md           ← generated when absent
```

See [Directory layout](../reference/directory-layout.md) for the full tree.

## Step 2 — Plan the change

Describe what you want in one line:

```text
/spec:plan fix-typo source intent="fix typo in user.rs"
```

`/spec:plan` writes three plan-time artifacts:

**`change.md`** — operator narrative:

```markdown
# Change — fix-typo

## Intent

fix typo in user.rs

## Scope

- One single-source slice driven by the operator's intent.
```

**`plan.yaml`** — slice table of contents:

```yaml
version: 1
name: fix-typo
sources:
  intent:
    adapter: intent
    value: "fix typo in user.rs"
slices:
  - name: fix-typo
    target: omnia
    sources: [intent]
    status: pending
```

**`discovery.md`** — what sources enumerated:

```markdown
## Summary

Sources: 1. Candidates: 1.

## Candidate inventory

### fix-typo

- id: fix-typo
- sources: [intent]
- summary: fix typo in user.rs
```

The skill exits at `plan.lifecycle: pending` and prints:

```text
Plan `fix-typo` is at `pending`. Run `specify plan transition fix-typo reviewed` to stamp Gate 1, then `/spec:execute` to drive the slices.
```

### Operator review step (Gate 1)

Before any slice work runs, inspect `change.md` and `plan.yaml`. This pause is the **operator review step** — Specify calls it **Gate 1**. `/spec:plan` never stamps `reviewed` itself; you do:

```bash
specify plan transition fix-typo reviewed
```

Learn more: [Amend a plan at Gate 1](../how-to/amend-plan-at-gate-1.md).

## Step 3 — Execute the slice

Drive the per-slice loop:

```text
/spec:execute
```

Inside execute, each slice runs **refine → build → merge**:

<div class="pipeline">

![Per-slice loop](../assets/diagrams/concepts/slice-loop.svg)

<p class="pipeline-caption">refine synthesizes artifacts; build implements tasks; merge folds specs into the baseline.</p>
</div>

### After refine

Under `.specify/slices/fix-typo/` you will find:

| File | Purpose |
| ---- | ------- |
| `proposal.md` | Why the slice exists |
| `specs/<unit>/spec.md` | Behavioral requirements (`ID:`, `Sources:`, `Status:`) |
| `design.md` | Technical approach |
| `tasks.md` | Implementation sequence |
| `evidence/intent.yaml` | What the intent source contributed |

Requirement blocks carry provenance, for example:

```markdown
### Requirement: User display typo corrected

ID: REQ-001
Sources: [intent]
Status: agreed
```

Exact wording varies with your intent; see refine fixtures for shape reference.

### After build

Source code changes land in your project tree (not under `.specify/`). Task checkboxes in `tasks.md` flip to complete via the CLI.

### After merge

- Spec deltas apply to `.specify/specs/`
- The slice directory moves to `.specify/archive/`
- `plan.yaml` marks the entry `done`

If execute parks on a failure, see [Drive a slice manually](../how-to/drive-slice-manually.md).

## Step 4 — Finalize the change

When every plan entry is `done`, close the change:

```text
/spec:finalize fix-typo
```

Finalize pushes branches (when configured), observes PR state, and archives the plan. On a local-only run without remotes, archive still runs once the plan is drained.

## What you learned

- **`/spec:init`** scaffolds `.specify/` once per project.
- **`/spec:plan`** authors the change and exits at `pending`.
- **Gate 1** is the operator review seam before execution.
- **`/spec:execute`** loops refine → build → merge per slice.
- **`/spec:finalize`** closes the change after drain.

## Next steps

- [Core concepts](../explanation/concepts.md) — vocabulary tour
- [Your first multi-slice change](first-change.md) — three slices from documentation
- [Quick reference card](../reference/quick-reference.md) — command cheat sheet
- [Change skills](../reference/change-skills/index.md) — `/spec:plan`, `/spec:execute`, `/spec:finalize` reference
