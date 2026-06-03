# RFC-39 — Multi-repo acceptance proof (RM-05)

## Status

Proposed. Implements roadmap item [RM-05](roadmap.md#rm-05-multi-repo-acceptance-suite) — the 2.0.0 release gate. This RFC turns the pending acceptance queue into an ordered, halt-gated execution plan with a fixed definition of done; it does not add new product behaviour.

## Motivation

Specify's deterministic substrate is well covered: the fan-in / fan-out path runs under `cargo make test` in `specify-cli` ([`tests/fan_in_fan_out.rs`](https://github.com/augentic/specify-cli/blob/main/tests/fan_in_fan_out.rs)), and `make lint` pins repo consistency. Neither surface exercises the **agent-driven** `/spec:plan → Gate 1 → /spec:execute → /spec:finalize` loop on realistic multi-repo flows.

The 20-scenario queue at [`tests/cross-repo/runs/2.0.0/`](../tests/cross-repo/runs/2.0.0/README.md) is written but **every run-summary is still pending**. Until scenario #1 passes and the queue drains, 2.0.0 has no end-to-end proof that an operator can drive a change from intent to merged PRs. This RFC defines how that proof is produced, ordered, and recorded so the release gate has an unambiguous green/red signal.

## Non-goals

- **No automated runner, fake forge, transcript replay, CI target, or required golden-output comparison.** These are the standing negative expectations in [`scenario.md`](../tests/cross-repo/scenario.md) and every queue stub; this RFC preserves them. Acceptance stays operator-driven against live `cursor-agent`.
- **No synthesis byte-replay.** Pinning the exact bytes a `/spec:refine` or `/spec:build` body emits is explicitly deferred ([`acceptance.md` §Synthesis byte-replay](../docs/contributing/acceptance.md#synthesis-byte-replay-deferred)) to a separate follow-up RFC. RM-05 asserts durable structure and state, not bytes.
- **No new scenarios beyond the documented gap.** The only fixture this RFC adds is the stale-workspace recovery scenario the roadmap already flags as missing.

## Design

### The two acceptance surfaces

A release is proven only when **both** surfaces are green:

1. **Deterministic CLI proof — automated, already shipped.** `tests/fan_in_fan_out.rs` asserts the envelope, ordering, and re-projection determinism of the whole CLI path. No work here beyond keeping it green.
2. **Operator scenario sweep — manual, the RM-05 debt.** The 20 cross-repo scenarios plus the per-target generated-output-correctness gate (`cargo check` / `cargo test` / target replay suites for emitted code). A schema-valid `build/report.yaml` with `status: success` proves the envelope held, not that the generated code compiles or replays.

### Execution order and the halt gate

The queue is drained in three waves. Each scenario fills the matching `<scenario-id>.md` stub from [`run-summary-template.md`](../tests/cross-repo/run-summary-template.md) and flips its **Status:** to `passed` / `failed` / `deferred`.

1. **Wave 0 — release blocker.** Scenario #1 (pure intent, N=1). **Hard halt:** if #1 fails, file the failure into `01-pure-intent.md`, do not run any other scenario, triage, then resume from #1 once green. No scenario in Wave 1 or 2 is meaningful while #1 is red.
2. **Wave 1 — core synthesis + routing.** Scenarios #2–#7 and the `5x` reconciliation set (`5`, `5a`, `5b`, `5c`, `5e`) — happy-path planning, multi-slice, multi-repo routing, authority/conflict tagging, and Gate-1 amend.
3. **Wave 2 — failure and breakout paths.** The negative and recovery scenarios (`5f`, `5g`, `5h`, `5j`, `8`, `9`, `10`, `11`, `12`) plus the new stale-workspace recovery scenario below.

Within a wave, scenarios are independent and may run in any order; a failure outside Wave 0 is recorded and triaged but does not halt sibling runs.

### New fixture: stale-workspace recovery

The roadmap's one acknowledged fixture gap. Add a stub under `tests/cross-repo/runs/2.0.0/` (id `13`, registered in the queue README and [`acceptance.md` §Scenario IDs](../docs/contributing/acceptance.md#scenario-ids)) that exercises re-entry after a workspace slot is left dirty mid-execute: operator interrupts `/spec:execute`, a slot has uncommitted work, and a fresh `specrun workspace sync` + resume must reconcile cleanly without losing slice state. It reuses the existing scenario stub shape — no runner, no fake forge.

### Evidence and the gate signal

- Each run commits its filled `<scenario-id>.md` as the audit trail.
- On failure, the operator preserves the workspace state, `plan.yaml`, `registry.yaml`, push/finalize output, and branch/PR identifiers per the template, and files a follow-up issue in `augentic/specify` linked back to the run-summary.
- The **release gate is green** when: `tests/fan_in_fan_out.rs` passes under `cargo make test`, scenario #1 is `passed`, and every non-deferred queue entry is `passed`. A `deferred` entry (capability genuinely missing on the binary under test) must carry a linked follow-up issue and an explicit release-owner sign-off.

## Migration sequencing

1. Run Wave 0 against a 2.0 `specrun` (`SPECIFY_BIN` exported per the queue README). Halt-gate until #1 is `passed`.
2. Add the stale-workspace recovery stub (id `13`) before Wave 2 begins.
3. Drain Wave 1, then Wave 2, filling each stub and filing follow-ups for any gap the deterministic harness does not cover.
4. When the whole queue is `passed` (or `deferred` with sign-off), record the gate as green in the queue README and flip RM-05 from *Partial* to *Done* in [`roadmap.md`](roadmap.md).

## Done definition

- [ ] Scenario #1 (`01-pure-intent.md`) is `passed`; the halt gate is cleared.
- [ ] Stale-workspace recovery stub (`13`) added to the queue README and `acceptance.md` scenario-id table.
- [ ] Every Wave 1 and Wave 2 scenario is `passed`, or `deferred` with a linked follow-up issue and release-owner sign-off.
- [ ] `tests/fan_in_fan_out.rs` is green under `cargo make test` for the binary under test.
- [ ] Per-target generated-output-correctness gate run for each exercised target (Omnia, Vectis, contracts); no slice marked done on a build whose generated code fails `cargo check` / `cargo test` / the target replay suite.
- [ ] Negative expectations held across the sweep (no runner, fake forge, transcript replay, CI target, or golden-output requirement added).
- [ ] RM-05 flipped to *Done* in `roadmap.md`; release gate recorded green in the queue README.

## Cross-repo touchpoints

| Change | Repo | Files |
| --- | --- | --- |
| Filled run-summaries (audit trail) | specify | `tests/cross-repo/runs/2.0.0/<scenario-id>.md` |
| Stale-workspace recovery stub | specify | `tests/cross-repo/runs/2.0.0/13-stale-workspace-recovery.md`, `runs/2.0.0/README.md` |
| Scenario-id registration | specify | `docs/contributing/acceptance.md` §Scenario IDs |
| Roadmap status flip | specify | `rfcs/roadmap.md` (RM-05 → Done) |
| Deterministic proof (kept green) | specify-cli | `tests/fan_in_fan_out.rs` |
