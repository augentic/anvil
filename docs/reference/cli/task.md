# specify task

Task progress tracking and checkbox manipulation.

## Subcommands

### specify task progress

Report task completion progress for a change.

```bash
specify task progress <change-dir>
```

Returns the count of completed and total tasks, parsed from `tasks.md` checkbox syntax.

### specify task mark

Mark a task as complete.

```bash
specify task mark <change-dir> <task-id>
```

Flips the checkbox from `- [ ]` to `- [x]` for the specified task. The task ID is the numbered identifier (e.g. `1.2`, `2.1`).

Used by `/spec:build` as it completes each task.

## See also

- [/spec:build](../change-skills/build.md) -- skill that drives task execution
- [Artifact Format](../artifact-format.md) -- tasks.md checkbox format
