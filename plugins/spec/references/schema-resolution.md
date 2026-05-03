# Schema Resolution

Schema resolution is handled by the `specify` CLI. Skills should not re-implement the algorithm documented here; they call the CLI, which enforces the cache rules below identically for every caller.

- `specify schema resolve <schema-value> --format json` → returns the resolved directory path plus a `source` flag (`local` | `cached`).
- `specify schema pipeline <phase> [--change <dir>] --format json` → returns the brief topology for a phase plus absolute paths to every brief markdown file. The CLI resolves the schema internally; callers do not need to run `schema resolve` first.

## Resolution modes

Resolution mode is chosen from the shape of the `schema` value in `.specify/project.yaml`:

| Format              | Example                                      | Resolution                                       |
|---------------------|----------------------------------------------|--------------------------------------------------|
| Bare name           | `schema: omnia`                              | Cache `.specify/.cache/omnia/`, then local `schemas/omnia/`. |
| URL (default ref)   | `schema: https://github.com/.../omnia`       | Cache `.specify/.cache/omnia/`.                  |
| URL with pinned ref | `schema: https://github.com/.../omnia@v1`    | Cache `.specify/.cache/omnia/`.                  |
| File URI            | `schema: file:///path/to/schemas/omnia`      | Cache `.specify/.cache/omnia/`.                  |

Regular `specify init --schema-uri <uri>` is the boundary that fetches or copies schemas into `.specify/.cache/`. After init, project-aware commands resolve the stored `schema` value from that cache and do not reach the network.

## Schema composition (`extends`)

A child schema can extend another via `extends: <schema-value>`. The CLI resolves the parent recursively, then merges the child on top:

- `pipeline`: per-phase, child entries override the parent entry sharing the same `id`; new `id`s are appended.
- `domain`: child replaces parent when present; otherwise inherited.
- Other top-level fields (`name`, `version`, `description`): child wins.

## Cache notes

- `.specify/.cache/` is gitignored. Regular `specify init --schema-uri` scaffolds and populates it.
- To force a refetch, delete `.specify/.cache/` and re-run `specify init --schema-uri <uri>`.
- `init` writes `.specify/.cache/.cache-meta.yaml` with the resolved schema URI. The CLI invalidates the cache automatically when the schema value changes.
