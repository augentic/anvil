# specify migrate

One-shot migrations for projects crossing Specify layout and naming cutovers.

## Synopsis

```bash
specify migrate v2-layout [--dry-run] [--format json]
specify migrate slice-layout [--dry-run] [--format json]
specify migrate change-noun [--dry-run] [--format json]
```

## Description

The v2 layout (specify-cli `0.2.0`) split Specify's on-disk shape along a clear boundary:

- **Operator artifacts** (`registry.yaml`, `plan.yaml`, `change.md`, `contracts/`) live at the repo root.
- **Framework state** (`project.yaml`, `slices/`, `specs/`, `archive/`, `.cache/`, `workspace/`, `plans/`, `plan.lock`) stays under `.specify/`.

`specify migrate v2-layout` walks the legacy platform artifact paths under `.specify/` and renames each one in place to its v2 destination at the repo root. It is the canonical recovery action when any other CLI verb refuses with `Error::LegacyLayout` (stable code `legacy-layout`, exit 1).

RFC-13 added two follow-on noun migrations:

- `specify migrate slice-layout` renames `.specify/changes/` to `.specify/slices/` and rewrites vendored skill references from `$CHANGE_DIR` to `$SLICE_DIR`.
- `specify migrate change-noun` renames root `initiative.md` to `change.md`.

Run the migrations in that order when upgrading an old project: `v2-layout`, then `slice-layout`, then `change-noun`.

Behaviour:

- **Idempotent.** Re-running on an already-migrated project exits 0 with `nothing to migrate` or the command-specific no-op message.
- **Refuses to clobber.** If both the legacy and current path exist, the verb errors with the colliding path and leaves both copies on disk so the operator can resolve manually.
- **Refuses unsafe active work.** `slice-layout` refuses when any legacy slice under `.specify/changes/` carries a non-terminal lifecycle status.
- **Refuses inside a workspace clone.** `v2-layout` does not touch peer clones under `.specify/workspace/<name>/`. Migrate the hub repo first, then iterate clones explicitly.
- **Atomic per move.** Each rename is independent. A partial failure leaves the project in a mixed state, with actionable output enumerating what moved and what did not.

## Options

| Option | Description |
|--------|-------------|
| `--dry-run` | Show what would move without writing anything. Combine with `--format json` for a machine-readable preview. |
| `--format` | Global output format: `text` (default) or `json` for structured automation output. |

## JSON output

For `v2-layout`, `--format json` returns:

- `moves` — array of `{ from, to, status }` rows, one per legacy path checked. `status` is one of:
  - `moved` — source moved to destination.
  - `would-move` — dry-run only; would have moved.
  - `absent-source` — no v1 artifact at this path; nothing to do.
  - `destination-exists` — refused; both legacy and destination present.
- `any-legacy-present` — `true` when at least one legacy artifact was found.
- `any-collisions` — `true` when at least one destination collision blocked a move.
- `dry-run` — present and `true` only when `--dry-run` was passed.

`slice-layout` and `change-noun` return command-specific JSON rows that report the attempted source, destination, status, and dry-run flag. Exit code is `0` when the migration completed or had nothing to do, and `1` when a collision or unsafe active slice blocked the migration.

## Worked example

A v1-layout single-repo project starts out like:

```text
my-project/
├── src/
└── .specify/
    ├── project.yaml
    ├── registry.yaml
    ├── plan.yaml
    ├── initiative.md
    └── contracts/
        └── http/user-api.yaml
```

After `specify migrate v2-layout`, the operator artifacts are at the root, but legacy RFC-13 names may still be present:

```text
my-project/
├── src/
├── registry.yaml
├── plan.yaml
├── initiative.md
├── contracts/
│   └── http/user-api.yaml
└── .specify/
    ├── project.yaml
    └── changes/
```

After the follow-on migrations:

```text
my-project/
├── src/
├── registry.yaml
├── plan.yaml
├── change.md
├── contracts/
│   └── http/user-api.yaml
└── .specify/
    ├── project.yaml
    └── slices/
```

`project.yaml` stays under `.specify/` (it is framework configuration, not operator content). The `specs/`, `archive/`, `.cache/`, `workspace/`, `plans/`, and `plan.lock` paths under `.specify/` are likewise untouched.

## See also

- [Migrating to the v2 layout](../../how-to/migrate-to-v2-layout.md) — operator-facing walkthrough.
- [Directory Layout](../directory-layout.md) — the v2 shape in full.
- [Decision Log: Platform artifacts at the repo root](../../explanation/decision-log.md) — why the boundary was drawn this way.
