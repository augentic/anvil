# Candidate algorithm

Full per-source candidate algorithm for `/change:survey`. The SKILL.md carries the summary; this file carries the normative detail.

## Per-source walk

For each `legacy-code` source, survey walks the source's `surfaces.json` and `metadata.json` independently. v1 only descends into each source independently — no cross-source pairing.

### Decision 1: Size check

Compute the source's union-of-`touches` LOC by deduplicating all `surfaces[].touches` entries and summing their production LOC (excluding tests, generated code, vendored deps, blank lines, comment-only lines).

If the union LOC is `acceptable` (< 1000), emit a single source-level terminal candidate covering every surface and stop for that source. The candidate's `touches` is the deduplicated union; its `surfaces` list is every surface in the source; its `handler` is omitted (multiple handlers).

### Decision 2: Surface candidates

When the source as a whole is `too-large` (>= 1000 LOC), treat each surface as the default candidate. Size each surface's candidate by its own `touches` LOC (deduplicated within that surface).

### Decision 3: Minimal clustering

Merge same-source surface candidates only when ALL of the following conditions hold:

- One of the three clustering signals fires:
  1. **Shared `touches` overlap >= 50%** — computed as `|intersection| / |smaller set|` between two surfaces' `touches` lists. When the overlap is >= 50%, the two surfaces share enough implementation to warrant a single candidate.
  2. **Documentation grouping** — when `discovery.md`'s `## Candidate inventory` or `## Adapter inventory` explicitly groups surfaces under one candidate heading, that grouping is authoritative even if identifiers do not match mechanically.
  3. **Shared handler or call site** — multiple routes, topics, or jobs handled by the same function or class (matching `surfaces[].handler` values within the source).

- The combined candidate's LOC remains `acceptable` (< 1000). If merging would push the candidate over the threshold, do not merge.

### Decision 4: `too-large` post-cluster

Any candidate whose LOC >= 1000 after clustering (or any surface candidate that was already `too-large` and could not be merged) is emitted with `unresolved: true`. Survey exits 0 in that case; `propose` is responsible for refusing to draft a plan entry from an unresolved leaf until the operator resolves it.

## Sizing

### What counts as production LOC

Count non-blank, non-comment-only lines in source files. Exclude:

- Test files (directories named `test`, `tests`, `__tests__`, `spec`, `specs`; files matching `*.test.*`, `*.spec.*`, `*_test.*`, `*_spec.*`).
- Generated code (files matching `*.gen.*`, `*.generated.*`, `*.pb.*`, `*_pb.*`).
- Vendored dependencies (`node_modules/`, `vendor/`, `target/`, `.venv/`, `dist/`, `build/`).
- Type declarations (`*.d.ts`).
- Blank lines and comment-only lines.

### v1 simplification

v1 sizing uses a simple line count over touched files applying the same skip patterns. The CLI's `metadata.json` records total LOC but does not expose per-file LOC. Per-file LOC from `metadata.json` is a deferred refinement.

## Candidate naming

Candidate names are kebab-case, derived from the dominant surface identifier or handler path:

- Single-surface candidates: derive from the surface's `identifier` (e.g. `POST /users` → `user-registration`, `email.send` queue → `email-send`).
- Multi-surface candidates (after clustering): derive from the shared handler, shared directory, or the first surface's identifier with a grouping suffix.
- Source-level candidates (Decision 1): use the source-key as the candidate name.

Names must be unique within the change. When a derived name collides, append a numeric suffix (`-2`, `-3`).

## Ordering

Candidates are emitted in source order (alphabetical by source-key), then within a source by surface order (alphabetical by the first surface's `id`). Documentation-derived candidates are placed where the existing `propose` flow can review them.
