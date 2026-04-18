# stuck-on-blocked — `--loop` drains what it can, then stops at `stuck`

A three-entry plan where `user-registration` is already `done`.
`email-verification` defers mid-run (scope question from the define
phase), becomes `blocked`, and `--loop` continues rather than
halting. `registration-duplicate-email-crash` has no `depends-on`
(it uses `affects` instead, which does NOT gate eligibility), so the
next iteration picks it up and runs it to `done`. The third
iteration's `specify plan next` returns no eligible entry —
`blocked` is the only non-terminal status left — so the loop exits
with `Completion: stuck`.

The key behaviour this fixture pins: **an individual deferral does
NOT halt `--loop`**. The loop continues draining eligible work and
only stops when `specify plan next` reports no eligible entry.

## Driver timeline

```text
$ /spec:execute --loop

# step 1: project resolution — silent on success.
# step 2: acquire lock — silent on success.

# step 3: self-heal (writing path).
#   plan.yaml has no in-progress entries → no-op.
Self-heal: no in-progress entries found.

# step 4 iteration 1/3: pick next.
#   specify plan next --format json
#     → { "next": "email-verification" }
#   (user-registration is done; email-verification is the first
#   pending entry whose depends-on list is all done. The entry after
#   — registration-duplicate-email-crash — is ALSO eligible but
#   specify plan next breaks ties by plan list order.)
#   specify plan transition email-verification in-progress
#   /spec:define email-verification → outcome: deferred
#     outcome.summary: "Notification channel scope not specified in
#     description; cannot safely design verification-email template
#     without it."
#   Driver reads outcome.outcome == deferred → steps 12a–c:
#     /spec:drop email-verification --reason "<outcome.summary>"
#     specify plan transition email-verification blocked \
#         --reason "<outcome.summary>"
#   (status-reason on the plan entry now matches outcome.summary
#    byte-for-byte.)

## /spec:execute — platform-v2

### Initiative: platform-v2
Progress: done 1, in-progress 1, pending 1, blocked 0, failed 0, skipped 0 (total 3)

---

### Processing: email-verification (sources: [monolith])

Step 1/3: define
  ⚠ Question recorded — change deferred to blocked

  Question: Notification channel scope not specified in description; cannot safely design verification-email template without it.
  Journal: .specify/changes/email-verification/journal.yaml
  Action needed: Update the plan description (specify plan amend …) with the missing
    scope, then unflag (blocked → pending) to retry.
  Status: blocked

---

# step 4 iteration 2/3: pick next.
#   specify plan next --format json
#     → { "next": "registration-duplicate-email-crash" }
#   (email-verification is now blocked — skipped. The remaining
#   pending entry has no depends-on, so it's eligible.)
#   specify plan transition registration-duplicate-email-crash in-progress
#   /spec:define  → outcome: success
#   /spec:build   → outcome: success
#   /spec:merge   → outcome: success
#   specify plan transition registration-duplicate-email-crash done

### Initiative: platform-v2
Progress: done 1, in-progress 1, pending 0, blocked 1, failed 0, skipped 0 (total 3)

---

### Processing: registration-duplicate-email-crash (affects: [user-registration])

Step 1/3: define
  Artifacts: proposal.md, specs, design.md, tasks.md ✓

Step 2/3: build
  Tasks: 2/2 complete ✓

Step 3/3: merge
  Baseline updated: .specify/specs/user-registration/spec.md ✓
  Status: done

---

# step 4 iteration 3 (terminating): pick next.
#   specify plan next --format json
#     → { "next": null, "reason": "stuck" }
#   The only non-terminal entry is email-verification, which is
#   blocked. No pending entries remain. Break out of the loop.

# step 5: emit terminal summary.
```

## Terminal summary (as rendered by `/spec:execute`)

```text
## /spec:execute — platform-v2 — terminated

### Final state
Progress: done 2, in-progress 0, pending 0, blocked 1, failed 0, skipped 0 (total 3)

Completion: stuck

Blocked:
  - email-verification (status-reason: "Notification channel scope not specified in description; cannot safely design verification-email template without it.")

Next action: Resolve blocked/failed entries (specify plan amend + specify plan transition <name> blocked → pending / failed → pending) or accept the partial initiative and run specify plan archive --force.
```

## Invariants pinned

1. **Deferral does NOT halt `--loop`.** After email-verification is
   transitioned to `blocked`, the loop's next iteration runs
   `specify plan next` again — which skips the `blocked` entry and
   returns `registration-duplicate-email-crash` as the next eligible.
2. **`specify plan next` skips `blocked` / `failed` naturally.**
   The driver does not need an explicit "skip this entry" branch;
   eligibility in the Layer 1 CLI is already defined as "pending AND
   all depends-on are done". `blocked` entries are not pending, so
   they are not eligible.
3. **`affects` does NOT gate eligibility.** The
   `registration-duplicate-email-crash` entry lists `affects:
   [user-registration]` but has no `depends-on`. It is eligible as
   soon as it becomes the first pending entry. The `affects` field
   is a baseline-delta targeting hint (see RFC-2 §"`affects`" and
   L2.I for the execution wiring), not an eligibility gate.
4. **Verbatim `outcome.summary` → `status-reason` → terminal
   summary.** The string in `plan.yaml.after`'s `status-reason` for
   `email-verification` is byte-identical to the `Question:` line in
   the mid-run transcript AND to the `status-reason:` quoted in the
   terminal summary. No paraphrasing at any hop.
5. **`stuck` renders empty sections as empty — but doesn't emit
   empty headings.** The terminal summary above has a `Blocked:`
   section but NO `Failed:` or `Pending (dependencies not
   satisfied):` sections. The renderer must omit empty list
   headings entirely.
6. **Exit code 0 on `stuck`.** Partial completion is observable via
   the plan; the driver did nothing wrong. Operator triage is
   signalled by the terminal summary text, not by the exit code.
