# specify change plan

Populate, validate, transition, and archive change plans. The `plan` subresource lives under `specify change`; each verb on this page is invoked as `specify change plan <verb>`.

> Looking for plan **scaffolding**? Plan creation now lives on the umbrella verb — see [`specify change create`](change.md#specify-change-create), which writes `change.md` and `plan.yaml` together in a single atomic step (with the same `--source <key>=<path-or-url>` flag the old `plan create` carried).

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`add`](#specify-change-plan-add) | Append a new entry to the plan in `pending` state (renamed from the v1 entry-append `plan create`). |
| [`amend`](#specify-change-plan-amend) | Edit non-status fields (`project`, `description`, `depends-on`, `sources`) on an existing entry. |
| [`transition`](#specify-change-plan-transition) | Move an entry through the status state machine (`pending` -> `in-progress` -> `done` / `failed` / `blocked`, plus `skipped`). |
| [`validate`](#specify-change-plan-validate) | Structural and referential integrity check (cycles, unknown deps, multi-repo invariants) plus the four health diagnostics (`cycle-in-depends-on`, `orphan-source-key`, `stale-workspace-clone`, `unreachable-entry`). First triage step when `/change:execute` reports `stuck`. |
| [`next`](#specify-change-plan-next) | Report the next eligible entry (used by `/change:execute` and ad-hoc operators). |
| [`status`](#specify-change-plan-status) | Render plan progress in topological order with per-status counts. |
| [`archive`](#specify-change-plan-archive) | Move a completed `plan.yaml` and `.specify/plans/<name>/` to `.specify/archive/plans/`. (Usually invoked by `specify change finalize` rather than directly.) |
| [`lock`](#specify-change-plan-lock) | Manage the advisory `.specify/plan.lock` PID stamp held by `/change:execute`. |

## Subcommands

### specify change plan validate

Check structural and referential integrity of the plan, plus the four
health diagnostics that previously lived on `change plan doctor`.

```bash
specify change plan validate
```

Base shape checks: duplicate entry names, dependency cycles, unknown `depends-on` / `sources` references, at most one `in-progress` entry, and the following cross-registry checks when `registry.yaml` is present:

- `project-not-in-registry` (error) -- every `project` value must match a `projects[].name` in the registry.
- `project-missing-multi-repo` (error) -- when the registry has multiple projects, every change must carry a `project` field.
- `description-missing-multi-repo` (error) -- when the registry has multiple projects, every project must carry a `description`.
- `capability-mismatch-workspace` (warning) -- a workspace clone's `project.yaml` declares a different schema than the corresponding registry entry.

Health diagnostics layered on top — first triage step when `/change:execute` reports `stuck`:

| Code | Severity | Meaning | Recovery |
|------|----------|---------|----------|
| `cycle-in-depends-on` | error | Dependency cycle in `depends-on`. `next_eligible` silently skips cycles at runtime; validate is the only place where the cycle structure surfaces. Payload carries the cycle path, e.g. `["a", "b", "a"]`. | `specify change plan amend <name> --depends-on …` to break the cycle, then re-run validate. |
| `orphan-source-key` | warning | Top-level `sources:` key declared but no plan entry references it (the inverse of `unknown-source`). | Either reference the key from an entry's `sources:` list or remove the declaration. |
| `stale-workspace-clone` | warning | Workspace clone's signature has drifted from the registry, or no signature is readable at all. Reason is one of `signature-changed` (URL or capability diverged) or `missing-sync-stamp` (no stamp file and no readable git remote). | `specify workspace sync` to refresh the clone. |
| `unreachable-entry` | error | Pending entry whose dependency closure is rooted in a `failed`/`skipped` predecessor. Payload lists the immediate blocking predecessors and their statuses. | `specify change plan transition <pred> pending` (after fixing the underlying issue) or `specify change plan transition <entry> skipped --reason …` to drop the leaf. |

JSON output (`--format json`) wraps every finding under `results[]` with a top-level `passed` boolean (`false` whenever any error-severity row is present). Each row carries `level`, `code`, `message`, optional `entry`, and an optional structured `data` payload (`kind` is one of `cycle` / `orphan-source` / `stale-clone` / `unreachable-entry`). Base validate findings carry no `data` field; the four health diagnostics always do.

Exit code: `0` when no error-severity finding fires (warnings are non-fatal); `2` when any error-severity finding fires.

### specify change plan next

Report the next eligible plan entry.

```bash
specify change plan next
```

Returns the first `pending` entry whose `depends-on` entries are all `done`. Returns an error if no eligible entry exists.

With `--format json`, when an eligible entry is found the response includes `project` (string or null), `description` (string or null), and `sources` (array or null) alongside `next`. These fields are absent when `reason` is non-null (`all-done`, `stuck`, `in-progress`).

### specify change plan status

Render plan progress.

```bash
specify change plan status [--format json|table]
```

Shows entries in topological order with per-status counts, the active `in-progress` entry (if any), and any `status-reason` annotations.

### specify change plan add

Append a new entry to the plan.

```bash
specify change plan add <name> [--project <name>] [--description "<text>"] [--depends-on <entry>...] [--sources <key>...]
```

Creates the entry in `pending` state.

### specify change plan amend

Edit non-status fields on an existing entry.

```bash
specify change plan amend <name> [--project <name>] [--description "<text>"] [--depends-on <entry>...] [--sources <key>...]
```

### specify change plan transition

Move a plan entry through the status state machine.

```bash
specify change plan transition <name> <target> [--reason "<text>"]
```

| Target | Legal from |
|--------|-----------|
| `in-progress` | `pending` |
| `done` | `in-progress` |
| `failed` | `in-progress` |
| `blocked` | `in-progress` |
| `skipped` | `pending` |

At most one entry may be `in-progress` at a time.

### specify change plan archive

Archive a completed plan.

```bash
specify change plan archive
```

Moves `plan.yaml` and `.specify/plans/<name>/` to `.specify/archive/plans/<YYYYMMDD>-<name>/`.

### specify change plan lock

Manage the advisory plan lock.

```bash
specify change plan lock acquire --pid <pid>
specify change plan lock release --pid <pid>
specify change plan lock status
```

The lock (`.specify/plan.lock`) is a PID stamp held by `/change:execute` to prevent concurrent execution. The CLI `pid` option defaults to the current process PID; `/change:execute` passes a stable agent-session PID so `release` can authenticate the holder. `status` reports whether the lock is held and by which PID.

## See also

- [specify change](change.md) -- the umbrella verbs (`create`, `show`, `finalize`) that scaffold `change.md` + `plan.yaml` together and trigger close-out.
- [specify slice](slice.md) -- the per-slice CLI verbs the plan loop drives.
- [/change:plan](../change-skills/plan.md) -- skill that authors plans
- [/change:execute](../change-skills/execute.md) -- skill that drives plan execution
- [Configuration Files](../configuration.md) -- plan.yaml and registry format
