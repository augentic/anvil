# specify change

Create, inspect, transition, and archive individual changes.

## Subcommands

### specify change create

Create a new change directory.

```bash
specify change create <name> [--if-exists skip|error]
```

| Argument | Description |
|----------|-------------|
| `name` | Kebab-case change name (validated) |
| `--if-exists` | Behavior when name exists: `skip` (no-op) or `error` (default) |

Creates `.specify/changes/<name>/` with an initial `.metadata.yaml` in `created` state.

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
| `target` | Target state: `defined`, `building`, `complete`, `merged`, `dropped` |

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
specify change phase-outcome <name> <outcome>
```

| Outcome | Meaning |
|---------|---------|
| `success` | Phase completed successfully |
| `failure` | Phase failed |
| `deferred` | Phase deferred (dependency or external blocker) |

Used by `/spec:execute` to determine plan entry transitions.

### specify change journal-append

Append an entry to the change's journal.

```bash
specify change journal-append <name> --type <question|failure|recovery> --message "<text>"
```

Records questions, failures, and recovery steps in `journal.yaml` for audit.
