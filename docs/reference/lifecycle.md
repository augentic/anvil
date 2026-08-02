# Lifecycle

Emery carries three stacked lifecycles. The plan lifecycle gates execution; the per-entry lifecycle drives the loop; the slice lifecycle stamps each per-slice phase. All transitions are enforced by the `emery` CLI — skills never write state directly.

<div class="pipeline">

![Lifecycle state machines](../assets/diagrams/lifecycle/state-machines.svg)

<p class="pipeline-caption">Plan pending→approved; per-entry pending→in-progress→done; slice refining→refined→built→merged (or dropped).</p>
</div>

Emery's layered design is explained in [The Layered Stack](../explanation/layered-stack.md).

## Plan lifecycle

Two stored states. The plan lifecycle does not move further during execution — "currently executing" and "drained" are computed from per-entry status.

### Gate 1

`/emery:plan` writes `pending`. The operator approves by running `emery plan execute` — this is **Gate 1**, the only review seam Emery ships; the first run stamps `approved` before the loop starts. `/emery:plan` never runs it itself. There is no separate approve verb: executing is approving, and the seam is observable on disk (`plan.lifecycle: approved`). A hand-driven plan (breakouts only, no execute loop) stays `pending`; the review still happens before you claim the first entry with `emery plan next`.

## Per-entry lifecycle

Each row under `plan.yaml.slices[]` carries its own status:

- `pending` is written by the reconcile leg inside `emery plan author` (the default slice writer, which replaces all rows on a replaceable plan), `emery plan add`, and `emery plan amend`.
- `in-progress` is written only by `emery plan next`. `plan next` returns the existing `in-progress` entry before selecting a new `pending` row.
- `done` is written only by `emery slice merge` after a successful merge.
- Build failures and merge conflicts leave the active entry `in-progress` — there is no per-entry `failed`, `blocked`, or `skipped` state in v1.

A plan is **drained** when no entry is `pending` or `in-progress`; `/emery:finalize` becomes legal at that point.

## Slice lifecycle

Each slice's `metadata.yaml` tracks an independent lifecycle:

| State      | Meaning                                                                 | Next states                  |
| ---------- | ----------------------------------------------------------------------- | ---------------------------- |
| `refining` | Slice directory created; `/emery:refine` in-flight (extract + synthesize) | `refined`, `dropped`         |
| `refined`  | Canonical artifacts present and validated; ready for build              | `built`, `dropped`           |
| `built`    | Tasks complete; ready for merge                                         | `merged`, `dropped`          |
| `merged`   | Specs applied to baseline; slice archived                               | (terminal)                   |
| `dropped`  | Slice discarded; archived without merging                               | (terminal)                   |

`refining` is the transient state used while `/emery:refine` runs. If extract fails for any bound source, the slice stays in `refining` until the operator amends the plan (e.g. via `emery plan amend <entry> --remove-source <key>`) or fixes the source binding. Synthesis tags (`[unknown]`, `[conflict]`, `[divergence]`) never park the slice — refine still transitions to `refined`.

## Transitions

| Trigger                                          | Transition                       | Performed by                                     |
| ------------------------------------------------ | -------------------------------- | ------------------------------------------------ |
| `/emery:plan` exits at validate                   | plan: `pending` (initial)         | `emery plan author` (scaffold leg)             |
| Operator runs the first `emery plan execute` (Gate 1) | plan: `pending → approved`        | the `emery plan execute` orchestration          |
| `emery plan next` picks next pending row       | per-entry: `pending → in-progress` | `emery plan next`                              |
| `/emery:refine` creates slice                      | slice: (none) → `refining`         | the `emery slice refine` orchestration          |
| `/emery:refine` completes synthesis                | slice: `refining → refined`        | the `emery slice refine` orchestration          |
| `/emery:build` completes tasks                     | slice: `refined → built`           | the `emery slice build` orchestration           |
| `/emery:merge` succeeds                            | slice: `built → merged`; per-entry: `in-progress → done` | `emery slice merge`                            |
| `/emery:drop` invoked                              | slice: `* → dropped`               | `emery slice drop <name> --reason "..."`       |

## `metadata.yaml`

Each slice directory contains a `metadata.yaml` file managed exclusively by the CLI. It records:

- **`status`** — the current slice lifecycle state.
- **`created_at`** / **`updated_at`** — ISO 8601 timestamps.
- **`target`** — the target adapter identifier used for this slice.
- **`touched_specs`** — the list of spec files this slice affects.

Never hand-edit `metadata.yaml`. All writes flow through the CLI.

## Archiving

Both terminal slice states (`merged` and `dropped`) result in the slice directory being moved to the archive:

```text
.emery/archive/YYYY-MM-DD-<slice-name>/
```

The full slice directory is preserved, including all artifacts and `metadata.yaml`. This is a **prunable convenience cache**, not the system of record: at merge time the CLI also appends a `slice.archive.created` entry to the append-only **outcome ledger** (`.emery/journal.jsonl`) capturing the slice name, touched baseline specs, a one-line outcome summary, and the git SHA. The durable history is git of the committed `.emery/specs/` baseline plus that ledger, so archived folders can be reclaimed with `emery archive prune --keep <n>` / `--older-than <days>` without losing the audit trail.

For plans, `emery plan archive` moves a drained `plan.yaml` and its associated `change.md` / `discovery.md` to `.emery/archive/plans/<YYYYMMDD>-<name>/`.
