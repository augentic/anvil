# contracts.merge

Landing brief for slices that target the `contracts` adapter. The standard delta-spec merge, baseline coherence validation, lifecycle transition, and archive move are delegated to the `specify` CLI (`specify slice merge`). The contracts target adds **one target-specific gate** on top of that flow: a post-merge baseline check via the declared `contract` WASI tool. Every other artefact under `specs/` and `contracts/` is promoted by the standard delta merge.

Follow the [`/spec:merge` skill](../../../../plugins/spec/skills/merge/SKILL.md) for the driver-side flow — slice selection, prerequisite checks, the AskQuestion confirmation around the merge preview, baseline-drift handling, and result rendering. The post-merge tool gate below is the contracts-specific delta on top of that flow.

## Target-specific adoption gate

After `specify slice merge` exits zero (i.e. the slice's `contracts/` deltas have been promoted into root `contracts/` and the lifecycle has transitioned to `merged`), run the declared tool against the now-updated baseline:

```bash
specify tool run contract -- "$PROJECT_ROOT/contracts" --format json > /tmp/contract-findings.json
case $? in
  0) ;;  # clean — baseline is well-formed
  1) ;;  # findings present — emit stop hint with `failure-kind: post-merge-validator`
  2) ;;  # tool/validator could not run — emit stop hint with `failure-kind: post-merge-validator`
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

When the slice does not touch `contracts/` at all (e.g. a planning-metadata-only contracts slice), still run the validator after merge — the baseline as a whole must remain well-formed, and the tool is cheap on a clean baseline. When `contracts/` is absent entirely, `specify tool run` exits `2` before or during validator invocation; emit a stop hint with `failure-kind: post-merge-validator`.

If the operator pipeline (CI annotations, dashboards, `specify lint`) needs to re-surface envelope findings as `LintFinding` records (see `schemas/diagnostics/diagnostic.schema.json` and [`../references/report-shape.md`](../references/report-shape.md#relationship-to-lintfinding)), the mapping is: `findings[].rule-id` → `rule-id`, `findings[].path` → `location.path`, `target-adapter: contracts`, and the contract-domain payload (`detail`, any compatibility classification such as `additive` / `breaking` / `ambiguous` / `unverifiable`) lives inside `evidence.kind: structured` with the contract data under `evidence.data`. The closed `LintFinding` severity enum (`critical` / `important` / `suggestion` / `optional`) is separate from any compatibility classification — classifiers remain contract-domain evidence fields.

The WASI tool is a deterministic, target-owned gate; it does not parse the slice's deltas in isolation. If the operator needs to inspect the slice's contributions before merge, run the build-time tool gate (Phase 5 of [`build.md`](build.md)) or the format-verifier `single` mode — the merge brief intentionally validates the merged baseline, not the staged delta, because cross-repo id uniqueness only resolves once the deltas are promoted.

### Consumer-project pin updates

When the slice's contributions need to flow into downstream consumer projects (per the registry's workspace clones), publish the prepared workspace branches **after** the validator gate clears:

1. `specify workspace push` — push the workspace clones' branches that already received the merged contract deltas.
2. Operator PR merge — review and merge those PRs through the forge UI, `gh pr merge`, or the team's normal merge queue.

Pin updates that the operator can publish proceed after the validator gate clears. Pin updates that surface drift the brief cannot auto-resolve (e.g. a consumer's workspace clone has uncommitted local edits, a consumer project is offline, or `workspace push` reports `no-branch` because the clone is not on the prepared `specify/<change-name>` branch) require operator reconciliation — emit a stop hint with `failure-kind: lifecycle-refused`.

## Stop hint contract

> See [Phase outcome contract](../references/spec-runtime/phase-outcome-contract.md).

When the pre-merge gate, the CLI delta merge, or the post-merge hook fails, emit a structured stop hint as the body's final output:

- `slice` — slice name from `specify plan next`.
- `phase` — `merge`.
- `failure-kind` — one of `pre-merge-gate`, `baseline-conflict`, `lifecycle-refused`, `post-merge-validator`.
- `paths` — for `baseline-conflict`: the conflicting baseline files reported by `specify slice merge`. For `pre-merge-gate` / `post-merge-validator`: the captured `$LOG_PATH` or `/tmp/contract-findings.json`.
- `next-action` — `resolve and re-run /spec:merge $SLICE` for conflicts; `queue repair slice` for `post-merge-validator` drift (validator findings or tool invocation failure after a successful `specify slice merge`).

Lifecycle invariants: `pre-merge-gate` and `baseline-conflict` leave the slice at `built` and the plan entry at `in-progress`. `post-merge-validator` runs after `specify slice merge` succeeded, so the slice is already `merged` and the plan entry is already `done` — the hint is observability, not a park. The brief MUST NOT attempt to roll back the merge on a post-merge validator failure.
