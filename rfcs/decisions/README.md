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

ADR-0001 through ADR-0006 are the **decision gate** from [architecture-review.md § Corrective programme](../architecture-review.md#corrective-programme). [ADR-0008](0008-spec-generator-programme.md) scopes the live programme to the specification generator and re-sequences the remaining spikes.

| ADR | Decision | Spike required |
| --- | --- | --- |
| [0001](0001-state-model.md) | **Proposed (re-scoped):** atomically swapped documents for the spec generator; SQLite/merge spike deferred | No (skeleton assertion, not a separate spike) |
| [0002](0002-deployment.md) | **Accepted: Wasm-primary** — one seam, one admission mode; native provider deleted | Extract-across-seam CI rung (ADR-0008); original survey+build spike withdrawn as a gate |
| [0003](0003-lifecycles.md) | **Accepted:** one spec-mining loop; `crates/system` archived | No |
| [0004](0004-conflict-disposition.md) | **Proposed (re-scoped):** conflicts inline in the spec; no build gate in this programme | No |
| [0005](0005-change-home.md) | **Proposed (re-scoped):** one output home for specs; in-place vs detached is a build-programme question | No |
| [0006](0006-rebuild-vs-refactor.md) | **Accepted (narrowed):** new spine for extract + synthesise; archive is tag `v1` | No |

Later ADRs record decisions made during the programme itself:

| ADR | Decision |
| --- | --- |
| [0007](0007-cache-seed-precedence.md) | **Accepted:** project cache seeds answer exact pins; pin digest verification is store-only |
| [0008](0008-spec-generator-programme.md) | **Accepted:** live product is the spec generator; survey collapses into extract; first artifacts `spec.md` / `design.md`; conservation is tag `v1` + worktree |
| [0009](0009-phase-3-surfaces.md) | **Accepted:** Phase 3 surfaces — source bindings at `init`, content-addressed generations behind one pointer, the closed required-extras table, and the scripted-model journey host |
| [0010](0010-remine-diff.md) | **Accepted:** the re-mine diff is computed at commit time and emitted in the `specify` success envelope — no new verb, no persisted artifact, no retained generations |
