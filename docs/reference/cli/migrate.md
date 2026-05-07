# specify migrate

One-shot layout migrations. Today the verb exposes a single subcommand, `v2-layout`, that moves the operator-facing platform artifacts from the legacy v1 location under `.specify/` to the repo root.

## Synopsis

```bash
specify migrate v2-layout [--dry-run] [--format json]
```

## Description

The v2 layout (specify-cli `0.2.0`) split Specify's on-disk shape along a clear boundary:

- **Operator artifacts** (`registry.yaml`, `plan.yaml`, `change.md`, `contracts/`) live at the repo root.
- **Framework state** (`project.yaml`, `changes/`, `specs/`, `archive/`, `.cache/`, `workspace/`, `plans/`, `plan.lock`) stays under `.specify/`.

`specify migrate v2-layout` walks the four legacy paths under `.specify/` and renames each one in place to its v2 destination at the repo root. It is the canonical recovery action when any other CLI verb refuses with `Error::LegacyLayout` (stable code `legacy-layout`, exit 1).

Behaviour:

- **Idempotent.** Re-running on an already-migrated project exits 0 with `nothing to migrate`.
- **Refuses to clobber.** If both the legacy and the v2 path exist (e.g. the operator hand-created a root `registry.yaml` before running the migrate), the verb errors with the colliding path and leaves both copies on disk so the operator can resolve manually.
- **Refuses inside a workspace clone.** The verb does not touch peer clones under `.specify/workspace/<name>/`. Migrate the hub repo first, then iterate clones explicitly (the hub's `specify workspace sync` will refresh them once they're upgraded).
- **Atomic per file.** Each `fs::rename` is independent — a partial failure leaves the project in a mixed state, with an actionable JSON output enumerating what moved and what didn't.

## Options

| Option | Description |
|--------|-------------|
| `--dry-run` | Show what would move without writing anything. Combine with `--format json` for a machine-readable preview. |
| `--format` | Global output format: `text` (default) or `json` for structured automation output. |

## JSON output

When `--format json` is provided, returns:

- `moves` — array of `{ from, to, status }` rows, one per legacy path checked. `status` is one of:
  - `moved` — source moved to destination.
  - `would-move` — dry-run only; would have moved.
  - `absent-source` — no v1 artifact at this path; nothing to do.
  - `destination-exists` — refused; both legacy and destination present.
- `any-legacy-present` — `true` when at least one legacy artifact was found.
- `any-collisions` — `true` when at least one destination collision blocked a move.
- `dry-run` — present and `true` only when `--dry-run` was passed.

Exit code: `0` when every present source moved (or there was nothing to migrate), `1` when at least one collision blocked a move.

## Worked example

A v1-layout single-repo project starts out like:

```text
my-project/
├── src/
└── .specify/
    ├── project.yaml
    ├── registry.yaml
    ├── plan.yaml
    ├── change.md
    └── contracts/
        └── http/user-api.yaml
```

After `specify migrate v2-layout`:

```text
my-project/
├── src/
├── registry.yaml
├── plan.yaml
├── change.md
├── contracts/
│   └── http/user-api.yaml
└── .specify/
    └── project.yaml
```

`project.yaml` stays under `.specify/` (it is framework configuration, not operator content). The `changes/`, `specs/`, `archive/`, `.cache/`, `workspace/`, `plans/`, and `plan.lock` paths under `.specify/` are likewise untouched.

## See also

- [Migrating to the v2 layout](../../how-to/migrate-to-v2-layout.md) — operator-facing walkthrough.
- [Directory Layout](../directory-layout.md) — the v2 shape in full.
- [Decision Log: Platform artifacts at the repo root](../../explanation/decision-log.md) — why the boundary was drawn this way.
