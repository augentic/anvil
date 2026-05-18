# Capability Resolution

Capability resolution is handled by the `specify` CLI. Skills should not re-implement the algorithm documented here; they call the CLI, which enforces the cache rules below identically for every caller.

- `specify capability resolve <capability-value> --format json` → returns the resolved directory path plus a `source` flag (`local` | `cached`).
- `specify capability pipeline <phase> [--change <dir>] --format json` → returns the brief topology for a phase plus absolute paths to every brief markdown file. The CLI resolves the capability internally; callers do not need to run `capability resolve` first.
- `specify codex export --format json` → returns the project-resolved codex in source order, including provenance for default, project capability, catalog, and repo overlay rules.

## Resolution modes

Resolution mode is chosen from the shape of the `capability` value in `.specify/project.yaml`:

| Format              | Example                                          | Resolution                                                       |
|---------------------|--------------------------------------------------|------------------------------------------------------------------|
| Bare name           | `capability: omnia`                              | Cache `.specify/.cache/omnia/`, then project-local `schemas/omnia/`. |
| URL (default ref)   | `capability: https://github.com/.../omnia`       | Cache `.specify/.cache/omnia/`.                                  |
| URL with pinned ref | `capability: https://github.com/.../omnia@v1`    | Cache `.specify/.cache/omnia/`.                                  |
| File URI            | `capability: file:///path/to/capabilities/omnia` | Cache `.specify/.cache/omnia/`.                                  |

Regular `specify init <capability>` is the boundary that fetches or copies capabilities into `.specify/.cache/`. After init, project-aware commands resolve the stored `capability` value from that cache and do not reach the network.

## Codex resolution

Codex rules are resolved by convention from `codex/**/*.md` under each active source. The CLI loads sources in deterministic order: foundational `default` capability first, project capability second, future shared catalogs third, and repo-root `codex/` overlay last. Duplicate rule ids across those sources fail with validation semantics.

First-party `default` rules are distributed as a normal capability at `capabilities/default`. When `specify init <capability>` copies a capability from a tree that also contains sibling `default`, it also copies that sibling into `.specify/.cache/default/`. Later `specify codex *` commands resolve `default` from cache first, just like project capabilities.

## Cache notes

- `.specify/.cache/` is gitignored. Regular `specify init <capability>` scaffolds and populates it.
- To force a refetch, delete `.specify/.cache/` and re-run `specify init <capability>`.
- `init` writes `.specify/.cache/.cache-meta.yaml` with the resolved capability identifier. The CLI invalidates the cache automatically when the capability value changes.
