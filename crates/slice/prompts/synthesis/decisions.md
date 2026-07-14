# Decision Records

The synthesis response MAY carry an optional `decisions[]` alongside the four core artifacts. Each entry becomes one slice-authored Decision Record — a durable, append-only statement of a design choice and its rejected alternatives. The persist tail renders each entry to `decisions/<slug>.md` in the slice tree; `specify slice merge` promotes accepted records into the project's baseline catalogue at `.specify/decisions/DEC-NNNN-<slug>.md` and stamps the engine-owned fields. Omit the key entirely when the slice sets no durable decision — most slices set none.

## The bar for a durable record

A Decision Record survives the slice that authored it: it shapes how *future* slices are planned, reconciled, and reviewed. Author one only when all three hold:

1. **A real choice was made.** At least one plausible alternative was considered and rejected. A statement with no alternative is a requirement or a design note, not a decision.
2. **The choice outlives the slice.** It constrains future work across slices (a transport choice, an id-scheme, an error-handling posture) rather than describing how this slice happens to be implemented.
3. **The choice is anchored in Evidence or explicit operator intent.** Never infer a durable decision from silence; if the sources do not state the rationale, the choice belongs in `design.md` prose, not in a record.

## Not the same as Evidence `kind: decision`

Evidence claims with `kind: decision` are *inputs*: a source recorded that a decision was made somewhere. Those claims fold into `design.md` under the H2 they inform (see [`claim-reconciliation.md`](claim-reconciliation.md)) and do not automatically become Decision Records. Emit a `decisions[]` entry from a `decision` claim only when the claim meets the durable bar above — typically when a documentation source explicitly records the alternatives and rationale, or the operator's intent statement demands the posture be locked in.

## Entry shape

Each entry carries only the slice-authored fields; the engine stamps `id` (`DEC-NNNN`), `slice`, `date`, and any `superseded-by` at merge — never author those.

- `slug` — stable kebab-case identifier; the baseline filename derives from it.
- `status` — `accepted` (the decision is in force) or `rejected` (considered and explicitly not taken; recording a rejection is itself durable knowledge). `superseded` is engine-only.
- `title` — the record's H1, a short noun phrase naming the choice.
- `context` / `decision` / `consequences` — the Nygard body sections, one or more paragraphs each. `context` states the forces; `decision` states the choice in the active voice; `consequences` states what becomes easier and harder.
- `supersedes` — optional. Each target is either a baseline `DEC-NNNN` from the inputs envelope's `baseline-decisions[]` or the `slug` of another entry in this same response. Cite a target only when the new record genuinely replaces the old posture; a superseded target is flipped to `status: superseded` at merge and never edited otherwise. Never invent a `DEC-NNNN` that `baseline-decisions[]` does not carry — an unresolvable target aborts the merge with `decision-supersede-orphan`.
- `related` — optional `REQ-NNN` references for traceability into this slice's requirements. Cite the declaration-order ids the kernel will assign (the same convention as `tasks[].satisfies`).
- `topics` — optional kebab-case topic slugs naming the domains the decision governs; plan-time reconciliation joins them against surveyed lead topics.

## Re-refine semantics

The response's `decisions[]` is the exact set: re-running refine replaces the slice's `decisions/` directory with the latest response's entries, and an omitted `decisions[]` clears it. Records already promoted to the baseline are never touched by refine — supersede them from a later slice instead.
