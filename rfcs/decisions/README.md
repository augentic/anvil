# Decision log (ADRs)

Architecture Decision Records for Emery. This directory is the single place product- and architecture-level decisions are recorded. It exists because the systemic failure documented in [architecture-review.md](../architecture-review.md) (findings R1–R4) was gradual: locally reasonable changes with no decision record accumulated into an incoherent whole, and at least one designed operator gate was deleted by a follow-up commit under lab pressure (finding P3 / R3).

## Rules

1. **ADR-or-it-didn't-happen.** Any change to an operator gate, authority rule, verb, operator-visible noun, artifact kind, or lifecycle stage requires an ADR in the same PR. Lab and eval needs get lab flags, never production policy changes.
2. **Deletions are mandatory.** Every ADR states what is deleted or simplified and its net effect on the operator-visible concept count ([product.md](../product.md) concept budget). "Nothing" is a legal answer only with justification.
3. **Revisit triggers, not relitigating.** Every ADR names the observable evidence that would reopen it. Disagreement later goes through the trigger, not through a fresh debate.
4. **Numbered, immutable once accepted.** Superseding is a new ADR that names the old one.

## Format

```markdown
# ADR-NNNN: Title
> Status: Proposed | Accepted | Superseded by ADR-MMMM
> Date: YYYY-MM-DD
## Context      — the problem and the evidence (cite review finding ids)
## Options      — considered alternatives, briefly
## Decision     — what is decided (or proposed)
## Deletions    — what this removes; concept-count effect
## Consequences — costs accepted
## Revisit trigger — the evidence that reopens this
```

## The decision gate

ADR-0001 through ADR-0006 are the **decision gate** from [architecture-review.md § Corrective programme](../architecture-review.md#corrective-programme): they precede corrective Cuts 1–5, which are re-derived from their outcomes. Cut 0 (containment) proceeds regardless.

| ADR | Decision | Spike required |
| --- | --- | --- |
| [0001](0001-state-model.md) | Transactional state store vs hardened event store | Yes |
| [0002](0002-deployment.md) | Native-only vs dual native/Wasm | Yes |
| [0003](0003-lifecycles.md) | One spec-mining loop vs definition + delivery | No |
| [0004](0004-conflict-disposition.md) | Operator gate for conflicts vs auto-defer | No |
| [0005](0005-change-home.md) | Detached-only change homes | No |
| [0006](0006-rebuild-vs-refactor.md) | Walking-skeleton rebuild vs six-cut refactor | After 0001/0002 spikes |
