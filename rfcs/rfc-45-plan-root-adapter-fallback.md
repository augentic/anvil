# RFC-45: Proving the Plan-Root Adapter Fallback

> Status: Draft · Serves: [RFC-43](rfc-43-release-proving.md) (the `workspace-execute-two-projects` release blocker runs through this seam) · Complements: [RFC-44](rfc-44-architecture-seams.md) F2/R2 (the `--plan-dir` seam as a worked control-flow-migration precedent)
> Provenance: 2026-06-11. The fallback landed mid-run as the fix for a live `adapter-not-found` miss in the `shop-backend` slot; its integration test was deleted unverified during the same day's CI-stabilisation wrap-up, and the run was parked before the failing extract was re-executed.

## Abstract

`specify source extract` (and the rest of the source-operation family behind the shared prep seam) now resolves a source adapter project-locally first and, on `adapter-not-found` under an active `--plan-dir` override, retries against the plan root. The behavior is live in [`source/prep.rs`](https://github.com/augentic/specify-cli/blob/main/src/runtime/commands/source/prep.rs) but has never executed anywhere — not in a test (the one covering it was removed unverified), not in a live run (the workspace run was parked at exactly this step) — and the committed decision record still promises the opposite ("Adapter resolution is untouched"). This RFC decides to keep the fallback, restores its integration proof as a positive/negative test pair, and sweeps the contradicting prose in both repos in the same change.

## Motivation — findings

**F1 — an unproven arm guards a release blocker.** The `workspace-execute-two-projects` scenario (gate tier: `release-blocker`) routes refine work into slots that, by design, carry no plan and no source adapters — the workspace owns `plan.yaml.sources` and vendors the adapters those bindings name. The live run hit `adapter-not-found` at the `oauth-token-exchange` extract, the fallback was written, and the run was parked before the extract was re-run. The restored test `adapter_falls_back_to_plan_root` was deleted before ever passing, so the only path the blocking scenario has through its first slot-side extract is code that has never been observed working.

**F2 — the fallback is coupled to a string discriminant.** The retry arm matches `Error::Diag { code: "adapter-not-found", .. }`. Renaming that wire id (or migrating the miss to a typed variant) silently disables the fallback with no compiler complaint; only a test that stages the miss can notice. The surviving `prepare_resolves_via_plan_dir` test deliberately stages the adapter slot-locally, so it pins the plan-file and relative-`path:` halves of the `--plan-dir` contract while never reaching this arm.

**F3 — the prose contract points the other way.** [`DECISIONS.md` §"Plan-root override"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#plan-root-override-global---plan-dir-env-specify_plan_dir) states "Adapter resolution is untouched — slot-side source adapters resolve project-locally", and the parent [`AGENTS.md` gotcha](../AGENTS.md#gotchas) says resolution is "project-local only … with no environment-variable fallback". Both predate the fallback. As written, a future cleanup honoring the recorded decision could delete `resolve_with_plan_fallback` and nothing would go red.

## Decision

The fallback stands. The workspace owns the plan's source bindings, so it coherently owns the adapters those bindings name; requiring every slot to duplicate the workspace's `adapters/sources/` tree would push vendoring churn into each project repo for adapters only the plan layer cares about. The resolution order is unchanged and closed: manifest cache → project-local vendored tree → plan root, with only the `adapter-not-found` miss falling through (schema violations and axis collisions surface from whichever root actually carried the manifest). What changes is that the contract becomes *proven and recorded* instead of implicit.

## Proposal

**R1 — restore the integration proof as a pair.** Both tests live in `tests/source/extract.rs` beside `prepare_resolves_via_plan_dir`, exercising the seam end-to-end through the real binary:

- `adapter_falls_back_to_plan_root` (positive): init a project with no local adapter; stage a workspace tempdir carrying `plan.yaml` (a `legacy` source bound to adapter `typescript`, `path: vendor/legacy`) and the vendored `adapters/sources/typescript/{adapter.yaml,briefs/extract.md}`; run

```bash
specify --format json --plan-dir <workspace> source extract legacy user-registration --slice identity
```

  and assert success, `briefs-dir` inside the workspace adapter tree (the envelope key — not the `brief` guess the deleted version probed first), `source-dir` = `<workspace>/vendor/legacy`, and slot-anchored `evidence-dir` / scratch (outputs never follow the fallback).
- `adapter_miss_without_plan_dir_stays_fatal` (negative): identical staging, no `--plan-dir`; assert failure with `adapter-not-found`. This pins the no-fallback condition and, together with the positive twin, the string discriminant from F2 — if the wire id drifts, one of the pair goes red.

No new test infrastructure: both reuse `Project::init()`, the in-repo typescript adapter fixture, and `specify_cmd()`'s env isolation. One extract-level pair suffices for the family — survey and extract share the single `prep::prepare` seam, and preview passes no plan root by construction.

**R2 — reconcile the prose, same change, both repos.** Amend the `DECISIONS.md` plan-root-override bullet from "Adapter resolution is untouched" to state the miss-only fallback, and add the matching sentence to §"Adapter loader axis routing" (the loader itself stays per-axis and project-local; the fallback is a second *probe location* owned by the source-prep seam, not an environment-variable escape hatch). Update the parent `AGENTS.md` gotcha to carry the same nuance. CLI repo rule 5 applies: `rg` both repos for "project-local only" and sweep every hit in the one PR.

**R3 — close the live loop with the parked run.** When the parked `workspace-execute-two-projects` sandbox resumes (RFC-43's remaining blocker), its first action is the `oauth-token-exchange` extract that motivated the fallback — the run record should cite the resolved adapter root as evidence that the seam executed live, complementing R1's hermetic proof.

## Execution plan

| Phase | Deliverable | Repo(s) | Effort | Depends |
| --- | --- | --- | --- | --- |
| 1 | R1 test pair; green under `cargo make ci` | specify-cli | S | none |
| 2 | R2 prose sweep (DECISIONS two sites, parent gotcha, any further `rg` hits) | both | S | 1 (land together or after) |
| 3 | R3 live citation in the resumed run record | specify | S | RFC-43 run resumption |

## Non-Goals

- **No target-axis fallback.** `TargetAdapter::resolve` is untouched; build/merge phases run slot-local against the slice's recorded target, and no live miss has demonstrated a need. Axis symmetry waits for evidence.
- **No sync-time adapter mirroring.** The considered alternative — `workspace sync` copying workspace `adapters/sources/` into each slot's manifest cache — would make the fallback redundant but turns sync into an adapter distributor with staleness semantics. Rejected for now; recorded here so the position is citable.
- **No typed-error refactor.** Migrating `adapter-not-found` from a string discriminant stays under the existing error-variant budget rule (typed variant after ≥3 identical call sites); R1's tests make the current coupling safe rather than redesigning it.
- **No resolution-order changes.** Cache → vendored → plan-root-on-miss is the whole surface; no environment-variable fallback returns.

## Open Questions

1. **Does the fallback consult the plan root's manifest cache, or only its vendored tree?** `SourceAdapter::resolve(name, plan_dir)` runs the standard probe order against the plan root, so both — worth a sentence in the DECISIONS amendment confirming that is intended rather than incidental.
2. **Should `specify source resolve` surface which root won?** A `resolved-from: project | plan-root` field in the resolve envelope would make slot debugging cheaper and give R1's tests a sharper assertion than path-prefix matching. Cheap if done while the seam is open.

## References

- [`src/runtime/commands/source/prep.rs`](https://github.com/augentic/specify-cli/blob/main/src/runtime/commands/source/prep.rs) — `resolve_with_plan_fallback`, the code under proof.
- [`tests/source/extract.rs`](https://github.com/augentic/specify-cli/blob/main/tests/source/extract.rs) — `prepare_resolves_via_plan_dir`, the surviving half of the `--plan-dir` contract and the staging pattern R1 mirrors.
- [`DECISIONS.md` §"Plan-root override"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#plan-root-override-global---plan-dir-env-specify_plan_dir) and §["Adapter loader axis routing"](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md#adapter-loader-axis-routing) — the two prose sites R2 amends.
- [`AGENTS.md` §Gotchas](../AGENTS.md#gotchas) — the parent-repo "project-local only" line R2 nuances.
- [workspace-routing.md](../plugins/spec/skills/execute/references/workspace-routing.md) — the slot choreography that makes the slot adapter-less by design.
- [RFC-43](rfc-43-release-proving.md) — the release-blocker scenario whose resumption is R3's vehicle.
