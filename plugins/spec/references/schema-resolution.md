# Schema Resolution

Schema resolution is handled by the `specify` CLI. Skills should not
re-implement the algorithm documented here; they call the CLI, which
enforces the local-first / cache / remote rules below identically for
every caller.

- `specify schema resolve <schema-value> --format json` → returns the
  resolved directory path plus a `source` flag (`local` | `cached`).
- `specify schema pipeline <phase> [--change <dir>] --format json` →
  returns the brief topology for a phase plus absolute paths to every
  brief markdown file. The CLI resolves the schema internally; callers
  do not need to run `schema resolve` first.

## Resolution modes

Resolution mode is chosen from the shape of the `schema` value in
`.specify/project.yaml`:

| Format              | Example                                      | Resolution                                       |
|---------------------|----------------------------------------------|--------------------------------------------------|
| Bare name           | `schema: omnia`                              | Local `schemas/omnia/`, then cache.              |
| URL (default ref)   | `schema: https://github.com/.../omnia`       | Cache, then remote fetch at `main`.              |
| URL with pinned ref | `schema: https://github.com/.../omnia@v1`    | Cache, then remote fetch at `v1`.                |

Bare names resolve locally first then fall back to the cache populated by
`init`; they never reach the network. URL values always route through the
cache or the agent-owned remote fetch (the CLI never fetches HTTP itself).

## Schema composition (`extends`)

A child schema can extend another via `extends: <schema-value>`. The CLI
resolves the parent recursively, then merges the child on top:

- `pipeline`: per-phase, child entries override the parent entry sharing
  the same `id`; new `id`s are appended.
- `domain`: child replaces parent when present; otherwise inherited.
- Other top-level fields (`name`, `version`, `description`): child wins.

## Cache notes

- `.specify/.cache/` is gitignored. `specify init` scaffolds the entry.
- To force a refetch, delete `.specify/.cache/` and re-run any skill that
  resolves the schema — the next call populates it again.
- `init` writes `.specify/.cache/.cache-meta.yaml` with the resolved
  `schema_url` (or `local:<name>` for bare-name resolution). The CLI
  invalidates the cache automatically when the schema value changes.
