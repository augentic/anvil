# RFC-54: Orchestration (Stage 3 — Deterministic Dispatch + Native Judgment)

> Status: Draft (skeleton) · Implements: the effect-oriented architecture (Stage 3) · Depends: RFC-52 (deterministic effects + the `references` shelf), [RFC-53](rfc-53-tool-server.md) (the native tool-use loop) · Absorbs from RFC-51: typed tool dispatch (`wasi:cli/run` retired on the tool path) + the typed brief contract · Async ABI: not required for this stage

## Abstract

This stage is where the [effect-oriented architecture](architecture.md) first becomes visible. It splits an adapter operation along its grain. The **deterministic** part — the `tool` operations (`contract`, `vectis`) — becomes a real typed guest export, called through the RFC-51 bindings instead of `wasi:cli/run`. The **judgment** part — `build` / `extract` / `merge` synthesis — stops being a single prepare/finalize handoff and becomes a **typed multi-step orchestration**, but that orchestration lives in the binary's **native tool-use loop** ([RFC-53](rfc-53-tool-server.md)), not in the guest: the guest contributes its brief (prose), its `references` shelf, and any deterministic setup export, while the native loop drives the model and sequences the verify-repair cycle.

## Motivation

The prepare/finalize handoff is already a degenerate two-step orchestration (`prepare` = "here is your request, run the brief"; `finalize` = "here is my output, validate and commit"). Deep adapter operations (multi-platform Vectis builds, multi-source extraction) want N typed steps, lazy reference loading, conditional sub-flows, and whole-operation replay — none of which a single handoff expresses. The native tool-use loop provides exactly that control flow as testable native code, with the model reached as a surgical, typed step through the `ModelClient` boundary; the deterministic `tool` operations, meanwhile, want nothing more than a real callable export.

## Scope

**In scope:** **typed tool dispatch** — routing the deterministic `tool` operations (`contract`, `vectis`) through the RFC-51 world bindings and retiring `wasi:cli/run` on that path (relocated from RFC-51 §B); how a judgment operation is expressed as native orchestration over the `references` shelf and the [RFC-53](rfc-53-tool-server.md) tool surface; and the brief-typing contract relocated here from RFC-51 (§E).

### Non-goals

- **The judgment loop's internals are out of scope.** The tool surface, the verify-repair cycle, the `ModelClient` boundary, and record/replay are [RFC-53](rfc-53-tool-server.md); this RFC consumes them.
- **The workflow layer is out of scope.** `/spec:plan` / `/spec:execute` orchestration is [RFC-57](rfc-57-specify-guests.md) (gated). This RFC is adapter-local.
- **No eager reference loading.** architecture invariant 4 holds: the loop carries `brief-path` + handles; bodies are pulled lazily through the shelf.

## The model (sketch)

**Typed tool dispatch comes first (relocated from RFC-51 §B).** The deterministic `tool` operations that exist today — the `contract` and `vectis` validators — are re-pointed from `wasi:cli/run` to their RFC-51 world export, and the `execution: tool` path is routed through the generated bindings (`instance.call_build(&mut store, &req)`). This is the lowest-risk realization of the RFC-51 contract: real callable exports, a typed `result<_, error>` in place of exit-code + stdout-JSON, and the argv contract retired on that path.

```wit
world target-adapter {
  export target;            // build / merge / guidance — callable through RFC-51 bindings
  export references;         // the reference shelf the native loop resolves against (RFC-52)
}
```

**Judgment orchestration is native.** A judgment operation (`build` / `extract` / `merge` synthesis) is sequenced by the native tool-use loop ([RFC-53](rfc-53-tool-server.md)), not by in-guest code. The loop hands the model the operation's brief and the tool surface; the model's `resolve` calls are forwarded to the guest's exported `references` shelf, its `read` / `list` / `write` calls hit the working tree, and its `verify` calls run the sandboxed checks. The N typed steps, conditional sub-flows, lazy reference loading, and whole-operation replay that an in-guest reducer would have provided are all properties of that native loop instead — with replay captured at the `ModelClient` boundary.

What stays in the guest is exactly what is deterministic (the `tool` exports) or pure data the model pulls (the `references` shelf and the brief prose); judgment itself runs natively, with no in-guest model channel.

## Brief-typing and lazy discovery (relocated from RFC-51 §E)

RFC-51 originally proposed binding each agent brief to the WIT signature it fulfils, plus a lazy reference-discovery model. With the native loop the signature is named at the loop's call-site (it knows which operation it is running), so most of that binding is **subsumed** — but the authoring-time *checks* it enabled are still worth keeping as `specify lint framework` rules. The candidate seams and their status here:

- **Signature binding.** A brief declares which operation it implements; a set-coverage check guarantees every agent operation has exactly one binding brief and every brief binds a real operation. *Survives as lint.*
- **Typed input environment.** A brief's placeholders (`$SLICE_NAME`, `inputs.artifacts.*`, `<lead>`) are checked against the request record's fields, so a brief can only reference real, typed inputs. *Survives as lint.*
- **Output example validation.** A brief's embedded fenced examples validate against the WIT-derived report schema at authoring time; the model's actual output validates when the loop commits the operation's terminal report. *Survives* — the runtime check is the loop's validate-and-commit step.
- **Capability binding.** A brief's declared capabilities mirror the world's imports and the tool surface it pulls on. *Folded into [RFC-52](rfc-52-effect.md) / [RFC-53](rfc-53-tool-server.md)* — the effect imports and the tool surface are the capability surface; the brief declaration becomes advisory lint.

**Lazy discovery is preserved by construction.** The contract governs the boundary (request in, report out, effects imported, tools pulled), not the interior navigation of the prose. Phase sub-briefs and the reference shelf load on demand — through the brief's own relative links (a filesystem-capable spawned-agent strategy follows them directly) or the `references` shelf the native loop resolves against (an API model) — and architecture invariant 4 forbids any step from pushing a corpus across the boundary. Only the parent brief binds the operation signature; sub-briefs are internal decomposition. Lint proves the discovery graph resolves without loading it.

## Decisions to record (open until reviewed)

- **Async ABI.** Whether the native loop's model leg ever needs the Component Model async path (streaming output, concurrency) and the wasmtime version that makes it safe. Out of this stage's baseline.
- **Reentrancy discipline.** Store-isolation rules if a deterministic guest export is invoked while the native loop has a session open over the same tree.
- **Fate of the relocated brief-frontmatter contract.** This RFC likely **supersedes** the heavy `implements` / `consumes` / `produces` / `capabilities` frontmatter: once the native loop owns judgment orchestration and names the operation at its call-site, the brief is a prompt body, not a contract-bearing artifact. Decide what survives as authoring-time lint.
- **Determinism boundary.** Confirm "deterministic modulo the `ModelClient` boundary" gives whole-operation replay goldens over the typed contract (the recording seam is [RFC-53](rfc-53-tool-server.md)'s).

## Phased plan

1. Route the existing `execution: tool` adapters (`contract`, `vectis`) through the RFC-51 generated world bindings, retiring `wasi:cli/run` on that path — the lowest-risk first step, since these are real callable exports today behind the argv contract.
2. Export the `references` shelf from one adapter and drive a judgment operation (candidate: a source `extract`) through the native loop against it.
3. Add whole-operation record/replay goldens for that operation, captured at the `ModelClient` boundary ([RFC-53](rfc-53-tool-server.md)).
4. Land one deep judgment operation (candidate: a Vectis multi-platform `build`) over the native loop, proving N typed steps + verify-repair without any in-guest model channel.
5. Generalize across adapter operations incrementally.

## Acceptance criteria

1. The deterministic `tool` adapters (`contract` / `vectis`) are invoked through the RFC-51 generated bindings — no argv packing or stdout-JSON parsing on that path; `wasi:cli/run` is retired for `execution: tool`.
2. At least one judgment operation runs as a typed multi-step orchestration owned by the native loop, against an adapter that exports its `references` shelf.
3. The operation replays deterministically end-to-end with the `ModelClient` boundary in replay mode (no `eval` effect to mock).
4. Lazy reference loading is preserved — the loop carries `brief-path` + handles and resolves the shelf on demand; no corpus crosses the boundary (architecture invariant 4).
5. `make lint` and `cargo make ci` stay green at each increment.

## Risks and invariants

- **Async maturity.** Instance-per-call and the `brief-path` simplification keep deterministic tool dispatch on the *synchronous* ABI. The Component-Model async path is relevant only to the native loop's model leg (streaming output, concurrent slices) — confirm it there, not in this stage.
- **Scope creep into the workflow.** Keep this adapter-local; the workflow is [RFC-57](rfc-57-specify-guests.md).
- **Prose holism.** The native loop hands the model *whole* briefs, not chopped micro-prompts — it sequences and types; it does not fragment the prompt.
- **RFC-50 preserved.** Adapters carry adapter logic in their exports and shelf; the runtime floor still holds zero adapter names and reaches them only through generic effects, and the model id stays behind the `ModelClient` boundary.
