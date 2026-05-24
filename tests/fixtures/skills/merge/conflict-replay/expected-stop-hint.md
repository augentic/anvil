# Expected stop hint

Final visible output of `/spec:merge identity-user-registration` on the baseline-conflict path:

```text
stop: merge-failed
  slice: identity-user-registration
  phase: merge
  failure-kind: baseline-conflict
  paths:
    - .specify/specs/omnia/spec.md (REQ-007 reset-link-expiry)
  next-action: resolve and re-run /spec:merge identity-user-registration
```

## Lifecycle invariants on this path

- `.specify/slices/identity-user-registration/.metadata.yaml` `status` stays `built`.
- `plan.yaml.slices[0].status` stays `in-progress` — `/spec:merge` only writes the per-entry `done` on the `specify slice merge` success path.
- The slice directory is not moved into `.specify/archive/`.
- The plan lock (acquired in standalone mode, held by parent in loop mode) releases on process exit.
- No `specify slice transition` call was made.

## Operator recovery

1. Read `.specify/specs/omnia/spec.md` to inspect the requirement that drifted (`REQ-007`).
2. Edit the slice's delta in `.specify/slices/identity-user-registration/specs/omnia/spec.md` to align with the new baseline (or amend `change.md` to capture the divergence).
3. Re-run `/spec:merge identity-user-registration` (or `/spec:execute` to resume the loop).
