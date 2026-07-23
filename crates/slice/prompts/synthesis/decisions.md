# Decision Records

The synthesis response MAY carry optional `decisions[]`. Each entry becomes one slice-authored Decision Record — a durable, append-only statement of a design choice and its rejected alternatives. The persist tail writes `decisions/<slug>.md`; `specify slice merge` promotes accepted records to `.specify/decisions/DEC-NNNN-<slug>.md` and stamps engine-owned fields. Omit the key when the slice sets no durable decision — most slices set none.

## The bar for a durable record

Author one only when all three hold:

1. **A real choice was made** — at least one plausible alternative was considered and rejected.
2. **The choice outlives the slice** — it constrains future work (transport, id-scheme, error posture), not this slice's incidental implementation.
3. **Anchored in Evidence or explicit operator intent** — never infer from silence; otherwise keep it in `design.md`.

## Not the same as Evidence `kind: decision`

Evidence `kind: decision` claims are *inputs* that fold into `design.md` (see [`claim-reconciliation.md`](claim-reconciliation.md)). Emit a `decisions[]` entry only when the claim meets the durable bar above.

## Entry shape

Slice-authored fields only; the engine stamps `id` (`DEC-NNNN`), `slice`, `date`, and `superseded-by` at merge — never author those.

- `slug` — stable kebab-case; baseline filename derives from it.
- `status` — `accepted` or `rejected` (`superseded` is engine-only).
- `title` — short noun phrase naming the choice.
- `context` / `decision` / `consequences` — Nygard body sections (forces; active-voice choice; what becomes easier/harder).
- `supersedes` — optional baseline `DEC-NNNN` from `baseline-decisions[]` or another entry's `slug`. Never invent a missing `DEC-NNNN` (`decision-supersede-orphan`).
- `related` — optional `REQ-NNN` ids in declaration order (same convention as `tasks[].satisfies`).
- `topics` — optional kebab-case domain slugs for plan-time topic join.

## Re-refine semantics

`decisions[]` is the exact set: re-running refine replaces the slice's `decisions/` directory; omitting `decisions[]` clears it. Baseline records are never touched by refine — supersede them from a later slice.
