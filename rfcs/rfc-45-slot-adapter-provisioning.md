# RFC-45: Slot Adapter Provisioning via Workspace Sync

> Status: Accepted · Serves: the eval catalog's [`workspace-execute-two-projects`](../evals/scenarios/workspace-execute-two-projects.md) release blocker · Complements: [RFC-44](rfc-44-architecture-seams.md) R2 (CLI-owned control flow; the `--plan-dir` seam as worked precedent)
> Provenance: 2026-06-11, revised the same day. The first draft of this RFC ratified a resolve-time plan-root fallback written mid-run; review flipped the decision to sync-time provisioning. The interim fallback was implemented and then removed in specify-cli `acceptance` (added `0e55a633`, removed `204e3867`) — the loader contract was never amended. Revised again the same day at implementation: the two open questions resolved into the Decision (per-name GC, cross-axis skip-at-mirror-time, local-slot inclusion) and R1–R3 landed.

## Abstract

Workspace routing runs phase work inside materialised slots that, by design, carry no plan and no adapters — the workspace owns `plan.yaml.sources` and vendors the adapters those bindings name. The live `workspace-execute-two-projects` run hit this on **both axes**: `source extract` failed with `adapter-not-found` in the slot, and the vectis target tool was invisible until its `tools.yaml` was hand-staged into the slot's manifest cache. This RFC keeps the adapter loader exactly as recorded — resolution is project-local only — and makes `specify workspace sync` provision each slot's manifest cache with the workspace's adapter set (both axes, including tool sidecars). The hand-staging trick that unblocked the live run becomes the mechanism, owned by the verb that already owns slot freshness.

## Motivation — findings

**F1 — slots cannot see plan-bound adapters, and a release blocker runs through the gap.** Slot-side `source extract` resolves adapters against the slot only (manifest cache, then vendored tree). The workspace's `documentation` adapter is in neither, so the blocking scenario's first slot-side extract fails. The matching target-axis gap (vectis `tools.yaml`) was bridged manually during the run by copying into the slot's cache — proof that cache provisioning suffices; it just has no owner.

**F2 — the interim fix was the wrong shape.** The mid-run fallback (`resolve_with_plan_fallback`: project-local miss → retry against `--plan-dir`) worked but had three flaws: it contradicted the recorded "resolution is project-local only" loader decision (two DECISIONS sites plus the parent `AGENTS.md` gotcha would have needed amending); it keyed on the string discriminant `adapter-not-found`, so a renamed wire id would silently disable it; and it covered only the source axis — targets would have needed a second, asymmetric fallback. It shipped unexercised (no test, run parked before re-execution) and has been removed.

**F3 — sync already stands at the right seam.** The `/spec:execute` choreography runs `specify workspace sync <project>` before each slice's phase work (journaled as `workspace.sync.completed`), and sync's contract is already "make the slot current for phase work". Adapter provisioning rides an existing, already-journaled step — freshness comes for free at exactly the moment it matters.

## Decision

`specify workspace sync` mirrors the workspace's adapter set — `adapters/sources/*` and `adapters/targets/*`, vendored tree and manifest-cache mirror alike, including `tools.yaml` sidecars — into each synced slot's `.specify/cache/manifests/{sources,targets}/`. The adapter loader is untouched: slot resolution remains project-local only, and the mirrored copies land in a probe location the loader already consults. Mirroring is unconditional (the whole workspace adapter set, not just plan-bound names): no plan parsing in sync, and the cache is gitignored so slots carry no repo residue. Staleness keeps its existing answer — re-run sync — and the per-slice sync in the execute loop makes that automatic in practice.

Three postures, resolved at implementation review (formerly the open questions):

1. **Mirror GC is per-name delete-then-copy; foreign cache entries are never pruned.** Each workspace-owned name is removed and re-copied per sync, so re-sync refreshes. Names the workspace does not own — e.g. a slot's init-time greenfield adapter seed — are left alone, because the slot cache has a second legitimate writer (`specify init`) and a per-axis wipe could delete an adapter only the slot has. Orphans from a renamed workspace adapter are benign: gitignored, unreferenced once `plan.yaml.sources` stops naming them.
2. **Slot-vendored names are skipped at mirror time, cross-axis.** The loader probes the manifest cache *before* the vendored tree, so "vendored wins" cannot come from probe order — the mirror instead skips any name the slot vendors under its own `adapters/` tree on either axis. Same-axis: the slot copy keeps winning resolution. Opposite-axis: the mirror can never manufacture an `adapter-name-axis-collision` in a previously healthy slot.
3. **Local symlink slots are mirrored too.** Unlike the contracts distribution (which skips local slots), the adapter gap is slot-side resolution regardless of slot backing, and the write lands only under the peer's gitignored `.specify/cache/`. Two exceptions: a `url: .` self-slot is skipped (mirroring the workspace onto itself would copy the cache over itself), and a peer without `.specify/` is skipped, never scaffolded.

## Proposal

**R1 — implement the mirror in sync.** Extend the slot-materialisation path in `specify workspace sync` (specify-cli, `crates/workflow` workspace module) to copy the workspace's adapter trees into the slot manifest cache, overwriting prior mirrored copies so re-sync refreshes. Journal payload unchanged (`workspace.sync.completed` already names the synced slots).

**R2 — prove it at both seams.** A sync-level suite asserting the mirror lands (workspace adapter visible in the slot cache after sync, refreshed on re-sync) plus the conflict pins from the Decision (a slot-vendored same-axis adapter is not shadowed, an opposite-axis vendored name is not mirrored, foreign cache entries survive, the self-slot is skipped), plus one end-to-end slot-extract integration test beside `prepare_resolves_via_plan_dir` in `tests/source/extract.rs`: adapter vendored only at the workspace, slot extract succeeds through ordinary cache resolution after sync. No new resolution semantics to pin — the string-discriminant fragility from F2 no longer exists.

**R3 — docs, one additive line each.** Sync's responsibility list in `docs/standards/workflow.md` and the workspace DECISIONS entry gain the mirror; [workspace-routing.md](../plugins/spec/skills/execute/references/workspace-routing.md) notes that adapters arrive in the slot via sync. The loader prose ("resolution is project-local only", "`--plan-dir` … adapter resolution is untouched") stays true verbatim and needs no edits.

**R4 — close the live loop.** Resume the parked `workspace-execute-two-projects` sandbox; with R1 landed, the resumed run needs no manual cache-stage — sync provisions the slot. The run record cites the slot-resolved adapter root as live evidence; on a green run, flip the catalog row and the RM-05 rollup.

## Wrap-up actions

The full action list this RFC inherits from the 2026-06-11 session, for one-pass execution:

| # | Action | Repo | Status |
| --- | --- | --- | --- |
| 1 | Back out parallel vectis changes (superseded by the vectis work on `main`, PR #64) | specify-cli | done — `cd2d892a` |
| 2 | Remove the interim resolve-time fallback | specify-cli | done — `204e3867` |
| 3 | Fix the two `wire.rs` clippy findings breaking remote CI (`too_long_first_doc_paragraph`, `map_unwrap_or`) and push `acceptance` green | specify-cli | this change |
| 4 | Land this RFC (rename from `rfc-45-plan-root-adapter-fallback.md`; the first draft's decision is superseded) | specify | this change |
| 5 | R1 sync mirror + R2 tests + R3 doc lines | specify-cli (+ specify docs) | this change |
| 6 | Verify the `main`-side vectis work covers the four live-run fixes (scaffold `.gitignore` merge; `verify` reading `.specify/project.yaml`; composition skip for core-only projects; tool clippy nits) — re-apply atop `main`'s vectis where not | specify-cli | next |
| 7 | Resume the parked run per R4; file `evals/runs/workspace-execute-two-projects.<result>.md`; flip catalog + RM-05 on green | specify | next |
| 8 | Reconcile `acceptance` with `main` (diverged at `ffaa0aa3`; the vectis back-out makes that subtree conflict-free) before release | both | next |
| 9 | `plan-lock.md`: document zsh's `zsystem flock` as the stock-macOS fallback ahead of the Python `fcntl` snippet | specify | next |
| 10 | Run-record hygiene: `execute-build-failure.pass.md` "Retained at" pointer is stale (sandbox pruned); note pruning or restore the snapshot | specify | minor |

## Non-Goals

- **No loader changes.** No resolve-time fallback (the removed interim is the recorded counter-example), no environment-variable escape hatch, no second probe root. Resolution stays project-local only.
- **No plan-aware filtering in sync.** Mirroring the whole adapter set is simpler than parsing `plan.yaml` for bound names; the cache is gitignored and small.
- **No sync-time tool fetching.** The mirror copies what the workspace already has; resolving and caching remote adapters stays an init/CLI concern.
- **The rejected alternative, recorded:** the lazy resolve-time fallback's one advantage — picking up a workspace adapter edited mid-slice without re-sync — does not outweigh contradicting the loader contract, the string-discriminant coupling, and its single-axis coverage. Re-sync is the staleness remedy everywhere else in workspace mode; adapters are not special.

## References

- [`workspace-execute-two-projects`](../evals/scenarios/workspace-execute-two-projects.md) — the release-blocker scenario whose resumption is R4's vehicle; the eval catalog and run-record contract live under [evals/](../evals/README.md).
- [workspace-routing.md](../plugins/spec/skills/execute/references/workspace-routing.md) — the slot choreography that makes slots adapter-less by design and runs sync before each slice.
- [specify-cli `DECISIONS.md`](https://github.com/augentic/specify-cli/blob/main/DECISIONS.md) — §"Adapter loader axis routing" (the contract this RFC preserves) and §"Plan-root override: global `--plan-dir`" (the plan-file seam this RFC leaves untouched).
- [`tests/source/extract.rs`](https://github.com/augentic/specify-cli/blob/main/tests/source/extract.rs) — `prepare_resolves_via_plan_dir`, the surviving `--plan-dir` proof R2's slot-extract test sits beside.
- [RFC-44](rfc-44-architecture-seams.md) — R2's migrate-control-flow-into-verbs direction; this RFC follows the same instinct at the provisioning seam.
