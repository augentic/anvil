# all-done — `--loop` drives a clean plan to completion

A three-entry `platform-v2` plan with `user-registration` as root and two siblings depending on it. Every phase on every change returns `outcome: success`. `--loop` picks each eligible entry in turn and exits with `Completion: all-done`.

## Driver timeline

```text
$ /spec:execute --loop

# step 1 of the --loop algorithm (project resolution) is silent on
# success; step 2 acquires the lock and emits no diagnostic when the
# acquire succeeds.

# step 3: self-heal (writing path) scans plan.yaml. No in-progress
# entries → no-op.
Self-heal: no in-progress entries found.

# step 4 iteration 1/3: pick next.
#   specify initiative next --format json  → { "next": "user-registration" }
#   specify initiative transition user-registration in-progress
#   /spec:define user-registration   → outcome: success
#   /spec:build  user-registration   → outcome: success
#   /spec:merge  user-registration   → outcome: success
#   specify initiative transition user-registration done

## /spec:execute — platform-v2

### Initiative: platform-v2
Progress: done 0, in-progress 1, pending 2, blocked 0, failed 0, skipped 0 (total 3)

---

### Processing: user-registration (sources: [monolith])

Step 1/3: define
  - extract sub-step (via /spec:extract)
      Source: /path/to/legacy-codebase
      Artifacts: specs/user-registration/spec.md, design.md ✓
  Artifacts: proposal.md, specs, design.md, tasks.md ✓

Step 2/3: build
  Tasks: 5/5 complete ✓

Step 3/3: merge
  Baseline updated: .specify/specs/user-registration/spec.md ✓
  Status: done

---

# step 4 iteration 2/3: pick next.
#   specify initiative next --format json  → { "next": "email-verification" }
# (The two siblings — email-verification and notification-preferences —
# both depend only on user-registration. specify initiative next breaks the
# tie by plan list order, picking email-verification.)
#   specify initiative transition email-verification in-progress
#   /spec:define email-verification  → outcome: success
#   /spec:build  email-verification  → outcome: success
#   /spec:merge  email-verification  → outcome: success
#   specify initiative transition email-verification done

### Initiative: platform-v2
Progress: done 1, in-progress 1, pending 1, blocked 0, failed 0, skipped 0 (total 3)

---

### Processing: email-verification (sources: [monolith])

Step 1/3: define
  - extract sub-step (via /spec:extract)
      Source: /path/to/legacy-codebase
      Artifacts: specs/email-verification/spec.md, design.md ✓
  Artifacts: proposal.md, specs, design.md, tasks.md ✓

Step 2/3: build
  Tasks: 4/4 complete ✓

Step 3/3: merge
  Baseline updated: .specify/specs/email-verification/spec.md ✓
  Status: done

---

# step 4 iteration 3/3: pick next.
#   specify initiative next --format json  → { "next": "notification-preferences" }
#   specify initiative transition notification-preferences in-progress
#   /spec:define notification-preferences → outcome: success
#   /spec:build  notification-preferences → outcome: success
#   /spec:merge  notification-preferences → outcome: success
#   specify initiative transition notification-preferences done

### Initiative: platform-v2
Progress: done 2, in-progress 1, pending 0, blocked 0, failed 0, skipped 0 (total 3)

---

### Processing: notification-preferences (greenfield)

Step 1/3: define
  Artifacts: proposal.md, specs, design.md, tasks.md ✓

Step 2/3: build
  Tasks: 3/3 complete ✓

Step 3/3: merge
  Baseline updated: .specify/specs/notification-preferences/spec.md ✓
  Status: done

---

# step 4 iteration 4 (terminating): pick next.
#   specify initiative next --format json  → { "next": null, "reason": "all-done" }
# Break out of the loop.

# step 5: emit terminal summary.
```

## Terminal summary (as rendered by `/spec:execute`)

```text
## /spec:execute — platform-v2 — terminated

### Final state
Progress: done 3, in-progress 0, pending 0, blocked 0, failed 0, skipped 0 (total 3)

Completion: all-done

Next action: Initiative complete — no further action needed.
```

## Invariants pinned

1. **Lock held once, across all iterations.** `specify initiative lock acquire` runs once at step 2 of the `--loop` algorithm; `specify plan lock release` runs once at step 6. No per-iteration lock churn appears anywhere in the timeline.
2. **Self-heal runs once.** The `Self-heal: no in-progress entries found.` line fires a single time, before any iteration starts. It is not repeated between iterations.
3. **`Blocked:` / `Failed:` / `Pending:` sections omitted when empty.** The `all-done` terminal summary has only `Final state`, `Completion`, and `Next action`. The summary renderer must not emit empty list headings.
4. **Progress line enumerates all six statuses in fixed order.** Even when a status bucket is zero, it appears in the line as `<status> 0`. Downstream parsers see a stable shape.
