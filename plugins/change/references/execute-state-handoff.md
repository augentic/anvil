# Execute state handoff

`/change:execute` coordinates one plan entry at a time by exchanging state with phase skills through CLI-owned files. This reference owns the common handoff rules so the execute skill and its sibling files do not repeat them.

## State channels

| Channel | Writer | Reader | Purpose |
|---|---|---|---|
| `plan.yaml` entry status | `/change:execute` via `specify plan transition` | Humans, `specify plan next`, `/change:execute` | Claims `pending -> in-progress` and records terminal `done`, `failed`, or `blocked`. |
| `.specify/slices/<name>/.metadata.yaml:outcome` | Phase skills via `specify slice outcome set`, except merge/drop CLI-stamped success paths | `/change:execute` | The only signal used to decide whether to continue, drop, block, fail, or complete an entry. |
| `.specify/slices/<name>/journal.yaml` | Phase skills, and the driver in the narrow cases below | Humans and audit tooling | Append-only audit log. It is never a signalling channel for the driver. |
| `.specify/plan.lock` | `specify plan lock` | `/change:execute` | Prevents concurrent driver runs from racing plan transitions. |

The driver never hand-edits these files. Every write goes through the CLI command named for that state channel.

## Outcome routing

After each phase returns, the driver reads `specify slice outcome show <name> --format json` and classifies `.outcome.outcome`:

| Outcome | Driver action |
|---|---|
| `success` from `define` or `build` | Continue to the next phase. |
| `success` from `merge` | Run the post-merge residue guard for routed workspace entries, then transition the plan entry to `done`. |
| `failure` | Run `/spec:drop <name> reason "<outcome.summary>"`, then transition the plan entry to `failed`. |
| `deferred` | Run `/spec:drop <name> reason "<outcome.summary>"`, then transition the plan entry to `blocked`. |
| `registry-amendment-required` | Append the registry proposal payload to the slice journal, then follow the `deferred` path. |
| Missing, malformed, or contradictory outcome | Halt for human triage. Do not speculate. |

`outcome.summary` is copied byte-for-byte into the drop skill's `reason` positional and into terminal plan-transition reason values for `failed` and `blocked`. Never paraphrase, truncate, prefix, or reformat it.

## Driver-owned journal appends

Phase skills own ordinary `question` and `failure` entries raised during define/build/merge/drop. `/change:execute` appends to `journal.yaml` only in these cases:

- One `recovery` entry for each self-heal action that resolves or resumes an `in-progress` entry.
- One `failure` entry for a branch-preparation failure during self-heal resume, only when the slice journal already exists.
- One `failure` entry with `summary` prefix `registry-amendment-required:` before dropping a slice whose outcome carries a registry proposal payload.

These entries preserve audit history. They do not drive phase or plan state transitions.

## Dry-run and interrupt rules

`dry-run` performs the same reads and classifications but substitutes every write with a report. It must not invoke phase skills, `/spec:drop`, `specify plan transition`, `specify slice journal append`, workspace push/merge/finalize, or residue commits.

On SIGINT or SIGTERM, finish the current phase if one is already running, skip later phases for that entry, leave the plan entry `in-progress`, release the driver lock, and let the next run's self-heal reconcile from `.metadata.yaml:outcome`.
