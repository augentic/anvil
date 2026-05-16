# `scaffold-example`

Illustrative fixture pinning the on-disk shapes emitted by `/change:analyze`. This fixture is **structural**, not a test target — it exists so reviewers and downstream briefs can eyeball the exact shapes without squinting at prose.

The real documentation extraction fixtures land with the per-capability brief fixtures.

## Contents

- [`inputs/README.md`](inputs/README.md) — describes the (tiny) hypothetical documentation input the expected output summarises.
- [`expected/discovery.md`](expected/discovery.md) — the expected `$DISCOVERY` after a single `/change:analyze` invocation against `inputs/`, showing the unified fenced-YAML candidate block shape under the pre-existing `## Candidate inventory` heading.

The expected `discovery.md` matches the shape pinned by [`../../SKILL.md` §*Output contract*](../../SKILL.md) — each candidate as a `### <name>` heading followed by a fenced YAML block with fields in fixed order (`kind`, `sources`, `handler`, `touches`, `surfaces`, `declared-at`, `unresolved`). Doc-derived blocks omit `handler` and `touches` when no hint applies.

No `metadata.json` sidecar is produced — the documentation branch does not write structural metadata.
