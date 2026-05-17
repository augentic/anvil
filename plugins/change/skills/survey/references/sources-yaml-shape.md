# `--sources` YAML shape

Shape of the `--sources` file consumed by `specify change survey --sources <file> --staged <dir> --out <dir>`. `/change:survey` writes this file from the change's recorded `legacy-code` sources before invoking the CLI; the verb processes one row at a time and writes the canonical sidecars under `<out>/<source-key>/`.

## Envelope

```yaml
version: 1
sources:
  - key: <kebab-case>
    path: <relative-or-absolute-source-root>
```

- **`version`** — integer, must equal `1`. Bumps go through an RFC update.
- **`sources`** — non-empty list of `{ key, path }` rows.

Each row:

- **`key`** — kebab-case identifier (`^[a-z][a-z0-9-]*$`). Unique within the file; duplicates fail with `sources-file-malformed`.
- **`path`** — path to the source root. Relative paths resolve against the project root.

## Staged-input convention

For every row in `sources[]`, `/change:survey` writes the candidate `surfaces.json` to `<staged-dir>/<key>.json` **before** invoking the verb. The CLI looks for the staged candidate at exactly that path; a missing file exits `staged-input-missing`, malformed JSON exits `staged-input-malformed`.

The `--staged` flag is mandatory in batch form: `--sources` requires `--staged` (and vice versa). Both directories belong to the skill, not the operator — `/change:survey` controls the layout under `.specify/plans/<change>/survey/`.

## Worked example

`/change:survey` running a change with two `legacy-code` sources:

```text
.specify/plans/migrate-billing/survey/
├── sources.yaml
├── staged/
│   ├── legacy-monolith.json
│   └── legacy-billing.json
├── legacy-monolith/
│   ├── surfaces.json   # written by CLI
│   └── metadata.json   # written by CLI
└── legacy-billing/
    ├── surfaces.json   # written by CLI
    └── metadata.json   # written by CLI
```

`sources.yaml`:

```yaml
version: 1
sources:
  - key: legacy-monolith
    path: ./legacy/monolith
  - key: legacy-billing
    path: ./legacy/billing
```

Invocation:

```text
specify change survey \
  --sources .specify/plans/migrate-billing/survey/sources.yaml \
  --staged .specify/plans/migrate-billing/survey/staged \
  --out .specify/plans/migrate-billing/survey/
```

The verb writes each row's canonical `surfaces.json` and `metadata.json` atomically under `<out>/<key>/`. Rows that complete cleanly are left on disk if a later row fails, so re-runs only re-do the failed work.

## Validation surface

The CLI validates the file before processing any row. Failures exit with kebab-case discriminants:

- `sources-file-missing` — the path passed to `--sources` does not exist.
- `sources-file-malformed` — wrong version, empty `sources[]`, duplicate `key`, or YAML that does not parse.

These are skill bugs (the skill writes the file); the repair loop does not retry them. See [`repair-loop.md`](repair-loop.md) §"Fail-closed rule".
