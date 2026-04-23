# `/spec:execute --loop` — crash-recovery meta-fixture

This meta-fixture pins the crash-recovery acceptance of RFC-2 Change L2.I: "An injected mid-build SIGKILL, followed by a re-run, recovers via self-heal and completes the initiative."

The seed is the same full `platform-v2` plan from [RFC-2 §"The Plan"](../../../docs/links.md#rfc-2-the-plan) used by the sibling `../e2e-platform-v2/`. The only difference is the scenario: a SIGKILL arrives mid-iteration-4 while `/spec:build product-catalog` is running. A second `/spec:execute --loop` run, launched against the unchanged workspace, picks up where Run 1 left off and drives the initiative to the same terminal state as the uncrashed sibling.

## Files

| File | Role |
|---|---|
| `plan.yaml.before` | Seed — identical to `../e2e-platform-v2/plan.yaml.before`. |
| `metadata-email-verification-crashed.yaml` | Same pre-existing crashed-merge metadata as in the sibling fixture; Run 1's self-heal reconciles it. |
| `plan.yaml.after-crash` | Plan state between the two runs: five `done` (user-registration, email-verification [from Run 1 self-heal], registration-duplicate-email-crash, notification-preferences, extract-shared-validation), `product-catalog` left `in-progress` by the SIGKILL, everything else unchanged. |
| `metadata-product-catalog-mid-build.yaml` | `.specify/changes/product-catalog/.metadata.yaml` at the SIGKILL moment: `status: building`, no `outcome` field yet. |
| `plan.yaml.after` | Terminal plan state after Run 2. Byte-for-byte identical to `../e2e-platform-v2/plan.yaml.after`. |
| `transcript.md` | Narrative for both runs: Run 1 crashes mid-build; Run 2's self-heal resumes and completes. |

## What this fixture proves

1. **Mid-build crashes are recoverable.** A SIGKILL between `/spec:build`'s start and its terminal `specify change phase-outcome` call leaves `.metadata.yaml` with `status: building` and no `outcome` — a state self-heal classifies as mid-change resume (§Self-heal on startup, step 3 → `LifecycleStatus=building` → resume `/spec:build`).
2. **Stale driver locks are reclaimed.** Run 2's `specify initiative lock acquire` does not fail with `Error::DriverBusy` — the CLI's liveness check notices Run 1's PID is gone and reclaims the stamp.
3. **The argument-resolution plumbing is stateless.** `/spec:execute` does not persist the resolved `--source` flags across runs. When the outer loop in Run 2 advances past the resumed `product-catalog` iteration to `shopping-cart`, it resolves that entry's `sources: [orders]` fresh from the plan's top-level `sources` map — same code path as Run 1 would have used.
4. **Crash recovery is observably indistinguishable at the plan level.** `plan.yaml.after` is byte-for-byte identical to the uncrashed sibling's. The only difference is one additional `type: recovery` entry in `product-catalog/journal.yaml`.

## Relationship to earlier fixtures

- `../self-heal/` pins the four classification paths in isolation (no in-progress, done reclaim, failed reclaim, ambiguity halt, mid-change resume) against minimal one-entry plans. This fixture exercises the mid-change-resume path against a realistic nine-entry plan.
- `../loop/driver-interrupted/` pins SIGINT (a graceful interrupt the skill's handler processes); this fixture pins SIGKILL (which the skill cannot trap — the process just disappears), so the recovery burden falls entirely on the next run's self-heal.
