# ADR-0010: The re-mine diff — computed at commit, emitted in the envelope

> Status: **Accepted** (remediation Phase 4, 2026-08-19). Implements remediation-plan
> Phase 4 item 2 ("a changed source shows the reviewer what changed") under the CC-05
> acceptance "Re-mining produces a meaningful diff" and product.md's "Diff-friendly"
> quality.
> Date: 2026-08-19

## Context

The output home is one generation behind one pointer (ADR-0001 Option C, ADR-0009 §2):
`emery specify` commits the incoming spec set and prunes everything the pointer no
longer names. That structure makes byte-stable re-runs and crash recovery cheap, but it
also destroys the outgoing generation at the very moment a reviewer would want to
compare against it — after a source changes, nothing tells the reviewer *what* changed
in the spec they are re-reviewing. ADR-0009 §2 additionally forbids persisted
timestamps and log lines in the output home, so the diff cannot become a retained
artifact without reopening that decision.

## Options

1. **Retain the outgoing generation and diff on demand** (a `diff` verb or flag).
   Reopens ADR-0009 §2's pruning, adds a verb/flag against ADR-0008 §3's frozen
   grammar, and makes the home's disk footprint grow with history — the state-model
   spike ADR-0001 deferred, smuggled in as a convenience.
2. **Persist a diff artifact into the generation.** The diff would name the *previous*
   generation, so identical sources would commit different bytes depending on history —
   breaking the content-addressed convergence that makes re-runs byte-stable.
3. **Compute the diff at commit time and emit it in the success envelope only.** The
   outgoing set is read before the swap-and-prune, compared with the incoming set in
   memory, and projected through the existing `specify` envelope (JSON field + human
   rendering). Nothing new persists; the grammar is untouched.

## Decision

Option 3. `emery specify` reports one **re-mine diff** in its success body:

- **When.** Present exactly when an outgoing generation existed before the commit; a
  first run has no diff. A byte-stable re-run reports an *empty* diff (same generation
  id), which keeps the journey assertion cheap and makes "nothing changed" an explicit,
  reviewable statement.
- **Shape.** `from` (the outgoing generation id), `artifacts` (the spec-set file names
  whose bytes changed, in the set's fixed on-disk order), and three section-level lists
  over `spec.md` requirement blocks keyed by heading subject: `added`, `removed`, and
  `changed` (a block whose status, tag, sources, or body differs). Subjects are the
  reconciliation join keys (dotted-kebab claim ids and gap descriptions), so the diff
  speaks the same vocabulary as the spec itself.
- **Where.** The diff kernel lives in `engine::home` next to the commit it observes;
  the envelope projection rides the existing `SpecifyBody` (`diff` is an optional
  field, omitted on first runs — additive on the wire per
  [cli-contract.md](../../docs/standards/cli-contract.md)).
- **Advisory, fail-open on the outgoing side only.** The commit is the authority; the
  diff is derived reporting. An outgoing set that cannot be read or no longer parses
  under the fail-closed AST (possible only across a binary upgrade, which pre-1.0
  means re-init) yields no diff rather than failing the run — the *incoming* set keeps
  its full fail-closed gates (AST, row check) unchanged.

## Deletions

Nothing is deleted; no operator-visible noun is added (the diff is a field of an
existing verb's output, not a verb, flag, or artifact). Concept-count effect: zero.

## Consequences

- The diff is ephemeral: scrollback or the JSON envelope is the only record, by
  design. A reviewer who wants it again re-runs `specify` against unchanged sources
  and reads the empty diff plus the committed set.
- Section-level granularity is the floor: the diff names changed requirement blocks,
  not intra-block word deltas. Finer rendering can layer on later without a wire
  change (the artifacts and subjects already identify what to open).
- Commit reads the outgoing generation once more per run — four small files; no
  measurable cost.

## Revisit trigger

A demonstrated reviewer need to compare against generations older than the immediately
superseded one (history, not adjacency), or a Propellerhead review where the ephemeral
envelope diff is shown to be insufficient for the CC-05 acceptance — either reopens
the retained-generations question as a successor to ADR-0001/ADR-0009 §2, not as a
patch here.
