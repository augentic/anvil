---
id: merge
description: Merge the slice into the repository
needs: [build]
---

Before merging, confirm all task checkboxes in `tasks.md` are complete and the slice status is `complete`. Delta-spec merging, baseline coherence validation, the lifecycle transition, and the archive move are delegated to the `specify` CLI: `specify slice merge preview`, `specify slice merge conflict-check`, `specify slice merge run`, and `specify slice validate`.

Follow the [`specify-merge`](../../../plugins/spec/skills/merge/SKILL.md) skill for the full driver-side flow — slice selection, prerequisite checks, the AskQuestion confirmation around the merge preview, baseline-drift handling, and result rendering. Omnia adds no capability-specific adoption mechanics on top of that flow: every artefact under `specs/` is promoted by the standard delta merge, and there are no extra format validators or generated outputs to refresh at merge time.

## Outcome signalling (Merge and adoption contract)

The merge brief is the omnia capability's first contact with the slice loop's adoption contract. RFC-13 §"Merge and adoption contract" pins the protocol: the brief decides go/no-go and signals it through `specify slice outcome set` plus `specify slice journal append`. The core proceeds with archival on `success` and halts on `failure` / `deferred`, surfacing the journal entries to the operator. Capability diagnostics round-trip as opaque journal entries — the core does not parse them.

The shared phase contract (outcome values, journal kinds, the verbatim-`summary` rule, plan-mutation rules) is authored once at [`plugins/spec/references/phase-outcome-contract.md`](../../../plugins/spec/references/phase-outcome-contract.md). The three terminal branches below are the merge-phase deltas; the brief MUST pick exactly one before returning control.

### success — merge applied, slice archived

`specify slice merge run` exited zero. The CLI atomically stamps `PhaseOutcome { phase: merge, outcome: success }` into `.metadata.yaml`, transitions the lifecycle to `merged`, and moves the slice directory into `.specify/archive/YYYY-MM-DD-<slice>/`.

The brief MUST NOT call `specify slice outcome set` on this path — the slice directory no longer exists under `.specify/slices/<slice>/` after archiving, so the call would fail with `not found`. The archived `.metadata.yaml` carries the success outcome; `/change:execute` reads it via `specify slice outcome show <slice>`, which falls back to the archive when the active directory is absent. See [`phase-outcome-contract.md`](../../../plugins/spec/references/phase-outcome-contract.md) §"Merge success path is CLI-stamped".

`/change:execute` translates `success` into a plan-entry transition to `done` and proceeds to the next entry.

### failure — merge halted, baselines untouched

`specify slice merge run` exited non-zero (a delta could not be applied, baseline coherence failed inside the merge call, the lifecycle gate refused the call). The filesystem is unchanged: no baseline was written and the slice directory was not moved.

Record the failure on the slice — first journal the diagnostic, then stamp the outcome:

```bash
specify slice journal append <slice> merge failure \
  --summary "<which CLI step failed and the load-bearing stderr line>" \
  --context "<verbatim stderr / coherence-check tail / failing delta path>"

specify slice outcome set <slice> merge failure \
  --summary "<same load-bearing summary, written so it is useful as a /spec:drop --reason>"
```

`/change:execute` translates `failure` into a plan-entry transition to `failed`, surfaces the journal entries to the operator, and stops the loop. The brief MUST NOT retry the merge automatically — the failing delta or lifecycle state needs human attention before a repeat attempt is safe.

### deferred — human judgement required

A merge prerequisite is unclear and `specify slice merge run` was never invoked. Typical triggers:

- The user declined the AskQuestion confirmation around the merge preview.
- `specify slice merge conflict-check` reported baseline drift (a sibling slice mutated the baseline after this slice was defined) that needs operator arbitration.
- `specify slice status` reports a lifecycle other than `complete` (e.g. `building`, `defining`) and the user declined to proceed.
- `specify slice validate` surfaced unmet `merge`-phase needs that the brief cannot resolve unattended.

Record the deferral on the slice — first journal the question, then stamp the outcome:

```bash
specify slice journal append <slice> merge question \
  --summary "<the question the operator must answer>" \
  --context "<verbatim conflict-check report / preview diff / lifecycle status>"

specify slice outcome set <slice> merge deferred \
  --summary "<same question, present-tense, self-contained>"
```

`/change:execute` translates `deferred` into a plan-entry transition to `blocked`, surfaces the journal entries, and stops the loop. The brief MUST NOT silently fall through — recording the deferral is the only way the operator hears about it.

### Summary writing rules

The `--summary` strings ride into `/spec:drop --reason` byte-for-byte when `/change:execute` reclaims a `failure` or `deferred` slice (see [`phase-outcome-contract.md`](../../../plugins/spec/references/phase-outcome-contract.md) §"Verbatim-`summary` rule"). Keep them present-tense, self-contained, and short enough to fit a CLI argument without truncation. Route any verbatim stderr or log tail through `--context` instead — that field is not forwarded to `--reason`.
