# Lifecycle

Every Specify slice moves through a sequence of lifecycle states. Transitions are enforced by the `specify` CLI -- skills never write state directly.

Specify's layered design is explained in [The Layered Stack](../explanation/layered-stack.md). For the rationale, see the [Decision Log](../explanation/decision-log.md#independently-useful-layers).

## State diagram

```d2
sliceLifecycle: {
  shape: state_diagram

  created -> defining: "/spec:define starts"
  defining -> defined: "/spec:define completes"
  defined -> building: "/spec:build starts"
  building -> complete: "all tasks done"
  complete -> merged: "/spec:merge succeeds"

  created -> dropped: "/spec:drop"
  defining -> dropped: "/spec:drop"
  defined -> dropped: "/spec:drop"
  building -> dropped: "/spec:drop"
  complete -> dropped: "/spec:drop"
}
```

## States

| State | Meaning | Next states |
|-------|---------|-------------|
| `created` | Slice directory exists, artifacts not yet generated | `defining`, `dropped` |
| `defining` | `/spec:define` is in-flight (transient) | `defined`, `dropped` |
| `defined` | All artifacts generated, ready for implementation | `building`, `dropped` |
| `building` | Implementation in progress, tasks being completed | `complete`, `dropped` |
| `complete` | All tasks done, ready for merge | `merged`, `dropped` |
| `merged` | Specs merged into baseline, slice archived | (terminal) |
| `dropped` | Slice discarded, archived without merging | (terminal) |

`defining` and `building` are **transient states** -- they indicate a phase is currently in-flight. Under normal operation, a phase enters the transient state at start and leaves it on completion. If the agent crashes mid-phase, the transient state remains on disk; `/change:execute`'s self-heal reads the transient state to determine which phase to resume.

## Transitions

Transitions are performed by `specify slice transition <name> <target>`. The CLI enforces which transitions are legal from each state and records timestamps in `.metadata.yaml`.

The phase skills trigger transitions at well-defined points:

| Trigger | Transition | Performed by |
|---------|------------|-------------|
| `/spec:define` starts | `created --> defining` | `specify slice transition` |
| `/spec:define` completes all artifacts | `defining --> defined` | `specify slice transition` |
| `/spec:build` starts implementation | `defined --> building` | `specify slice transition` |
| All tasks marked complete | `building --> complete` | `specify slice transition` |
| `/spec:merge` succeeds | `complete --> merged` | `specify slice merge run` |
| `/spec:drop` invoked | `* --> dropped` | `specify slice drop` |

## `.metadata.yaml`

Each slice directory contains a `.metadata.yaml` file managed exclusively by the CLI. It records:

- **`status`** -- the current lifecycle state.
- **`created_at`** / **`updated_at`** -- ISO 8601 timestamps.
- **`outcome`** -- phase outcome (`success`, `failure`, `deferred`) written by `specify slice outcome set`. Used by `/change:execute` to determine whether to transition a plan entry to `done`, `failed`, or `blocked`.
- **`adapter`** -- the adapter identifier used for this slice.
- **`touched_specs`** -- the list of spec files this slice affects.

Never hand-edit `.metadata.yaml`. All writes flow through the CLI.

## Plan entry states

When a slice is part of a change plan, the plan entry has its own status tracked in `plan.yaml`:

```d2
planEntryLifecycle: {
  shape: state_diagram

  pending -> in-progress: "specify plan transition"
  in-progress -> done: "slice merged successfully"
  in-progress -> failed: "slice failed"
  in-progress -> blocked: "slice deferred"
  pending -> skipped: "manually skipped"
}
```

| State | Meaning |
|-------|---------|
| `pending` | Not yet started; waiting for dependencies |
| `in-progress` | Currently being executed (at most one at a time) |
| `done` | Slice merged successfully |
| `failed` | Slice failed during define, build, or merge |
| `blocked` | Slice deferred -- dependency issue or external blocker |
| `skipped` | Manually skipped by operator |

Plan entry transitions are performed by `specify plan transition <name> <target>`.

## Archiving

Both terminal states (`merged` and `dropped`) result in the slice directory being moved to the archive:

```
.specify/archive/YYYY-MM-DD-<slice-name>/
```

The full slice directory is preserved, including all artifacts and `.metadata.yaml`. This provides an audit trail of every slice the project has been through.

For plans, `specify plan archive` moves a completed `plan.yaml` and its working directory to `.specify/archive/plans/<YYYYMMDD>-<name>/`.
