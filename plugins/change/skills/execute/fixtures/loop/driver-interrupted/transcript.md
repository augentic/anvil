# driver-interrupted — SIGINT arrives mid-build; `--loop` exits cleanly

A three-entry plan; `user-registration` is already `done`. `/change:execute loop` picks `email-verification` as the next eligible entry, transitions it to `in-progress`, runs `/spec:define` (success) and `/spec:build` (success). Between `/spec:build` returning and the driver invoking `/spec:merge`, the operator presses Ctrl-C. The agent surfaces the interrupt to the skill.

Per §Loop mode → SIGINT / SIGTERM handling:

- Rule 1 is trivially satisfied: `/spec:build` has already returned, no phase is mid-invocation, nothing to finish.
- Rule 2: skip `/spec:merge`. Do NOT invoke it.
- Rule 3: leave `email-verification` as `in-progress`. Do NOT call `specify plan transition`.
- Rule 4: release the driver lock.
- Rule 5: emit the terminal summary with `Completion: driver-interrupted`.
- Rule 6: exit non-zero (130 for SIGINT).

On the next `/change:execute loop` invocation, self-heal (step 3 of the `loop` algorithm) reads `.specify/slices/email-verification/.metadata.yaml`, sees `outcome.outcome == success` with `outcome.phase == build`, and resumes by invoking `/spec:merge` (mid-change resume path, step 3 of the self-heal algorithm; RFC-2 §"Context Threading → Resumption Within a Change"). The interrupt therefore costs one restart but loses no completed phase work.

## Driver timeline

```text
$ /change:execute loop

# step 1 (project resolution): silent on success.
# step 2: acquire lock.

# step 3: self-heal (writing path).
#   No in-progress entries in plan.yaml.before → no-op.
Self-heal: no in-progress entries found.

# step 4 iteration 1: pick next.
#   specify plan next --format json → { "next": "email-verification", "project": null, "description": "...", "sources": ["monolith"] }
#   specify plan transition email-verification in-progress

## /change:execute — platform-v2

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

# /spec:build has just stamped outcome: { phase: build, outcome:
# success }. The driver is about to invoke /spec:merge.
#
# ^C (SIGINT arrives here, before /spec:merge is invoked)

# SIGINT handling:
#   Rule 1 — no phase is mid-invocation; trivially satisfied.
#   Rule 2 — skip /spec:merge. Do NOT invoke it.
#   Rule 3 — leave email-verification as in-progress. Do NOT call
#            specify plan transition. The on-disk state at this
#            moment:
#              plan.yaml: email-verification status: in-progress
#              .metadata.yaml: LifecycleStatus: complete
#                              outcome: { phase: build, outcome: success }
#            Self-heal on the next run will resume /spec:merge
#            using the mid-slice-resume branch of the self-heal
#            algorithm.
#   Rule 4 — release the driver lock:
#            specify plan lock release --pid <agent-session-pid>
#   Rule 5 — emit terminal summary with Completion:
#            driver-interrupted.
#   Rule 6 — exit 130.

⚠ Interrupt received — finishing up and exiting.

# step 5: emit terminal summary.
```

## Terminal summary (as rendered by `/change:execute`)

```text
## /change:execute — platform-v2 — terminated

### Final state
Progress: done 1, in-progress 1, pending 1, blocked 0, failed 0, skipped 0 (total 3)

Completion: driver-interrupted

Next action: Re-run /change:execute loop — self-heal will reclaim the interrupted change on the next startup.
```

## Invariants pinned

1. **Active entry preserved as `in-progress`.** Unlike the other `--loop` exit paths, `driver-interrupted` leaves the entry the driver was working on in `in-progress` state. The next run's self-heal is responsible for reclaiming it — the interrupted driver does not second-guess the on-disk state.
2. **`.metadata.yaml.outcome` is the reconciliation signal.** The phase (`/spec:build` in this fixture) wrote its outcome before returning, so the on-disk state after the interrupt is recoverable: self-heal next run reads the outcome, classifies as "success on build" (not a terminal phase), and invokes `/spec:merge`.
3. **Lock released on interrupt.** Rule 4 is non-negotiable. A stranded lock after an interrupt would block every subsequent `/change:execute` until an operator manually removed the stamp. The skill runs the release step even on the interrupt path.
4. **Terminal summary emitted on interrupt.** The `Completion: driver-interrupted` line and its `Next action` tell the operator exactly how to recover. The summary is part of the interrupt handling contract (Rule 5), not optional.
5. **Exit non-zero.** SIGINT typically maps to exit 130, SIGTERM to 143; CI / scripting treats either as "abnormal termination" and can distinguish from an `all-done` exit 0 or a `halted` exit 1.
6. **`in-progress 1` in the progress line.** This is the observable tell that the run was interrupted: no other terminal classification leaves an `in-progress` entry. (Self-heal ambiguity `halted` also has `in-progress 1` in its progress line, but its `Completion:` value differentiates.)
7. **No terminal plan transition on interrupt.** The driver does NOT call `specify plan transition email-verification failed` (or anything similar). The phase succeeded; the driver simply ran out of wall-clock before invoking the next phase. Self-heal on the next run is the correct actor to advance the plan.
