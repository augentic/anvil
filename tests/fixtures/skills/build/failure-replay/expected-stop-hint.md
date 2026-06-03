# Expected stop hint

Final visible output of `/spec:build session-cookie-harden` on the failure path:

```text
stop: build-failed
  slice: session-cookie-harden
  phase: build
  failing-task: cargo test (test session_cookie_secure_flag_set)
  log-path: .specify/slices/session-cookie-harden/.build-log
  next-action: re-run /spec:build session-cookie-harden after fix
```

## Lifecycle invariants on this path

- `.specify/slices/session-cookie-harden/.metadata.yaml` `status` stays `refined`.
- `plan.yaml.slices[0].status` stays `in-progress` — `/spec:build` never writes `plan.yaml`.
- The plan lock (acquired in standalone mode, held by parent in loop mode) releases on process exit.
- No `specify slice transition` call was made (success-only step).
