# RFC-53: Orchestration Components (Stage 3 — Adapters Orchestrate)

> Status: Draft (skeleton) · Implements: the effect-oriented architecture (Stage 3) · Depends: RFC-52 (effect interfaces) · Absorbs from RFC-51: typed tool dispatch (`wasi:cli/run` retired on the tool path) + the typed brief contract · Async ABI: not required for this stage — only for streaming `judge` / concurrent slices (see Risks)

## Abstract

This stage is where the [effect-oriented architecture](architecture.md) first becomes visible. An adapter operation stops being a single prepare/finalize handoff and becomes a **typed multi-step orchestration**: the adapter's wasm component owns the control flow of its own `build` / `extract` / `merge`, and reaches the model through the `judge` effect (RFC-52) rather than by handing the whole operation back to the agent. It lands in two steps — **Realization B** (a serializable step-reducer driven over the existing handoff) first, then **Realization A** (the component calls `judge` directly through an imported effect) where operation depth justifies the cost.

## Motivation

The prepare/finalize handoff is already a degenerate two-step orchestration (`prepare` = "here is your request, run the brief"; `finalize` = "here is my output, validate and commit"). Deep adapter operations (multi-platform Vectis builds, multi-source extraction) want N typed steps, lazy reference loading, conditional sub-flows, and whole-operation replay — none of which a single handoff expresses. Putting that control flow in the component makes it testable code and makes the LLM a surgical, typed effect.

## Scope

**In scope:** **typed tool dispatch** — routing the deterministic `tool` operations (`contract`, `vectis`) through the RFC-51 world bindings and retiring `wasi:cli/run` on that path (relocated from RFC-51 §B); the orchestration model for adapter operations on both axes; Realization B (step-reducer + serializable continuation); the migration path to Realization A (the `judge` import called from inside an export); and the brief-typing contract relocated here from RFC-51 (§E).

### Non-goals

- **The workflow layer is out of scope.** `/spec:plan` / `/spec:execute` orchestration is [RFC-57](rfc-57-specify-guests.md) (gated). This RFC is adapter-local.
- **No eager reference loading.** architecture invariant 4 holds: steps carry `brief-path` + handles; bodies are pulled lazily.

## The model (sketch)

**Realization B — serializable step-reducer (lands first).** The component is a pure reducer; the agent is its effect runtime; the CLI runs each step and stays LLM-free. This is a direct generalization of the two-phase kernel in [`engine/src/runtime/commands/source/op.rs`](../engine/src/runtime/commands/source/op.rs).

```wit
interface types {
  type op-state = list<u8>;                 // component-owned, serialized into the resume token
  variant directive {
    judge(judge-request),                   // run a brief (whole prose) with a typed request
    resolve(string),                        // FALLBACK ref fetch by id (non-filesystem backends)
    done(build-report),                     // validated terminal report
    fail(adapter-error),
  }
  record judge-request { brief-path: string, request: string }
  record step-result { state: op-state, directive: directive }
}

interface target {
  build-begin: func(req: build-request) -> result<step-result, adapter-error>;
  build-step:  func(state: op-state, fulfillment: string) -> result<step-result, adapter-error>;
}
```

The agent driver loop runs each `judge` directive in its own context, exactly as today's single handoff does — so context inheritance is preserved with no async ABI and no store snapshots.

**Realization A — the `judge` import (migration target).** Where depth justifies it, collapse the external loop into a straight-line export that calls the RFC-52 `judge` effect directly:

```wit
world target-adapter {
  import host-config;
  import host-data;
  import judge;            // RFC-52 — the upward model channel
  export target;           // build/merge/shape always callable
}
```

The data contract, the `brief-path` discipline, and the validation points are identical across B and A; A is an ABI/topology change, not a redesign.

**Typed tool dispatch comes first (relocated from RFC-51 §B).** Before any multi-step orchestration, the deterministic `tool` operations that exist today — the `contract` and `vectis` validators — are re-pointed from `wasi:cli/run` to their RFC-51 world export, and the `execution: tool` path is routed through the generated bindings (`instance.call_build(&mut store, &req)`). This is the lowest-risk realization of the RFC-51 contract: real callable exports, a typed `result<_, adapter-error>` in place of exit-code + stdout-JSON, and the argv contract retired on that path. Orchestration (B and A above) then builds on the same exports rather than inventing a second invocation surface.

## Brief-typing and lazy discovery (relocated from RFC-51 §E)

RFC-51 originally proposed binding each agent brief to the WIT signature it fulfils, plus a lazy reference-discovery model. With orchestration components the signature is already named at the `judge` call-site (Realization A) or by the step `directive` (Realization B), so most of that binding is **subsumed** — but the authoring-time *checks* it enabled are still worth keeping as `specify lint framework` rules. The candidate seams and their status here:

- **Signature binding.** A brief declares which operation it implements; a set-coverage check guarantees every agent operation has exactly one binding brief and every brief binds a real operation. *Survives as lint* — the binding may move from frontmatter to the `judge` call-site.
- **Typed input environment.** A brief's placeholders (`$SLICE_NAME`, `inputs.artifacts.*`, `<lead>`) are checked against the request record's fields, so a brief can only reference real, typed inputs. *Survives as lint.*
- **Output example validation.** A brief's embedded fenced examples validate against the WIT-derived report schema at authoring time; the agent's actual output validates at the step's terminal `done(report)`. *Survives* — the runtime check is already the validation point in both realizations.
- **Capability binding.** A brief's declared capabilities mirror the world's host-data imports. *Folded into [RFC-52](rfc-52-effect.md)* — the effect imports are the capability surface; the brief declaration becomes advisory lint.

**Lazy discovery is preserved by construction.** The contract governs the boundary (request in, report out, effects imported), not the interior navigation of the prose. Phase sub-briefs and the reference shelf load on demand — through the brief's own relative links (a filesystem-capable backend follows them directly) or, as a fallback, the RFC-52 `references` effect (a backend that cannot read disk) — and architecture invariant 4 forbids any step from pushing a corpus across the boundary. Only the parent brief binds the operation signature; sub-briefs are internal decomposition. Lint proves the discovery graph resolves without loading it.

## Decisions to record (open until reviewed)

- **B→A migration trigger.** What operation depth / shape justifies moving an operation from the step-reducer to the direct `judge` import (or whether some operations stay on B indefinitely).
- **Async ABI.** Whether A adopts the Component Model async path (streaming `judge`, cancellation, concurrency) and the wasmtime version that makes it safe.
- **Reentrancy discipline.** Depth limits and store-isolation rules when an `judge` call's LLM triggers another component operation.
- **Fate of the relocated brief-frontmatter contract.** This RFC likely **supersedes** the heavy `implements` / `consumes` / `produces` / `capabilities` frontmatter: once the component owns orchestration and the `judge` call-site declares the signature, the brief is an effect body, not a contract-bearing artifact. Decide what survives as authoring-time lint.
- **Determinism boundary.** Confirm "deterministic modulo the `judge` oracle" gives whole-operation replay goldens over the typed contract.

## Phased plan

1. Route the existing `execution: tool` adapters (`contract`, `vectis`) through the RFC-51 generated world bindings, retiring `wasi:cli/run` on that path — the lowest-risk first step, since these are real callable exports today behind the argv contract.
2. Land Realization B for one deep operation (candidate: a Vectis multi-platform `build`) as a step-reducer over the existing CLI driver.
3. Add whole-operation record/replay goldens for that operation.
4. Confirm the async ABI; migrate the same operation to Realization A as a proof of the topology change.
5. Generalize the chosen realization across adapter operations incrementally.

## Acceptance criteria

1. The deterministic `tool` adapters (`contract` / `vectis`) are invoked through the RFC-51 generated bindings — no argv packing or stdout-JSON parsing on that path; `wasi:cli/run` is retired for `execution: tool`.
2. At least one adapter operation runs as a typed multi-step orchestration owned by the component.
3. The operation replays deterministically end-to-end under a mocked `judge` effect.
4. Lazy reference loading is preserved — steps carry `brief-path` + handles; no corpus crosses the boundary (architecture invariant 4).
5. The B and A forms share one data contract; migrating B→A requires no record/report shape change.
6. `make lint` and `cargo make ci` stay green at each increment.

## Risks and invariants

- **Async maturity.** Instance-per-call and the `brief-path` simplification close the reference loop on the *synchronous* ABI, so neither Realization B nor a synchronous Realization A needs async. The Component-Model async path is required only for **streaming** `judge` output and **concurrent** slices — confirm it before those, not before this stage.
- **Scope creep into the workflow.** Keep this adapter-local; the workflow is [RFC-57](rfc-57-specify-guests.md).
- **Prose holism.** `judge` passes *whole* briefs, not chopped micro-prompts — the component sequences and types; it does not fragment the prompt.
- **RFC-50 preserved.** Orchestration components carry adapter logic; the host still holds zero adapter names and reaches them only through generic effects.
