# Emery Constitution

> Human-owned. Two pages, maximum, forever. This document carries the invariants that keep Emery one product; everything else (AGENTS.md, docs, prompts) must be derivable from the product surface and may be regenerated. If a rule cannot be reached from the product surface in three hops, the surface is wrong — fix the surface, not this file.
>
> Why this exists: the codebase was substantially agent-built, one RFC at a time, each locally coherent, with no force holding the composed product together ([rfcs/architecture-review.md](rfcs/architecture-review.md) findings R1–R4). Agents faithfully extend whatever exists; humans under lab pressure delete gates. These invariants are therefore enforced mechanically, not by prose.

## Invariants

1. **One product, one journey.** The full operator journey — sources in, reviewable specification out, slice built and merged — runs scripted and offline in CI at all times. It is the definition of done for every change. A change that breaks the journey does not land, whatever else it fixes.
2. **The surface is the spec.** The operator surface is 4 verbs and ≤10 nouns ([rfcs/product.md](rfcs/product.md)). Adding a verb or operator-visible noun requires an ADR naming a deletion. The conceptual model must be inferable from `emery --help` plus one reviewable document per slice.
3. **Authority is typed, singular, and fail-closed.** Exactly one authority answers each lifecycle question. Progress labels are computed, never stored. A read failure on an authority is an error, never an empty result. Every irreversible effect verifies its complete authorization immediately before acting.
4. **Delete before add.** Every RFC and ADR names its deletions and its net concept-count effect. A change that only adds is presumed wrong until justified.
5. **Policy changes are decisions.** Any change to an operator gate, authority rule, verb, artifact kind, or lifecycle stage requires an ADR in the same PR ([rfcs/decisions/](rfcs/decisions/)). Lab and eval convenience gets lab flags, never production policy. (This rule exists because it was violated once, expensively: review finding P3/R3.)
6. **Measured, not asserted.** "Fast" and "reliable" are the numbers in product.md. The graded eval suite gates release. An unmeasured number is written `unconfirmed`.
7. **Agents implement; humans decide.** Agent task briefs cite the target-architecture section they implement against. AGENTS.md is a map, never a spec. This file changes only by explicit human decision.

## Mechanical enforcement (fitness functions)

Each is a small CI check; together they convert gradual drift into individual red builds. Status: to be landed in remediation Phase 2 ([rfcs/remediation-plan.md](rfcs/remediation-plan.md)).

| Check | Enforces | Mechanism |
| --- | --- | --- |
| Journey test | Invariant 1 | Scripted offline journey over the mock catalog; every push |
| Route budget | Invariant 2 | Test enumerating the CLI router against the verb/noun list in product.md |
| LOC ratchet | Invariant 4 | Committed per-crate baseline (`scripts/ratchet.toml`); growth past ceiling fails unless the PR cites an ADR; shrink updates are free |
| Layering test | Invariant 3 | Assert the crate dependency DAG over `cargo metadata` |
| Seam-copy counter | Invariant 3 | Golden test asserting one DTO family for the adapter seam |
| Gate tripwires | Invariant 5 | One integration test per operator gate, named `adr_NNNN_*`; deleting a gate means deleting a test that names its decision record |
| Prose budgets | Invariant 2 | AGENTS.md and this file under a line ceiling; prompt-corpus lines in the ratchet |
| ADR-required paths | Invariant 5 | PRs touching gate/authority/verb/artifact paths must include a `rfcs/decisions/` file |

## Ritual

Monthly, 30 minutes: walk the acceptance list in [rfcs/architecture-review.md](rfcs/architecture-review.md) and the ratchet deltas. Record the scorecard as a dated note. No meeting, no slides — a diff.
