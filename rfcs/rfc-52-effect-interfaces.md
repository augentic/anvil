# RFC-52: Effect Interfaces (Stage 2 — Name the Effects)

> Status: Draft (skeleton) · Implements: the effect-oriented harness architecture (Stage 2) · Depends: RFC-51 (typed records + host bindings) · Sequences into: RFC-53 (orchestration components)

## Abstract

This is the **pivot stage** of the [effect-oriented harness](architecture.md). It names — as typed WIT interfaces — the small, fixed vocabulary of effects the harness already performs implicitly: `infer` (run a brief on the LLM), the host-data accessors (already seeded by RFC-51 §D), `load-reference` (lazy adapter-bundle prose), and the `journal` / `transition` lifecycle hooks. Crucially, S2 changes **no runtime behavior**: each named effect is *initially backed by the machinery that exists today* (the prepare/finalize handoff for `infer`, the CLI for lifecycle). The value is that the implicit boundary becomes explicit, typed, and — above all — **mockable**, which is what unlocks deterministic record/replay.

## Motivation

Today the LLM step is an out-of-band convention: the CLI prints a handoff envelope and the agent reads a brief. Nothing types "this step is an inference against this brief returning this report." Naming it as an effect interface:

- makes the seam a single, typed surface (one place for context-injection policy, recording, rate-limiting);
- lets a **replay stub** satisfy `infer` so a whole run is deterministic in CI (architecture acceptance #4);
- gives RFC-53 a stable import to build orchestration components against, without RFC-53 also having to invent the effect vocabulary.

## Scope

**In scope:** the WIT interface definitions for `infer`, host-data (promote RFC-51 §D), `load-reference`, and `journal` / `transition`; the host-side handlers that satisfy them using existing machinery; a replay/record backend for `infer` sufficient for CI.

### Non-goals

- **No orchestration components yet.** S2 names the effects; S3 (RFC-53) makes components *call* them. The two-phase handoff stays the execution model through S2.
- **No async ABI commitment.** `infer` is specified synchronously here (`-> result<output, error>`); streaming/cancellation is deferred to RFC-53 where the async-ABI bet is made.
- **No brief-frontmatter expansion.** RFC-51 §F's `implements` / `consumes` / `produces` machinery is *not* extended here; this RFC may subsume part of it (see Decisions).

## The model (sketch)

```wit
// The marquee effect. `brief` is a REFERENCE, never a body (architecture invariant 4).
interface infer {
  use types.{adapter-error};
  record brief-ref { adapter: string, operation: string, briefs-dir: string }
  // request: JSON projection of the typed op request (handles, not corpora).
  run-brief: func(brief: brief-ref, request: string) -> result<string, adapter-error>;
}

// Lazy adapter-bundle prose. Pull by id; bodies never pushed.
interface references {
  use types.{adapter-error};
  record reference-meta { id: string, title: string }
  list: func() -> list<reference-meta>;
  get: func(id: string) -> result<string, adapter-error>;
}

// Narrow, host-provided accessor onto project.yaml (promoted from RFC-51 §D).
// The CLI validates project.yaml first, so the typed getters skip result<>;
// `get` is the open scalar fallback for other fields.
interface host-config {
  project-name: func() -> string;
  target-ref: func() -> string;
  platforms: func() -> list<string>;
  workspace: func() -> bool;
  get: func(key: string) -> option<string>;
}

// Narrow host resources replacing the raw $CAPABILITY_DIR + preopen grant:
// the adapter reads host/project data through typed methods, never by walking
// a preopened tree (promoted from RFC-51 §D).
interface host-data {
  use types.{adapter-error};
  record asset { id: string, content-type: string, data: list<u8> }
  resource project {
    get-asset: func(id: string) -> option<asset>;
    read-config: func() -> result<string, adapter-error>;
  }
  resource slice {
    read-artifact: func(path: string) -> result<string, adapter-error>;
  }
}

// A lifecycle interface (journal / transition) is named here too.
```

The host satisfies `infer` with today's handoff in S2 (print envelope, run brief, parse report); the typed interface is the contract, the handoff is the temporary implementation.

**Host-data narrows the blast radius (promoted from RFC-51 §D).** The `host-config` / `host-data` accessors name exactly the host capabilities an adapter may use, replacing the broad `$CAPABILITY_DIR` + preopened-directory grant. They target *host/project* data — the slice tree, artifacts, and assets that flow *into* an operation — and deliberately do **not** govern an adapter reading its *own* bundled prose: that corpus is reached lazily through `references` (tool path) or the brief's own links (agent path). Keeping the two access kinds distinct is what lets reference discovery stay open while host data stays narrowly typed.

## Decisions to record (open until reviewed)

- **The `infer` signature.** `brief-ref` + JSON `request` + (later) a context-injection policy knob (same-thread vs subagent). Confirm `request` is a JSON projection of the stratum-1 record, not a new shape.
- **Lifecycle as effect vs CLI-only.** Whether `journal` / `transition` become component-callable effects now, or stay CLI-owned and are reached only through the driver. (Leaning: stay CLI-owned through S2; expose later only if S4 needs it.)
- **Fate of RFC-51 §F frontmatter.** How much of `implements` / `consumes` / `produces` / `capabilities` is subsumed by the typed `infer` boundary, and how much survives as authoring-time lint.
- **Replay backend shape.** The on-disk format of recorded `(brief-ref, request) -> output` pairs and how a run selects record vs replay.
- **Sync vs async deferral.** Confirm the synchronous `infer` here is forward-compatible with the async ABI RFC-53 may adopt (no signature churn).

## Phased plan

1. Define the effect interfaces in `wit/adapter.wit` (or a sibling `wit/effects.wit`); wire host bindings; assert no behavior change.
2. Implement the host handlers over existing machinery (handoff for `infer`, CLI for lifecycle).
3. Add the `infer` record/replay backend; prove a single operation replays deterministically in CI.

## Acceptance criteria

1. The effect vocabulary (`infer`, host-data, `references`, lifecycle) exists as typed WIT interfaces.
2. A replay stub can satisfy `infer`, making at least one operation deterministic end-to-end.
3. No runtime behavior changes versus RFC-51; `make lint` and `cargo make ci` stay green.
4. Every effect carries handles/references — no brief body or artifact content crosses as an inlined value (architecture invariant 4).

## Risks and invariants

- **Pivot must be behavior-neutral.** If S2 changes execution, it has overreached — it only *names* what exists.
- **No corpus across the boundary.** `infer` takes a `brief-ref`; `references.get` is pull-by-id. Regressing this re-introduces the context-budget blow-up architecture invariant 4 forbids.
- **RFC-50 preserved.** Effect interfaces are generic — no adapter name, no taxonomy, no LLM vendor in the contract.
