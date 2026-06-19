# RFC-54: Workflow as Effects (Stage 4 — The Thin Interpreter)

> Status: Draft (skeleton) · **Deferred / optional** · Implements: the effect-oriented harness architecture (Stage 4) · Depends: RFC-53 (orchestration components proven on the adapter axis) · Decided with: data from RFC-53, not now

## Abstract

The final, optional stage of the [effect-oriented harness](architecture.md): apply the orchestration-component model proven on adapters (RFC-53) to the **workflow** itself, so `/spec:plan`, `/spec:execute`, and the slice loop run as effect-driven orchestration and the harness becomes a thin interpreter. This RFC exists to capture the end-state so it is not lost — **not** to commit to it. The architecture north star establishes a coherent stopping point after RFC-53; this stage is a deliberate, separately-justified bet.

## Motivation (and the case against)

**For:** the workflow lifecycle is a state machine (plan → approved → execute loop → finalize) whose deterministic skeleton already lives in the CLI; formalizing the remaining glue as orchestration over effects would give whole-workflow record/replay, typed control flow, and one uniform model across both layers.

**Against (load-bearing):** the workflow is exactly where the LLM's adaptability — handling unexpected project states, operator intent, recovery — is a *feature*, and where the CLI's existing guardrails (`plan.lock`, Gate 1, `specify plan next` refusing illegal transitions) already bound any agent fumbling. Prose is also more malleable for operator-facing UX evolution. **This RFC must clear the "against" case before it ships**, per the architecture's per-layer line: the workflow leans toward an agent-driver over deterministic guardrails, not toward a monolithic component.

## Scope

**In scope (if activated):** expressing one or more workflow phases as effect-driven orchestration; the boundary between CLI-owned deterministic transitions and component-owned sequencing; what becomes of skill markdown.

### Non-goals

- **No lifecycle authority moves out of the CLI.** `transition` / `journal` / lock ownership stay CLI-owned (roadmap principle: "Keep the CLI authoritative"); the component *requests* transitions as effects, it does not own them.
- **No removal of operator adaptability.** Phases that are primarily judgment/recovery stay agent-driven.

## The model (sketch)

`/spec:plan` becomes an orchestration component that issues effects — `infer(survey-brief)` per bound source, `infer(reconcile-leads)` for reconciliation — with the CLI's existing deterministic operations (`plan add`, validation, Gate 1) as **non-yielding** steps surfaced through the lifecycle effect. `/spec:execute` becomes the drained-loop reducer it already morally is. Skills thin from orchestrators to launchers.

## Decisions to record (open — and gating)

- **Which phases, if any, graduate.** Per-phase judgment: does this phase's value come from deterministic sequencing (graduate) or from adaptive judgment (stay agent-driven)?
- **Skill fate.** Whether skills dissolve into typed orchestration + `infer` bodies, or remain the operator entry surface with orchestration underneath.
- **Operator UX.** Whether moving orchestration prose into compiled control flow regresses the "tweak the skill" malleability operators rely on.
- **Lifecycle authority boundary.** Exactly which transitions remain CLI-only versus reachable as effects.
- **Activation trigger.** What evidence from RFC-53 (adapter orchestration in production, replay paying off, async ABI stable) justifies starting this stage at all.

## Phased plan (only if activated)

1. Pick one phase whose value is dominated by deterministic sequencing; express it as orchestration over effects with the CLI still owning transitions.
2. Add whole-phase record/replay; compare operator ergonomics against the prose skill it replaces.
3. Decide — with that evidence — whether to generalize, stop, or revert.

## Acceptance criteria (if activated)

1. At least one workflow phase runs as effect-driven orchestration with whole-phase record/replay.
2. The CLI retains sole lifecycle authority; the component reaches transitions only as effects.
3. The per-layer line (architecture north star) is respected — adaptive phases remain agent-driven; only deterministic-sequencing phases graduate.
4. Operator ergonomics are demonstrably not worse than the prose skill replaced.
5. `make lint` and `cargo make ci` stay green.

## Risks and invariants

- **Ossifying the fluid.** The chief risk is encoding adaptive, recovery-heavy orchestration as rigid control flow. The "against" case is the guard; if a phase needs the LLM's judgment to sequence, it does not belong here.
- **CLI authority.** Lifecycle authority must not migrate into components, services, or skills (roadmap Non-Goal).
- **Deferral is the default.** Absent a clear trigger and owner, this stage stays parked; RFC-53 is the coherent endpoint.
