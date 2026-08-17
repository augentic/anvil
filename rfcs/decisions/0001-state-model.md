# ADR-0001: State model — transactional store vs hardened event store

> Status: Proposed — pending spike and operator acceptance
> Date: 2026-08-17

## Context

The journal is documented as observability and used as the database ([architecture-review.md](../architecture-review.md) S1): per-writer unlocked NDJSON, wall-clock union order, fail-open reads, env-var writer identity, no generation identity. The second pass ([addendum](../architecture-review-addendum.md)) shows the same class of defect throughout: authorization is a journal citation with a fabricable `sequence: 0` (S12), merge resume fail-opens and can permanently poison the accepted-CID chain (S13), deferrals and corrections are immortal and unscoped (S15, S16), the scheduler picks build records by mtime (S18), authoring predicates disagree (S19, S26), and survey/plan persist is a pipeline of uncommitted file writes (S41).

Emery is a single-node, single-operator CLI already guarded by a global lock.

## Options

- **A. One transactional state store per change home** (SQLite, or a single atomically swapped state document). The journal becomes pure observability and never gates lifecycle.
- **B. Hardened event store**: `ChangeGenerationId`, `AuthorizationToken`, content-addressed receipts, an atomically replaced receipt manifest, self-recovering leases, causal ordering, compaction.

## Decision (proposed)

**Option A.** It *dissolves* rather than fixes S2, S3, S6–S8, S10, S11, D9, D10 (standing review) and the reducer-class addendum findings, with less machinery than Option B adds. "Resumable on failure" is what transactions are; later parallelism is concurrent readers of one store, not a multi-writer fact-union protocol (S4 notes no CAS backend exists).

**What does not dissolve** (per the addendum's interaction table): missing *types and seams* — authoring generation identity (S26), `CorrectionTarget` (S25), `SurveyReceipt` / evidence manifest (P11, S42), the claim family (A8), wave-membership identity (S32), executor stage design (S33–S35, S38–S39). Those become store rows and preconditions, but they must still be designed.

## Deletions

Fact-union reducers as lifecycle authority; `FactEpochRef`; the multi-writer claim protocol (S21); hand-deleted `guest.lock` recovery; per-writer journal as authority (retained as observability only); the planned receipt/manifest/lease machinery of Option B. Concept-count effect: removes journal internals from the operator's mental model entirely.

## Consequences

A store schema to own and migrate (pre-1.0: hard resets, no migration framework). Audit history moves to the observability journal, which must be explicitly non-authoritative in code, not just comments.

## Spike (before acceptance)

Implement the store behind `plan status` (the widest projection) and one merge commit. Crash-inject at every write boundary; demonstrate re-run-the-verb recovery. Measure LOC delta against the reducers replaced. Budget: 2–3 days.

## Revisit trigger

A real requirement for multi-process writers on one change home (e.g. distributed execution activated by measured engagement pull per [platform.md](../platform.md) parked RFC-100).
