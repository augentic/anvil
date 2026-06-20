# RFC-54: Workflow as Effects (Stage 4 — The Thin Interpreter)

> Status: Draft (skeleton) · Implements: the effect-oriented architecture (Stage 4) · Depends: RFC-53 (orchestration components proven on the adapter axis) · Runtime move (workflow → guest, retire bespoke host): **committed** · Per-phase compile-vs-delegate: **gated on RFC-53 data**

## Abstract

The final stage of the [effect-oriented architecture](architecture.md) splits into two postures. The **runtime move is committed**: the workflow — `/spec:plan`, `/spec:execute`, the slice loop — runs on Omnia as a guest like every adapter, the bespoke `specify` host retires, and Omnia becomes the thin interpreter this stage's title names. That move *supersedes the prior north star's coherent stop after RFC-53*. What stays **gated** is *how much* of each phase compiles into the guest versus stays agent-driven behind `infer` — and either way the workflow's adaptability now lives behind the `infer` effect, not in prose. This RFC exists to gate that per-phase compile-vs-delegate call with data from RFC-53, not to re-litigate the committed runtime move.

## Motivation (and the case against)

**For:** the workflow lifecycle is a state machine (plan → approved → execute loop → finalize) whose deterministic skeleton already lives in the CLI; formalizing the remaining glue as orchestration over effects would give whole-workflow record/replay, typed control flow, and one uniform model across both layers.

**Against (load-bearing):** the workflow is exactly where the LLM's adaptability — handling unexpected project states, operator intent, recovery — is a *feature*, and where the CLI's existing guardrails (`plan.lock`, Gate 1, `specify plan next` refusing illegal transitions) already bound any agent fumbling. Prose is also more malleable for operator-facing UX evolution. **This RFC must clear the "against" case before any phase graduates to compiled orchestration**, per the architecture's per-layer line: the workflow leans toward an agent-driver over deterministic guardrails, not toward a monolithic component.

## Scope

**In scope (if activated):** expressing one or more workflow phases as effect-driven orchestration; the boundary between CLI-owned deterministic transitions and component-owned sequencing; what becomes of skill markdown.

### Non-goals

- **No lifecycle authority moves into the guest.** `transition` / `journal` / lock ownership stay in the runtime's deterministic lifecycle host service (roadmap principle: "Keep the CLI authoritative" — the authority stays on the deterministic floor, not in a model-driven guest); the workflow guest *requests* transitions as effects, it does not own them.
- **No removal of operator adaptability.** Phases that are primarily judgment/recovery stay agent-driven.

## The model (sketch)

`/spec:plan` becomes an orchestration component that issues effects — `infer(survey-brief)` per bound source, `infer(reconcile-leads)` for reconciliation — with the runtime's deterministic operations (`plan add`, validation, Gate 1) as **non-yielding** steps surfaced through the lifecycle effect. `/spec:execute` becomes the drained-loop reducer it already morally is. Skills thin from orchestrators to launchers.

## Decisions to record (open — and gating)

- **Which phases, if any, graduate.** Per-phase judgment: does this phase's value come from deterministic sequencing (graduate) or from adaptive judgment (stay agent-driven)?
- **Skill fate.** Whether skills dissolve into typed orchestration + `infer` bodies, or remain the operator entry surface with orchestration underneath.
- **Operator UX.** Whether moving orchestration prose into compiled control flow regresses the "tweak the skill" malleability operators rely on.
- **Lifecycle authority boundary.** Exactly which transitions remain CLI-only versus reachable as effects.
- **Per-phase activation trigger.** What evidence from RFC-53 (adapter orchestration in production, replay paying off, async ABI stable) justifies graduating a given phase to compiled orchestration.

## Phased plan (per-phase, gated)

1. Pick one phase whose value is dominated by deterministic sequencing; express it as orchestration over effects with the CLI still owning transitions.
2. Add whole-phase record/replay; compare operator ergonomics against the prose skill it replaces.
3. Decide — with that evidence — whether to generalize, stop, or revert.

## Acceptance criteria (per graduated phase)

1. At least one workflow phase runs as effect-driven orchestration with whole-phase record/replay.
2. The CLI retains sole lifecycle authority; the component reaches transitions only as effects.
3. The per-layer line (architecture north star) is respected — adaptive phases remain agent-driven; only deterministic-sequencing phases graduate.
4. Operator ergonomics are demonstrably not worse than the prose skill replaced.
5. `make lint` and `cargo make ci` stay green.

## Risks and invariants

- **Ossifying the fluid.** The chief risk is encoding adaptive, recovery-heavy orchestration as rigid control flow. The "against" case is the guard; if a phase needs the LLM's judgment to sequence, it does not belong here.
- **CLI authority.** Lifecycle authority must not migrate into components, services, or skills (roadmap Non-Goal).
- **Per-phase deferral is the default.** The runtime move (workflow → guest) is committed, but absent a clear trigger and owner *no individual phase compiles* — each stays agent-driven behind `infer` until its case is made. RFC-53 is the last unconditional stage; graduating a workflow phase past it is opt-in and evidence-gated, not the architecture's stopping point.
