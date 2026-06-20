# RFC-55: Workflow and Development as Guests (Stage 4 — The Thin Interpreter)

> Status: Draft (skeleton) · Implements: the effect-oriented architecture (Stage 4 — guests) · Depends: [RFC-54](rfc-54-omnia-runtime-move.md) (the Omnia runtime move), RFC-53 (orchestration components proven on the adapter axis) · Per-phase compile-vs-delegate: **gated on RFC-53 data**

## Abstract

Once [RFC-54](rfc-54-omnia-runtime-move.md) has put the generic runtime under everything, two first-party concerns still run *beside* it rather than *on* it: the **workflow** (`/spec:plan`, `/spec:execute`, the slice loop) and the framework's own **development tooling** (authoring and standards checks). This RFC moves both onto the runtime as guests, so the architecture's "everything is a guest" claim becomes literally true and the runtime is the thin interpreter the stage title names. The workflow move is where the one genuinely contested decision lives — *how much* of each phase compiles into the guest versus stays agent-driven behind `infer` — and that per-phase compile-vs-delegate call is what this RFC exists to gate with data from RFC-53. The development-guest move is the mechanical tail.

## Motivation (and the case against)

**For:** the workflow lifecycle is a state machine (plan → approved → execute loop → finalize) whose deterministic skeleton already lives in the CLI; formalizing the remaining glue as orchestration over effects would give whole-workflow record/replay, typed control flow, and one uniform model across adapters and workflow alike.

**Against (load-bearing):** the workflow is exactly where the model's adaptability — handling unexpected project states, operator intent, recovery — is a *feature*, and where the existing guardrails (`plan.lock`, Gate 1, `specify plan next` refusing illegal transitions) already bound any agent fumbling. Prose is also more malleable for operator-facing UX evolution. **This RFC must clear the "against" case before any phase graduates to compiled orchestration**, per the architecture's per-layer line: the workflow leans toward an agent-driver over deterministic guardrails, not toward a monolithic component.

## Scope

**In scope:** running the workflow as a guest on the RFC-54 runtime; per-phase expression of one or more workflow phases as effect-driven orchestration; the boundary between runtime-owned deterministic transitions and guest-owned sequencing; what becomes of skill markdown; and moving the framework's development / standards tooling onto the runtime as a guest.

### Non-goals

- **No lifecycle authority moves into the guest.** `transition` / `journal` / lock ownership stay in the runtime's deterministic lifecycle host service (roadmap principle: "Keep the CLI authoritative" — the authority stays on the deterministic floor, not in a model-driven guest); the workflow guest *requests* transitions as effects, it does not own them.
- **No removal of operator adaptability.** Phases that are primarily judgment / recovery stay agent-driven behind `infer`.
- **No runtime engineering.** The generic binary, the effect backends, instance-per-call — all of that is [RFC-54](rfc-54-omnia-runtime-move.md); this RFC consumes it.

## The model (sketch)

`/spec:plan` becomes an orchestration guest that issues effects — `infer(survey-brief)` per bound source, `infer(reconcile-leads)` for reconciliation — with the runtime's deterministic operations (`plan add`, validation, Gate 1) as **non-yielding** steps surfaced through the lifecycle effect. `/spec:execute` becomes the drained-loop reducer it already morally is. Skills thin from orchestrators to launchers.

## Development tooling as a guest

The framework's own authoring and standards tooling — `specify lint framework`, `rules export`, the `CORE-*` checkers — is itself Specify behaviour, so by the architecture's own rule it belongs in a guest too. It is the lowest-urgency move (the tooling works as the CLI today and gates nothing) and mechanically simpler than the workflow: it carries no contested compile-vs-delegate question because the standards checks are already deterministic. It rides the same contract and runtime, is sequenced last, and may be deferred indefinitely without blocking the rest of the architecture.

## Decisions to record (open — and gating)

- **Which phases, if any, graduate.** Per-phase judgment: does this phase's value come from deterministic sequencing (graduate) or from adaptive judgment (stay agent-driven)?
- **Skill fate.** Whether skills dissolve into typed orchestration + `infer` bodies, or remain the operator entry surface with orchestration underneath.
- **Operator UX.** Whether moving orchestration prose into compiled control flow regresses the "tweak the skill" malleability operators rely on.
- **Lifecycle authority boundary.** Exactly which transitions remain runtime-only versus reachable as effects.
- **Per-phase activation trigger.** What evidence from RFC-53 (adapter orchestration in production, replay paying off, async ABI stable) justifies graduating a given phase to compiled orchestration.
- **Development-guest scope.** Whether the development tooling becomes a guest at all, and which of its operations are worth moving off the CLI.

## Phased plan (per-phase, gated)

1. Run the workflow as a guest on the RFC-54 runtime with **every** phase still agent-driven behind `infer` — the no-compile baseline that proves the move without ossifying anything.
2. Pick one phase whose value is dominated by deterministic sequencing; express it as orchestration over effects with the runtime still owning transitions.
3. Add whole-phase record/replay; compare operator ergonomics against the prose skill it replaces.
4. Decide — with that evidence — whether to generalize, stop, or revert; move the development tooling onto the runtime if and when it earns the change.

## Acceptance criteria

1. The workflow runs as a guest on the RFC-54 runtime; the bespoke driver is gone.
2. The no-compile baseline holds: every phase is reachable behind `infer`, with lifecycle authority still in the runtime.
3. Any graduated phase runs as effect-driven orchestration with whole-phase record/replay, and the runtime retains sole lifecycle authority.
4. The per-layer line is respected — adaptive phases remain agent-driven; only deterministic-sequencing phases graduate.
5. Operator ergonomics are demonstrably not worse than the prose skill replaced.
6. `make lint` and `cargo make ci` stay green.

## Risks and invariants

- **Ossifying the fluid.** The chief risk is encoding adaptive, recovery-heavy orchestration as rigid control flow. The "against" case is the guard; if a phase needs the model's judgment to sequence, it does not belong here.
- **Lifecycle authority.** Authority must not migrate into guests, services, or skills (roadmap Non-Goal); it stays in the runtime's lifecycle host service.
- **Per-phase deferral is the default.** The runtime move (RFC-54) is committed, but absent a clear trigger and owner *no individual phase compiles* — each stays agent-driven behind `infer` until its case is made. RFC-53 is the last unconditional stage; graduating a workflow phase past it is opt-in and evidence-gated, not the architecture's stopping point.
