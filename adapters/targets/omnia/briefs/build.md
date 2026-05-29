# Omnia target — build brief

> `/spec:build` loads this brief when it walks an `in-progress` plan entry whose slice has `target: omnia`. The brief dispatches to five phase sub-briefs under [`build/`](build/). Read this orchestrator linearly; load each phase sub-brief at the marked step, follow it end-to-end, and return here for the next step. Synthesis idioms (provider DI, WASM guardrails, error variants, validation placement) live in [`shape.md`](shape.md) and must already be reflected in the slice's `specs/<unit>/spec.md` + `design.md` before this brief runs.

## Inputs and bindings

```text
$SLICE_NAME    = active in-progress plan entry's slice name (from `specrun plan next`)
$SLICE_DIR     = .specify/slices/$SLICE_NAME
$UNIT_NAME     = unit slug from proposal.md ## Units (typically equals crate name for single-crate slices)
$SPEC_PATH     = $SLICE_DIR/specs/$UNIT_NAME/spec.md
$DESIGN_PATH   = $SLICE_DIR/design.md
$TASKS_PATH    = $SLICE_DIR/tasks.md
$CRATE_NAME    = $SLICE_NAME with kebab → snake (or the slice's plan-level `crate:` override)
$CRATE_PATH    = crates/$CRATE_NAME
$GUEST_PATH    = workspace root (single `src/lib.rs` exports HTTP / Messaging / WebSocket guests)
$REVIEW_OUTPUT = $CRATE_PATH/REVIEW.md
```

`/spec:build` resolves `$SLICE_NAME` from `specrun plan next`. The brief uses that name throughout.

## Mode detection

Check whether `$CRATE_PATH/Cargo.toml` exists:

- **Missing** → **create mode**: generate the crate, tests, and (if `src/lib.rs` is absent at the guest root) guest scaffolding.
- **Present** → **update mode**: incremental change against the existing crate; guest wiring updates are folded into the crate-writer step (skip the guest phase).

## Phase order

1. Read [`shape.md`](shape.md) refresher and the slice's `specs/<unit>/spec.md` + `design.md` + `tasks.md`.
2. Load and follow [`build/crate.md`](build/crate.md) — generates or updates the crate.
3. Load and follow [`build/test.md`](build/test.md) — generates or updates the tests.
4. (Create mode only) Load and follow [`build/guest.md`](build/guest.md) — scaffolds the WASM guest wrapper.
5. Run the § verify-repair loop below — cross-phase, classifies failures back to the matching phase brief.
6. Load and follow [`build/review.md`](build/review.md) — its remediation cycle may re-enter the verify-repair loop with tighter caps.
7. When the slice has a `captures` source binding, load and follow [`build/replay.md`](build/replay.md) — optional runtime capture replay. Omission when unbound is not an error.
8. Mark `tasks.md` checkboxes complete as each task lands; the slice transitions to `built` by `/spec:build` itself.

## § Verify-repair loop (max 3 iterations)

Run after both crate writer and test writer have completed. Each iteration runs the four checks below; if any fail, classify the failure, apply the targeted fix, and start a new iteration.

```bash
cd $CRATE_PATH && cargo fmt --check
cd $CRATE_PATH && cargo check
cd $CRATE_PATH && cargo clippy -- -D warnings
cd $CRATE_PATH && cargo test
```

If `cargo fmt --check` fails, run `cargo fmt` once. Formatting is mechanical; one pass suffices.

If `cargo check` or `cargo clippy` fails, re-enter [`build/crate.md`](build/crate.md) with the error output as context. Apply minimum-change repair discipline (see [`repair-patterns.md`](../references/repair-patterns.md)).

If `cargo test` fails, classify each failure:

| Failure signal | Classification | Fix action |
|---|---|---|
| Error in `tests/` paths, `MockProvider`, or `provider.rs` | Test issue | Re-enter [`build/test.md`](build/test.md) |
| Error in `src/` paths, missing trait impls in production | Code issue | Re-enter [`build/crate.md`](build/crate.md) |
| Assertion mismatch where *actual* matches spec | Test issue | Test expectation is stale |
| Assertion mismatch where *expected* matches spec | Code issue | Handler returns the wrong result |
| MockProvider missing a trait impl the handler now requires | Test issue | Update MockProvider |
| Unresolved import or missing crate in `Cargo.toml` | Workspace issue | Fix `Cargo.toml` paths or workspace member list directly |

**Repair discipline.** Minimum change only — fix the reported error and nothing else. Scope the diff to files and functions named in the error output. Group failures by classification and re-enter each phase brief once with all same-class errors. Full repair recipes: [`repair-patterns.md`](../references/repair-patterns.md).

**Update-mode regression check.** Before iteration 1, record the baseline: `cd $CRATE_PATH && cargo test 2>&1 | tee /tmp/${SLICE_NAME}-${CRATE_NAME}-baseline.txt`. After each iteration, for each test that passed before and now fails: if the spec explicitly changes the asserted behaviour → expected behavioural change, re-enter test writer to align expectations; if the spec does not change the asserted behaviour → true regression, route the fix through the classification table.

Repeat until all four checks pass or 3 iterations exhausted. If still failing after 3 iterations: **STOP**. Do not mark the slice complete. Report the remaining failures with full error output to the operator and signal the build phase outcome accordingly.

## § Stop hint contract

A build failure surfaces a stop hint as the body's final output — a single structured message the parent skill or the parent loop can act on without re-deriving context:

- `slice` — slice name from `specrun plan next`.
- `phase` — `build`.
- `failing-task` — the `tasks.md` checkbox (or sub-step) that exited non-zero.
- `log-path` — absolute path to the captured stdout/stderr.
- `next-action` — typically `re-run /spec:build $SLICE after fix`.

Render the hint as the final visible output of the run. Do not call `specrun slice transition` on the failure path — the slice stays `refined` so the loop (or a re-invocation) re-enters cleanly.

## § Deterministic review

Phase 6 writes `$REVIEW_OUTPUT` (`REVIEW.md`) — that is the model-assisted surface: specialist + antagonist judgment per [`review-team-protocol.md`](../references/review-team-protocol.md) and [`build/review.md`](build/review.md). `specrun review --format json` is the **deterministic complement**. It resolves applicable codex rules via `specrun codex export`, evaluates declarative `deterministic_hints`, and emits findings in the same `ReviewFinding` shape (`rule-id`, `fingerprint`, severity, `evidence`) operators already see in that export. The two surfaces are layered, not alternatives — model-assisted judgment sits on top of the deterministic scan.

Per RFC-32 [§"Principles"](../../../../rfcs/done/rfc-32-standards-enforcement.md#principles) — **"No lifecycle authority in review"** — deterministic findings may block CI but never transition plan entries, slices, or changes. CI wiring is consumer-project policy, not adapter policy; this brief acknowledges the surface and links out for the contract.

## References

- [`shape.md`](shape.md), [`merge.md`](merge.md) — sibling briefs.
- [`build/crate.md`](build/crate.md), [`build/test.md`](build/test.md), [`build/guest.md`](build/guest.md), [`build/review.md`](build/review.md), [`build/replay.md`](build/replay.md) — phase sub-briefs.
- [`../../../sources/captures/references/capture-format.md`](../../../sources/captures/references/capture-format.md) — runtime capture wire format (when `captures` is bound).
- [`hard-rules.md`](../references/hard-rules.md) — full authority hierarchy and hard-rules set.
- [`guardrails.md`](../references/guardrails.md), [`wasm-constraints.md`](../references/wasm-constraints.md) — forbidden crates / APIs, statelessness, serde / DST idioms.
- [`capabilities.md`](../references/capabilities.md), [`capability-mapping.md`](../references/capability-mapping.md) — provider traits and artifact-to-trait mapping.
- [`sdk-api.md`](../references/sdk-api.md), [`cargo-toml.md`](../references/cargo-toml.md), [`error-handling.md`](../references/error-handling.md), [`configuration.md`](../references/configuration.md) — SDK / workspace / error / guest-config templates.
- [`cross-cutting-matrices.md`](../references/cross-cutting-matrices.md), [`update-patterns.md`](../references/update-patterns.md), [`change-classification.md`](../references/change-classification.md), [`repair-patterns.md`](../references/repair-patterns.md), [`todo-markers.md`](../references/todo-markers.md), [`checklists.md`](../references/checklists.md), [`output-documents.md`](../references/output-documents.md) — analysis tables, strategy patterns, recipes.
- [`mock-provider.md`](../references/mock-provider.md), [`spec-to-test-mapping.md`](../references/spec-to-test-mapping.md), [`replay-fixtures.md`](../references/replay-fixtures.md), [`replay-crate-layout.md`](../references/replay-crate-layout.md) — test depth.
- [`handlers.md`](../references/handlers.md), [`guest-patterns.md`](../references/guest-patterns.md), [`guest-wiring.md`](../references/guest-wiring.md), [`runtime.md`](../references/runtime.md), [`project-layout.md`](../references/project-layout.md) — guest depth.
- [`review-categories.md`](../references/review-categories.md), [`review-team-protocol.md`](../references/review-team-protocol.md), [`review-auto-fix.md`](../references/review-auto-fix.md), [`review-output-template.md`](../references/review-output-template.md), [`agent-teams.md`](../references/agent-teams.md), [`../codex/`](../codex/) (Omnia overlay), [`../../../shared/codex/universal/`](../../../shared/codex/universal/) (shared `UNI-*`) — review depth.
- [`providers/`](../references/providers/) — per-trait deep dives.
- [`examples/`](../references/examples/) — worked examples for crate writing (single/multi-handler, per-capability, per-update-category) and test writing (per-provider).
