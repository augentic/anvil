# RFC-90: Build Verification

> Status: Draft — step 5 of the platform-migration series, scale track ([platform.md](platform.md))
>
> Owns: the engine-owned build phase machine, separate `target.build` / `target.repair` / `target.verify` / `target.review` WIT operations, bounded verification and repair rounds, typed intermediate reports, and removal of repair-loop control from adapter prose.
>
> Builds on completed [RFC-87](rfc-87-working-trees.md). The observable sequential gate becomes [RFC-91](rfc-91-concurrent-execution.md)'s worker and conflict-domain convergence loop; [RFC-92](rfc-92-node-sync.md) transports its phase records without redefining it. Deterministic native-tool verification is deferred until WASI exposes an execution API suitable for host toolchains.



## Intent

Move build verification and repair control out of opaque agent prose and into the workflow engine before making builds concurrent.

### Today

The engine dispatches one `target.build` call for a slice. Inside that call the Omnia target runs preparation, generation, optional replay, verification and repair, standards review, and review remediation. Cargo commands, retry counts, failure routing, and stopping conditions live in prompt text. The engine observes only the final report.

That shape hides the most important control loop from lifecycle policy and makes RFC-91 concurrency unsafe: splitting writers before replacing the loop would remove the only mechanism that currently returns failed checks to the code writer.

The engine guest also cannot execute the target project's native toolchain. Its `workflow` world imports `source`, `target`, and `workspaces`; neither Emery nor Omnia currently supplies native process execution. The `wasi-model` `verify` grant authorizes model tooling but is not an execution capability.

### Goal

The engine owns a closed sequence of **build phases** over one disposable RFC-87 workspace:

```text
build → verify ⇄ repair → review ⇄ repair → complete
```

The target adapter still owns specialist behavior: generation prompts, platform-specific checks, repair instructions, and engineering-standards review. Each adapter invocation is exactly one WIT operation — `build`, `verify`, `repair`, or `review` — and returns a typed **phase report**. The engine persists the report, decides the next operation, enforces repair budgets, and assembles the final build report.

Verification in this RFC remains **model-assisted**. During a `verify` call, the target's agent runs its declared commands inside the lent workspace and returns findings. Moving orchestration into the engine makes the loop bounded and observable; it does not make command selection, execution, or findings deterministic or trustworthy.

## Flow and terms

1. The engine prepares one writable RFC-87 **build workspace** from the recorded target base.
2. It dispatches `build` once. Adapter-internal preparation and target-specific writer ordering may remain inside that call, but generation may not run a verification or repair loop.
3. It dispatches `verify` once. The adapter runs its current model-assisted checks and returns one phase report.
4. Blocking verification findings cause one `repair` dispatch carrying those exact findings, followed by another `verify`, while the verification-repair budget remains.
5. A passing verification dispatches `review` once. Blocking review findings route through one `repair → verify → review` remediation round while the review-remediation budget remains.
6. The engine assembles `build/report.yaml` from the terminal phase reports, applies the existing blocking-finding and output-existence gates, captures the workspace result, and completes the `built` transition.
7. Budget exhaustion returns a typed failed build report with the unresolved findings. The workspace is discarded without changing authoritative target or slice state.

A **build phase** is one engine-selected target operation: `build`, `verify`, `repair`, or `review`. A **phase report** is the typed result of exactly one operation. A **repair round** is one repair dispatch followed by the verification and, when applicable, review needed to decide whether that repair succeeded.

## Worked example

Suppose an Omnia-bound build generates a payment handler. The engine dispatches `build`, then `verify`. The verification agent runs the adapter's existing Rust checks and returns a blocking finding for an avoidable clone in `crates/payments/src/handler.rs`.

The engine persists that verification report and dispatches `repair` with the exact finding and the generation continuation. The adapter performs one repair pass and returns. The engine—not prompt prose—then dispatches `verify` again, decrements the verification-repair budget, and records the result.

After verification passes, the engine dispatches standards `review`. If review reports a blocking API-design finding, the engine permits the configured review-remediation round, verifies the repaired tree again, and reruns review. If the finding remains, the final build report fails with that finding. Every phase is visible even though the agent still owns the Cargo shell calls.

## Decisions



### D1 — The engine owns the build phase machine

Operation order, repair routing, budgets, terminal success, and terminal failure are deterministic engine policy. A target adapter cannot select its next operation, reset a budget, silently retry, or claim completion while a blocking phase report remains.

The initial policy permits at most three verification repair dispatches and one review remediation dispatch. These are engine constants, not fields supplied by a model or adapter.

### D2 — Separate WIT operations; no `build-phase` enum

The target axis grows from three operations to six: `guidance`, `build`, `repair`, `verify`, `review`, and `merge`. Generation, repair, verification, and standards review are distinct methods. There is no `build-phase` enum and no phased overload of a single `build` call.

`wit/emery.wit` adds a shared `phase-report` (outcome, findings with optional artifact-relative path and span, outputs, UI surface, written paths, and next continuation state), and richer findings than today's compact WIT `finding` so the engine can route exact locations into `repair`. The caller widens that projection into the shared `Diagnostic`. There is no `build-scope` variant: every operation names a slice (or, for `verify`, only the workspace). Domain identity for RFC-91 convergence stays engine-side; the adapter runs the same check pass against whatever workspace the engine lends.

Conceptually:

```wit
/// Generation only: preparation, writers, capture replay.
/// Must not verify, repair, or run standards remediation.
build: async func(
  id: adapter-id,
  slice: string,
  inputs: list<input>,
  context: build-context,
  workspace: workspace,
) -> result<phase-report, error>;

/// One model-assisted check pass on the lent workspace.
verify: async func(
  id: adapter-id,
  workspace: workspace,
) -> result<phase-report, error>;

/// One findings-directed repair pass.
repair: async func(
  id: adapter-id,
  slice: string,
  findings: list<phase-finding>,
  continuation: list<u8>,
  workspace: workspace,
) -> result<phase-report, error>;

/// One engineering-standards review pass.
review: async func(
  id: adapter-id,
  slice: string,
  continuation: option<list<u8>>,
  workspace: workspace,
) -> result<phase-report, error>;
```

Continuation state is an adapter-opaque byte payload that may represent several writer or reviewer sessions. The engine persists and echoes it only to the same target identity and build workspace. It never interprets it or treats it as lifecycle authority.

Separating `verify` and `review` keeps mechanical checks and standards judgment as different product surfaces: different prompts, budgets, and (later) execution trust. RFC-91 reuses the same `verify` against a composed candidate workspace; domain id never crosses this seam.

### D3 — Every dispatch performs exactly one operation

`build` may perform target-specific preparation, scaffolding, writer ordering, and capture replay needed to produce the candidate. It must not verify, repair, or run standards remediation.

`verify` performs one model-assisted check pass and reports its findings. `repair` receives the engine-selected unresolved findings and performs one repair pass. `review` performs one standards review pass and reports its findings. Prompts may describe how to perform their single operation; they may not contain retry loops or choose another operation.

### D4 — Findings are the repair currency

Phase findings use the shared diagnostic shape and artifact-relative locations. The engine routes the exact unresolved finding set into `repair`; prose does not reconstruct failures from a transcript or decide which failures count.

Because verification remains model-assisted, its findings use `source: model-assisted` or `source: hybrid` as appropriate. This RFC does not label them `source: tool`: the engine does not receive and parse trusted tool output.

### D5 — One workspace spans the complete loop

All phases for one build run against the same disposable RFC-87 workspace. Generation, verification, repair, and review therefore observe one candidate tree, and existing workspace-local build artifacts may be reused naturally.

The engine captures only after terminal success. A dispatch error, budget exhaustion, blocking final report, or cancellation discards the workspace; none writes into authoritative source, target, or slice state.

### D6 — Phase reports are durable and the final report stays authoritative

The engine writes each successful dispatch result under `.emery/slices/<slice>/build/phases/` in ordinal order and emits a `slice.build.phase-completed` event naming its digest and which operation produced it. The existing `build/request.yaml` remains the immutable build input record. The engine alone assembles the terminal `build/report.yaml`, which remains the report consumed by lifecycle gates and later domain convergence.

Phase reports are evidence of orchestration, not new lifecycle states. The slice still transitions once from `refined` to `built`.

### D7 — Existing target behavior moves without changing ownership

Omnia splits its current preparation/generation/replay work into `build`, its Cargo check pass into `verify`, one findings-directed writer pass into `repair`, and one engineering-standards pass into `review`. Vectis and contracts implement the same four operations with their own specialist behavior; an inapplicable operation returns a typed non-blocking report rather than inventing adapter-specific operation names.

The adapter repository continues to own prompts, engineering standards, target composition, and report content. Emery owns the operation vocabulary, loop, persistence, budgets, and final lifecycle gate.

### D8 — Deterministic native verification is deferred

Closed command profiles, native process execution, sandbox and resource policy, trusted tool-output parsing, cache isolation across policies, execution grants, and typed toolchain unavailability are not part of this RFC.

That work depends on `[wasi:exec](https://github.com/WebAssembly/WASI/issues/899)` or another standardized WASI capability that can run native toolchains with a working directory, controlled environment, bounded stdio, cancellation, resource limits, and enforceable sandbox policy. The current `wasi:exec` discussion does not yet guarantee those semantics.

When that capability exists, a follow-up replaces the model-assisted implementation of `verify`; the engine phase machine, repair routing, separate WIT operations, workspace bracket, and final report assembly remain unchanged.

## Implementation requirements

- Extend `wit/emery.wit`, the adapter SDK `Target` trait and export macro, the guest provider, native provider, mock catalog, and wire DTOs with `repair`, `verify`, and `review` alongside the narrowed `build`, plus shared phase-report types.
- Replace the single target-build dispatch in `slice::orchestrate` with the closed engine-owned phase machine over those four operations, including the initial three verification repair dispatches and one review remediation dispatch.
- Persist ordinal phase reports and continuations, add the `slice.build.phase-completed` event (naming the operation), and keep `build/report.yaml` as the engine-assembled terminal authority.
- Keep one RFC-87 workspace for the complete loop; capture only on terminal success and discard on every failure or cancellation path.
- Split Omnia, Vectis, and contracts target implementations across `build` / `repair` / `verify` / `review` without moving specialist prompts or engineering standards into Emery.
- Delete verification, retry, writer re-entry, and review-remediation loops from adapter prose. An operation prompt may perform one pass only.
- Preserve the existing final blocking-finding, declared-output, patch-capture, and `refined → built` gates.
- State explicitly in operator output and phase reports that verification is model-assisted; do not claim deterministic, sandboxed, or trusted native execution.



## Acceptance criteria

1. One build produces an ordered persisted sequence of engine-selected `build`, `verify`, optional `repair`, and `review` phase reports before one final build report.
2. No target prompt contains a verification-repair or review-remediation retry loop; injected failures return to the engine after one operation.
3. Verification failures route as exact typed findings to one `repair` dispatch, then back through `verify`. The fourth requested verification repair and the second requested review remediation are refused by engine policy.
4. The adapter cannot select the next operation, alter the attempt count, reset a budget, or write the terminal build report.
5. The same RFC-87 workspace id is used throughout one build. Failed or exhausted builds leave authoritative target and slice state unchanged.
6. Omnia/Rust preserves its current generation, replay, Cargo-check, repair, standards-review, output, and finding behavior through the four operations.
7. Vectis and contracts implement every operation, using typed non-applicability where an operation has no target-specific work.
8. Native and Wasm integration suites cover success, verification repair, review remediation, both budget exhaustions, continuation routing, phase-report persistence, workspace discard, and final report assembly.
9. `wasm-omnia-r9k` records operation count, model calls, repair routing, and wall time; every formerly hidden retry is visible as an engine phase.
10. Documentation describes this gate as model-assisted and identifies deterministic native verification as deferred on a suitable WASI execution capability.



## Rejected alternatives

- **Wait for** `wasi:exec` **before exposing the loop** — leaves retry authority hidden in prose and blocks RFC-91's observable worker orchestration on an unsettled external API.
- **Move the loop from prose into adapter Rust** — improves prompt size but leaves operation order, budgets, and retries adapter-owned and invisible to the workflow engine.
- **Have the engine repeat today's monolithic** `target.build` — replays preparation, generation, and review against a mutating workspace without identifying which action is intended.
- **One** `build` **method with a** `build-phase` **enum** — collapses generation, repair, verification, and standards review behind a stage parameter. Rejected so `verify` and `review` stay distinct product surfaces (different prompts, budgets, and later execution trust) and RFC-91 can call `verify` alone without a phase discriminator.
- **A** `build-scope` **(**`slice` **|** `domain`**) parameter on** `verify` — adapter check behavior does not branch on scope; domain identity belongs in engine-side round records. Deferred until something in the adapter needs it.
- **Let adapters define arbitrary operation names or next actions** — fragments workflow policy and allows an adapter response to control orchestration.
- **Call model-assisted shell execution deterministic verification** — overstates the trust boundary; the agent still controls command invocation, output interpretation, and reporting.

