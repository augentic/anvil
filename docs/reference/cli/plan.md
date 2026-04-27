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

Checks for: duplicate entry names, dependency cycles, unknown `depends-on` / `affects` / `sources` references, at most one `in-progress` entry, and the following RFC-3b cross-registry checks when `registry.yaml` is present:

- `project-not-in-registry` (error) -- every `project` value must match a `projects[].name` in the registry.
- `project-missing-multi-repo` (error) -- when the registry has multiple projects, every change must carry a `project` field.
- `description-missing-multi-repo` (error) -- when the registry has multiple projects, every project must carry a `description`.
- `schema-mismatch-workspace` (warning) -- a workspace clone's `project.yaml` declares a different schema than the corresponding registry entry.

### specify plan next

Report the next eligible plan entry.

```bash
specify plan next
```

Returns the first `pending` entry whose `depends-on` entries are all `done`. Returns an error if no eligible entry exists.

With `--format json`, when an eligible entry is found the response includes `project` (string or null), `description` (string or null), and `sources` (array or null) alongside `next`. These fields are absent when `reason` is non-null (`all-done`, `stuck`, `in-progress`).

### specify plan status

Render plan progress.

```bash
specify plan status [--format json|table]
```

Shows entries in topological order with per-status counts, the active `in-progress` entry (if any), and any `status-reason` annotations.

### specify plan create

Append a new entry to the plan.

```bash
specify plan create <name> [--project <name>] [--description "<text>"] [--depends-on <entry>...] [--sources <key>...] [--affects <spec>...]
```

Creates the entry in `pending` state.

### specify plan amend

Edit non-status fields on an existing entry.

```bash
specify plan amend <name> [--project <name>] [--description "<text>"] [--depends-on <entry>...] [--sources <key>...] [--affects <spec>...]
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
specify plan lock acquire --pid <pid>
specify plan lock release --pid <pid>
specify plan lock status
```

The lock (`.specify/plan.lock`) is a PID stamp held by `/spec:execute` to prevent concurrent execution. `--pid` defaults to the current process PID; `/spec:execute` passes a stable agent-session PID so `release` can authenticate the holder. `status` reports whether the lock is held and by which PID.

## See also

- [/spec:plan](../initiative-skills/plan.md) -- skill that authors plans
- [/spec:execute](../initiative-skills/execute.md) -- skill that drives plan execution
- [Configuration Files](../configuration.md) -- plan.yaml and registry format
