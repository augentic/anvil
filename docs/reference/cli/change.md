# specify change

Create, inspect, validate, transition, merge, and archive individual changes. The `change` noun group absorbs the per-change operations that previously lived at the top level (`validate`, `merge`, `spec`, `task`); see [Migrating CLI v1](../../explanation/migrating-cli-v1.md) for the rename map.

Every per-change verb takes the change `<name>`. The CLI resolves the on-disk directory from the name internally (no `<change-dir>` arg).

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`create`](#specify-change-create) | Create a new change directory with an initial `.metadata.yaml`. |
| [`list`](#specify-change-list) | List all active changes. |
| [`status`](#specify-change-status) | Detailed status for one change (lifecycle state, artifacts, tasks, timestamps). |
| [`transition`](#specify-change-transition) | Move a change through the lifecycle state machine (`created` -> `defining` -> `defined` -> `building` -> `complete` -> `merged`/`dropped`). |
| [`validate`](#specify-change-validate) | Run artifact validation (formerly top-level `specify validate`). |
| [`merge`](#specify-change-merge) | `merge {preview, conflict-check, run}` -- preview the delta merge, detect baseline conflicts, or execute the merge. |
| [`task`](#specify-change-task) | `task {progress, mark}` -- inspect or update the task checkbox state in `tasks.md`. |
| [`outcome`](#specify-change-outcome) | `outcome {set, show}` -- write or read the phase outcome that `/spec:execute` consumes. |
| [`journal`](#specify-change-journal) | `journal {append, show}` -- append or read `journal.yaml` entries (questions, failures, recoveries). |
| [`touched-specs`](#specify-change-touched-specs) | Scan or set the spec files this change affects. |
| [`overlap`](#specify-change-overlap) | Find changes whose touched specs overlap. |
| [`archive`](#specify-change-archive) | Move a `merged` or `dropped` change into `.specify/archive/`. |
| [`drop`](#specify-change-drop) | Discard a change without merging. |

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

Returns lifecycle state, artifact completion, task progress, and timestamps. The bare project dashboard lives at [`specify status`](status.md).

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

### specify change validate

Run structural and semantic artifact validation against a change.

```bash
specify change validate <name> [--format json]
```

Checks include:

- **Structural checks** -- artifact files exist, conform to expected format, required sections present.
- **Referential checks** -- specs referenced in the proposal exist, requirement IDs are unique and stable.
- **Capability checks** -- artifacts conform to the active capability's rules.
- **Composition checks** (Vectis only) -- structural validation of `composition.yaml` plus cross-artifact checks (field coverage, event coverage, ViewModel mapping, overlay trigger consistency, navigation graph consistency). See [Artifact Format > Composition](../artifact-format.md#composition-document-vectis-only) for the full checklist.

Returns a JSON report with `Pass` / `Fail` / `Deferred` classifications. The Pass/Fail/Deferred model lets the CLI handle structural checks while the agent evaluates semantic ones; see the [Decision Log](../../explanation/decision-log.md) for the rationale.

### specify change merge

Three subcommands cover the merge surface (renamed from the old top-level `specify merge` / `specify spec preview` / `specify spec conflict-check`).

#### specify change merge preview

Preview what a merge would do without writing anything.

```bash
specify change merge preview <name> [--format json]
```

Shows which baseline specs would be created, modified, or removed. For Vectis changes, also previews composition delta operations (screen-level `added`/`modified`/`removed`). Used by `/spec:merge` before committing.

#### specify change merge conflict-check

Detect whether the baseline has changed since the change was defined.

```bash
specify change merge conflict-check <name> [--format json]
```

Returns a pass/fail result. Checks for both spec conflicts and composition conflicts (Vectis only -- detects when a baseline screen has been modified by another merged change since this change was created, using per-screen checksums). If conflicts are detected, the change's specs may need to be regenerated against the current baseline.

#### specify change merge run

The terminal merge operation. Commits the delta merge and archives the change.

```bash
specify change merge run <name> [--format json]
```

Performs:

1. Applies spec deltas from the change to the baseline at `.specify/specs/`.
2. Applies composition deltas (Vectis only) -- merges `composition.yaml` screen-level `added`/`modified`/`removed` operations into the baseline `composition.yaml`, using per-screen SHA-256 checksums (`.composition-checksums.yaml`) for conflict detection.
3. Validates coherence of the merged baseline.
4. Transitions the change to `merged` and stamps `PhaseOutcome { phase: merge, outcome: success }` atomically with the status transition.
5. Moves the change directory to `.specify/archive/YYYY-MM-DD-<name>/`.

This is the CLI command invoked by `/spec:merge` after preview and conflict-check pass. It is a single atomic operation -- if any step fails, no changes are committed.

**Workspace clone auto-commit (RFC-3b).** When `change merge run` runs inside a workspace clone (CWD is under `.specify/workspace/*/` and contains `.specify/project.yaml`), it auto-commits the merged baseline and archived change directory with message `"specify: merge <change-name>"`. Only `.specify/` subtrees are staged. A commit failure is a warning, not an error -- the spec-merge still succeeds. The operator uses `specify workspace push` to publish commits to remotes.

**Preconditions.** Change must be in `complete` state; `change merge preview` and `change merge conflict-check` should pass (the skill checks these before calling `merge run`).

### specify change task

Two subcommands cover the task surface (renamed from the old top-level `specify task progress` / `specify task mark`).

#### specify change task progress

Report task completion progress for a change.

```bash
specify change task progress <name> [--format json]
```

Returns the count of completed and total tasks, parsed from `tasks.md` checkbox syntax.

#### specify change task mark

Mark a task as complete.

```bash
specify change task mark <name> <task-id> [--format json]
```

Flips the checkbox from `- [ ]` to `- [x]` for the specified task. The task ID is the numbered identifier (e.g. `1.2`, `2.1`).

Used by `/spec:build` as it completes each task.

### specify change outcome

Two subcommands cover the phase outcome surface (renamed from `specify change phase-outcome` / bare `specify change outcome`).

#### specify change outcome set

Write the phase outcome for a change.

```bash
specify change outcome set <name> <phase> <outcome> --summary "..." [--context "..."]
```

| Argument | Description |
|----------|-------------|
| `name` | Change name |
| `phase` | Phase that completed: `define`, `build`, or `merge` |
| `outcome` | One of `success`, `failure`, or `deferred` |
| `--summary` | Short description of the outcome |
| `--context` | Optional verbatim detail (stderr tail, failing test, etc.) |

Used by `/spec:execute` to determine plan entry transitions. For merge success, the CLI stamps the outcome automatically during `change merge run` -- skills do not call `outcome set` on the merge success path.

#### specify change outcome show

Read the phase outcome for a change.

```bash
specify change outcome show <name> [--format json]
```

Returns the `outcome` field from `.metadata.yaml`. Falls back to the archive when the active change directory is absent (e.g. after a successful merge archives the change). Used by `/spec:execute` to read the result of a phase after it returns.

### specify change journal

Two subcommands cover the change journal surface (renamed from `specify change journal-append`; `show` is new).

#### specify change journal append

Append an entry to the change's journal.

```bash
specify change journal append <name> <phase> <kind> --summary "..." [--context "..."]
```

| Argument | Description |
|----------|-------------|
| `name` | Change name |
| `phase` | Phase context: `define`, `build`, or `merge` |
| `kind` | Entry type: `question`, `failure`, or `recovery` |
| `--summary` | Short description |
| `--context` | Optional verbatim detail |

Records questions, failures, and recovery steps in `journal.yaml` for audit. The journal is append-only and never consumed as a signalling channel -- `.metadata.yaml:outcome` is the only state `/spec:execute` reads.

#### specify change journal show

Read the journal entries for a change.

```bash
specify change journal show <name> [--format json]
```

Renders the journal in chronological order. Useful for triaging failed or deferred runs.

## See also

- [/spec:define](../change-skills/define.md) -- skill that creates changes
- [/spec:build](../change-skills/build.md) -- skill that drives build, calls `change task progress`/`mark`
- [/spec:merge](../change-skills/merge.md) -- skill that orchestrates `change merge {preview, conflict-check, run}`
- [/spec:drop](../change-skills/drop.md) -- skill that drops changes
- [Lifecycle](../lifecycle.md) -- change state machine reference
- [Configuration Files](../configuration.md) -- project and change metadata
- [Migrating CLI v1](../../explanation/migrating-cli-v1.md) -- rename map for the cleanup.
