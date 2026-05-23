# Adapter Resolution

Adapter resolution is handled by the `specify` CLI. Skills should not re-implement the algorithm documented here; they call the CLI, which enforces the cache rules below identically for every caller.

- `specify adapter resolve <adapter-value> --format json` → returns the resolved directory path plus a `source` flag (`local` | `cached`).
- `specify adapter pipeline <phase> [--change <dir>] --format json` → returns the brief topology for a phase plus absolute paths to every brief markdown file. The CLI resolves the adapter internally; callers do not need to run `adapter resolve` first.

## Resolution modes

Resolution mode is chosen from the shape of the `adapter` value in `.specify/project.yaml`:

| Format              | Example                                          | Resolution                                                       |
|---------------------|--------------------------------------------------|------------------------------------------------------------------|
| Bare name           | `adapter: omnia`                              | Cache `.specify/.cache/omnia/`, then project-local `schemas/omnia/`. |
| URL (default ref)   | `adapter: https://github.com/.../omnia`       | Cache `.specify/.cache/omnia/`.                                  |
| URL with pinned ref | `adapter: https://github.com/.../omnia@v1`    | Cache `.specify/.cache/omnia/`.                                  |
| File URI            | `target: file:///path/to/adapters/targets/omnia` | Cache `.specify/.cache/omnia/`.                                  |

Regular `specify init <adapter>` is the boundary that fetches or copies adapters into `.specify/.cache/`. After init, project-aware commands resolve the stored `adapter` value from that cache and do not reach the network.

## Cache notes

- `.specify/.cache/` is gitignored. Regular `specify init <adapter>` scaffolds and populates it.
- To force a refetch, delete `.specify/.cache/` and re-run `specify init <adapter>`.
- `init` writes `.specify/.cache/.cache-meta.yaml` with the resolved adapter identifier. The CLI invalidates the cache automatically when the adapter value changes.
