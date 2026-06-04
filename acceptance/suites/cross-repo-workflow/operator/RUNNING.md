# Running the cross-repo acceptance suite

This is the operator's guide to producing the 2.0.0 release-gate proof: the agent-driven `/spec:plan → Gate 1 → /spec:execute → /spec:finalize` loop exercised across the cross-repo scenario queue. It defines the two acceptance surfaces, the wave ordering and halt gate, the meta-prompt-driven run flow, and the green-gate signal.

## The two acceptance surfaces

A release is proven only when **both** surfaces are green:

1. **Deterministic CLI proof — automated, already shipped.** `tests/fan_in_fan_out.rs` in [`augentic/specify-cli`](https://github.com/augentic/specify-cli/blob/main/tests/fan_in_fan_out.rs) asserts the envelope, ordering, and re-projection determinism of the whole CLI path. It runs under `cargo make test`. No work here beyond keeping it green.
2. **Operator scenario sweep — manual.** The cross-repo scenarios in [`queue/`](queue/README.md) plus the per-target generated-output-correctness gate. A schema-valid `build/report.yaml` with `status: success` proves the build envelope held, not that the generated code compiles or replays — so each exercised target must also pass `cargo check` / `cargo test` / its replay suite (and the equivalent verification for non-Rust targets). A slice whose generated output fails these checks is not done, regardless of envelope validity.

This pack is intentionally **not** an automated harness: no runner, fake forge, recorded transcript, CI target, or golden-output comparison. See the negative expectations in [`scenario.md`](scenario.md).

## Execution order and the halt gate

The queue is drained in three waves. Each scenario fills its matching `<scenario-id>.md` stub from [`run-summary-template.md`](run-summary-template.md) and flips its **Status:** to `passed` / `failed` / `deferred`.

1. **Wave 0 — release blocker.** Scenario #1 (pure intent, N=1). **Hard halt:** if #1 fails, file the failure into [`queue/01-pure-intent.md`](queue/01-pure-intent.md), do not run any other scenario, triage, then resume from #1 once green. No scenario in Wave 1 or 2 is meaningful while #1 is red.
2. **Wave 1 — core synthesis + routing.** Scenarios #2–#7 and the `5x` reconciliation set (`5`, `5a`, `5b`, `5c`, `5e`) — happy-path planning, multi-slice, multi-repo routing, authority/conflict tagging, and Gate-1 amend.
3. **Wave 2 — failure and breakout paths.** The negative and recovery scenarios (`5f`, `5g`, `5h`, `5j`, `8`, `9`, `10`, `11`, `12`) plus the stale-workspace recovery scenario (`13`).

Within a wave, scenarios are independent and may run in any order; a failure outside Wave 0 is recorded and triaged but does not halt sibling runs.

## Run flow (meta-prompt-driven)

Build a 2.0 `specify` binary in the sibling [`specify-cli`](https://github.com/augentic/specify-cli) repo and export `SPECIFY_BIN=/abs/path/to/specify`. The `PATH` default `specify` is the historical 0.1.0 build and is **not** the 2.0 binary.

For each scenario, paste the two reusable prompts from [`meta-prompts.md`](meta-prompts.md) into a live `cursor-agent` session, in order:

1. **Prompt A — setup** brings up a fresh disposable environment (init, registry, brief) using real CLI only, and stops before `/spec:plan`.
2. **Prompt B — run + confirm** drives the full lifecycle, captures per-stage output, self-grades the structurally-checkable assertions and negative expectations, and fills the run-summary into the stub.

The prompts are operator aids, not a harness — they hand back to the operator at three human seams: real forge merges between the two `/spec:finalize` invocations, ergonomics/judgment assertions (marked `needs-human`), and `deferred` / scenario-#1 sign-off.

Operators who prefer a fully manual run can follow [`scenario.md`](scenario.md) and the per-scenario stub directly instead of using the prompts.

## Evidence and the gate signal

- Each run commits its filled `<scenario-id>.md` as the audit trail.
- On failure, preserve the workspace state, `plan.yaml`, `registry.yaml`, push/finalize output, and branch/PR identifiers per the template, and file a follow-up issue in `augentic/specify` linked back to the run-summary.
- The **release gate is green** when: `tests/fan_in_fan_out.rs` passes under `cargo make test`, scenario #1 is `passed`, and every non-deferred queue entry is `passed`. A `deferred` entry (capability genuinely missing on the binary under test) must carry a linked follow-up issue and an explicit release-owner sign-off.

When the whole queue is `passed` (or `deferred` with sign-off), record the gate as green in the [queue README](queue/README.md) and flip RM-05 from *Partial* to *Done* in [`rfcs/roadmap.md`](../../../rfcs/roadmap.md).

## See also

- [`README.md`](README.md) — scenario-pack overview and shape.
- [`scenario.md`](scenario.md) — the shared cross-repo operator script.
- [`run-summary-template.md`](run-summary-template.md) — the field-set every run fills.
- [`queue/README.md`](queue/README.md) — the scenario queue and status legend.
- [`meta-prompts.md`](meta-prompts.md) — the setup and run + confirm operator prompts.
