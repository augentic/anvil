# specify change

Create, inspect, transition, and archive individual changes.

## Subcommands

### specify change create

Create a new change directory.

```bash
specify change create <name> [--if-exists fail|continue|restart] [--format json]
```

| Argument | Description |
|----------|-------------|
| `name` | Kebab-case change name (validated) |
| `--if-exists` | Behavior when name exists: `fail` (default, refuse), `continue` (reuse existing -- requires valid `.metadata.yaml`), or `restart` (delete and recreate -- destructive) |
| `--format` | Output format: `json` for structured output |

Creates `.specify/changes/<name>/` with an initial `.metadata.yaml`.

### specify change list

List all active changes.

```bash
specify change list [--format json|table]
```

### specify change status

Show detailed status for a change.

```bash
specify change status <name>
```

Returns lifecycle state, artifact completion, task progress, and timestamps.

### specify change transition

Move a change through the lifecycle state machine.

```bash
specify change transition <name> <target>
```

| Argument | Description |
|----------|-------------|
| `name` | Change name |
| `target` | Target state: `defining`, `defined`, `building`, `complete`, `merged`, `dropped`. The transient states (`defining`, `building`) are typically set by skills, not operators. |

Enforces legal transitions. Records timestamps in `.metadata.yaml`.

### specify change touched-specs

Scan or set the specs affected by a change.

```bash
specify change touched-specs <name> --scan
specify change touched-specs <name> --set <spec-path>...
```

### specify change overlap

Check for spec overlap between active changes.

```bash
specify change overlap <name>
```

Reports which specs are touched by multiple active changes.

### specify change archive

Archive a change (move to `.specify/archive/`).

```bash
specify change archive <name>
```

### specify change drop

Drop a change (transition to `dropped` and archive).

```bash
specify change drop <name> [--reason "<rationale>"]
```

### specify change phase-outcome

Write the phase outcome for a change.

```bash
specify change phase-outcome <name> <phase> <outcome> --summary "..." [--context "..."]
```

| Argument | Description |
|----------|-------------|
| `name` | Change name |
| `phase` | Phase that completed: `define`, `build`, or `merge` |
| `outcome` | One of `success`, `failure`, or `deferred` |
| `--summary` | Short description of the outcome |
| `--context` | Optional verbatim detail (stderr tail, failing test, etc.) |

Used by `/spec:execute` to determine plan entry transitions. For merge success, the CLI stamps the outcome automatically during `specify merge` -- skills do not call `phase-outcome` on the merge success path.

### specify change outcome

Read the phase outcome for a change.

```bash
specify change outcome <name> [--format json]
```

Returns the `outcome` field from `.metadata.yaml`. Falls back to the archive when the active change directory is absent (e.g. after a successful merge archives the change). Used by `/spec:execute` to read the result of a phase after it returns.

### specify change journal-append

Append an entry to the change's journal.

```bash
specify change journal-append <name> <phase> <kind> --summary "..." [--context "..."]
```

| Argument | Description |
|----------|-------------|
| `name` | Change name |
| `phase` | Phase context: `define`, `build`, or `merge` |
| `kind` | Entry type: `question`, `failure`, or `recovery` |
| `--summary` | Short description |
| `--context` | Optional verbatim detail |

Records questions, failures, and recovery steps in `journal.yaml` for audit. The journal is append-only and never consumed as a signalling channel -- `.metadata.yaml:outcome` is the only state `/spec:execute` reads.

## See also

- [/spec:define](../change-skills/define.md) -- skill that creates changes
- [/spec:drop](../change-skills/drop.md) -- skill that drops changes
- [Lifecycle](../lifecycle.md) -- change state machine reference
- [Configuration Files](../configuration.md) -- project and change metadata
