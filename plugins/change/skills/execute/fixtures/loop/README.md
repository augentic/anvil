# `/change:execute loop` — behavioural fixtures

These fixtures pin the five terminal paths a `--loop` invocation can take: drain the plan to completion (`all-done`), refuse the run because another driver is already holding the lock (`driver-busy`), halt on a self-heal ambiguity before any iteration runs (`halted`), drain eligible work until only `blocked` / `failed` entries remain (`stuck`), or handle a SIGINT mid-run cleanly (`driver-interrupted`). They correspond to RFC-2 Change L2.H; the algorithms they illustrate live in [`../../SKILL.md` → §Loop mode (`--loop`)](../../SKILL.md) and [`../../SKILL.md` → §Terminal summary (`--loop` exit)](../../SKILL.md).

There is no automated harness that runs these fixtures. They are prose artefacts: a human reviewing a change to `/change:execute`'s loop behaviour should be able to trace each on-disk transition by reading the files in order, and any drift between the `.metadata.yaml` outcome and the `plan.yaml.after` `status-reason` is a regression.

## Layout

```text
loop/
├── all-done/
│   ├── plan.yaml.before     # three entries, all pending (user-registration as root)
│   ├── plan.yaml.after      # every entry: done
│   └── transcript.md        # full multi-iteration run + summary (Completion: all-done)
├── halted-on-self-heal-ambiguity/
│   ├── plan.yaml.before     # shopping-cart: in-progress (stranded from prior crashed run)
│   ├── metadata.yaml        # contradictory: status=defining + outcome.phase=merge success
│   ├── plan.yaml.after      # IDENTICAL to plan.yaml.before (self-heal halted, no writes)
│   └── transcript.md        # halt diagnostic + terminal summary (Completion: halted) + exit 1
├── stuck-on-blocked/
│   ├── plan.yaml.before     # user-registration: done; email-verification + crash-fix: pending
│   ├── plan.yaml.after      # email-verification: blocked; registration-duplicate-email-crash: done
│   └── transcript.md        # mid-run deferral → loop continues → no eligible entries → stuck
├── driver-busy/
│   └── transcript.md        # refused second invocation; no plan.yaml touched
└── driver-interrupted/
    ├── plan.yaml.before     # user-registration: done; email-verification + notification: pending
    ├── metadata.yaml        # email-verification: /spec:build just stamped outcome: success
    ├── plan.yaml.after      # email-verification: in-progress (preserved by interrupt handler)
    └── transcript.md        # SIGINT mid-run + terminal summary (Completion: driver-interrupted)
```

`driver-busy/` is the odd fixture out: it has no `plan.yaml.before` / `plan.yaml.after` because the refused invocation never enters the protected region that would make plan contents observable. It pins only the diagnostic an operator sees and the exit code.

## `Completion:` coverage matrix

| Fixture | `Completion:` | Iteration runs? | Lock state on exit |
|---|---|---|---|
| `all-done/` | `all-done` | yes (3 iterations) | released |
| `halted-on-self-heal-ambiguity/` | `halted` | no (halt at step 3, before step 4) | released |
| `stuck-on-blocked/` | `stuck` | yes (2 iterations) | released |
| `driver-busy/` | (no summary — never entered protected region) | no (refused at step 2) | not acquired |
| `driver-interrupted/` | `driver-interrupted` | yes (1 iteration, interrupted) | released |

## Invariants every fixture asserts

1. **Lock held once, across all iterations.** `specify change plan lock acquire` runs once at step 2 of the `--loop` algorithm; `specify change plan lock release` runs once at step 6. Per-iteration lock churn is visible nowhere in any transcript.
2. **Self-heal runs once.** The pre-iteration `Self-heal: …` diagnostic fires a single time per run. It is not repeated between iterations.
3. **Individual failure / deferral does NOT halt `--loop`.** The `stuck-on-blocked/` fixture pins this: an `outcome: deferred` on `email-verification` transitions the entry to `blocked` via the supervised-run defer path (steps 12a–c), and the outer loop simply iterates again. `specify change plan next` skips `blocked` on the next iteration.
4. **`Completion: halted` is reserved for self-heal ambiguity.** `halted-on-self-heal-ambiguity/` is the only fixture that reaches `halted`. A mid-loop deferral (per `stuck-on-blocked/`) becomes `stuck`, not `halted`.
5. **`driver-interrupted` leaves the active entry as `in-progress`.** The interrupt handler does NOT run `specify change plan transition` on the active entry; self-heal on the next run is responsible for reclaiming it based on `.metadata.yaml:outcome`.
6. **Verbatim `outcome.summary` → `status-reason` → terminal summary `Blocked:` / `Failed:` quoting.** The `stuck-on-blocked/` fixture demonstrates this for the deferred path; the failure path follows the same rule with `Failed:` instead of `Blocked:`.
7. **Empty terminal-summary sections are omitted entirely.** The `all-done/` terminal summary has no `Blocked:` / `Failed:` / `Pending (dependencies not satisfied):` headings; `stuck-on- blocked/` has `Blocked:` but no `Failed:` or `Pending:`. The renderer emits only non-empty sections.

## Using these fixtures

- Before changing the `--loop` algorithm or terminal summary format in `SKILL.md`, re-read the before / after pairs and transcripts here and confirm the new algorithm still maps them cleanly. If it does not, update the fixtures in the same commit as the SKILL.md change.
- The `.metadata.yaml` files in these fixtures are illustrative snapshots shaped per `crates/change/src/lib.rs::ChangeMetadata` in `augentic/specify-cli`. They are prose, not a validated schema input — convenience fields like `updated-at` and a top-level `name` appear for readability and mirror the style of `../self-heal/` and `../single-slice/`; the real on-disk file is whatever `specify slice outcome set` writes.
