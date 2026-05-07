# Capability Resolution

Capability resolution is handled by the `specify` CLI. Skills should not re-implement the algorithm documented here; they call the CLI, which enforces the cache rules below identically for every caller.

- `specify capability resolve <capability-value> --format json` → returns the resolved directory path plus a `source` flag (`local` | `cached`).
- `specify capability pipeline <phase> [--change <dir>] --format json` → returns the brief topology for a phase plus absolute paths to every brief markdown file. The CLI resolves the capability internally; callers do not need to run `capability resolve` first.

## Resolution modes

Resolution mode is chosen from the shape of the `capability` value in `.specify/project.yaml`:

| Format              | Example                                          | Resolution                                                       |
|---------------------|--------------------------------------------------|------------------------------------------------------------------|
| Bare name           | `capability: omnia`                              | Cache `.specify/.cache/omnia/`, then local `capabilities/omnia/`. |
| URL (default ref)   | `capability: https://github.com/.../omnia`       | Cache `.specify/.cache/omnia/`.                                  |
| URL with pinned ref | `capability: https://github.com/.../omnia@v1`    | Cache `.specify/.cache/omnia/`.                                  |
| File URI            | `capability: file:///path/to/capabilities/omnia` | Cache `.specify/.cache/omnia/`.                                  |

Regular `specify init <capability>` is the boundary that fetches or copies capabilities into `.specify/.cache/`. After init, project-aware commands resolve the stored `capability` value from that cache and do not reach the network.

## Cache notes

- `.specify/.cache/` is gitignored. Regular `specify init <capability>` scaffolds and populates it.
- To force a refetch, delete `.specify/.cache/` and re-run `specify init <capability>`.
- `init` writes `.specify/.cache/.cache-meta.yaml` with the resolved capability identifier. The CLI invalidates the cache automatically when the capability value changes.
