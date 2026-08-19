# ADR-0001: State model — transactional store vs hardened event store

> Status: **Accepted** (operator decision, 2026-08-19) — Option C as re-scoped 2026-08-18 for the spec-generator programme ([ADR-0008](0008-spec-generator-programme.md)), with the set-atomicity mechanism and observability channel fixed below.
> Date: 2026-08-17

## Context

The journal is documented as observability and used as the database ([architecture-review.md](../architecture-review.md) S1): per-writer unlocked NDJSON, wall-clock union order, fail-open reads, env-var writer identity, no generation identity. The second pass ([addendum](../architecture-review-addendum.md)) shows the same class of defect throughout: authorization is a journal citation with a fabricable `sequence: 0` (S12), merge resume fail-opens and can permanently poison the accepted-CID chain (S13), deferrals and corrections are immortal and unscoped (S15, S16), the scheduler picks build records by mtime (S18), authoring predicates disagree (S19, S26), and survey/plan persist is a pipeline of uncommitted file writes (S41).

Emery is a single-node, single-operator CLI already guarded by a global lock.

[ADR-0008](0008-spec-generator-programme.md) narrows the live product to a specification generator. The state that exists in this programme is source bindings, extract receipts, and the spec set — not waves, epochs, merges, or claims. The original spike (store behind `plan status` and one merge commit) prices a system that is frozen.

## Options

- **A. One transactional state store per change home** (SQLite, or a single atomically swapped state document). The journal becomes pure observability and never gates lifecycle.
- **B. Hardened event store**: `ChangeGenerationId`, `AuthorizationToken`, content-addressed receipts, an atomically replaced receipt manifest, self-recovering leases, causal ordering, compaction.
- **C. Atomically swapped documents** (narrowed A): the spec generator writes bindings, receipts, and spec artifacts as documents replaced as a unit. No SQLite schema in this programme. A store for waves/merges is a build-programme question.

## Decision

**Option C for this programme.** Fail-closed reads, one authority per question, no journal-as-authority, no fact-union reducers. Resume is re-run `emery specify`.

**Set atomicity is the generation-pointer pattern.** Rename is atomic per file, so per-file replacement of a live set does not satisfy this decision: a crash between renames leaves a mixed-generation set. Instead, each `specify` run writes the complete spec set (bindings, receipts, `spec.md` / `design.md`) into a fresh generation directory, then commits it with one atomic swap of a `current` pointer document (the temp-write / `sync_all` / rename envelope already in `crates/artifacts`). Readers trust only what the pointer names and fail closed on everything else — a missing or corrupt document is an error, never a defaulted value (the P15 lesson). The pattern is substrate-independent: filesystem rename implements the pointer swap for the single-writer CLI today; a store capability can implement it later. The new spine keeps every spec-set read and write behind one narrow module so that substitution stays one module wide.

**Observability is `wasi:otel`, and the journal is deleted rather than demoted.** The journal became the database because the engine could read it back (S1, S12, S13, S18); the OTel capability is emit-only from the guest, so journal-as-authority becomes unrepresentable at the seam instead of prohibited by prose. No log file exists in the output home. Telemetry never feeds grading or product behavior (the T6 lesson). Omnia is first-party, so unmet telemetry needs are met by extending `wasi:otel` in the platform — never by reintroducing a readable log.

Option A remains the preferred answer for the deferred build programme; it is not a spike that gates the spec skeleton. Option B is rejected for the same reasons as the original proposal (more machinery than the single-operator CLI needs).

**What this does not dissolve:** missing *types and seams* for the generator — the claim family with extras (A8), extract receipts, generation identity on a re-mine. Those must still be designed. Wave-membership, `CorrectionTarget`, and executor stage design wait with the annex.

## Deletions

Fact-union reducers as lifecycle authority; `FactEpochRef`; the multi-writer claim protocol (S21); hand-deleted `guest.lock` recovery; the per-writer journal in any form — observability moves to `wasi:otel` spans, and nothing in the output home is a log; the planned receipt/manifest/lease machinery of Option B; the SQLite-or-merge spike as a gate on the spec skeleton. Concept-count effect: removes the journal from the operator's mental model entirely.

## Consequences

The generator has no store schema to migrate. The build programme, if it chooses Option A, designs that schema against the annex — not by patching this decision.

## Spike

**Not run as written.** The original 2–3 day spike (store behind `plan status` + merge commit, crash-injection) is a build-programme cost. Option C is accepted with integration assertions in its place: `specify` commits the spec set behind the generation pointer; a crash mid-write leaves the previous set intact; re-run converges. Those land as journey-test assertions on the walking skeleton (an `adr_0001_*` gate tripwire per the anti-reversion strategy), not as a separate spike.

## Revisit trigger

A real requirement for multi-process writers on one change home (e.g. distributed execution activated by measured engagement pull per [platform.md](../platform.md) parked RFC-100). Independently: opening the build programme reopens Option A vs C for *that* programme's state (waves, accepted CIDs, workspaces). The standing candidate mechanism for Option A is this same generation-pointer pattern over `wasi:keyvalue`: generation-keyed documents committed by compare-and-swap on the pointer key (the `atomics` CAS interface), pending a persistent backend — recorded here so it is a decision input then, not relitigated from scratch.
