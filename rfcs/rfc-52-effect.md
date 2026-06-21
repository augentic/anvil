# RFC-52: Effect Interfaces (Stage 2 — Name the Effects)

> Status: Draft (skeleton) · Implements: the effect-oriented architecture (Stage 2) · Depends: RFC-51 (typed records + host bindings) · Absorbs from RFC-51: agent-envelope typing + `*_JSON_SCHEMA` retirement, and the named host-data capability model · Sequences into: RFC-53 (orchestration components)

## Abstract

This is the **pivot stage** of the [effect-oriented architecture](architecture.md). It names — as typed WIT interfaces — the small, fixed vocabulary of effects the runtime already performs implicitly: `judge` (run a brief on a model), the host-data accessors (already seeded by RFC-51), `resolve` (the fallback resolver for adapter-bundle prose), `kv` (host-held state for memoization), and the `journal` / `transition` lifecycle hooks. Crucially, S2 changes **no runtime behavior**: each named effect is *initially backed by the machinery that exists today* (the prepare/finalize handoff for `judge`, the CLI for lifecycle). The value is that the implicit boundary becomes explicit, typed, and — above all — **mockable**, which is what unlocks deterministic record/replay.

## Motivation

Today the LLM step is an out-of-band convention: the CLI prints a handoff envelope and the agent reads a brief. Nothing types "this step is an inference against this brief returning this report." Naming it as an effect interface:

- makes the seam a single, typed surface (one place for context-injection policy, recording, rate-limiting);
- lets a **replay stub** satisfy `judge` so a whole run is deterministic in CI (architecture acceptance #4);
- gives RFC-53 a stable import to build orchestration components against, without RFC-53 also having to invent the effect vocabulary.

## Scope

**In scope:** the WIT interface definitions for `judge`, host-data (promote RFC-51), `resolve`, `kv`, and `journal` / `transition`; the host-side handlers that satisfy them using existing machinery; a replay/record backend for `judge` sufficient for CI; and **typing the agent handoff** — projecting the RFC-51 request/report records into the prepare/finalize handoff envelopes and retiring the `*_JSON_SCHEMA` constants + byte-parity test in favour of the package (relocated from RFC-51 §C).

### Non-goals

- **No orchestration components yet.** S2 names the effects; S3 (RFC-53) makes components *call* them. The two-phase handoff stays the execution model through S2.
- **No async ABI commitment.** `judge` is specified synchronously here (`-> result<output, error>`); streaming/cancellation is deferred to RFC-53 where the async-ABI bet is made.
- **No brief-frontmatter expansion.** The `implements` / `consumes` / `produces` machinery RFC-51 handed to [RFC-53](rfc-53-orchestration.md) is *not* extended here; this RFC may subsume part of it (see Decisions).

## The model (sketch)

```wit
// The marquee effect. `brief-path` is a HANDLE — the brief's on-disk path,
// never its body (architecture invariant 4). A filesystem-capable backend
// reads the brief and follows its relative links itself; only a backend that
// cannot read disk falls back to `references.resolve` below.
interface judge {
  use types.{adapter-error};
  // request: JSON projection of the typed op request (handles, not corpora).
  run-brief: func(brief-path: string, request: string) -> result<string, adapter-error>;
}

// FALLBACK reference resolver for non-filesystem backends. Most references are
// static prose a host file-read satisfies, so filesystem-capable backends never
// call this; bodies are pulled by id, never pushed.
interface references {
  use types.{adapter-error};
  resolve: func(id: string) -> result<list<u8>, adapter-error>;
}

// Narrow, host-provided accessor onto project.yaml (promoted from RFC-51).
// The CLI validates project.yaml first, so the typed getters skip result<>;
// `get` is the open scalar fallback for other fields.
interface host-config {
  project-name: func() -> string;
  target-ref: func() -> string;
  platforms: func() -> list<string>;
  workspace: func() -> bool;
  get: func(key: string) -> option<string>;
}

// Narrow host accessor replacing the raw $CAPABILITY_DIR + preopen grant: the
// adapter reads host/project data (slice artifacts, project assets) through one
// typed call, never by walking a preopened tree (promoted from RFC-51).
// Bytes in; the caller deserializes at the call site (String::from_utf8,
// serde_yaml, raw bytes) — the host attaches no content-type.
interface host-data {
  use types.{adapter-error};
  read: func(id: string) -> result<list<u8>, adapter-error>;
  read-config: func() -> result<string, adapter-error>;
}

// Host-held state: memoize expensive-to-compute resources so stateless,
// instance-per-call guests keep nothing durable in process. Backed per
// deployment (filesystem · Redis · NATS); also where resolve memoizes.
interface kv {
  use types.{adapter-error};
  get: func(key: string) -> option<list<u8>>;
  set: func(key: string, value: list<u8>) -> result<_, adapter-error>;
}

// A lifecycle interface (journal / transition) is named here too.
```

The host satisfies `judge` with today's handoff in S2 (print envelope, run brief, parse report); the typed interface is the contract, the handoff is the temporary implementation. `brief-path` is the load-bearing simplification: a filesystem-capable backend (a frontier agent CLI, or a local agent) is handed the brief's on-disk path and follows its relative links itself, so the common path makes no callback into the runtime; `references.resolve` is the fallback only a backend that cannot read disk uses.

**Host-data narrows the blast radius (promoted from RFC-51).** The `host-config` / `host-data` accessors name exactly the host capabilities an adapter may use, replacing the broad `$CAPABILITY_DIR` + preopened-directory grant. They target *host/project* data — the slice tree, artifacts, and assets that flow *into* an operation — and deliberately do **not** govern an adapter reading its *own* bundled prose: that corpus is reached by the brief's own relative links (a filesystem-capable backend follows them directly) or, for a backend that cannot read disk, through the `references` fallback. Keeping the two access kinds distinct is what lets reference discovery stay open while host data stays narrowly typed.

**Typing the agent handoff (relocated from RFC-51 §C).** `judge` is the typed seam the prepare/finalize handoff already morally is, so naming it is also where the handoff's *envelope* becomes typed: the host serializes the RFC-51 `build-request` (and the survey/extract/shape/merge requests) into the brief handoff and validates the `build-report` back at `finalize` against the WIT-derived report type. With the records as the live envelope, the `*_JSON_SCHEMA` constants and their byte-parity test are retired or regenerated from the package — one definition for the tool path and the agent path alike, and the drift surface RFC-51 named is finally removed rather than policed.

## Decisions to record (open until reviewed)

- **The `judge` signature.** `brief-path` (the brief's on-disk path handle) + JSON `request` + (later) a context-injection policy knob (same-thread vs subagent). Confirm `request` is a JSON projection of the stratum-1 record, not a new shape.
- **Model-service routing key → [RFC-56](rfc-56-judge-fleet.md).** The router that keys backend selection on the `brief-path` / an abstract difficulty hint (never a vendor model id) is the fleet's concern; this RFC fixes only the `judge` *signature* it routes over.
- **`kv` exposure.** Whether guests call `kv` directly to memoize their own deterministic sub-results, or it stays the host's backing for `resolve` memoization. Either way the backend set (filesystem · Redis · NATS) is the runtime's concern ([RFC-55](rfc-55-runtime-move.md)).
- **Lifecycle as effect vs CLI-only.** Whether `journal` / `transition` become component-callable effects now, or stay CLI-owned and are reached only through the driver. (Leaning: stay CLI-owned through S2; expose later only if S4 needs it.)
- **Fate of the relocated brief-frontmatter contract.** How much of `implements` / `consumes` / `produces` / `capabilities` (now carried by [RFC-53](rfc-53-orchestration.md)) is subsumed by the typed `judge` boundary, and how much survives as authoring-time lint.
- **Replay backend shape.** The on-disk format of recorded `(brief-path, request) -> output` pairs and how a run selects record vs replay.
- **Sync vs async deferral.** Confirm the synchronous `judge` here is forward-compatible with the async ABI RFC-53 may adopt (no signature churn).

## Phased plan

1. Define the effect interfaces in `wit/specify.wit` (or a sibling `wit/effects.wit`); wire host bindings; assert no behavior change.
2. Implement the host handlers over existing machinery (handoff for `judge`, CLI for lifecycle).
3. Add the `judge` record/replay backend; prove a single operation replays deterministically in CI.
4. Project the RFC-51 records into the live survey/extract/shape/build/merge handoff envelopes and retire (or regenerate) the `*_JSON_SCHEMA` constants + parity test against the package, so the records are the single source of truth.

## Acceptance criteria

1. The effect vocabulary (`judge`, host-data, `references`, `kv`, lifecycle) exists as typed WIT interfaces.
2. A replay stub can satisfy `judge`, making at least one operation deterministic end-to-end.
3. No runtime behavior changes versus RFC-51; `make lint` and `cargo make ci` stay green.
4. Every effect carries handles/references — no brief body or artifact content crosses as an inlined value (architecture invariant 4).
5. The agent handoff envelopes are typed against the RFC-51 records, and the `*_JSON_SCHEMA` constants + byte-parity test are retired or regenerated from the WIT package — no hand-maintained schema duplicates the records.

## Risks and invariants

- **Pivot must be behavior-neutral.** If S2 changes execution, it has overreached — it only *names* what exists.
- **No corpus across the boundary.** `judge` takes a `brief-path`; `references.resolve` is pull-by-id. Regressing this re-introduces the context-budget blow-up architecture invariant 4 forbids.
- **RFC-50 preserved.** Effect interfaces are generic — no adapter name, no taxonomy, no LLM vendor in the contract.
