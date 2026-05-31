# specrun plan

Scaffold, populate, validate, transition, and archive change plans. The `plan` verb is the top-level home of every `plan.yaml` operation; each verb on this page is invoked as `specrun plan <verb>`.

## Verb cheat-sheet

| Verb | When to use |
|------|-------------|
| [`create`](#specify-plan-create) | Scaffold an empty `plan.yaml` at the repo root. Refuses to overwrite an existing plan. |
| [`add`](#specify-plan-add) | Append a new entry to the plan in `pending` state (renamed from the v1 entry-append `plan create`). |
| [`amend`](#specify-plan-amend) | Edit non-status fields (`project`, `description`, `depends-on`, `sources`) on an existing entry. |
| [`remove`](#specify-plan-remove) | Drop a pending entry while the plan is still replaceable (Gate 1 deferral). |
| [`transition`](#specify-plan-transition) | Stamp Gate 1 (`specrun plan transition <plan-name> approved`) or close a merged entry (`specrun plan transition <entry-name> done`). Per-entry status is `pending | in-progress | done` only. |
| [`validate`](#specify-plan-validate) | Structural and referential integrity check (cycles, unknown deps, multi-repo invariants) plus three health diagnostics (`cycle-in-depends-on`, `orphan-source-key`, `stale-workspace-clone`). First triage step when `/spec:execute` reports `stuck`. |
| [`next`](#specify-plan-next) | Report the next eligible entry (used by `/spec:execute` and ad-hoc operators). |
| [`archive`](#specify-plan-archive) | Move a completed `plan.yaml` and `.specify/plans/<name>/` to `.specify/archive/plans/`. Usually invoked by `/spec:finalize` after it observes merged PRs. |

## Subcommands

### specrun plan create

Scaffold an empty plan.

```bash
specrun plan create <name> [--source <key>=<adapter>:<path>...] [--source <key>=<adapter>:value:<literal>...]
```

Writes `plan.yaml` at the repo root with the given kebab-case name and an empty `slices:` list. Each optional `--source` carries the structured binding shape: an explicit kebab-case `<adapter>` followed by a colon and either a path (`<adapter>:<path>` — URLs containing `:` like `git@github.com:org/foo.git` round-trip cleanly because only the first colon is significant) or a `value:`-prefixed literal (`<adapter>:value:<literal>` — used by `intent`). Refuses with `already-exists` when `plan.yaml` is already present.

### specrun plan validate

Check structural and referential integrity of the plan, plus the four
health diagnostics that previously lived on `change plan doctor`.

```bash
specrun plan validate
```

Base shape checks: duplicate entry names, dependency cycles, unknown `depends-on` / `sources` references, at most one `in-progress` entry, and the following cross-registry checks when `registry.yaml` is present:

- `project-not-in-registry` (error) -- every `project` value must match a `projects[].name` in the registry.
- `project-missing-multi-repo` (error) -- when the registry has multiple projects, every change must carry a `project` field.
- `description-missing-multi-repo` (error) -- when the registry has multiple projects, every project must carry a `description`.
- `adapter-mismatch-workspace` (warning) -- a workspace clone's `project.yaml` declares a different schema than the corresponding registry entry.

Health diagnostics layered on top — first triage step when `/spec:execute` reports `stuck`:

| Code | Severity | Meaning | Recovery |
|------|----------|---------|----------|
| `cycle-in-depends-on` | error | Dependency cycle in `depends-on`. `next_eligible` silently skips cycles at runtime; validate is the only place where the cycle structure surfaces. Payload carries the cycle path, e.g. `["a", "b", "a"]`. | `specrun plan amend <name> --depends-on …` to break the cycle, then re-run validate. |
| `orphan-source-key` | warning | Top-level `sources:` key declared but no plan entry references it (the inverse of `unknown-source`). | Either reference the key from an entry's `sources:` list or remove the declaration. |
| `stale-workspace-clone` | warning | Workspace clone's signature has drifted from the registry, or no signature is readable at all. Reason is one of `signature-changed` (URL or adapter diverged) or `slot-mismatch` (slot materialisation does not match the registry). | `specrun workspace sync` to refresh the clone. |

JSON output (`--format json`) wraps every finding under `results[]` with a top-level `passed` boolean (`false` whenever any error-severity row is present). Each row carries `level`, `code`, `message`, optional `entry`, and an optional structured `data` payload (`kind` is one of `cycle` / `orphan-source` / `stale-clone`). Base validate findings carry no `data` field; the three health diagnostics always do.

Exit code: `0` when no error-severity finding fires (warnings are non-fatal); `2` when any error-severity finding fires.

### specrun plan next

Report the next eligible plan entry.

```bash
specrun plan next
```

Returns the first `pending` entry whose `depends-on` entries are all `done`. Returns an error if no eligible entry exists.

With `--format json`, when an eligible entry is found the response includes `project` (string or null), `description` (string or null), and `sources` (array or null) alongside `next`. These fields are absent when `reason` is non-null (`all-done`, `stuck`, `in-progress`).

### specrun plan add

Append a new entry to the plan.

```bash
specrun plan add <name> [--project <name>] [--description "<text>"] [--depends-on <entry>...] [--sources <key>...]
```

Creates the entry in `pending` state.

### specrun plan amend

Edit non-status fields on an existing entry.

```bash
specrun plan amend <name> [--project <name>] [--description "<text>"] [--depends-on <entry>...] [--sources <key>...]
```

### specrun plan remove

Drop a pending plan entry while the plan is still replaceable (`lifecycle: pending` and every entry `pending`). Gate 1 only — defers the entry's lead(s) without re-surveying `discovery.md`.

```bash
specrun plan remove <entry>
```

Refuses with `plan-remove-plan-not-replaceable` when the plan is approved or any entry is non-pending. Refuses with `plan-remove-entry-referenced` when another entry lists `<entry>` in `depends-on`.

### specrun plan transition

Stamp Gate 1 or close a merged plan entry.

```bash
specrun plan transition <name> <target> [--reason "<text>"]
```

| Target | Applies to | Meaning |
|--------|------------|---------|
| `approved` | `<plan-name>` (matches `plan.yaml` `name`) | Gate 1 — operator-only stamp after `/spec:plan`. |
| `done` | `<entry-name>` (a `slices[]` row) | Close the entry after `/spec:merge` folded the slice. |

Per-entry `pending` is written by `specrun plan add` / `plan amend`; `in-progress` is written only by `specrun plan next`. v1 has no per-entry `failed`, `blocked`, or `skipped` — build failures and merge conflicts leave the active entry `in-progress`.

At most one entry may be `in-progress` at a time.

### specrun plan archive

Archive a completed plan.

```bash
specrun plan archive
```

Moves `plan.yaml` and `.specify/plans/<name>/` to `.specify/archive/plans/<YYYYMMDD>-<name>/`.

## See also

- [specrun slice](slice.md) -- the per-slice CLI verbs the plan loop drives.
- [/spec:plan](../change-skills/plan.md) -- skill that authors plans
- [/spec:execute](../change-skills/execute.md) -- skill that drives plan execution
- [/spec:finalize](../change-skills/finalize.md) -- skill that closes out a completed change
- [Configuration Files](../configuration.md) -- plan.yaml and registry format
