# #9 — Build failure park + recovery

Pins the structured stop on a build non-zero exit and the resume-from-failed-task contract. A slice's `cargo test` fails inside `/spec:build`; `/spec:execute` renders the `stop: build-failed` block `specify plan status` classifies from the journal. The operator patches the source, retries `/spec:build` per the hint, and the next `/spec:execute` dispatches forward without flags.

## Starting state

- `plan.yaml.lifecycle == approved`.
- Slice 1 `password-hash-rotate` already `done`.
- Slice 2 `session-cookie-harden` is `in-progress`; its slice lifecycle is `refined` (refine landed cleanly on the first `/spec:execute` pass).
- Slice 3 `reset-flow-retire` is still `pending`.

## Trace

1. **First `/spec:execute` pass.**
   - Lock acquired.
   - `specify plan status` returns `next-action: build session-cookie-harden`; `specify plan next` confirms the active entry; the loop dispatches `/spec:build`.
   - `/spec:build` runs tasks 1–4 successfully (each `specify slice task mark` flips the checkbox). Task 5 (`cargo test`) fails: a regression test asserts session cookie `Secure` flag is set; production code path forgot it. The brief writes a `status: failure` report; `specify slice build --phase finalize` journals `slice.build.failed` (reason `task-5 cargo test failed: session_cookie_secure_flag_set`) and exits non-zero. The build skill prints its own stop hint (failing task + log path).
   - The next `specify plan status` classifies the failure from the journal; `/spec:execute` renders its block verbatim per [`../../../../../plugins/spec/references/stop-conditions.md`](../../../../../plugins/spec/references/stop-conditions.md):

     ```text
     stop: build-failed
       slice: session-cookie-harden
       project: -
       detail: task-5 cargo test failed: session_cookie_secure_flag_set
     hint: Fix the failure, then retry /spec:build for the slice. The plan entry stays in-progress.
     ```

   - The `specify plan lock` process holding the lock exits; lock released.
   - Plan entry stays `in-progress`; slice lifecycle stays `refined` (no per-entry advance; build did not converge).

2. **Operator patches the cookie path.** Adds `.secure(true)` to the cookie builder, re-runs the failing test locally, confirms green.

3. **Operator runs `/spec:build session-cookie-harden` per the hint.**
   - The breakout acquires the lock, resolves slice 2 via `specify plan next`.
   - `/spec:build` reads tasks 1–4 as already-flipped (idempotent re-mark is a no-op), re-runs task 5, passes; finalize journals `slice.build.succeeded` — clearing the stop — and transitions the slice to `built`.

4. **Operator re-runs `/spec:execute`.**
   - Lock acquired.
   - `specify plan status` returns `next-action: merge session-cookie-harden` (the success terminal superseded the failure); loop dispatches `/spec:merge`.
   - Merge success → `specify slice merge run` stamps the entry `done`.
   - Next iteration runs slice 3 end-to-end → `specify plan status` reports `drained`.

## Terminal state

- Every per-entry `status: done`.
- Closing hint: `drained — run /spec:finalize identity-revamp`.

## Stress test

- The structured stop block is reproducible: `specify plan status` classifies from the newest journal terminal, so the same `detail` and `slice` render across re-runs until the build converges.
- The plan entry never advanced past `in-progress` on the failure; merge is still the sole writer of `done`.
- Re-entry needs no `--continue` and no resume token; the slice lifecycle plus the journal tail — both read by `plan status` — are the only resume state.
