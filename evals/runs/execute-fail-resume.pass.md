# Run: `execute-fail-resume` — **pass**

## Context

- **Scenario:** `execute-fail-resume`
- **Operator:** Cursor agent (agent-as-operator, per the single-scenario runbook)
- **CLI:** `/Users/andrewweston/.local/bin/specify` — `specify 0.2.0` (built from the `Specify.toml` `cli` source via `make install-cli`)
- **Sandbox:** `evals/.sandbox/execute-fail-resume/` (recreated fresh 2026-06-15)

## Assertions

| Assertion | Verdict | Evidence |
| --- | --- | --- |
| `plan-exists` | pass | |
| `build-failure-stop-hint` | pass | |
| `build-resumes-from-failed-task` | pass | |
| `loop-continues-to-merge` | pass | |

Probe transcript highlights: `plan.yaml` present for plan `rate-limit` with four slices (`auth-rate-limit`, `password-hash-rotate`, `session-cookie-harden`, `reset-flow-retire`). At park on `session-cookie-harden`, `specify journal show --filter slice.build.failed` emitted `{"slice-name":"session-cookie-harden","reason":"target-build-failed"}`; `grep -c 'status: in-progress' plan.yaml` returned `1`; `specify plan status --format json` carried `"action":"stop"` with `"stop".reason == "build-failed"` and `"resume":"/spec:build session-cookie-harden"`. After patching `crates/session_cookie_harden/src/lib.rs` to set `secure: true`, breakout `/spec:build` journals `slice.build.failed → slice.build.started → slice.build.succeeded` with no `slice.synthesize.*` between; parked `tasks.md` kept prior checkboxes complete. Resumed `/spec:execute` merged `session-cookie-harden` and `reset-flow-retire`; `slice.merge.succeeded` fired for both; closing `specify plan status` reported `"action":"drained"` / `drained — run /spec:finalize rate-limit`.

**Negative expectations:** held (manual-by-design posture unchanged; live interactive drive against the real CLI).

## Deviations

- `specify init omnia@v1` substituted with the documented offline fallback `specify init <framework>/adapters/targets/omnia`; `intent` source adapter symlinked into the sandbox per setup prerequisites.
- Gate 1 stamped with `specify plan transition rate-limit approved --actor agent`.
- Four-slice `rate-limit` plan decomposed from compound intent at propose time; engineered failure follows fixture #9 (`session-cookie-harden`, missing `Secure` flag, regression test `session_cookie_secure_flag_set`).
- Minimal serde-only library crates and lightweight merge path (`cargo fmt` + `cargo test` + `specify slice merge run`) — no omnia guest / `wasm32-wasip2` pre-merge gate in this sandbox (eval focuses on execute park/resume mechanics).
- Plan lock held for the session via `specify plan lock -- <cmd>` per [`plan-lock.md`](../../plugins/spec/references/plan-lock.md).

## Notes

- `.build-log` at `.specify/slices/session-cookie-harden/.build-log` captures failing `cargo test` output naming `session_cookie_secure_flag_set`.
- `plan.entry.advanced` events cover each slice claim; no duplicate advance after the build-failed park.
- Driver: `evals/drivers/execute_fail_resume.py` with shared loop helper `evals/drivers/execute_loop.py`.

## Evidence

- **Reproduce:** `scripts/snapshot.sh evals/.sandbox/execute-fail-resume`
- **Retained at:** `evals/.sandbox/execute-fail-resume/`
- **Key paths:** `plan.yaml`, `change.md`, `discovery.md`, `crates/session_cookie_harden/`, `.specify/archive/`, `.specify/journal.jsonl`, `evals/drivers/execute_fail_resume.py`
