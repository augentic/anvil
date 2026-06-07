# #9 — Build failure park + recovery

Pins the structured stop on a build non-zero exit and the resume-from-failed-task contract. A slice's `cargo test` fails inside `/spec:build`; `/spec:execute` stops with the task id and log path. The operator patches the source, re-runs `/spec:execute`, and the loop picks up at the failing task without flags.

## Starting state

- `plan.yaml.lifecycle == approved`.
- Slice 1 `password-hash-rotate` already `done`.
- Slice 2 `session-cookie-harden` is `in-progress`; its slice lifecycle is `refined` (refine landed cleanly on the first `/spec:execute` pass).
- Slice 3 `reset-flow-retire` is still `pending`.

## Trace

1. **First `/spec:execute` pass.**
   - Lock acquired.
   - `specify plan next` returns slice 2 (already `in-progress`).
   - Slice lifecycle `refined` → loop dispatches `/spec:build`.
   - `/spec:build` runs tasks 1–4 successfully (each `specify slice task mark` flips the checkbox). Task 5 (`cargo test`) fails: a regression test asserts session cookie `Secure` flag is set; production code path forgot it.
   - `/spec:build` records `PhaseOutcome { phase: build, outcome: failure, summary: "task-5 cargo test failed: session_cookie_secure_flag_set" }` to `.specify/slices/session-cookie-harden/.metadata.yaml`, then returns non-zero.
   - `/spec:execute` reads the outcome, prints the templated stop hint from [`../../../../../plugins/spec/references/stop-conditions.md`](../../../../../plugins/spec/references/stop-conditions.md):

     ```text
     stop: build-failed
       slice: session-cookie-harden
       project: -
       task: task-5
       log: .specify/slices/session-cookie-harden/journal.yaml
     hint: Fix the failure, then re-run /spec:execute (or /spec:build to retry the
           failing task in isolation). The plan entry stays in-progress; the slice
           lifecycle stays where /spec:build left it.
     ```

   - The shell holding `flock` exits; lock released.
   - Plan entry stays `in-progress`; slice lifecycle stays `refined` (no per-entry advance; build did not converge).

2. **Operator patches the cookie path.** Adds `.secure(true)` to the cookie builder, re-runs the failing test locally, confirms green.

3. **Operator re-runs `/spec:execute`.**
   - Lock acquired.
   - `specify plan next` returns slice 2 (still `in-progress`).
   - Slice lifecycle is `refined` → loop dispatches `/spec:build`.
   - `/spec:build` reads tasks 1–4 as already-flipped (idempotent re-mark is a no-op), re-runs task 5, passes.
   - Slice lifecycle transitions to `built`; loop dispatches `/spec:merge`.
   - Merge success → `specify plan transition session-cookie-harden done`.
   - Next iteration runs slice 3 end-to-end → drained.

## Terminal state

- Every per-entry `status: done`.
- Closing hint: `drained — run /spec:finalize identity-revamp`.

## Stress test

- The structured stop hint is reproducible: same `task` field, same `log` path, same `slice` name across runs.
- The plan entry never advanced past `in-progress` on the failure; merge is still the sole writer of `done`.
- Re-entry needs no `--continue` and no resume token; the slice lifecycle on disk is the only resume state.
