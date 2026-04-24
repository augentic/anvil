# specify initiative

Scaffold, populate, validate, transition, and archive initiative plans.

## Subcommands

### specify initiative init

Scaffold an empty plan.

```bash
specify initiative init <name> [--source <key>=<path>...]
```

Creates `.specify/plan.yaml` with the given name and an empty `changes:` list. Optional `--source` entries are recorded in the plan's `sources:` section.

### specify initiative validate

Check structural and referential integrity of the plan.

```bash
specify initiative validate
```

Checks for: duplicate entry names, dependency cycles, unknown `depends-on` / `affects` / `sources` references, and at most one `in-progress` entry.

### specify initiative next

Report the next eligible plan entry.

```bash
specify initiative next
```

Returns the first `pending` entry whose `depends-on` entries are all `done`. Returns an error if no eligible entry exists.

### specify initiative status

Render plan progress.

```bash
specify initiative status [--format json|table]
```

Shows entries in topological order with per-status counts, the active `in-progress` entry (if any), and any `status-reason` annotations.

### specify initiative create

Append a new entry to the plan.

```bash
specify initiative create <name> [--description "<text>"] [--depends-on <entry>...] [--sources <key>...] [--affects <spec>...]
```

Creates the entry in `pending` state.

### specify initiative amend

Edit non-status fields on an existing entry.

```bash
specify initiative amend <name> [--description "<text>"] [--depends-on <entry>...] [--sources <key>...] [--affects <spec>...]
```

### specify initiative transition

Move a plan entry through the status state machine.

```bash
specify initiative transition <name> <target> [--reason "<text>"]
```

| Target | Legal from |
|--------|-----------|
| `in-progress` | `pending` |
| `done` | `in-progress` |
| `failed` | `in-progress` |
| `blocked` | `in-progress` |
| `skipped` | `pending` |

At most one entry may be `in-progress` at a time.

### specify initiative archive

Archive a completed plan.

```bash
specify initiative archive
```

Moves `plan.yaml` and `.specify/plans/<name>/` to `.specify/archive/plans/<YYYYMMDD>-<name>/`.

### specify initiative lock

Manage the advisory plan lock.

```bash
specify initiative lock acquire
specify initiative lock release
specify initiative lock status
```

The lock (`.specify/plan.lock`) is a PID stamp held by `/spec:execute` to prevent concurrent execution. `status` reports whether the lock is held and by which PID.

### specify initiative brief

Manage the operator-authored initiative brief.

```bash
specify initiative brief init
specify initiative brief show
```

`init` scaffolds `.specify/initiative.md` with frontmatter template. `show` renders the brief content.

### specify initiative registry

Manage the platform registry.

```bash
specify initiative registry show
specify initiative registry validate
```

`show` renders `registry.yaml` content. `validate` checks required fields (e.g. `description` required when multiple projects exist).

### specify initiative workspace

Manage the multi-repo workspace.

```bash
specify initiative workspace sync
specify initiative workspace status
```

`sync` clones every project declared in `registry.yaml` into `.specify/workspace/<project>/`. `status` reports which repos are cloned and their baseline state.
