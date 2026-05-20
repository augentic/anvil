# Legacy Migration at Scale

> **This page is intentionally a stub.** It will be expanded once the source catalogue and migration ledger features land. The content below frames the problem and points to where the solutions will live.

## The scale problem

A single `/change:draft` cycle decomposes one set of legacy sources into slice-sized candidates, produces a plan, and hands it to the operator. Per-source decomposition runs through `/change:survey`: for each `legacy-code` input, the skill drives an LLM with a per-language enumeration brief to produce a candidate `surfaces.json`, and `specify change survey` validates that candidate against a closed schema, canonicalises field order, and writes the canonical sidecars deterministically. Producer (the brief plus the LLM) and validator (the CLI) sit on opposite sides of a sharp seam, which is what keeps the artifact contract enforceable while still covering several languages without growing the CLI binary.

That pattern works well for a monolith or a small fleet of repositories. But large-scale migrations involve many changes over many sources — potentially hundreds of repositories, dozens of plans, and months of execution. At that scale, single-change mechanics are necessary but not sufficient.

The missing primitives for cross-change scale include:

- **Durable source catalogues.** A registry of known legacy sources, their survey state, and which changes have consumed them — so a second change against the same source can reuse the canonical sidecars from the first survey instead of re-running enumeration.
- **Migration ledger.** A cumulative record of which surfaces have been migrated, which remain, and which are blocked — across all changes, not just the current one.
- **Cross-source pairing.** Automated matching of related surfaces across repositories (e.g. an outbound HTTP call in one repo and the corresponding route in another) to propose cross-source candidates that today require manual operator intervention during `propose`.
- **Dependency ordering across the fleet.** Inferring `depends-on` edges from contract edges so the operator does not have to manually sequence related candidates across sources.
- **Cross-change reconciliation.** Detecting when two in-flight changes touch the same legacy surface, or when a completed change invalidates an in-progress plan's assumptions.

## Deferred adapters

These items were considered for the v1 survey implementation and explicitly deferred. Each has a concrete re-open trigger documented in the design:

- Cross-source contract pairing (pub/sub, HTTP, WebSocket)
- Survey-inferred dependency ordering from contract edges
- Survey-emitted `target-project` and canonical-owner routing
- Operator-authored `identifier-aliases.yaml` and per-adapter alias bundles
- Durable source catalogues and cross-change source caches
- Migration ledger and cumulative surface-migration tracking
- Cross-change reconciliation against existing baselines

## Where the solutions will live

The source catalogue and migration ledger are planned as separate features. They will expand this page with the full explanation once their implementations land. See the [Decision log](decision-log.md) for the architectural rationale behind these deferrals.

## Today

For the v1 mechanics that are available now:

- [Monolith Decomposition](../tutorials/monolith-decomposition.md) — decomposing a single legacy source into slice-sized candidates.
- [Legacy Fleet Decomposition](../tutorials/legacy-fleet-decomposition.md) — multi-source changes with source-local candidates and operator review.
- [Legacy Migration at Scale (tutorial)](../tutorials/legacy-migration-at-scale.md) — the end-to-end migration workflow including execution and landing.
