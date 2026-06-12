# Run: `execute-fail-resume` — **pass**

## Context

- **Scenario:** `execute-fail-resume`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook; operator seams driven at the operator's standing direction)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.toml` `cli` source via `make install-cli`)
- **Sandbox:** `evals/.sandbox/execute-fail-resume/`

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `build-failure-stop-hint` | pass | |
| `build-resumes-from-failed-task` | pass | |
| `loop-continues-to-merge` | pass | |

Probe transcript highlights: at the park, `slice.build.failed` carried `{"slice-name":"rate-limit","reason":"target-build-failed"}` and `specify plan next` returned `{"reason":"in-progress","active":"rate-limit"}` with the entry left `in-progress`; the loop's final output was the structured `stop: build-failed` hint naming task `4.1 verify-repair loop (cargo test)` blocked on `1.2 operator-supplied ops/quota.txt`, plus the `.build-log` path. After the fix, the journal reads `slice.build.failed → slice.build.started → slice.build.succeeded` with no `slice.synthesize.*` between and exactly one `plan.entry.advanced` for the whole run (resume, not restart); the completed `tasks.md` checkboxes (1.1/2.1/3.1) survived the park and the build re-entered in update mode. The resumed loop ran the omnia pre-merge gate (crate fmt/clippy `--all-targets`, workspace check, crate tests 8/8, `cargo build --target wasm32-wasip2 --release --workspace`), then `slice.archive.created` + `slice.merge.succeeded` fired and `specify plan next` drained.

**Negative expectations:** held (manual-by-design posture unchanged; live interactive drive against the real CLI and a real cargo workspace).

## Deviations

- `specify init omnia@v1` substituted with the documented offline fallback `specify init ../../../adapters/targets/omnia` (local adapter path), as in the `intent-only` run; `intent` source adapter symlinked into the sandbox.
- Gate 1 stamped with `--actor agent` (agent stamping at the operator's standing direction; the journal payload records who stamped).
- The engineered first-attempt failure rides the scenario's "task that needs an operator-supplied fix": spec REQ-003 pins the enforced quota to an operations-supplied fixture (`crates/rate_limit/ops/quota.txt`, tasks.md task 1.2), so the verify-repair loop honestly exhausts its 3-iteration budget — fmt/clippy and a test-writer type-annotation repair land in iterations 1–2; the missing operator fixture is the sole residue (the build may not invent the approved value per the artifact authority hierarchy). The operator fix supplied the fixture; resume went green.
- Guest scaffolding kept lean for the sandbox: no `.github/` workflows, `Makefile.toml`, `deny.toml`, `supply-chain/`, or `examples/<guest>.rs` native runtime (and hence no native-only `wasmtime` dev-deps). Workspace root, config templates, wasm32-gated `src/lib.rs` guest, and `examples/.env.example` were generated per the brief.
- `flock(1)` absent on this macOS host; the plan lock used `plan-lock.md`'s documented Python `fcntl` fallback, wrapped per CLI invocation (the lock guards every plan-state write; agent file edits between locked calls hold no lock).
- Build-phase code review run by a single agent walking the SEC / COR / QUA / UNI categories plus an antagonist pass sequentially (REVIEW.md per the output template), not as concurrently spawned specialist subagents.

## Notes

- Merge-gate contract seam worth a doc pass: the omnia merge brief routes a gate clippy failure to "re-enter `/spec:build`", but `/spec:build` refuses post-`refined` slices ("no rebuild needed" on `built`). This run hit the seam live — the gate's `cargo clippy --all-targets` flagged test-side pedantic lints the build loop's bare `cargo clippy` (lib-only) does not cover. Repaired mechanically in place and re-ran the gate; the catalog's loop mechanics were unaffected. Candidate fixes: align the verify-repair loop on `--all-targets`, or give the merge brief an explicit gate-repair allowance.
- The `429` denied path returns `Reply::ok(decision).status(429)` per design; the wasm32-wasip2 release build of the full workspace (guest + crate against omnia-sdk 0.33.0) is the definitive gate and passed.
- REVIEW.md carries three suggestion-grade findings (non-atomic read-modify-write counter — scoped out by `proposal.md` non-goals; TTL-slide window anchoring — provider surface cannot express first-request anchoring; state-key client-id interpolation) — all documented as accepted debt / follow-up candidates, none blocking.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/execute-fail-resume`
- **Retained at:** `evals/.sandbox/execute-fail-resume/` — since pruned (2026-06-12); re-create from the scenario setup to reproduce
- **Key paths:** `plan.yaml`, `change.md`, `discovery.md`, `.specify/specs/rate-limit/spec.md` (merged baseline), `.specify/archive/2026-06-11-rate-limit/` (archived slice incl. `build/report.yaml` history and `.build-log`), `crates/rate_limit/` (crate, tests, `ops/quota.txt`, `REVIEW.md`), `src/lib.rs` (guest), `.specify/journal.jsonl`
