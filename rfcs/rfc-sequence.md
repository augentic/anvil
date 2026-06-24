# Implementation Sequence Plan

> Status: Planning note · Scope: RFCs 51-60 and [architecture.md](architecture.md)

## Sequence

1. **Freeze the typed contract** — Land [RFC-51](rfc-51-adapter-wit.md): the `augentic:specify` WIT package, generated bindings, cross-cutting records, source / target interfaces, and references shelf. This is the shared language both repositories need before other work has stable edges.

2. **Land the effect map** — Land [RFC-52](rfc-52-effect.md): name the effect imports and exports, assign each effect to its owning implementation RFC, and include any WIT wiring needed for the next steps.

3. **Build the minimal Omnia floor** — Land the domain-free runtime capabilities from [RFC-56](rfc-56-runtime-move.md): component instantiation, instance-per-call execution, host-service binding, `wasi:filesystem`, `wasi:keyvalue`, lifecycle / journal effects, and a basic multi-guest registry. Keep this layer free of adapter names, workflow policy, and model identity.

4. **Add `wasi-model` core** — Land the core part of [RFC-53](rfc-53-wasi-model.md): `eval`, prompt / answer types, backend trait, answer validation, and a fake or replay-capable backend. Do not wait for every real backend. Replay gives deterministic CI and lets Specify migration proceed without binding every test to a live model or editor agent.

5. **Add the model tool loop** — Land [RFC-59](rfc-59-model-tool-loop.md): `resolve`, `read`, `list`, `write`, session state, and the repair loop. `resolve` should call the adapter `references` shelf through the same host-mediated guest selection the runtime will use elsewhere.

6. **Prove one Specify vertical slice** — Land [RFC-54](rfc-54-orchestration.md): one deterministic tool operation through generated bindings and one judgment operation through `wasi-model.eval`, including the adapter `references` shelf. This validates the contract, runtime, model boundary, and prose-resolution model against real Specify behavior before the architecture spreads.

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
