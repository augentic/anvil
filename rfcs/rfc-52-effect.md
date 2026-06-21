# RFC-52: Effect Interfaces

> Status: Reviewed · Implements: Effect-oriented architecture · Depends: RFC-51 · Sequences into: RFC-53

## Abstract

This RFC defines explicit, typed WIT interfaces for the fixed vocabulary of effects the runtime currently performs implicitly: `eval` (to run a brief or prompt) and the host-data accessors (which also serve the `references` resolve fallback, pull-by-id). Note that `kv` (host-held state) and lifecycle hooks (`journal` / `transition`) are provided by the Omnia runtime as `KeyValue` and `JsonDb` WIT worlds, respectively. It changes no runtime behavior—each named effect is initially backed by existing machinery. The goal is to make the implicit boundaries explicit, typed, and mockable, unlocking deterministic record/replay.

## Motivation

Currently, LLM inference is an out-of-band convention (the CLI prints an envelope, the agent reads a brief) with no typed contract. Defining it as an effect interface:

- Creates a single, typed surface for context-injection, recording, and rate-limiting.
- Enables a **replay stub** to satisfy `eval`, making runs deterministic in CI.
- Provides a stable import for future orchestration components to build against.

## Scope

**In scope:**
- WIT interface definitions for `eval` (in [`../wit/model.wit`](../wit/model.wit)) and `host-config` / `host-data` (in [`../wit/specify.wit`](../wit/specify.wit)); the `references` resolve fallback is served by `host-data` (`read` by id), not a standalone interface.
- Host-side handlers that satisfy these interfaces using existing machinery.
- A record/replay backend for `eval` sufficient for CI.
- Typing the agent handoff by projecting request/report records into handoff envelopes, replacing hand-maintained JSON schema constants.

**Out of scope:**
- Orchestration components (execution remains a two-phase handoff).
- Async ABI commitments (e.g., streaming or cancellation).
- Brief frontmatter expansion.

## The model

The authoritative WIT interfaces are complete and defined in [`../wit/model.wit`](../wit/model.wit) (the `eval` effect) and [`../wit/specify.wit`](../wit/specify.wit) (the `host-config` / `host-data` accessors and the per-axis worlds).

The host satisfies `eval` with the existing two-phase handoff; the typed interface is the contract, while the handoff is the temporary implementation. Passing `path` instead of the brief body prevents context-budget blowup. Filesystem-capable backends resolve links themselves, while others use the `references` fallback.

**Host-data narrows the blast radius.** The `host-config` and `host-data` accessors restrict adapters to exact capabilities, replacing broad preopened-directory grants. They govern input artifacts and assets, but explicitly do **not** handle an adapter reading its own bundled prose (handled via relative paths or `references`).

**Typing the agent handoff.** Naming the `eval` seam types the live handoff envelope. The host serializes the structured build request into the brief handoff and validates the report against the WIT-derived type, allowing us to remove duplicate JSON schema constants and parity tests.

## Decisions (reviewed)

- **The `eval` signature (resolved).** `eval-request` is a variant of `prompt(string)` | `path(string)`, and `eval: func(request: eval-request) -> result<string, error>`. The structured request record is not embedded in `eval`; the host projects it into the brief handoff envelope (see "Typing the agent handoff") while `eval` carries only the `path` handle (or a direct `prompt`). Context-injection policy is deferred and is not part of the synchronous v1 signature.
- **Model-service routing key (resolved).** Out of scope here; routing belongs to the fleet ([RFC-56](rfc-56-eval-fleet.md)). This RFC fixes only the `eval` signature.
- **Fate of the brief-frontmatter contract (resolved).** Deferred to [RFC-53](rfc-53-orchestration.md): once the `eval` call-site names the signature, the heavy `implements` / `consumes` / `produces` / `capabilities` frontmatter is subsumed, surviving only as authoring-time `specify lint framework` rules.
- **Replay backend shape (resolved).** Recorded `eval` outputs are stored on disk keyed by a stable digest of the `eval-request`; record vs replay is selected by the bound `eval` backend (configuration; replay in CI). This RFC ships only the replay backend — real backends are [RFC-56](rfc-56-eval-fleet.md).
- **Sync vs async deferral (resolved).** The synchronous `eval` signature is forward-compatible: instance-per-call plus the `path` handle close the reference loop without async. Streaming / cancellation are deferred to a later decision (gated in [RFC-53](rfc-53-orchestration.md) / [RFC-56](rfc-56-eval-fleet.md)).

## Phased plan

1. **Done — interfaces defined.** The effect interfaces are authored and complete in `wit/model.wit` and `wit/specify.wit`; remaining work is wiring the host bindings and asserting no behavior change.
2. Implement the host handlers over existing machinery.
3. Add the `eval` record/replay backend; prove a single operation replays deterministically in CI.
4. Project request/report records into live handoff envelopes and retire the JSON schema constants against the package.

## Acceptance criteria

1. The effect vocabulary exists as typed WIT interfaces.
2. A replay stub satisfies `eval`, making at least one operation deterministic end-to-end.
3. No runtime behavior changes; existing checks pass.
4. Every effect passes handles/references—no bodies or artifacts cross as inlined values.
5. Agent handoff envelopes are typed against structured records; JSON schema constants are retired or generated from WIT.

## Risks and invariants

- **Pivot must be behavior-neutral:** Only names what exists.
- **No corpus across the boundary:** `eval` takes `path(string)`; `references` is pull-by-id.
- **Adapter agnostic:** Effect interfaces are generic—no adapter name, taxonomy, or LLM vendor in the contract.
