# RFC-52: Effect Interfaces

> Status: Draft · Implements: Effect-oriented architecture · Depends: RFC-51 · Sequences into: RFC-53

## Abstract

This RFC defines explicit, typed WIT interfaces for the fixed vocabulary of effects the runtime currently performs implicitly: `eval` (formerly `judge`, to run a brief or prompt), host-data accessors, and `resolve` (fallback reference resolver). Note that `kv` (host-held state) and lifecycle hooks (`journal` / `transition`) are provided by the Omnia runtime as `KeyValue` and `JsonDb` WIT worlds, respectively. It changes no runtime behavior—each named effect is initially backed by existing machinery. The goal is to make the implicit boundaries explicit, typed, and mockable, unlocking deterministic record/replay.

## Motivation

Currently, LLM inference is an out-of-band convention (the CLI prints an envelope, the agent reads a brief) with no typed contract. Defining it as an effect interface:

- Creates a single, typed surface for context-injection, recording, and rate-limiting.
- Enables a **replay stub** to satisfy `eval`, making runs deterministic in CI.
- Provides a stable import for future orchestration components to build against.

## Scope

**In scope:**
- WIT interface definitions for `eval`, `host-data`, and `resolve`.
- Host-side handlers that satisfy these interfaces using existing machinery.
- A record/replay backend for `eval` sufficient for CI.
- Typing the agent handoff by projecting request/report records into handoff envelopes, replacing hand-maintained JSON schema constants.

**Out of scope:**
- Orchestration components (execution remains a two-phase handoff).
- Async ABI commitments (e.g., streaming or cancellation).
- Brief frontmatter expansion.

## The model (sketch)

The authoritative WIT interfaces are defined in [`../wit/model.wit`](../wit/model.wit).

The host satisfies `eval` with the existing two-phase handoff; the typed interface is the contract, while the handoff is the temporary implementation. Passing `path` instead of the brief body prevents context-budget blowup. Filesystem-capable backends resolve links themselves, while others use the `references` fallback.

**Host-data narrows the blast radius.** The `host-config` and `host-data` accessors restrict adapters to exact capabilities, replacing broad preopened-directory grants. They govern input artifacts and assets, but explicitly do **not** handle an adapter reading its own bundled prose (handled via relative paths or `references`).

**Typing the agent handoff.** Naming the `eval` seam types the live handoff envelope. The host serializes the structured build request into the brief handoff and validates the report against the WIT-derived type, allowing us to remove duplicate JSON schema constants and parity tests.

## Decisions to record (open until reviewed)

- **The `eval` signature:** `eval-request` with `prompt(string)` or `path(string)` + (later) context-injection policy. Ensure `request` matches the structured request record.
- **Model-service routing key:** Model routing logic belongs to the fleet; this RFC fixes only the `eval` signature.
- **Fate of the brief-frontmatter contract:** Decide what parts of the frontmatter contract are subsumed by the `eval` boundary.
- **Replay backend shape:** Define the on-disk format for recorded outputs and the mechanism for selecting record vs replay.
- **Sync vs async deferral:** Confirm the synchronous `eval` signature is forward-compatible.

## Phased plan

1. Define the effect interfaces in `wit/model.wit` and `wit/specify.wit`; wire host bindings; assert no behavior change.
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
