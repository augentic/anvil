# ADR-0001: State model — transactional store vs hardened event store

> Status: Proposed — re-scoped 2026-08-18 for the spec-generator programme ([ADR-0008](0008-spec-generator-programme.md)); pending operator acceptance of the narrowed decision
> Date: 2026-08-17

## Context

The journal is documented as observability and used as the database ([architecture-review.md](../architecture-review.md) S1): per-writer unlocked NDJSON, wall-clock union order, fail-open reads, env-var writer identity, no generation identity. The second pass ([addendum](../architecture-review-addendum.md)) shows the same class of defect throughout: authorization is a journal citation with a fabricable `sequence: 0` (S12), merge resume fail-opens and can permanently poison the accepted-CID chain (S13), deferrals and corrections are immortal and unscoped (S15, S16), the scheduler picks build records by mtime (S18), authoring predicates disagree (S19, S26), and survey/plan persist is a pipeline of uncommitted file writes (S41).

Emery is a single-node, single-operator CLI already guarded by a global lock.

[ADR-0008](0008-spec-generator-programme.md) narrows the live product to a specification generator. The state that exists in this programme is source bindings, extract receipts, and the spec set — not waves, epochs, merges, or claims. The original spike (store behind `plan status` and one merge commit) prices a system that is frozen.

## Options

- **A. One transactional state store per change home** (SQLite, or a single atomically swapped state document). The journal becomes pure observability and never gates lifecycle.
- **B. Hardened event store**: `ChangeGenerationId`, `AuthorizationToken`, content-addressed receipts, an atomically replaced receipt manifest, self-recovering leases, causal ordering, compaction.
- **C. Atomically swapped documents** (narrowed A): the spec generator writes bindings, receipts, and spec artifacts as documents replaced as a unit. No SQLite schema in this programme. A store for waves/merges is a build-programme question.

## Decision (proposed)

**Option C for this programme.** Fail-closed reads, one authority per question, no journal-as-authority, no fact-union reducers. Resume is re-run `emery specify`.

Option A remains the preferred answer for the deferred build programme; it is not a spike that gates the spec skeleton. Option B is rejected for the same reasons as the original proposal (more machinery than the single-operator CLI needs).

**What this does not dissolve:** missing *types and seams* for the generator — the claim family with extras (A8), extract receipts, generation identity on a re-mine. Those must still be designed. Wave-membership, `CorrectionTarget`, and executor stage design wait with the annex.

## Deletions

Fact-union reducers as lifecycle authority; `FactEpochRef`; the multi-writer claim protocol (S21); hand-deleted `guest.lock` recovery; per-writer journal as authority (retained as observability only, if an observability log exists at all in this programme); the planned receipt/manifest/lease machinery of Option B; the SQLite-or-merge spike as a gate on the spec skeleton. Concept-count effect: removes journal internals from the operator's mental model entirely.

## Consequences

The generator has no store schema to migrate. The build programme, if it chooses Option A, designs that schema against the annex — not by patching this decision.

## Spike (before acceptance)

**Not run as written.** The original 2–3 day spike (store behind `plan status` + merge commit, crash-injection) is a build-programme cost. Acceptance of Option C needs only: specify writes the spec set atomically; a crash mid-write leaves the previous set; re-run converges. That is an ordinary integration assertion on the walking skeleton, not a separate spike.

## Revisit trigger

A real requirement for multi-process writers on one change home (e.g. distributed execution activated by measured engagement pull per [platform.md](../platform.md) parked RFC-100). Independently: opening the build programme reopens Option A vs C for *that* programme's state (waves, accepted CIDs, workspaces).
