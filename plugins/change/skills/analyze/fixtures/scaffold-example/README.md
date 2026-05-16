# `scaffold-example`

Illustrative fixture pinning the on-disk shapes emitted by `/change:analyze`. This fixture is **structural**, not a test target — it exists so reviewers and downstream briefs can eyeball the exact shapes without squinting at prose.

The real per-kind fixtures (with actual clustering / extraction over a realistic monolith or runbook) land with:

- RFC-3a C18 — Omnia documentation branch brief.
- RFC-3a C21 — Omnia code branch brief.
- RFC-3a C22 — monolith fixture + expected capability inventory.

## Contents

- [`inputs/README.md`](inputs/README.md) — describes the (tiny) hypothetical monolith the expected output summarises.
- [`expected/discovery.md`](expected/discovery.md) — the expected `$DISCOVERY` after a single `/change:analyze legacy-code monolith` invocation against `inputs/`.
- [`expected/plans/scaffold-example/analyze/monolith/metadata.json`](expected/plans/scaffold-example/analyze/monolith/metadata.json) — the structural-metadata sidecar written alongside `$DISCOVERY` by the same invocation. Populated per [`../../SKILL.md` §*Structural metadata*](../../SKILL.md). `scaffold-example` is the stand-in change name for this fixture; in a real run the segment is the actual `<change-name>` under `.specify/plans/<change-name>/`.

The expected `discovery.md` matches the shape pinned by [`../../SKILL.md` §*Output contract*](../../SKILL.md) — one `### <name>` heading per capability, followed by a fenced YAML block with fields in fixed order.

The expected `metadata.json` matches the shape pinned by [`../../SKILL.md` §*Structural metadata*](../../SKILL.md): required fields in the documented order, `top_level_modules` sorted alphabetically, no timestamps or host state. The documentation branch does not write this sidecar; only `--kind legacy-code` runs produce it.
