# Emery Product Definition

> Status: **Authoritative product yardstick.** Owned by the operator (human), not by any agent. Changes require an ADR under [decisions/](decisions/). Every architecture document, RFC, and corrective cut is audited against this file — including [platform.md](platform.md), which describes the services programme, not the product.
>
> This document is deliberately short. If the product cannot be described in two pages, it has too many concepts.

## What Emery is

Emery mines what a system must do — from legacy code, documentation, API contracts, operator intent (prompts), screenshots, and designs — into structured specifications that humans **and** agents can review quickly and reliably. Optionally, Emery takes those specifications and, slice by slice, creates or updates the application they specify: a backend service (Omnia) or a frontend app (Vectis).

It must be **extremely simple for an operator to use.**

## The operator journey

There is one journey. Everything else is an advanced or debug surface.

1. **Point Emery at sources.** Code, docs, contracts, intent, screenshots, designs. No hand-authored configuration files before the first useful output.
2. **Review the specifications.** One reviewable document per slice. A human (or a reviewing agent) can approve or reject it without opening a second artifact. Gaps and conflicts are visible in that document; conflicts require an operator disposition, unknowns may be carried as typed debt.
3. **Build, slice by slice.** Fast, reliable, resumable on failure — re-running the verb is always the recovery — and parallelisable once serial execution is proven crash-safe.
4. **Correct conversationally.** When a slice sticks, the operator says what to change; the guidance becomes a durable, digest-bound input to the retry. Conversation never gains lifecycle authority; the recorded guidance fact does.

## The verbs and the concept budget

Four operator verbs:

| Verb | Does |
| --- | --- |
| `spec` | Mine the declared sources into reviewable per-slice specifications |
| `build` | Deliver reviewed slices into the target application |
| `status` | Project the one next action |
| `fix` | Record correction guidance for a stuck slice or a wrong specification |

Everything else (adapter management, journal/state inspection, GC, debug survey/extract) lives behind an advanced namespace and does not appear in the primary help.

**Concept budget:** an operator must be productive knowing at most 4 verbs and at most 10 nouns (source, specification, slice, target, gap, conflict, correction, baseline — the exact list is closed here when the target architecture lands). Adding a verb or an operator-visible noun requires an ADR that names what is deleted to make room.

## The deliverable: a reviewable specification

The specification is the product, whether or not a build follows. Required properties:

- **One document per slice** that a reviewer reads top to bottom and approves or rejects.
- **Gaps inline.** `[unknown]` and `[conflict]` appear in the document, not in a separate verb's output.
- **Provenance on demand.** Which source said what is one gesture away, never required reading.
- **Diff-friendly.** A re-mined specification shows the reviewer what changed.
- **Agent-reviewable.** The same document is structured enough for a reviewing agent to gate on.

## Measured qualities

"Quick and reliable" are numbers, not adjectives. Targets below are **starting values to be confirmed by measurement** — an unmeasured number stays marked `unconfirmed`:

| Quality | Target (unconfirmed) | Measured by |
| --- | --- | --- |
| Time to first reviewable specification (bounded estate) | ≤ 30 minutes | graded eval telemetry |
| Time per built + merged slice | ≤ 60 minutes | graded eval telemetry |
| Per-operation success rate (survey, extract, synthesize, build, merge) | ≥ 95% | graded eval suite |
| Recovery | re-run the stopped verb, always; zero manual state surgery | crash-injection suite |

The graded eval suite gates release. A regression in these numbers blocks a release the same way a failing test does.

## Targets

First-party only: **Omnia** backend services and **Vectis** frontend apps, plus **contracts** (API contract authoring/validation). Sources: intent, documentation, code, contracts, screenshots, captures, designs.

## Foundational architecture commitments

Two requirements shape the platform beneath this product and are settled ([ADR-0002](decisions/0002-deployment.md)); they are invisible to the operator journey above but not negotiable beneath it:

- **Adapters are Wasm components added dynamically** — a new source or target reaches a running installation without rebuilding the host. A prior non-Wasm generation failed on exactly this.
- **One core runs as a desktop CLI and as a web service.** Deployment duality is a platform property, not two products.

The *distribution machinery* around components (registries, pull-on-miss, marketplaces) is a non-goal below; the component seam itself is not.

## Non-goals

These fail the product test and are not on any roadmap by default (evidence triggers can reopen them via ADR):

- A third-party adapter marketplace or component *distribution* platform (registries, pull-on-miss, bare-version resolution). Dynamic admission of a component at an exact version is foundational and stays; the distribution UX around it is not a product surface.
- Multi-node or distributed execution; hosted fleets; streaming execution.
- Unattended merge or autonomous accepted-state mutation.
- An SDLC-wide automation platform.
- A mandatory second lifecycle (architecture modelling as its own product) upstream of specification mining.
- Hand-authored configuration as a precondition for first output.

## See also

- [remediation-plan.md](remediation-plan.md) — how we get from the current implementation to this product
- [target-architecture.md](target-architecture.md) — the architecture that serves this definition
- [decisions/](decisions/) — the decision log this document's changes flow through
- [architecture-review.md](architecture-review.md) + [architecture-review-addendum.md](architecture-review-addendum.md) — why the current implementation misses this definition
