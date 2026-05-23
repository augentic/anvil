# contracts.merge

Landing brief for slices that target the `contracts` adapter. The standard delta-spec merge, baseline coherence validation, lifecycle transition, and archive move are delegated to the `specify` CLI (`specify slice merge`). The contracts target adds **one target-specific gate** on top of that flow: a post-merge baseline check via the declared `contract` WASI tool. Every other artefact under `specs/` and `contracts/` is promoted by the standard delta merge.

Follow the [`/spec:merge` skill](../../../../plugins/spec/skills/merge/SKILL.md) for the driver-side flow — slice selection, prerequisite checks, the AskQuestion confirmation around the merge preview, baseline-drift handling, and result rendering. The post-merge tool gate below is the contracts-specific delta on top of that flow.

## Target-specific adoption gate

After `specify slice merge` exits zero (i.e. the slice's `contracts/` deltas have been promoted into root `contracts/` and the lifecycle has transitioned to `merged`), run the declared tool against the now-updated baseline:

```bash
specify tool run contract -- "$PROJECT_ROOT/contracts" --format json > /tmp/contract-findings.json
case $? in
  0) ;;  # clean — baseline is well-formed; record success
  1) ;;  # findings present — record `failure` (see §failure below)
  2) ;;  # tool/validator could not run — record `failure` (see §failure below)
esac
```

The tool enforces the contract validation rules across every top-level OpenAPI 3.1 / AsyncAPI 3.0 document under the supplied directory:

- `contract.version-is-semver` — `info.version` parses as SemVer per [semver.org](https://semver.org).
- `contract.id-format` — when `info.x-specify-id` is present, the value matches `^[a-z][a-z0-9-]*$` and is ≤ 64 characters.
- `contract.id-unique` — every present `info.x-specify-id` is unique across the baseline.

The JSON envelope is the canonical shape callers parse. Field reference (matches the verifier siblings' [`cross-project` mode](../references/openapi/verifier.md#cross-project-mode)):

```json
{
  "envelope-version": 2,
  "contracts-dir": "<absolute-baseline-path>",
  "ok": false,
  "findings": [
    { "path": "contracts/http/user-api.yaml", "rule-id": "contract.id-unique", "detail": "..." }
  ],
  "exit-code": 1
}
```

When the slice does not touch `contracts/` at all (e.g. a planning-metadata-only contracts slice), still run the validator after merge — the baseline as a whole must remain well-formed, and the tool is cheap on a clean baseline. When `contracts/` is absent entirely, `specify tool run` exits `2` before or during validator invocation; treat that as `failure` per the §failure branch below (the merge brief should not be running for a contracts slice that has no baseline to validate).

The WASI tool is a deterministic, target-owned gate; it does not parse the slice's deltas in isolation. If the operator needs to inspect the slice's contributions before merge, run the build-time tool gate (Phase 5 of [`build.md`](build.md)) or the format-verifier `single` mode — the merge brief intentionally validates the merged baseline, not the staged delta, because cross-repo id uniqueness only resolves once the deltas are promoted.

### Consumer-project pin updates

When the slice's contributions need to flow into downstream consumer projects (per the registry's workspace clones), publish the prepared workspace branches **after** the validator gate clears:

1. `specify workspace push` — push the workspace clones' branches that already received the merged contract deltas.
2. Operator PR merge — review and merge those PRs through the forge UI, `gh pr merge`, or the team's normal merge queue.

Pin updates that the operator can publish map to `success` (the merge brief returns control with the lifecycle already stamped). Pin updates that surface drift the brief cannot auto-resolve (e.g. a consumer's workspace clone has uncommitted local edits, a consumer project is offline, or `workspace push` reports `no-branch` because the clone is not on the prepared `specify/<change-name>` branch) map to `deferred` — the operator must reconcile the pins by hand. See §deferred below.

## Outcome signalling

The contracts target merge brief is the slice loop's first target-owned baseline gate. The brief decides go/no-go and signals it through `specify slice outcome set` plus `specify slice journal append`. The core proceeds with archival on `success` and halts on `failure` / `deferred`, surfacing the journal entries to the operator. Target diagnostics round-trip as opaque journal entries — the core does not parse them.

The shared phase contract (outcome values, journal kinds, the verbatim-`summary` rule, plan-mutation rules) is authored once at [`plugins/spec/references/phase-outcome-contract.md`](../../../../plugins/spec/references/phase-outcome-contract.md). The three terminal branches below are the merge-phase deltas; the brief MUST pick exactly one before returning control.

### success — merge applied, validator clean, slice archived

`specify slice merge` exited zero AND `specify tool run contract -- "$PROJECT_ROOT/contracts" --format json` exited `0`. The CLI atomically stamps `PhaseOutcome { phase: merge, outcome: success }` into `.metadata.yaml`, transitions the lifecycle to `merged`, and moves the slice directory into `.specify/archive/YYYY-MM-DD-<slice>/`.

The brief MUST NOT call `specify slice outcome set` on this path — the slice directory no longer exists under `.specify/slices/<slice>/` after archiving, so the call would fail with `not found`. The archived `.metadata.yaml` carries the success outcome; downstream readers fall back to the archive when the active directory is absent. See [`phase-outcome-contract.md`](../../../../plugins/spec/references/phase-outcome-contract.md) §"Merge success path is CLI-stamped".

`/spec:execute` translates `success` into a plan-entry transition to `done` and proceeds to the next entry.

### failure — merge halted or baseline rejected

This branch covers two distinct failure modes:

1. **`specify slice merge` exited non-zero** (a delta could not be applied, baseline coherence failed inside the merge call, the lifecycle gate refused the call). The filesystem is unchanged: no baseline was written and the slice directory was not moved.
2. **`specify tool run contract -- "$PROJECT_ROOT/contracts" --format json` returned exit code `1` or `2` after a successful `specify slice merge`.** The deltas have already landed in the baseline (the CLI stamped `success` atomically), but the merged baseline is now invalid under the contract validation rules or the declared tool could not run. **The brief MUST NOT attempt to roll back the merge** — `specify slice merge` is not transactional with the validator. Instead, journal the validator's findings or tool diagnostic on the now-archived slice and surface the failure to the operator; the operator opens a follow-up slice (or `/spec:drop reason …` and re-refines) to repair the baseline.

In both modes, record the failure on the slice — first journal the diagnostic, then stamp the outcome (when the slice is still under `.specify/slices/`):

```bash
# Mode 1: pre-merge failure (slice is still active)
specify slice journal append <slice> merge failure \
  --summary "<which CLI step failed and the load-bearing stderr line>" \
  --context "<verbatim stderr / coherence-check tail / failing delta path>"

specify slice outcome set <slice> merge failure \
  --summary "<same load-bearing summary, written so it is useful as a /spec:drop reason>"
```

```bash
# Mode 2: post-merge validator failure (slice is already archived)
specify slice journal append <slice> merge failure \
  --summary "<rule-id>: <one-line restatement of findings[0].detail>" \
  --context "<verbatim contents of /tmp/contract-findings.json>"

# Do NOT call `specify slice outcome set` — the slice directory has been moved
# to .specify/archive/<…>/ and the call fails with `not found`. The archived
# `.metadata.yaml` carries the CLI-stamped `success` outcome from the
# `specify slice merge` step; the journal `failure` entry is what surfaces
# the post-merge baseline regression to the operator.
```

For mode 1, `/spec:execute` reads the `failure` outcome and translates it into a plan-entry transition to `failed`, surfaces the journal entries to the operator, and stops the loop. For mode 2, `/spec:execute` reads the CLI-stamped `success` outcome and proceeds with the next plan entry; the operator separately triages the journal `failure` entry on the archived slice and queues a repair slice when ready. In neither mode does the brief retry the merge automatically — the failing delta or invalid baseline state needs human attention before a repeat attempt is safe.

`--summary` writing rules for validator findings: the load-bearing string is `"<findings[0].rule-id>: <one-line summary of findings[0].detail>"`. Keep it short enough to fit a CLI argument without truncation; route the full JSON envelope through `--context` instead. When `specify tool run contract` returns exit `2` (resolver, permission, runtime, or invocation error), use `"contract tool could not run: <stderr first line or JSON error message>"` and put the full stderr/stdout diagnostic on `--context`.

### deferred — human judgement required

A merge prerequisite is unclear and `specify slice merge` was never invoked. Typical triggers:

- The user declined the AskQuestion confirmation around the merge preview.
- The merge preview reported baseline drift (a sibling slice mutated the baseline after this slice was refined) that needs operator arbitration.
- `.specify/slices/<slice>/.metadata.yaml` reports a lifecycle other than `built` (e.g. `building`, `refining`) and the user declined to proceed.
- `specify slice validate` surfaced unmet `merge`-phase needs that the brief cannot resolve unattended.

Plus contracts-specific triggers around the consumer-project pin update step:

- A consumer project's workspace clone has uncommitted local edits that block `specify workspace push`.
- A consumer project is offline / unreachable when the brief tries to push its PR branch.
- `specify workspace push` reports `no-branch` because the consumer clone is not on the prepared `specify/<change-name>` branch.
- The operator wants to inspect the merged contract before propagating pins or merging the resulting PR.

Record the deferral on the slice — first journal the question, then stamp the outcome:

```bash
specify slice journal append <slice> merge question \
  --summary "<the question the operator must answer>" \
  --context "<verbatim conflict-check report / preview diff / lifecycle status / workspace-push diagnostic>"

specify slice outcome set <slice> merge deferred \
  --summary "<same question, present-tense, self-contained>"
```

`/spec:execute` translates `deferred` into a plan-entry transition to `blocked`, surfaces the journal entries, and stops the loop. The brief MUST NOT silently fall through — recording the deferral is the only way the operator hears about it.

### Summary writing rules

The `--summary` strings ride into `/spec:drop reason` byte-for-byte when `/spec:execute` reclaims a `failure` or `deferred` slice (see [`phase-outcome-contract.md`](../../../../plugins/spec/references/phase-outcome-contract.md) §"Verbatim-`summary` rule"). Keep them present-tense, self-contained, and short enough to fit a CLI argument without truncation. Route any verbatim stderr, validator JSON, or log tail through `--context` instead — that field is not forwarded to `--reason`.
