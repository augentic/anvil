# Implementation Sequence Plan

> Status: Planning note · Scope: RFCs 51-60 and [architecture.md](architecture.md) · Premise: fresh implementation judgment, not current dependency metadata

## Opinion

Implement the generic Omnia runtime floor before the deep Specify rewrite, but do not let Omnia progress as a long isolated branch. The healthiest path is: build enough Omnia to host typed, stateless guests; immediately prove it with one thin Specify vertical slice; then broaden from adapter operations to workflow-as-guest.

If "Omnia changes" means the generic runtime, those should come first. If it means the Omnia target adapter, it should come first only if it is chosen as the proving vertical slice.

## RFC Shape

The RFC set now separates the architectural boundaries that should be implemented independently:

- Keep [RFC-51](rfc-51-adapter-wit.md) separate as the contract RFC. It should own WIT records, worlds, generated bindings, and publishing / pinning.
- Keep [RFC-52](rfc-52-effect.md) as a short effect map, not a large implementation RFC. Concrete implementation ownership lives in the RFCs that implement each effect: `wasi-model` core in [RFC-53](rfc-53-wasi-model.md), working trees in [RFC-55](rfc-55-working-tree.md), runtime host binding in [RFC-56](rfc-56-runtime-move.md), and lifecycle / journal in RFC-56 unless split into a small lifecycle-specific RFC later.
- Keep [RFC-53](rfc-53-wasi-model.md), [RFC-59](rfc-59-model-tool-loop.md), and [RFC-60](rfc-60-verify-profiles.md) separate: `wasi-model` core (`eval`, prompt / answer types, backend trait, validation, minimal replay); model tool loop (`resolve`, `read`, `list`, `write`, session state, repair loop); and verify profiles (closed check names, sandboxing, report mapping). The verify seam is security-sensitive enough to deserve its own decision surface.
- Keep [RFC-54](rfc-54-orchestration.md) narrowed to the first vertical adapter-operation proof: one deterministic tool operation through generated bindings and one judgment operation through `eval`. Broad workflow sequencing belongs in RFC-57.
- Keep [RFC-55](rfc-55-working-tree.md), [RFC-56](rfc-56-runtime-move.md), and [RFC-57](rfc-57-specify-guests.md) separate. Each owns a real architectural boundary.
- Keep [RFC-58](rfc-58-model-backends.md) separate as the backend catalogue and router RFC. It should expand backend variety after the core `wasi-model` seam exists, not carry the first definition of core replay semantics.

## Sequence

1. **Freeze the typed contract** — Land [RFC-51](rfc-51-adapter-wit.md): the `augentic:specify` WIT package, generated bindings, cross-cutting records, source / target interfaces, and references shelf. This is the shared language both repositories need before other work has stable edges.

2. **Reduce RFC-52 to the effect map** — Name the effect imports and exports, but avoid trying to implement all of them in one pass. The output should be a stable map from effect to owning implementation RFC, plus any WIT wiring needed for the next steps.

3. **Build the minimal Omnia floor** — Land the domain-free runtime capabilities from [RFC-56](rfc-56-runtime-move.md): component instantiation, instance-per-call execution, host-service binding, `wasi:filesystem`, `wasi:keyvalue`, lifecycle / journal effects, and a basic multi-guest registry. Keep this layer free of adapter names, workflow policy, and model identity.

4. **Add `wasi-model` core** — Land the core part of [RFC-53](rfc-53-wasi-model.md): `eval`, prompt / answer types, backend trait, answer validation, and a fake or replay-capable backend. Do not wait for every real backend. Replay gives deterministic CI and lets Specify migration proceed without binding every test to a live model or editor agent.

5. **Add the model tool loop** — Land [RFC-59](rfc-59-model-tool-loop.md): `resolve`, `read`, `list`, `write`, session state, and the repair loop. `resolve` should call the adapter `references` shelf through the same host-mediated guest selection the runtime will use elsewhere.

6. **Prove one Specify vertical slice** — Land the sharpened [RFC-54](rfc-54-orchestration.md): one deterministic tool operation through generated bindings and one judgment operation through `wasi-model.eval`, including the adapter `references` shelf. This validates the contract, runtime, model boundary, and prose-resolution model against real Specify behavior before the architecture spreads.

7. **Materialize working trees** — Land [RFC-55](rfc-55-working-tree.md) before serious target `build` / `merge` migration or spawned-agent work. The `revision -> working-tree -> changeset` loop is the portability hinge and should be proven while the vertical slice is still small.

8. **Lock down verify profiles** — Land [RFC-60](rfc-60-verify-profiles.md) before broad target migration. The model may name only closed check profiles; the host owns argv, sandboxing, severity mapping, and report normalization. This is the security gate before generated code starts running at scale.

9. **Complete the runtime move** — Finish [RFC-56](rfc-56-runtime-move.md): host-mediated dynamic linking, registry selection by identity, component-on-both-axes, CLI trigger behavior, and real backend bindings. At this point the architecture becomes operational rather than aspirational.

10. **Expand model backends** — Broaden [RFC-58](rfc-58-model-backends.md) after the `wasi-model` seam and working-tree backend are stable. Frontier and replay should be available early; spawned-agent follows `local-path`; router and SLM support can wait until real routing pressure exists.

11. **Move workflow and development tooling last** — Land [RFC-57](rfc-57-specify-guests.md) after adapter orchestration, lifecycle effects, working trees, and model evaluation are proven. Workflow-as-guest is the payoff, not the bootstrap. Moving it too early risks compiling uncertainty into guest control flow.

## Rationale

The riskiest abstraction is not the WIT contract; it is the claim that a generic runtime plus typed effects can host real Specify work without hidden state, transcript dependence, or local-path coupling. The fastest way to retire that risk is a thin vertical path through a real adapter operation, not a complete rewrite of either repository.

This sequence keeps the runtime floor generic while forcing it to serve real Specify behavior early. It also keeps judgment behind `wasi-model` from the start, but separates the model seam from backend variety and from the security-sensitive verify path. Workflow migration waits until the lower-level execution model has enough evidence to be trusted.

## Guardrails

- Keep Omnia core domain-free: no adapter names, workflow rules, model ids, or Specify-specific taxonomy in the runtime floor.
- Keep Specify-specific behavior in guests, backends, and native orchestration bound behind typed effects.
- Prefer deterministic replay before live model breadth; every early judgment path should be recordable and replayable.
- Treat `verify` as a security boundary, not as a convenience helper inside the model loop.
- Do not move the whole workflow first. Start with one adapter operation, then widen.
- Treat the Omnia target adapter as a possible proving workload, not as a prerequisite to the generic runtime floor.
