# e2e-platform-v2-with-crash — crash mid-build, re-run recovers

Same seed as the sibling [`../e2e-platform-v2/`](../e2e-platform-v2/)
fixture, but a SIGKILL arrives mid-iteration-4 (while `/spec:build
product-catalog` is running), leaving the workspace partially-
progressed. An operator re-runs `/spec:execute --loop` against the
unchanged workspace; self-heal picks up where the crash left off and
the initiative continues to the same terminal state as the
uncrashed sibling.

This fixture is the Layer 2 exit-gate acceptance for crash recovery:
"An injected mid-build SIGKILL, followed by a re-run, recovers via
self-heal and completes the initiative."

There is no automated harness. The two runs below are narrated as
documentation.

## Files in this fixture

| File | Role |
|---|---|
| `plan.yaml.before` | Seed — RFC-2 §"The Plan" verbatim. |
| `metadata-email-verification-crashed.yaml` | Seeded `.metadata.yaml` for `email-verification` — the same pre-existing crash from a still-earlier run that the sibling fixture's self-heal also reconciles. |
| `metadata-product-catalog-mid-build.yaml` | `.metadata.yaml` for `product-catalog` as it sits on disk after the SIGKILL — `status: building`, no `outcome` field. |
| `plan.yaml.after-crash` | Plan state between the two runs: `email-verification → done` (from first-run self-heal), four `done` siblings, `product-catalog` left `in-progress` by the SIGKILL, everything else unchanged. |
| `plan.yaml.after` | Final state after the second run completes. Identical to `../e2e-platform-v2/plan.yaml.after`. |

## Run 1 — crash mid-`/spec:build product-catalog`

```text
$ /spec:execute --loop

# step 1 + 2: project resolution, lock acquire — silent.

# step 3: self-heal (writing path).
#   email-verification was in-progress with outcome: success on merge.
Self-heal: email-verification → done (merge success from prior run)
#   specify initiative transition email-verification done
#   specify change journal-append email-verification merge recovery \
#       --summary "Self-heal on startup: applied terminal transition done after finding success outcome on merge" \
#       --context "before=in-progress/merged, after=done"

# Iterations 1–3 run exactly as in ../e2e-platform-v2/transcript.md:
#   - registration-duplicate-email-crash → done (--affects user-registration)
#   - notification-preferences           → done (greenfield)
#   - extract-shared-validation          → done (--affects user-registration --affects email-verification)

# ───────────────────────────────────────────────────────────
# Iteration 4 — product-catalog (crashes mid-/spec:build)
# ───────────────────────────────────────────────────────────
# specify initiative next → { "next": "product-catalog" }

# Argument resolution:
#   sources: [monolith] → --source monolith=/path/to/legacy-codebase
#   affects: []         → no --affects flags

# specify initiative transition product-catalog in-progress
# /spec:define product-catalog --source monolith=/path/to/legacy-codebase → success
#   .specify/changes/product-catalog/.metadata.yaml:
#     status: defined
#     outcome.outcome=success outcome.phase=define  (step 9 read)

# /spec:build product-catalog
#   .specify/changes/product-catalog/.metadata.yaml:
#     status: building
#   (specify change transition product-catalog building)
#   tasks 1–3 marked complete in tasks.md
#   ⟵ SIGKILL at this point (illustratively: OOM / host restart /
#     operator kills the agent session). No clean shutdown. No
#     SIGTERM contract, not even the graceful §SIGINT / SIGTERM
#     handling path — the process simply disappears.

# The plan and on-disk state immediately after SIGKILL:
#   .specify/plan.yaml:
#     user-registration            → done
#     email-verification           → done
#     registration-duplicate-email-crash → done
#     notification-preferences     → done
#     extract-shared-validation    → done
#     product-catalog              → in-progress
#     shopping-cart                → pending
#     checkout-api                 → failed (unchanged)
#     checkout-ui                  → pending
#   .specify/changes/product-catalog/.metadata.yaml: status=building, no outcome field
#   .specify/plan.lock: PID <agent-session-pid> (stale — process is gone)
```

`plan.yaml.after-crash` in this directory is the snapshot of
`.specify/plan.yaml` at exactly this point in time.

## Run 2 — re-run recovers and continues

```text
$ /spec:execute --loop

# step 1: project resolution.

# step 2: acquire the driver lock.
#   specify initiative lock acquire --pid <new-agent-session-pid>
#   The lock CLI's liveness check notices the stamped PID from Run 1
#   is no longer alive, reclaims the stale stamp, and stamps the new
#   PID. No Error::DriverBusy surfaces.

# step 3: self-heal (writing path) scans for in-progress entries.
#   Entry product-catalog: status=in-progress in plan.yaml.
#   .specify/changes/product-catalog/.metadata.yaml:
#     status: building
#     (no outcome field)
#   Self-heal classification: §Self-heal on startup → step 2 "outcome
#   field is absent + LifecycleStatus non-terminal" → fall through
#   to step 3 mid-change resume with LifecycleStatus=building.
Self-heal: product-catalog — resuming build (LifecycleStatus=building)
#   specify change journal-append product-catalog build recovery \
#       --summary "Self-heal on startup: resumed mid-change build phase (LifecycleStatus=building)" \
#       --context "before=in-progress/building, after=resume-build"
#   NO plan transition here — the entry stays in-progress while the
#   resumed /spec:build completes. The step-9 phase-outcome read at
#   the end of the supervised-run body will take care of the
#   terminal transition.

# Post-self-heal: the outer --loop iteration body (step 4) is SKIPPED
# for this entry; self-heal jumped directly into the supervised-run
# algorithm at step 7 (invoke /spec:build) per §Self-heal on startup
# → step 3's resumption table.

# ───────────────────────────────────────────────────────────
# Resumed iteration — product-catalog
# ───────────────────────────────────────────────────────────
# /spec:build product-catalog
#   tasks.md is read as-is (tasks 1–3 already marked complete from
#   Run 1; /spec:build resumes at task 4).
#   /spec:build stamps outcome: success on completion.
# /spec:merge product-catalog → success
# specify initiative transition product-catalog done

### Processing: product-catalog (sources: [monolith])  [resumed]

Step 1/3: define  (already complete; skipped on resume)
Step 2/3: build   (resumed — tasks 1–3 done at crash, resuming at task 4)
  Tasks: 5/5 complete ✓
Step 3/3: merge
  Baseline updated: .specify/specs/product-catalog/spec.md ✓
  Status: done

# The Processing block's "resumed" annotation and the "skipped on
# resume" parenthetical on Step 1/3 are illustrative — the skill's
# rendering conventions are pinned under §Output format for the
# ordinary (non-resumed) case; a resumed iteration additionally
# marks the already-complete phase(s) as such so the operator can
# correlate with self-heal's recovery journal entry.

# Iteration 5 (in the outer loop): back to the normal §Loop mode
# body — specify initiative next returns shopping-cart.
# ───────────────────────────────────────────────────────────
# Iteration 5 — shopping-cart (git-URL source)
# ───────────────────────────────────────────────────────────
# Invocation: /spec:define shopping-cart --source orders=git@github.com:org/orders-service.git
# (see ../e2e-platform-v2/transcript.md iteration 5 for detail)
# … runs through to done …

# Iteration 6 (terminating): specify initiative next → stuck
#   checkout-api is failed; checkout-ui's dep (checkout-api) is not
#   done. Same terminal classification as ../e2e-platform-v2/.
```

## Terminal summary (Run 2)

Identical to the uncrashed sibling's terminal summary:

```text
## /spec:execute — platform-v2 — terminated

### Final state
Progress: done 7, in-progress 0, pending 1, blocked 0, failed 1, skipped 0 (total 9)

Completion: stuck

Failed:
  - checkout-api (status-reason: "Type mismatch between cart line-item schema and payment gateway contract. Needs design revision after shopping-cart specs are updated.")

Pending (dependencies not satisfied):
  - checkout-ui (waits on: checkout-api)

Next action: Resolve blocked/failed entries (specify initiative amend + specify initiative transition <name> blocked → pending / failed → pending) or accept the partial initiative and run specify initiative archive --force.
```

Exit code: 0.

## Journal entries after Run 2

After Run 2 completes, `journal.yaml` files under `.specify/changes/`
contain exactly two `type: recovery` entries authored by the driver
across both runs:

```yaml
# .specify/changes/email-verification/journal.yaml
- type: recovery
  phase: merge
  summary: "Self-heal on startup: applied terminal transition done after finding success outcome on merge"
  context: "before=in-progress/merged, after=done"
  recorded-at: <run-1-startup-timestamp>
```

```yaml
# .specify/changes/product-catalog/journal.yaml
- type: recovery
  phase: build
  summary: "Self-heal on startup: resumed mid-change build phase (LifecycleStatus=building)"
  context: "before=in-progress/building, after=resume-build"
  recorded-at: <run-2-startup-timestamp>
```

Both entries are `type: recovery` written via `specify change
journal-append <name> <phase> recovery …`. Phase-authored entries
(`type: question`, `type: failure`) from mid-run work are
preserved unchanged; the driver only appends, never rewrites.

## Invariants pinned by this fixture

1. **Stale lock stamps are reclaimed by the CLI.** Run 2's
   `specify initiative lock acquire` does not fail with `Error::DriverBusy`
   — the CLI-level liveness check notices Run 1's PID is gone and
   reclaims the stamp before the skill sees it.
2. **Mid-build crash leaves `.metadata.yaml.outcome` absent.** The
   phase writes `outcome` via `specify change phase-outcome` as its
   terminal action; a SIGKILL mid-phase never reaches that call, so
   the field is missing rather than malformed. Self-heal treats
   missing-`outcome` + non-terminal `LifecycleStatus` as mid-change
   resume (NOT as an ambiguity halt — the ambiguity branch is
   reserved for contradictions, e.g. `outcome.phase=merge` with
   `LifecycleStatus=defining`).
3. **Resume does NOT write a plan transition.** Self-heal's
   `product-catalog — resuming build` diagnostic reflects a journal
   append + a phase re-invocation; the plan entry remains
   `in-progress` until the supervised-run body's normal terminal
   transition fires after `/spec:merge` completes.
4. **Argument resolution re-runs against the same plan.** When the
   resumed `/spec:build` eventually finishes and the outer loop
   advances to `shopping-cart`, argument resolution starts fresh
   from `plan.yaml` — self-heal does not cache or replay the
   Run 1 argument set. The `--source orders=…` flag on
   `shopping-cart` is constructed during Run 2 from the same
   top-level `sources` map it would have used in Run 1.
5. **Tasks.md progress survives the crash.** `/spec:build`'s
   resume semantics (already documented in `plugins/spec/skills/build/SKILL.md`)
   rely on the checkbox state in `tasks.md`; nothing the driver
   does interferes with that.
6. **Final state matches the uncrashed run exactly.** The crash
   adds one extra self-heal journal entry and one resumed phase
   invocation; it does NOT change the terminal plan shape. Both
   `plan.yaml.after` files (this fixture's and the sibling's) are
   byte-for-byte identical.
