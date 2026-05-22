# specify slice

Create, inspect, validate, transition, merge, and archive individual slices. The `slice` noun group covers every per-slice operation; the `change` noun belongs to the umbrella surface.

Every per-slice verb takes the slice `<name>`. The CLI resolves the on-disk directory from the name internally (no `<slice-dir>` arg).

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`create`](#specify-slice-create) | Create a new slice directory with an initial `.metadata.yaml`. |
| [`status`](#specify-slice-status) | Detailed status for one slice (lifecycle state, artifacts, tasks, timestamps). The multi-slice dashboard lives at [`specify status`](status.md). |
| [`transition`](#specify-slice-transition) | Move a slice through the lifecycle state machine (`refining` -> `refined` -> `built` -> `merged`/`dropped`). |
| [`validate`](#specify-slice-validate) | Run artifact validation. |
| [`merge`](#specify-slice-merge) | `merge {preview, conflict-check, run}` -- preview the delta merge, detect baseline conflicts, or execute the merge. |
| [`task`](#specify-slice-task) | `task {progress, mark}` -- inspect or update the task checkbox state in `tasks.md`. |
| [`outcome`](#specify-slice-outcome) | `outcome {set, show}` -- write or read the phase outcome that `/spec:execute` consumes. |
| [`journal`](#specify-slice-journal) | `journal {append, show}` -- append or read `journal.yaml` entries (questions, failures, recoveries). |
| [`touched-specs`](#specify-slice-touched-specs) | Scan or set the spec files this slice affects. |
| [`overlap`](#specify-slice-overlap) | Find slices whose touched specs overlap. |
| [`drop`](#specify-slice-drop) | Discard a slice without merging. Archive moves are owned by `slice merge run`, `slice drop`, and `change finalize`. |

## Subcommands

### specify slice create

Create a new slice directory.

```bash
specify slice create <name> [--if-exists fail|continue|restart] [--format json]
```

| Argument | Description |
|----------|-------------|
| `name` | Kebab-case slice name (validated) |
| `--if-exists` | Behavior when name exists: `fail` (default, refuse), `continue` (reuse existing -- requires valid `.metadata.yaml`), or `restart` (delete and recreate -- destructive) |
| `--format` | Output format: `json` for structured output |

Creates `.specify/slices/<name>/` with an initial `.metadata.yaml`.

### specify slice status

Show detailed status for a slice.

```bash
specify slice status <name>
```

Returns lifecycle state, artifact completion, task progress, and timestamps. The multi-slice project dashboard lives at [`specify status`](status.md) — there is no `specify slice list`; skills enumerate slices through the umbrella status verb.

### specify slice transition

Move a slice through the lifecycle state machine.

```bash
specify slice transition <name> <target>
```

| Argument | Description |
|----------|-------------|
| `name` | Slice name |
| `target` | Target state: `refining`, `refined`, `built`, `dropped`. Skills stamp `refined` and `built` after `/spec:refine` and `/spec:build`. The `merged` status is intentionally absent — `slice merge run` is the sole legal writer of `merged`, since landing a slice requires the spec merge, status transition, and archive move to happen atomically. |

Enforces legal transitions. Records timestamps in `.metadata.yaml`.

### specify slice touched-specs

Scan or set the specs affected by a slice.

```bash
specify slice touched-specs <name> --scan
specify slice touched-specs <name> --set <spec-path>...
```

### specify slice overlap

Check for spec overlap between active slices.

```bash
specify slice overlap <name>
```

Reports which specs are touched by multiple active slices.

### specify slice drop

Drop a slice (transition to `dropped` and archive).

```bash
specify slice drop <name> [--reason "<rationale>"]
```

### specify slice validate

Run structural and semantic artifact validation against a slice.

```bash
specify slice validate <name> [--format json]
```

Checks include:

- **Structural checks** -- artifact files exist, conform to expected format, required sections present.
- **Referential checks** -- specs referenced in the proposal exist, requirement IDs are unique and stable.
- **Adapter checks** -- artifacts conform to the active adapter's rules.
- **Composition checks** (Vectis only) -- structural validation of `composition.yaml` plus cross-artifact checks (field coverage, event coverage, ViewModel mapping, overlay trigger consistency, navigation graph consistency). See [Artifact Format > Composition](../artifact-format.md#composition-document-vectis-only) for the full checklist.

Returns a JSON report with `Pass` / `Fail` / `Deferred` classifications. The Pass/Fail/Deferred model lets the CLI handle structural checks while the agent evaluates semantic ones; see the [Decision Log](../../explanation/decision-log.md) for the rationale.

### specify slice merge

Three subcommands cover the merge surface.

#### specify slice merge preview

Preview what a merge would do without writing anything.

```bash
specify slice merge preview <name> [--format json]
```

Shows which baseline specs would be created, modified, or removed. For Vectis slices, also previews composition delta operations (screen-level `added`/`modified`/`removed`). Used by `/spec:merge` before committing.

#### specify slice merge conflict-check

Detect whether the baseline has changed since the slice was defined.

```bash
specify slice merge conflict-check <name> [--format json]
```

Returns a pass/fail result. Checks for both spec conflicts and composition conflicts (Vectis only -- detects when a baseline screen has been modified by another merged slice since this slice was created, using per-screen checksums). If conflicts are detected, the slice's specs may need to be regenerated against the current baseline.

#### specify slice merge run

The terminal merge operation. Commits the delta merge and archives the slice.

```bash
specify slice merge run <name> [--format json]
```

Performs:

1. Applies spec deltas from the slice to the baseline at `.specify/specs/`.
2. Applies composition deltas (Vectis only) -- merges `composition.yaml` screen-level `added`/`modified`/`removed` operations into the baseline `composition.yaml`, using per-screen SHA-256 checksums (`.composition-checksums.yaml`) for conflict detection.
3. Validates coherence of the merged baseline.
4. Transitions the slice to `merged` and stamps `PhaseOutcome { phase: merge, outcome: success }` atomically with the status transition.
5. Moves the slice directory to `.specify/archive/YYYY-MM-DD-<name>/`.

This is the CLI command invoked by `/spec:merge` after preview and conflict-check pass. It is a single atomic operation -- if any step fails, no changes are committed.

**Workspace clone auto-commit.** When `slice merge run` runs inside a workspace clone (CWD is under `.specify/workspace/*/` and contains `.specify/project.yaml`), it auto-commits the merged baseline and archived slice directory with message `"specify: merge <slice-name>"`. Only `.specify/` subtrees are staged. A commit failure is a warning, not an error -- the spec-merge still succeeds. Use `specify workspace push` to publish commits to remotes.

**Preconditions.** Slice must be in `built` state; `slice merge preview` and `slice merge conflict-check` should pass (the skill checks these before calling `merge run`).

### specify slice task

Two subcommands cover the task surface (renamed from the old top-level `specify task progress` / `specify task mark`).

#### specify slice task progress

Report task completion progress for a slice.

```bash
specify slice task progress <name> [--format json]
```

Returns the count of completed and total tasks, parsed from `tasks.md` checkbox syntax.

#### specify slice task mark

Mark a task as complete.

```bash
specify slice task mark <name> <task-id> [--format json]
```

Flips the checkbox from `- [ ]` to `- [x]` for the specified task. The task ID is the numbered identifier (e.g. `1.2`, `2.1`).

Used by `/spec:build` as it completes each task.

### specify slice outcome

Two subcommands cover the phase outcome surface (renamed from the historical `specify slice outcome set` / bare `specify change outcome` forms).

#### specify slice outcome set

Write the phase outcome for a slice.

```bash
specify slice outcome set <name> <phase> <outcome> --summary "..." [--context "..."]
```

| Argument | Description |
|----------|-------------|
| `name` | Slice name |
| `phase` | Phase that completed: `define`, `build`, or `merge` |
| `outcome` | One of `success`, `failure`, or `deferred` |
| `--summary` | Short description of the outcome |
| `--context` | Optional verbatim detail (stderr tail, failing test, etc.) |

Used by `/spec:execute` to determine plan entry transitions. For merge success, the CLI stamps the outcome automatically during `slice merge run` -- skills do not call `outcome set` on the merge success path.

#### specify slice outcome show

Read the phase outcome for a slice.

```bash
specify slice outcome show <name> [--format json]
```

Returns the `outcome` field from `.metadata.yaml`. Falls back to the archive when the active slice directory is absent (e.g. after a successful merge archives the slice). Used by `/spec:execute` to read the result of a phase after it returns.

### specify slice journal

Two subcommands cover the slice journal surface (renamed from the historical `specify slice journal append`; `show` is new).

#### specify slice journal append

Append an entry to the slice's journal.

```bash
specify slice journal append <name> <phase> <kind> --summary "..." [--context "..."]
```

| Argument | Description |
|----------|-------------|
| `name` | Slice name |
| `phase` | Phase context: `define`, `build`, or `merge` |
| `kind` | Entry type: `question`, `failure`, or `recovery` |
| `--summary` | Short description |
| `--context` | Optional verbatim detail |

Records questions, failures, and recovery steps in `journal.yaml` for audit. The journal is append-only and never consumed as a signalling channel -- `.metadata.yaml:outcome` is the only state `/spec:execute` reads.

#### specify slice journal show

Read the journal entries for a slice.

```bash
specify slice journal show <name> [--format json]
```

Renders the journal in chronological order. Useful for triaging failed or deferred runs.

## See also

- [/spec:refine](../slice-skills/define.md) -- skill that creates slices
- [/spec:build](../slice-skills/build.md) -- skill that drives build, calls `slice task progress`/`mark`
- [/spec:merge](../slice-skills/merge.md) -- skill that orchestrates `slice merge {preview, conflict-check, run}`
- [/spec:drop](../slice-skills/drop.md) -- skill that drops slices
- [specify change](change.md) -- umbrella surface that coordinates one or more slices through `change.md` + `plan.yaml`.
- [Lifecycle](../lifecycle.md) -- slice state machine reference
- [Configuration Files](../configuration.md) -- project and slice metadata
