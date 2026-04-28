# specify plan

Scaffold, populate, validate, transition, and archive initiative plans.

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`create`](#specify-plan-create) | Scaffold an empty `.specify/plan.yaml` at the start of an initiative (renamed from v1 `plan init`). |
| [`add`](#specify-plan-add) | Append a new entry to the plan in `pending` state (renamed from the v1 entry-append `plan create`). |
| [`amend`](#specify-plan-amend) | Edit non-status fields (`project`, `description`, `depends-on`, `sources`) on an existing entry. |
| [`transition`](#specify-plan-transition) | Move an entry through the status state machine (`pending` -> `in-progress` -> `done` / `failed` / `blocked`, plus `skipped`). |
| [`validate`](#specify-plan-validate) | Structural and referential integrity check (cycles, unknown deps, multi-repo invariants). |
| [`doctor`](#specify-plan-doctor) | First triage step when `/spec:execute` reports `stuck`. Strict superset of `validate` with cycle / orphan-source / stale-clone / unreachable-entry diagnostics. |
| [`next`](#specify-plan-next) | Report the next eligible entry (used by `/spec:execute` and ad-hoc operators). |
| [`status`](#specify-plan-status) | Render plan progress in topological order with per-status counts. |
| [`archive`](#specify-plan-archive) | Move a completed `plan.yaml` and `.specify/plans/<name>/` to `.specify/archive/plans/`. (Usually invoked by `specify initiative finalize` rather than directly.) |
| [`lock`](#specify-plan-lock) | Manage the advisory `.specify/plan.lock` PID stamp held by `/spec:execute`. |

## Subcommands

### specify plan create

Scaffold an empty plan.

```bash
specify plan create <name> [--source <key>=<path>...]
```

Creates `.specify/plan.yaml` with the given name and an empty `changes:` list. Optional `--source` entries are recorded in the plan's `sources:` section.

### specify plan validate

Check structural and referential integrity of the plan.

```bash
specify plan validate
```

Checks for: duplicate entry names, dependency cycles, unknown `depends-on` / `sources` references, at most one `in-progress` entry, and the following RFC-3b cross-registry checks when `registry.yaml` is present:

- `project-not-in-registry` (error) -- every `project` value must match a `projects[].name` in the registry.
- `project-missing-multi-repo` (error) -- when the registry has multiple projects, every change must carry a `project` field.
- `description-missing-multi-repo` (error) -- when the registry has multiple projects, every project must carry a `description`.
- `schema-mismatch-workspace` (warning) -- a workspace clone's `project.yaml` declares a different schema than the corresponding registry entry.

### specify plan doctor

Diagnose plan health (RFC-9 §4B). `doctor` is a strict superset of `validate`: it runs every check `validate` runs (preserving every diagnostic code listed above) and then layers four additional health diagnostics on top.

```bash
specify plan doctor
```

| Code | Severity | Meaning | Recovery |
|------|----------|---------|----------|
| `cycle-in-depends-on` | error | Dependency cycle in `depends-on`. `next_eligible` silently skips cycles at runtime; doctor is the only place where the cycle structure surfaces. Payload carries the cycle path, e.g. `["a", "b", "a"]`. | `specify plan amend <name> --depends-on …` to break the cycle, then re-run doctor. |
| `orphan-source-key` | warning | Top-level `sources:` key declared but no plan entry references it (the inverse of `unknown-source`). | Either reference the key from an entry's `sources:` list or remove the declaration. |
| `stale-workspace-clone` | warning | Workspace clone's signature has drifted from the registry, or no signature is readable at all. Reason is one of `signature-changed` (URL or schema diverged) or `missing-sync-stamp` (no stamp file and no readable git remote). | `specify workspace sync` to refresh the clone. |
| `unreachable-entry` | error | Pending entry whose dependency closure is rooted in a `failed`/`skipped` predecessor. Payload lists the immediate blocking predecessors and their statuses. | `specify plan transition <pred> pending` (after fixing the underlying issue) or `specify plan transition <entry> skipped --reason …` to drop the leaf. |

JSON output (`--format json`) wraps the rows under `diagnostics:` with a top-level `ok:` boolean (`false` whenever any error-severity diagnostic was emitted). Each row carries `severity`, `code`, `message`, optional `entry`, and an optional structured `data` payload (`kind` is one of `cycle` / `orphan-source` / `stale-clone` / `unreachable-entry`). Validate-level findings carry no `data` field; doctor-only diagnostics always do.

Exit code: 0 when no error-severity diagnostic fires (warnings are non-fatal — matches `validate`); `2` when any error-severity diagnostic fires.

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

### specify plan add

Append a new entry to the plan.

```bash
specify plan add <name> [--project <name>] [--description "<text>"] [--depends-on <entry>...] [--sources <key>...]
```

Creates the entry in `pending` state.

### specify plan amend

Edit non-status fields on an existing entry.

```bash
specify plan amend <name> [--project <name>] [--description "<text>"] [--depends-on <entry>...] [--sources <key>...]
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
