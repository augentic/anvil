# specify plan

Scaffold, populate, validate, transition, and archive initiative plans.

## Subcommands

### specify plan init

Scaffold an empty plan.

```bash
specify plan init <name> [--source <key>=<path>...]
```

Creates `.specify/plan.yaml` with the given name and an empty `changes:` list. Optional `--source` entries are recorded in the plan's `sources:` section.

### specify plan validate

Check structural and referential integrity of the plan.

```bash
specify plan validate
```

Checks for: duplicate entry names, dependency cycles, unknown `depends-on` / `affects` / `sources` references, and at most one `in-progress` entry.

### specify plan next

Report the next eligible plan entry.

```bash
specify plan next
```

Returns the first `pending` entry whose `depends-on` entries are all `done`. Returns an error if no eligible entry exists.

### specify plan status

Render plan progress.

```bash
specify plan status [--format json|table]
```

Shows entries in topological order with per-status counts, the active `in-progress` entry (if any), and any `status-reason` annotations.

### specify plan create

Append a new entry to the plan.

```bash
specify plan create <name> [--description "<text>"] [--depends-on <entry>...] [--sources <key>...] [--affects <spec>...]
```

Creates the entry in `pending` state.

### specify plan amend

Edit non-status fields on an existing entry.

```bash
specify plan amend <name> [--description "<text>"] [--depends-on <entry>...] [--sources <key>...] [--affects <spec>...]
```

### specify plan transition

Move a plan entry through the status state machine.

```bash
specify plan transition <name> <target> [--reason "<text>"]
```

| Target | Legal from |
|--------|-----------|
| `in-progress` | `pending` |
| `done` | `in-progress` |
| `failed` | `in-progress` |
| `blocked` | `in-progress` |
| `skipped` | `pending` |

At most one entry may be `in-progress` at a time.

### specify plan archive

Archive a completed plan.

```bash
specify plan archive
```

Moves `plan.yaml` and `.specify/plans/<name>/` to `.specify/archive/plans/<YYYYMMDD>-<name>/`.

### specify plan lock

Manage the advisory plan lock.

```bash
specify plan lock acquire
specify plan lock release
specify plan lock status
```

The lock (`.specify/plan.lock`) is a PID stamp held by `/spec:execute` to prevent concurrent execution. `status` reports whether the lock is held and by which PID.
