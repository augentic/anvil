# Decision Records

The staged bundle MAY carry optional `decisions/<slug>.md` files. Each is one slice-authored Decision Record — a durable, append-only statement of a design choice and its rejected alternatives. The merge phase promotes accepted records to `.emery/decisions/DEC-NNNN-<slug>.md` and stamps engine-owned fields. Write none when the slice sets no durable decision — most slices set none.

## The bar for a durable record

Author one only when all three hold:

1. **A real choice was made** — at least one plausible alternative was considered and rejected.
2. **The choice outlives the slice** — it constrains future work (transport, id-scheme, error posture), not this slice's incidental implementation.
3. **Anchored in Evidence or explicit operator intent** — never infer from silence; otherwise keep it in `design.md`.

## Not the same as Evidence `kind: decision`

Evidence `kind: decision` claims are *inputs* that fold into `design.md` (see [`claim-reconciliation.md`](claim-reconciliation.md)). Write a `decisions/<slug>.md` only when the claim meets the durable bar above.

## File shape

YAML front-matter with slice-authored fields only, then the Nygard body (`# <title>`, `## Context`, `## Decision`, `## Consequences`). The engine stamps `id` (`DEC-NNNN`), `slice`, `date`, and `superseded-by` at merge — never author those.

- `slug` — stable kebab-case matching the filename; baseline filename derives from it.
- `status` — `accepted` or `rejected` (`superseded` is engine-only).
- `supersedes` — optional baseline `DEC-NNNN` from `baseline-decisions[]` or a sibling record's `slug`. Never invent a missing `DEC-NNNN` (`decision-supersede-orphan`).
- `related` — optional `REQ-NNN` ids in declaration order (same convention as `tasks[].satisfies`).
- `topics` — optional kebab-case domain slugs for plan-time topic join.

## Re-refine semantics

The staged `decisions/` directory is the exact set: delete a seeded record you no longer author; the persisted slice mirrors the stage exactly. Baseline records are never touched by refine — supersede them from a later slice.
